//! Operator precedence for binary and unary operators.

use rscript_ast::syntax_kind::SyntaxKind;

/// Operator precedence levels, matching TypeScript's precedence table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
#[allow(dead_code)]
pub enum OperatorPrecedence {
    // Ranges from lowest to highest
    Comma = 0,
    Spread = 1,
    Yield = 2,
    Assignment = 3,
    Conditional = 4,
    NullishCoalescing = 5,
    LogicalOr = 6,
    LogicalAnd = 7,
    BitwiseOr = 8,
    BitwiseXor = 9,
    BitwiseAnd = 10,
    Equality = 11,
    Relational = 12,
    Shift = 13,
    Additive = 14,
    Multiplicative = 15,
    Exponentiation = 16,
    Unary = 17,
    Update = 18,
    LeftHandSide = 19,
    Member = 20,
    Primary = 21,
    Highest = 22,
    Invalid = 255,
}

/// Get the binary operator precedence for a given token kind.
pub fn get_binary_operator_precedence(kind: SyntaxKind) -> OperatorPrecedence {
    match kind {
        SyntaxKind::QuestionQuestionToken => OperatorPrecedence::NullishCoalescing,
        SyntaxKind::BarBarToken => OperatorPrecedence::LogicalOr,
        SyntaxKind::AmpersandAmpersandToken => OperatorPrecedence::LogicalAnd,
        SyntaxKind::BarToken => OperatorPrecedence::BitwiseOr,
        SyntaxKind::CaretToken => OperatorPrecedence::BitwiseXor,
        SyntaxKind::AmpersandToken => OperatorPrecedence::BitwiseAnd,
        SyntaxKind::EqualsEqualsToken
        | SyntaxKind::ExclamationEqualsToken
        | SyntaxKind::EqualsEqualsEqualsToken
        | SyntaxKind::ExclamationEqualsEqualsToken => OperatorPrecedence::Equality,
        SyntaxKind::LessThanToken
        | SyntaxKind::GreaterThanToken
        | SyntaxKind::LessThanEqualsToken
        | SyntaxKind::GreaterThanEqualsToken
        | SyntaxKind::InstanceOfKeyword
        | SyntaxKind::InKeyword
        | SyntaxKind::AsKeyword
        | SyntaxKind::SatisfiesKeyword => OperatorPrecedence::Relational,
        SyntaxKind::LessThanLessThanToken
        | SyntaxKind::GreaterThanGreaterThanToken
        | SyntaxKind::GreaterThanGreaterThanGreaterThanToken => OperatorPrecedence::Shift,
        SyntaxKind::PlusToken | SyntaxKind::MinusToken => OperatorPrecedence::Additive,
        SyntaxKind::AsteriskToken | SyntaxKind::SlashToken | SyntaxKind::PercentToken => {
            OperatorPrecedence::Multiplicative
        }
        SyntaxKind::AsteriskAsteriskToken => OperatorPrecedence::Exponentiation,
        _ => OperatorPrecedence::Invalid,
    }
}
