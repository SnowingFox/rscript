//! rscript_ast: Abstract Syntax Tree definitions for the TypeScript compiler.
//!
//! This module defines all AST node types, the SyntaxKind enum, and associated
//! flag types. The design faithfully mirrors TypeScript's AST structure.

pub mod generated;
pub mod node;
pub mod syntax_kind;
pub mod types;
pub mod visitor;

// Re-export key types
pub use node::*;
pub use syntax_kind::SyntaxKind;
pub use types::*;
