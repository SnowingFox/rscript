//! rscript_evaluator: Constant expression evaluation.
//!
//! Evaluates constant expressions at compile time (enum member values,
//! const assertions, etc.).

use rscript_ast::node::Expression;
use rscript_ast::syntax_kind::SyntaxKind;

/// Evaluate a constant numeric expression.
pub fn evaluate_constant_numeric_expression(expr: &Expression<'_>) -> Option<f64> {
    match expr {
        Expression::NumericLiteral(n) => {
            n.text_name.parse().ok()
        }
        Expression::Binary(binary) => {
            let left_val = evaluate_constant_numeric_expression(binary.left)?;
            let right_val = evaluate_constant_numeric_expression(binary.right)?;
            let operator = binary.operator_token.data.kind;
            
            match operator {
                SyntaxKind::PlusToken => Some(left_val + right_val),
                SyntaxKind::MinusToken => Some(left_val - right_val),
                SyntaxKind::AsteriskToken => Some(left_val * right_val),
                SyntaxKind::SlashToken => {
                    if right_val == 0.0 {
                        None // Division by zero
                    } else {
                        Some(left_val / right_val)
                    }
                }
                SyntaxKind::PercentToken => {
                    if right_val == 0.0 {
                        None // Modulo by zero
                    } else {
                        Some(left_val % right_val)
                    }
                }
                SyntaxKind::AsteriskAsteriskToken => Some(left_val.powf(right_val)),
                _ => None, // Not a numeric operator
            }
        }
        Expression::PrefixUnary(unary) => {
            let operand_val = evaluate_constant_numeric_expression(unary.operand)?;
            match unary.operator {
                SyntaxKind::PlusToken => Some(operand_val),
                SyntaxKind::MinusToken => Some(-operand_val),
                _ => None, // Not a numeric unary operator
            }
        }
        Expression::Parenthesized(paren) => {
            evaluate_constant_numeric_expression(paren.expression)
        }
        _ => None, // Not a constant numeric expression
    }
}

/// Evaluate a constant string expression.
pub fn evaluate_constant_string_expression(expr: &Expression<'_>) -> Option<String> {
    match expr {
        Expression::StringLiteral(s) => {
            Some(s.text_name.clone())
        }
        Expression::Binary(binary) => {
            // Only support + for string concatenation
            if binary.operator_token.data.kind == SyntaxKind::PlusToken {
                let left_val = evaluate_constant_string_expression(binary.left)?;
                let right_val = evaluate_constant_string_expression(binary.right)?;
                Some(format!("{}{}", left_val, right_val))
            } else {
                None
            }
        }
        Expression::Parenthesized(paren) => {
            evaluate_constant_string_expression(paren.expression)
        }
        _ => None, // Not a constant string expression
    }
}

/// The result of evaluating a constant expression.
#[derive(Debug, Clone)]
pub enum ConstantValue {
    Number(f64),
    String(String),
    Boolean(bool),
    Undefined,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rscript_ast::node::*;
    use rscript_ast::syntax_kind::SyntaxKind;

