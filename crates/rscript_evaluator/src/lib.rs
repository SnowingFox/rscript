//! rscript_evaluator: Constant expression evaluation.
//!
//! Evaluates constant expressions at compile time (enum member values,
//! const assertions, etc.).

/// Evaluate a constant numeric expression.
pub fn evaluate_constant_numeric_expression(_text: &str) -> Option<f64> {
    // TODO: Implement constant folding
    None
}

/// The result of evaluating a constant expression.
#[derive(Debug, Clone)]
pub enum ConstantValue {
    Number(f64),
    String(String),
    Boolean(bool),
    Undefined,
}
