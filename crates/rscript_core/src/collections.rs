//! Custom collection types used throughout the compiler.

use rustc_hash::FxHashMap;
use std::hash::Hash;

/// An ordered map that preserves insertion order.
/// Used where TypeScript uses `Map` (which preserves insertion order in JS).
#[derive(Debug, Clone)]
pub struct OrderedMap<K, V> {
    entries: Vec<(K, V)>,
    index: FxHashMap<K, usize>,
}

impl<K: Eq + Hash + Clone, V> OrderedMap<K, V> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            index: FxHashMap::default(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            index: FxHashMap::with_capacity_and_hasher(capacity, Default::default()),
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if let Some(&idx) = self.index.get(&key) {
            let old = std::mem::replace(&mut self.entries[idx].1, value);
            Some(old)
        } else {
            let idx = self.entries.len();
            self.index.insert(key.clone(), idx);
            self.entries.push((key, value));
            None
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.index.get(key).map(|&idx| &self.entries[idx].1)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.index
            .get(key)
            .copied()
            .map(move |idx| &mut self.entries[idx].1)
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.index.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.entries.iter().map(|(k, _)| k)
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entries.iter().map(|(_, v)| v)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.index.clear();
    }
}

impl<K: Eq + Hash + Clone, V> Default for OrderedMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

/// A multimap that stores multiple values per key.
/// Used where TypeScript uses `Map<K, V[]>`.
#[derive(Debug, Clone)]
pub struct MultiMap<K, V> {
    map: FxHashMap<K, Vec<V>>,
}

impl<K: Eq + Hash, V> MultiMap<K, V> {
    pub fn new() -> Self {
        Self {
            map: FxHashMap::default(),
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        self.map.entry(key).or_default().push(value);
    }

    pub fn get(&self, key: &K) -> Option<&[V]> {
        self.map.get(key).map(|v| v.as_slice())
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &[V])> {
        self.map.iter().map(|(k, v)| (k, v.as_slice()))
    }
}

impl<K: Eq + Hash, V> Default for MultiMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

/// A set that uses FxHash for fast hashing, suitable for compiler internals
/// where DoS resistance is not needed.
pub type FxHashSet<T> = rustc_hash::FxHashSet<T>;

/// Re-export FxHashMap for convenience.
pub type FxMap<K, V> = FxHashMap<K, V>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ordered_map_preserves_order() {
        let mut map = OrderedMap::new();
        map.insert("c", 3);
        map.insert("a", 1);
        map.insert("b", 2);

        let keys: Vec<_> = map.keys().copied().collect();
        assert_eq!(keys, vec!["c", "a", "b"]);
    }

    #[test]
    fn test_ordered_map_update() {
        let mut map = OrderedMap::new();
        map.insert("a", 1);
        let old = map.insert("a", 2);
        assert_eq!(old, Some(1));
        assert_eq!(map.get(&"a"), Some(&2));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_multi_map() {
        let mut map = MultiMap::new();
        map.insert("key", 1);
        map.insert("key", 2);
        map.insert("key", 3);
        assert_eq!(map.get(&"key"), Some(&[1, 2, 3][..]));
        assert_eq!(map.get(&"other"), None);
    }
}