    fn create_numeric_literal(value: &str) -> Expression<'static> {
        Expression::NumericLiteral(NumericLiteral {
            data: NodeData::new(SyntaxKind::NumericLiteral, 0, value.len() as u32),
            text: rscript_core::intern::InternedString::dummy(),
            text_name: value.to_string(),
            numeric_literal_flags: rscript_ast::types::TokenFlags::NONE,
        })
    }

    fn create_string_literal(value: &str) -> Expression<'static> {
        Expression::StringLiteral(StringLiteral {
            data: NodeData::new(SyntaxKind::StringLiteral, 0, value.len() as u32),
            text: rscript_core::intern::InternedString::dummy(),
            text_name: value.to_string(),
            is_single_quote: false,
        })
    }

    fn create_binary_expression(left: Expression<'static>, op: SyntaxKind, right: Expression<'static>) -> Expression<'static> {
        // We need to allocate these in an arena, but for tests we'll use a simplified approach
        // In real usage, these would be arena-allocated
        Expression::Binary(BinaryExpression {
            data: NodeData::new(SyntaxKind::BinaryExpression, 0, 0),
            left: Box::leak(Box::new(left)),
            operator_token: Token::new(op, 0, 0),
            right: Box::leak(Box::new(right)),
        })
    }

    fn create_unary_expression(op: SyntaxKind, operand: Expression<'static>) -> Expression<'static> {
        Expression::PrefixUnary(PrefixUnaryExpression {
            data: NodeData::new(SyntaxKind::PrefixUnaryExpression, 0, 0),
            operator: op,
            operand: Box::leak(Box::new(operand)),
        })
    }

    fn create_parenthesized(expr: Expression<'static>) -> Expression<'static> {
        Expression::Parenthesized(ParenthesizedExpression {
            data: NodeData::new(SyntaxKind::ParenthesizedExpression, 0, 0),
            expression: Box::leak(Box::new(expr)),
        })
    }

    #[test]
    fn test_numeric_literal() {
        let expr = create_numeric_literal("42");
        assert_eq!(evaluate_constant_numeric_expression(&expr), Some(42.0));
        
        let expr = create_numeric_literal("3.14");
        assert_eq!(evaluate_constant_numeric_expression(&expr), Some(3.14));
        
        let expr = create_numeric_literal("-10");
        assert_eq!(evaluate_constant_numeric_expression(&expr), Some(-10.0));
    }

    #[test]
    fn test_binary_addition() {
        let left = create_numeric_literal("10");
        let right = create_numeric_literal("20");
        let expr = create_binary_expression(left, SyntaxKind::PlusToken, right);
        assert_eq!(evaluate_constant_numeric_expression(&expr), Some(30.0));
    }

    #[test]
    fn test_binary_subtraction() {
        let left = create_numeric_literal("20");
        let right = create_numeric_literal("10");
        let expr = create_binary_expression(left, SyntaxKind::MinusToken, right);
        assert_eq!(evaluate_constant_numeric_expression(&expr), Some(10.0));
    }

    #[test]
    fn test_binary_multiplication() {
        let left = create_numeric_literal("6");
        let right = create_numeric_literal("7");
        let expr = create_binary_expression(left, SyntaxKind::AsteriskToken, right);
        assert_eq!(evaluate_constant_numeric_expression(&expr), Some(42.0));
    }

    #[test]
    fn test_binary_division() {
        let left = create_numeric_literal("20");
        let right = create_numeric_literal("4");
        let expr = create_binary_expression(left, SyntaxKind::SlashToken, right);
        assert_eq!(evaluate_constant_numeric_expression(&expr), Some(5.0));
    }

    #[test]
    fn test_binary_modulo() {
        let left = create_numeric_literal("17");
        let right = create_numeric_literal("5");
        let expr = create_binary_expression(left, SyntaxKind::PercentToken, right);
        assert_eq!(evaluate_constant_numeric_expression(&expr), Some(2.0));
    }

    #[test]
    fn test_binary_exponentiation() {
        let left = create_numeric_literal("2");
        let right = create_numeric_literal("8");
        let expr = create_binary_expression(left, SyntaxKind::AsteriskAsteriskToken, right);
        assert_eq!(evaluate_constant_numeric_expression(&expr), Some(256.0));
    }

    #[test]
    fn test_unary_plus() {
        let operand = create_numeric_literal("42");
        let expr = create_unary_expression(SyntaxKind::PlusToken, operand);
        assert_eq!(evaluate_constant_numeric_expression(&expr), Some(42.0));
    }

    #[test]
    fn test_unary_minus() {
        let operand = create_numeric_literal("42");
        let expr = create_unary_expression(SyntaxKind::MinusToken, operand);
        assert_eq!(evaluate_constant_numeric_expression(&expr), Some(-42.0));
    }

    #[test]
    fn test_nested_expressions() {
        // (10 + 20) * 2
        let inner_left = create_numeric_literal("10");
        let inner_right = create_numeric_literal("20");
        let inner = create_binary_expression(inner_left, SyntaxKind::PlusToken, inner_right);
        let outer_right = create_numeric_literal("2");
        let expr = create_binary_expression(inner, SyntaxKind::AsteriskToken, outer_right);
        assert_eq!(evaluate_constant_numeric_expression(&expr), Some(60.0));
    }

    #[test]
    fn test_parenthesized_expression() {
        let inner = create_numeric_literal("42");
        let expr = create_parenthesized(inner);
        assert_eq!(evaluate_constant_numeric_expression(&expr), Some(42.0));
    }

    #[test]
    fn test_string_literal() {
        let expr = create_string_literal("hello");
        assert_eq!(evaluate_constant_string_expression(&expr), Some("hello".to_string()));
    }

    #[test]
    fn test_string_concatenation() {
        let left = create_string_literal("hello");
        let right = create_string_literal("world");
        let expr = create_binary_expression(left, SyntaxKind::PlusToken, right);
        assert_eq!(evaluate_constant_string_expression(&expr), Some("helloworld".to_string()));
    }

    #[test]
    fn test_string_parenthesized() {
        let inner = create_string_literal("test");
        let expr = create_parenthesized(inner);
        assert_eq!(evaluate_constant_string_expression(&expr), Some("test".to_string()));
    }
}
