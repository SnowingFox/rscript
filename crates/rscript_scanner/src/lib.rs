//! rscript_scanner: Lexer/tokenizer for TypeScript source code.
//!
//! This is a faithful port of TypeScript's scanner.ts, producing tokens
//! from source text with full support for:
//! - All JavaScript/TypeScript token types
//! - Template literals
//! - JSX tokens
//! - Regular expression literals
//! - Unicode identifiers

mod char_codes;
mod scanner;
mod token;

pub use scanner::Scanner;
pub use token::TokenInfo;
