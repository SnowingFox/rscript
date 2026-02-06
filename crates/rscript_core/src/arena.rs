//! Arena allocation for the compiler.
//!
//! All AST nodes and types are allocated from a bump arena to minimize
//! allocation overhead and improve cache locality.

use bumpalo::Bump;

/// The compiler arena wraps a bump allocator for all compiler allocations.
///
/// All AST nodes, type objects, and other compilation artifacts are allocated
/// from this arena. When the compilation is done, the entire arena is freed
/// at once (O(1) deallocation).
pub struct CompilerArena {
    bump: Bump,
}

impl CompilerArena {
    /// Create a new compiler arena with default capacity.
    pub fn new() -> Self {
        Self {
            bump: Bump::new(),
        }
    }

    /// Create a new compiler arena with the specified initial capacity in bytes.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bump: Bump::with_capacity(capacity),
        }
    }

    /// Get a reference to the underlying bump allocator.
    #[inline]
    pub fn bump(&self) -> &Bump {
        &self.bump
    }

    /// Allocate a value in the arena and return a reference to it.
    #[inline]
    pub fn alloc<T>(&self, val: T) -> &T {
        self.bump.alloc(val)
    }

    /// Allocate a value in the arena and return a mutable reference to it.
    #[inline]
    pub fn alloc_mut<T>(&self, val: T) -> &mut T {
        self.bump.alloc(val)
    }

    /// Allocate a string slice in the arena.
    #[inline]
    pub fn alloc_str(&self, s: &str) -> &str {
        self.bump.alloc_str(s)
    }

    /// Allocate a slice from an iterator in the arena.
    #[inline]
    pub fn alloc_slice_copy<T: Copy>(&self, src: &[T]) -> &[T] {
        self.bump.alloc_slice_copy(src)
    }

    /// Returns the total bytes allocated in this arena.
    pub fn allocated_bytes(&self) -> usize {
        self.bump.allocated_bytes()
    }

    /// Reset the arena, deallocating all objects but keeping the memory.
    pub fn reset(&mut self) {
        self.bump.reset();
    }
}

impl Default for CompilerArena {
    fn default() -> Self {
        Self::new()
    }
}
