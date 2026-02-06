//! rscript_core: Core utilities for the rscript TypeScript compiler.
//!
//! Provides fundamental types, string interning, text spans, and collections
//! used throughout the compiler pipeline.

pub mod arena;
pub mod collections;
pub mod intern;
pub mod text;

// Re-export commonly used types
pub use arena::CompilerArena;
pub use intern::{InternedString, StringInterner};
pub use text::{TextRange, TextSpan};
