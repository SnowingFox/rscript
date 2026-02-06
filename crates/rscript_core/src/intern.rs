//! String interning for the compiler.
//!
//! All identifiers, keywords, and frequently used strings are interned
//! to enable O(1) comparison via integer IDs.

use lasso::{Key, Spur, ThreadedRodeo};
use std::fmt;
use std::sync::Arc;

/// An interned string identifier. This is a lightweight handle (u32)
/// that can be used to look up the actual string content.
///
/// Comparing two `InternedString` values is an O(1) integer comparison.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct InternedString(Spur);

impl InternedString {
    /// Create from a raw lasso key.
    #[inline]
    pub fn from_spur(spur: Spur) -> Self {
        Self(spur)
    }

    /// Get the raw lasso key.
    #[inline]
    pub fn as_spur(self) -> Spur {
        self.0
    }

    /// Create a "dummy" interned string for placeholder purposes.
    /// This should only be used during parsing when the actual string
    /// will be set later or is not needed.
    #[inline]
    pub fn dummy() -> Self {
        // Safety: Spur(NonZeroU32) requires non-zero, use key 1 as dummy
        Self(Spur::try_from_usize(0).unwrap())
    }
}

impl fmt::Debug for InternedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InternedString({:?})", self.0)
    }
}

/// Thread-safe string interner.
///
/// Stores one copy of each unique string and returns lightweight handles.
/// Used for identifiers, keywords, and other frequently compared strings.
#[derive(Clone)]
pub struct StringInterner {
    rodeo: Arc<ThreadedRodeo>,
}

impl StringInterner {
    /// Create a new string interner.
    pub fn new() -> Self {
        Self {
            rodeo: Arc::new(ThreadedRodeo::new()),
        }
    }

    /// Create a new string interner with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            rodeo: Arc::new(ThreadedRodeo::with_capacity(
                lasso::Capacity::new(
                    capacity,
                    std::num::NonZeroUsize::new(capacity * 8).unwrap_or(std::num::NonZeroUsize::new(1).unwrap()),
                ),
            )),
        }
    }

    /// Intern a string, returning a handle to the interned value.
    /// If the string was already interned, returns the existing handle.
    #[inline]
    pub fn intern(&self, s: &str) -> InternedString {
        InternedString::from_spur(self.rodeo.get_or_intern(s))
    }

    /// Intern a static string. More efficient than `intern` for string literals.
    #[inline]
    pub fn intern_static(&self, s: &'static str) -> InternedString {
        InternedString::from_spur(self.rodeo.get_or_intern_static(s))
    }

    /// Look up an already-interned string without interning it if absent.
    #[inline]
    pub fn get(&self, s: &str) -> Option<InternedString> {
        self.rodeo.get(s).map(InternedString::from_spur)
    }

    /// Resolve an interned string handle back to its string content.
    #[inline]
    pub fn resolve(&self, key: InternedString) -> &str {
        self.rodeo.resolve(&key.as_spur())
    }

    /// Returns the number of interned strings.
    pub fn len(&self) -> usize {
        self.rodeo.len()
    }

    /// Returns true if no strings have been interned.
    pub fn is_empty(&self) -> bool {
        self.rodeo.is_empty()
    }
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for StringInterner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StringInterner")
            .field("len", &self.len())
            .finish()
    }
}

/// Display an `InternedString` requires access to the interner.
/// This wrapper provides a Display impl.
pub struct DisplayInterned<'a> {
    pub key: InternedString,
    pub interner: &'a StringInterner,
}

impl<'a> fmt::Display for DisplayInterned<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.interner.resolve(self.key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intern_and_resolve() {
        let interner = StringInterner::new();
        let a = interner.intern("hello");
        let b = interner.intern("hello");
        let c = interner.intern("world");

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(interner.resolve(a), "hello");
        assert_eq!(interner.resolve(c), "world");
    }

    #[test]
    fn test_get() {
        let interner = StringInterner::new();
        assert!(interner.get("hello").is_none());
        let a = interner.intern("hello");
        assert_eq!(interner.get("hello"), Some(a));
    }

    #[test]
    fn test_intern_static() {
        let interner = StringInterner::new();
        let a = interner.intern_static("static_string");
        let b = interner.intern("static_string");
        assert_eq!(a, b);
    }
}
