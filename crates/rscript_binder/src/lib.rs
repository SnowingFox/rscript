//! rscript_binder: Symbol table construction and scope analysis.
//!
//! The binder walks the AST and creates symbols, links declarations,
//! and builds scope chains. It also sets up flow containers for
//! control flow analysis.

mod binder;
mod scope;
mod symbol;

pub use binder::Binder;
pub use symbol::{Symbol, SymbolTable};
