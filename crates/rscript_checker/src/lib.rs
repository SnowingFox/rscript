//! rscript_checker: The TypeScript type checker.
//!
//! This is the heart of the TypeScript compiler - the type checker resolves
//! types, checks type assignability, performs type inference, and reports
//! type errors. This is a faithful port of TypeScript's checker.ts.

mod checker;
mod types;

pub use checker::Checker;
pub use types::{Type, TypeTable};
