//! rscript_transformers: AST transformations.
//!
//! Transforms the AST for various purposes:
//! - ES target downleveling (e.g., async/await -> generators)
//! - JSX transformation
//! - Decorator transformation
//! - TypeScript stripping (remove type annotations for JS emit)

/// A transformer that modifies the AST.
pub trait Transformer {
    /// Transform a source file AST.
    fn transform<'a>(&self, node: &rscript_ast::node::SourceFile<'a>) -> rscript_ast::node::SourceFile<'a>;
}

/// Strip TypeScript-specific syntax for JavaScript emit.
pub struct TypeScriptStripper;

/// Transform JSX to function calls.
pub struct JsxTransformer;

/// Transform decorators.
pub struct DecoratorTransformer;

/// Downlevel ES features to older targets.
pub struct EsDownlevelTransformer;
