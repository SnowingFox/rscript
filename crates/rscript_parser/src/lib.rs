//! rscript_parser: Recursive descent parser for TypeScript.
//!
//! Parses token streams from the scanner into an arena-allocated AST.
//! This is a faithful port of TypeScript's parser.ts.

mod parser;
mod precedence;
mod utilities;

pub use parser::Parser;
