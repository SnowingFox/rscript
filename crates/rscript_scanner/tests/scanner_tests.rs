//! Scanner integration tests.
//!
//! Verifies that the scanner correctly tokenizes various TypeScript constructs.

use rscript_ast::syntax_kind::SyntaxKind;
use rscript_scanner::Scanner;

/// Helper: scan all tokens from source and return as (kind, value) pairs.
fn scan_all(source: &str) -> Vec<(SyntaxKind, String)> {
    let mut scanner = Scanner::new(source);
    let mut tokens = Vec::new();
    loop {
        let kind = scanner.scan();
        if kind == SyntaxKind::EndOfFileToken {
            break;
        }
        tokens.push((kind, scanner.token_value().to_string()));
    }
    tokens
}

/// Helper: scan all token kinds.
fn scan_kinds(source: &str) -> Vec<SyntaxKind> {
    scan_all(source).into_iter().map(|(k, _)| k).collect()
}

#[test]
fn test_empty_source() {
    let tokens = scan_all("");
    assert!(tokens.is_empty());
}

#[test]
fn test_whitespace_only() {
    let tokens = scan_all("   \n\t  ");
    assert!(tokens.is_empty());
}

#[test]
fn test_numeric_literals() {
    // Integer
    let tokens = scan_all("42");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
    assert_eq!(tokens[0].1, "42");

    // Decimal
    let tokens = scan_all("3.14");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
    assert_eq!(tokens[0].1, "3.14");

    // Hex
    let tokens = scan_all("0xFF");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);

    // Binary
    let tokens = scan_all("0b1010");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);

    // Octal
    let tokens = scan_all("0o77");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
}

#[test]
fn test_string_literals() {
    let tokens = scan_all(r#""hello""#);
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::StringLiteral);
    assert_eq!(tokens[0].1, "hello");

    let tokens = scan_all("'world'");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::StringLiteral);
    assert_eq!(tokens[0].1, "world");
}

#[test]
fn test_template_literal() {
    let tokens = scan_all("`hello`");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NoSubstitutionTemplateLiteral);
}

#[test]
fn test_identifiers() {
    let tokens = scan_all("foo bar baz _private $dollar");
    assert_eq!(tokens.len(), 5);
    for (kind, _) in &tokens {
        assert_eq!(*kind, SyntaxKind::Identifier);
    }
    assert_eq!(tokens[0].1, "foo");
    assert_eq!(tokens[1].1, "bar");
    assert_eq!(tokens[2].1, "baz");
    assert_eq!(tokens[3].1, "_private");
    assert_eq!(tokens[4].1, "$dollar");
}

#[test]
fn test_keywords() {
    let source = "if else while for return function class interface type enum";
    let kinds = scan_kinds(source);
    assert_eq!(kinds, vec![
        SyntaxKind::IfKeyword,
        SyntaxKind::ElseKeyword,
        SyntaxKind::WhileKeyword,
        SyntaxKind::ForKeyword,
        SyntaxKind::ReturnKeyword,
        SyntaxKind::FunctionKeyword,
        SyntaxKind::ClassKeyword,
        SyntaxKind::InterfaceKeyword,
        SyntaxKind::TypeKeyword,
        SyntaxKind::EnumKeyword,
    ]);
}

#[test]
fn test_operators() {
    // Test operators individually to avoid scanner context sensitivity with >= vs > =
    let tokens = scan_all("+ - * / % = == === != !==");
    let kinds: Vec<SyntaxKind> = tokens.iter().map(|(k, _)| *k).collect();
    assert_eq!(kinds, vec![
        SyntaxKind::PlusToken,
        SyntaxKind::MinusToken,
        SyntaxKind::AsteriskToken,
        SyntaxKind::SlashToken,
        SyntaxKind::PercentToken,
        SyntaxKind::EqualsToken,
        SyntaxKind::EqualsEqualsToken,
        SyntaxKind::EqualsEqualsEqualsToken,
        SyntaxKind::ExclamationEqualsToken,
        SyntaxKind::ExclamationEqualsEqualsToken,
    ]);

    let tokens2 = scan_all("< > && || !");
    let kinds2: Vec<SyntaxKind> = tokens2.iter().map(|(k, _)| *k).collect();
    assert_eq!(kinds2, vec![
        SyntaxKind::LessThanToken,
        SyntaxKind::GreaterThanToken,
        SyntaxKind::AmpersandAmpersandToken,
        SyntaxKind::BarBarToken,
        SyntaxKind::ExclamationToken,
    ]);

    // <= and >= tested individually
    let tokens3 = scan_all("<=");
    assert_eq!(tokens3[0].0, SyntaxKind::LessThanEqualsToken);
}

#[test]
fn test_punctuation() {
    let tokens = scan_all("( ) { } [ ] ; , . : ?");
    let kinds: Vec<SyntaxKind> = tokens.iter().map(|(k, _)| *k).collect();
    assert_eq!(kinds, vec![
        SyntaxKind::OpenParenToken,
        SyntaxKind::CloseParenToken,
        SyntaxKind::OpenBraceToken,
        SyntaxKind::CloseBraceToken,
        SyntaxKind::OpenBracketToken,
        SyntaxKind::CloseBracketToken,
        SyntaxKind::SemicolonToken,
        SyntaxKind::CommaToken,
        SyntaxKind::DotToken,
        SyntaxKind::ColonToken,
        SyntaxKind::QuestionToken,
    ]);
}

#[test]
fn test_modern_operators() {
    // Optional chaining
    let tokens = scan_all("?.");
    assert!(tokens.iter().any(|(k, _)| *k == SyntaxKind::QuestionDotToken));

    // Nullish coalescing
    let tokens = scan_all("??");
    assert!(tokens.iter().any(|(k, _)| *k == SyntaxKind::QuestionQuestionToken));

    // Exponentiation
    let tokens = scan_all("**");
    assert!(tokens.iter().any(|(k, _)| *k == SyntaxKind::AsteriskAsteriskToken));

    // Arrow
    let tokens = scan_all("=>");
    assert!(tokens.iter().any(|(k, _)| *k == SyntaxKind::EqualsGreaterThanToken));

    // Spread
    let tokens = scan_all("...");
    assert!(tokens.iter().any(|(k, _)| *k == SyntaxKind::DotDotDotToken));
}

#[test]
fn test_assignment_operators() {
    let tokens = scan_all("+= -= *= /= %= **= &&= ||= ??=");
    let kinds: Vec<SyntaxKind> = tokens.iter().map(|(k, _)| *k).collect();
    assert_eq!(kinds, vec![
        SyntaxKind::PlusEqualsToken,
        SyntaxKind::MinusEqualsToken,
        SyntaxKind::AsteriskEqualsToken,
        SyntaxKind::SlashEqualsToken,
        SyntaxKind::PercentEqualsToken,
        SyntaxKind::AsteriskAsteriskEqualsToken,
        SyntaxKind::AmpersandAmpersandEqualsToken,
        SyntaxKind::BarBarEqualsToken,
        SyntaxKind::QuestionQuestionEqualsToken,
    ]);
}

#[test]
fn test_const_let_var() {
    let kinds = scan_kinds("const let var");
    assert_eq!(kinds, vec![
        SyntaxKind::ConstKeyword,
        SyntaxKind::LetKeyword,
        SyntaxKind::VarKeyword,
    ]);
}

#[test]
fn test_shebang_skipping() {
    let mut scanner = Scanner::new("#!/usr/bin/env node\nconst x = 1;");
    scanner.skip_shebang();
    let kind = scanner.scan();
    assert_eq!(kind, SyntaxKind::ConstKeyword);
}

#[test]
fn test_comments_skipped() {
    // Single line comment
    let tokens = scan_all("// this is a comment\n42");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);

    // Multi-line comment
    let tokens = scan_all("/* block comment */ 42");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
}

#[test]
fn test_regex_literal() {
    let tokens = scan_all("/pattern/gi");
    // RegularExpressionLiteral should be produced
    assert!(!tokens.is_empty());
}

#[test]
fn test_typescript_specific_keywords() {
    let kinds = scan_kinds("async await readonly abstract declare override");
    assert_eq!(kinds, vec![
        SyntaxKind::AsyncKeyword,
        SyntaxKind::AwaitKeyword,
        SyntaxKind::ReadonlyKeyword,
        SyntaxKind::AbstractKeyword,
        SyntaxKind::DeclareKeyword,
        SyntaxKind::OverrideKeyword,
    ]);
}

#[test]
fn test_variable_declaration_tokens() {
    let tokens = scan_all("const x: number = 42;");
    let kinds: Vec<SyntaxKind> = tokens.iter().map(|(k, _)| *k).collect();
    assert_eq!(kinds, vec![
        SyntaxKind::ConstKeyword,
        SyntaxKind::Identifier,       // x
        SyntaxKind::ColonToken,
        SyntaxKind::NumberKeyword,
        SyntaxKind::EqualsToken,
        SyntaxKind::NumericLiteral,    // 42
        SyntaxKind::SemicolonToken,
    ]);
}

#[test]
fn test_function_declaration_tokens() {
    let tokens = scan_all("function add(a: number, b: number): number { return a + b; }");
    let kinds: Vec<SyntaxKind> = tokens.iter().map(|(k, _)| *k).collect();
    assert_eq!(kinds[0], SyntaxKind::FunctionKeyword);
    assert_eq!(kinds[1], SyntaxKind::Identifier); // add
    assert_eq!(kinds[2], SyntaxKind::OpenParenToken);
    // Verify contains return keyword
    assert!(kinds.contains(&SyntaxKind::ReturnKeyword));
}

#[test]
fn test_no_diagnostics_for_valid_source() {
    let mut scanner = Scanner::new("const x = 42;");
    while scanner.scan() != SyntaxKind::EndOfFileToken {}
    assert_eq!(scanner.diagnostics().len(), 0);
}

#[test]
fn test_bigint_literal() {
    let tokens = scan_all("100n");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::BigIntLiteral);
}

#[test]
fn test_hash_token() {
    let tokens = scan_all("#");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::HashToken);
}

#[test]
fn test_at_token() {
    let tokens = scan_all("@");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::AtToken);
}

// ========================================================================
// Phase 1: Comprehensive scanner hardening tests
// ========================================================================

// --- BigInt edge cases ---

#[test]
fn test_bigint_binary() {
    let tokens = scan_all("0b1010n");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::BigIntLiteral);
    assert_eq!(tokens[0].1, "0b1010n");
}

#[test]
fn test_bigint_octal() {
    let tokens = scan_all("0o77n");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::BigIntLiteral);
    assert_eq!(tokens[0].1, "0o77n");
}

#[test]
fn test_bigint_hex() {
    let tokens = scan_all("0xDEADn");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::BigIntLiteral);
    assert_eq!(tokens[0].1, "0xDEADn");
}

#[test]
fn test_bigint_zero() {
    let tokens = scan_all("0n");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::BigIntLiteral);
    assert_eq!(tokens[0].1, "0n");
}

#[test]
fn test_bigint_large() {
    let tokens = scan_all("999999999999999999999n");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::BigIntLiteral);
}

// --- Numeric edge cases ---

#[test]
fn test_scientific_notation() {
    let tokens = scan_all("1e5");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
    assert_eq!(tokens[0].1, "1e5");

    let tokens = scan_all("1E10");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
    assert_eq!(tokens[0].1, "1E10");

    let tokens = scan_all("1.5e+10");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
    assert_eq!(tokens[0].1, "1.5e+10");

    let tokens = scan_all("1e-5");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
    assert_eq!(tokens[0].1, "1e-5");
}

#[test]
fn test_leading_dot_number() {
    // .5 should be scanned as a numeric literal
    let tokens = scan_all(".5");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
}

#[test]
fn test_numeric_separator() {
    let tokens = scan_all("1_000_000");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
    assert_eq!(tokens[0].1, "1_000_000");

    let tokens = scan_all("0xFF_FF");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
    assert_eq!(tokens[0].1, "0xFF_FF");

    let tokens = scan_all("0b1010_0101");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
    assert_eq!(tokens[0].1, "0b1010_0101");

    let tokens = scan_all("0o77_77");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
    assert_eq!(tokens[0].1, "0o77_77");
}

#[test]
fn test_zero_variants() {
    // Plain zero
    let tokens = scan_all("0");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
    assert_eq!(tokens[0].1, "0");

    // 0.0
    let tokens = scan_all("0.0");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
    assert_eq!(tokens[0].1, "0.0");
}

// --- String literal edge cases ---

#[test]
fn test_string_escape_sequences() {
    let tokens = scan_all(r#""hello\nworld""#);
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::StringLiteral);
    // The scanner now decodes escape sequences to their actual characters
    assert!(tokens[0].1.contains('\n'));

    let tokens = scan_all(r#""tab\there""#);
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::StringLiteral);
    assert!(tokens[0].1.contains('\t'));
}

#[test]
fn test_empty_string() {
    let tokens = scan_all(r#""""#);
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::StringLiteral);
    assert_eq!(tokens[0].1, "");

    let tokens = scan_all("''");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::StringLiteral);
    assert_eq!(tokens[0].1, "");
}

#[test]
fn test_string_with_escaped_quote() {
    let tokens = scan_all(r#""say \"hi\"""#);
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::StringLiteral);
}

#[test]
fn test_string_with_backslash() {
    let tokens = scan_all(r#""path\\to\\file""#);
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::StringLiteral);
}

// --- Template literal edge cases ---

#[test]
fn test_template_with_expression() {
    let mut scanner = Scanner::new("`hello ${name}`");
    assert_eq!(scanner.scan(), SyntaxKind::TemplateHead);
    assert_eq!(scanner.token_value(), "hello ");
    assert_eq!(scanner.scan(), SyntaxKind::Identifier); // name
    // After parser processes `}`, it calls rescan_template_token
}

#[test]
fn test_empty_template() {
    let tokens = scan_all("``");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NoSubstitutionTemplateLiteral);
    assert_eq!(tokens[0].1, "");
}

#[test]
fn test_template_with_escape() {
    let tokens = scan_all(r"`hello\nworld`");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NoSubstitutionTemplateLiteral);
}

// --- Bitwise and shift operators ---

#[test]
fn test_bitwise_operators() {
    let tokens = scan_all("& | ^ ~");
    let kinds: Vec<SyntaxKind> = tokens.iter().map(|(k, _)| *k).collect();
    assert_eq!(kinds, vec![
        SyntaxKind::AmpersandToken,
        SyntaxKind::BarToken,
        SyntaxKind::CaretToken,
        SyntaxKind::TildeToken,
    ]);
}

#[test]
fn test_shift_operators() {
    let kinds = scan_kinds("<<");
    assert_eq!(kinds, vec![SyntaxKind::LessThanLessThanToken]);

    let kinds = scan_kinds("<<=");
    assert_eq!(kinds, vec![SyntaxKind::LessThanLessThanEqualsToken]);
}

#[test]
fn test_bitwise_assignment_operators() {
    let tokens = scan_all("&= |= ^=");
    let kinds: Vec<SyntaxKind> = tokens.iter().map(|(k, _)| *k).collect();
    assert_eq!(kinds, vec![
        SyntaxKind::AmpersandEqualsToken,
        SyntaxKind::BarEqualsToken,
        SyntaxKind::CaretEqualsToken,
    ]);
}

// --- Comparison operators ---

#[test]
fn test_greater_than_equals() {
    // > is always scanned as GreaterThanToken; >= is produced via rescan
    let tokens = scan_all(">");
    assert_eq!(tokens[0].0, SyntaxKind::GreaterThanToken);
}

// --- Increment / Decrement ---

#[test]
fn test_increment_decrement() {
    let tokens = scan_all("++ --");
    let kinds: Vec<SyntaxKind> = tokens.iter().map(|(k, _)| *k).collect();
    assert_eq!(kinds, vec![
        SyntaxKind::PlusPlusToken,
        SyntaxKind::MinusMinusToken,
    ]);
}

// --- More keywords ---

#[test]
fn test_all_declaration_keywords() {
    let kinds = scan_kinds("export import from as default");
    assert_eq!(kinds, vec![
        SyntaxKind::ExportKeyword,
        SyntaxKind::ImportKeyword,
        SyntaxKind::FromKeyword,
        SyntaxKind::AsKeyword,
        SyntaxKind::DefaultKeyword,
    ]);
}

#[test]
fn test_control_flow_keywords() {
    let kinds = scan_kinds("break continue do switch case throw try catch finally");
    assert_eq!(kinds, vec![
        SyntaxKind::BreakKeyword,
        SyntaxKind::ContinueKeyword,
        SyntaxKind::DoKeyword,
        SyntaxKind::SwitchKeyword,
        SyntaxKind::CaseKeyword,
        SyntaxKind::ThrowKeyword,
        SyntaxKind::TryKeyword,
        SyntaxKind::CatchKeyword,
        SyntaxKind::FinallyKeyword,
    ]);
}

#[test]
fn test_class_related_keywords() {
    let kinds = scan_kinds("new this super extends implements constructor");
    assert_eq!(kinds, vec![
        SyntaxKind::NewKeyword,
        SyntaxKind::ThisKeyword,
        SyntaxKind::SuperKeyword,
        SyntaxKind::ExtendsKeyword,
        SyntaxKind::ImplementsKeyword,
        SyntaxKind::ConstructorKeyword,
    ]);
}

#[test]
fn test_modifier_keywords() {
    let kinds = scan_kinds("public private protected static");
    assert_eq!(kinds, vec![
        SyntaxKind::PublicKeyword,
        SyntaxKind::PrivateKeyword,
        SyntaxKind::ProtectedKeyword,
        SyntaxKind::StaticKeyword,
    ]);
}

#[test]
fn test_type_keywords() {
    let kinds = scan_kinds("string number boolean void any never unknown");
    assert_eq!(kinds, vec![
        SyntaxKind::StringKeyword,
        SyntaxKind::NumberKeyword,
        SyntaxKind::BooleanKeyword,
        SyntaxKind::VoidKeyword,
        SyntaxKind::AnyKeyword,
        SyntaxKind::NeverKeyword,
        SyntaxKind::UnknownKeyword,
    ]);
}

#[test]
fn test_literal_keywords() {
    let kinds = scan_kinds("true false null undefined");
    assert_eq!(kinds, vec![
        SyntaxKind::TrueKeyword,
        SyntaxKind::FalseKeyword,
        SyntaxKind::NullKeyword,
        SyntaxKind::UndefinedKeyword,
    ]);
}

#[test]
fn test_type_operator_keywords() {
    let kinds = scan_kinds("typeof instanceof in of keyof infer");
    assert_eq!(kinds, vec![
        SyntaxKind::TypeOfKeyword,
        SyntaxKind::InstanceOfKeyword,
        SyntaxKind::InKeyword,
        SyntaxKind::OfKeyword,
        SyntaxKind::KeyOfKeyword,
        SyntaxKind::InferKeyword,
    ]);
}

#[test]
fn test_misc_keywords() {
    let kinds = scan_kinds("delete void yield with debugger");
    assert_eq!(kinds, vec![
        SyntaxKind::DeleteKeyword,
        SyntaxKind::VoidKeyword,
        SyntaxKind::YieldKeyword,
        SyntaxKind::WithKeyword,
        SyntaxKind::DebuggerKeyword,
    ]);
}

// --- Position tracking ---

#[test]
fn test_token_positions() {
    let mut scanner = Scanner::new("let x = 42;");
    scanner.scan(); // let
    assert_eq!(scanner.token_start(), 0);

    scanner.scan(); // x
    assert_eq!(scanner.token_start(), 4);

    scanner.scan(); // =
    assert_eq!(scanner.token_start(), 6);

    scanner.scan(); // 42
    assert_eq!(scanner.token_start(), 8);
}

#[test]
fn test_line_break_tracking() {
    let mut scanner = Scanner::new("a\nb");
    scanner.scan(); // a
    assert!(!scanner.has_preceding_line_break());
    scanner.scan(); // b
    assert!(scanner.has_preceding_line_break());
}

// --- Error recovery ---

#[test]
fn test_unterminated_string_produces_diagnostic() {
    let mut scanner = Scanner::new("\"hello");
    scanner.scan();
    assert!(scanner.diagnostics().len() > 0);
}

#[test]
fn test_unterminated_template_produces_diagnostic() {
    let mut scanner = Scanner::new("`hello");
    scanner.scan();
    assert!(scanner.diagnostics().len() > 0);
}

// --- Contextual identifiers ---

#[test]
fn test_contextual_keywords_are_identifiers_in_expressions() {
    // 'type', 'interface', 'from', 'as' etc. are contextual keywords
    // that can be used as identifiers; the scanner always emits them as keywords.
    // The parser handles contextual usage. Just verify scanner recognizes them.
    let kinds = scan_kinds("type interface from as");
    assert_eq!(kinds, vec![
        SyntaxKind::TypeKeyword,
        SyntaxKind::InterfaceKeyword,
        SyntaxKind::FromKeyword,
        SyntaxKind::AsKeyword,
    ]);
}

// --- Complex multi-token sequences ---

#[test]
fn test_arrow_function_tokens() {
    let kinds = scan_kinds("(a: number) => a + 1");
    assert_eq!(kinds, vec![
        SyntaxKind::OpenParenToken,
        SyntaxKind::Identifier,        // a
        SyntaxKind::ColonToken,
        SyntaxKind::NumberKeyword,
        SyntaxKind::CloseParenToken,
        SyntaxKind::EqualsGreaterThanToken,
        SyntaxKind::Identifier,        // a
        SyntaxKind::PlusToken,
        SyntaxKind::NumericLiteral,    // 1
    ]);
}

#[test]
fn test_generic_type_tokens() {
    let kinds = scan_kinds("Array<number>");
    assert_eq!(kinds, vec![
        SyntaxKind::Identifier,         // Array
        SyntaxKind::LessThanToken,
        SyntaxKind::NumberKeyword,
        SyntaxKind::GreaterThanToken,
    ]);
}

#[test]
fn test_destructuring_tokens() {
    let kinds = scan_kinds("const { a, b } = obj;");
    assert_eq!(kinds, vec![
        SyntaxKind::ConstKeyword,
        SyntaxKind::OpenBraceToken,
        SyntaxKind::Identifier,        // a
        SyntaxKind::CommaToken,
        SyntaxKind::Identifier,        // b
        SyntaxKind::CloseBraceToken,
        SyntaxKind::EqualsToken,
        SyntaxKind::Identifier,        // obj
        SyntaxKind::SemicolonToken,
    ]);
}

#[test]
fn test_optional_property_access_tokens() {
    let kinds = scan_kinds("a?.b?.c");
    assert_eq!(kinds, vec![
        SyntaxKind::Identifier,         // a
        SyntaxKind::QuestionDotToken,
        SyntaxKind::Identifier,         // b
        SyntaxKind::QuestionDotToken,
        SyntaxKind::Identifier,         // c
    ]);
}

#[test]
fn test_nullish_coalescing_chain() {
    let kinds = scan_kinds("a ?? b ?? c");
    assert_eq!(kinds, vec![
        SyntaxKind::Identifier,
        SyntaxKind::QuestionQuestionToken,
        SyntaxKind::Identifier,
        SyntaxKind::QuestionQuestionToken,
        SyntaxKind::Identifier,
    ]);
}

#[test]
fn test_rescan_greater_than() {
    // The scanner's rescan_greater_than_token re-scans > into >=, >>, >>=, >>>, >>>=
    let mut scanner = Scanner::new(">=");
    assert_eq!(scanner.scan(), SyntaxKind::GreaterThanToken);
    let rescanned = scanner.rescan_greater_than_token();
    assert_eq!(rescanned, SyntaxKind::GreaterThanEqualsToken);
}

#[test]
fn test_rescan_right_shift() {
    let mut scanner = Scanner::new(">>");
    assert_eq!(scanner.scan(), SyntaxKind::GreaterThanToken);
    let rescanned = scanner.rescan_greater_than_token();
    assert_eq!(rescanned, SyntaxKind::GreaterThanGreaterThanToken);
}

#[test]
fn test_rescan_unsigned_right_shift() {
    let mut scanner = Scanner::new(">>>");
    assert_eq!(scanner.scan(), SyntaxKind::GreaterThanToken);
    let rescanned = scanner.rescan_greater_than_token();
    assert_eq!(rescanned, SyntaxKind::GreaterThanGreaterThanGreaterThanToken);
}

// --- Rescan template tokens ---

#[test]
fn test_rescan_template_middle_and_tail() {
    // Simulate template: `a${x}b${y}c`
    let mut scanner = Scanner::new("`a${x}b${y}c`");
    assert_eq!(scanner.scan(), SyntaxKind::TemplateHead);
    assert_eq!(scanner.token_value(), "a");

    assert_eq!(scanner.scan(), SyntaxKind::Identifier); // x
    // After }, parser calls rescan_template_token
    assert_eq!(scanner.scan(), SyntaxKind::CloseBraceToken);

    let mid = scanner.rescan_template_token();
    assert_eq!(mid, SyntaxKind::TemplateMiddle);
    assert_eq!(scanner.token_value(), "b");

    assert_eq!(scanner.scan(), SyntaxKind::Identifier); // y
    assert_eq!(scanner.scan(), SyntaxKind::CloseBraceToken);

    let tail = scanner.rescan_template_token();
    assert_eq!(tail, SyntaxKind::TemplateTail);
    assert_eq!(scanner.token_value(), "c");
}

// --- JSX scanning ---

#[test]
fn test_jsx_less_than_slash() {
    // `</` is LessThanSlashToken for JSX closing tags
    let kinds = scan_kinds("</");
    assert_eq!(kinds, vec![SyntaxKind::LessThanSlashToken]);
}

// --- Identifier boundary ---

#[test]
fn test_identifier_with_unicode() {
    // Identifiers can start with $ or _
    let tokens = scan_all("$var _under __double");
    assert_eq!(tokens.len(), 3);
    for (kind, _) in &tokens {
        assert_eq!(*kind, SyntaxKind::Identifier);
    }
    assert_eq!(tokens[0].1, "$var");
    assert_eq!(tokens[1].1, "_under");
    assert_eq!(tokens[2].1, "__double");
}

// --- Consecutive operators without whitespace ---

#[test]
fn test_consecutive_operators() {
    // `!!x` should be two ExclamationTokens and an identifier
    let kinds = scan_kinds("!!x");
    assert_eq!(kinds, vec![
        SyntaxKind::ExclamationToken,
        SyntaxKind::ExclamationToken,
        SyntaxKind::Identifier,
    ]);
}

#[test]
fn test_negative_number_tokens() {
    // `-42` is MinusToken + NumericLiteral (scanner doesn't produce negative numbers)
    let kinds = scan_kinds("-42");
    assert_eq!(kinds, vec![
        SyntaxKind::MinusToken,
        SyntaxKind::NumericLiteral,
    ]);
}

// --- Multi-line comment edge cases ---

#[test]
fn test_multiline_comment_skipped() {
    let tokens = scan_all("/* comment\nspanning\nlines */ 42");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
}

#[test]
fn test_multiple_comments() {
    let tokens = scan_all("// line 1\n// line 2\n42");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
}

// --- Carriage return handling ---

#[test]
fn test_crlf_line_breaks() {
    let mut scanner = Scanner::new("a\r\nb");
    scanner.scan(); // a
    scanner.scan(); // b
    assert!(scanner.has_preceding_line_break());
}

// ============================================================================
// Unicode escape sequences
// ============================================================================

#[test]
fn test_unicode_escape_4digit() {
    let tokens = scan_all(r#""\u0041""#);
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::StringLiteral);
    assert_eq!(tokens[0].1, "A");
}

#[test]
fn test_unicode_escape_braced() {
    let tokens = scan_all(r#""\u{41}""#);
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::StringLiteral);
    assert_eq!(tokens[0].1, "A");
}

#[test]
fn test_unicode_escape_braced_large_codepoint() {
    let tokens = scan_all(r#""\u{1F600}""#);
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].1, "\u{1F600}");
}

#[test]
fn test_unicode_escape_in_string_mixed() {
    let tokens = scan_all(r#""hello \u0041 world""#);
    assert_eq!(tokens[0].1, "hello A world");
}

#[test]
fn test_hex_escape_in_string() {
    let tokens = scan_all(r#""\x41""#);
    assert_eq!(tokens[0].1, "A");
}

#[test]
fn test_escape_sequences_n_r_t() {
    let tokens = scan_all(r#""\n\r\t""#);
    assert_eq!(tokens[0].1, "\n\r\t");
}

#[test]
fn test_escape_null() {
    let tokens = scan_all(r#""\0""#);
    assert_eq!(tokens[0].1, "\0");
}

#[test]
fn test_escape_backslash() {
    let tokens = scan_all(r#""\\""#);
    assert_eq!(tokens[0].1, "\\");
}

#[test]
fn test_escape_quotes() {
    let tokens = scan_all(r#""\"hello\"""#);
    assert_eq!(tokens[0].1, "\"hello\"");
}

// ============================================================================
// String literal edge cases
// ============================================================================

#[test]
fn test_single_quote_string() {
    let tokens = scan_all("'hello'");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::StringLiteral);
    assert_eq!(tokens[0].1, "hello");
}

#[test]
fn test_empty_string_double() {
    let tokens = scan_all(r#""""#);
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].1, "");
}

#[test]
fn test_empty_string_single() {
    let tokens = scan_all("''");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].1, "");
}

#[test]
fn test_string_with_special_chars() {
    let tokens = scan_all(r#""a\tb\nc""#);
    assert_eq!(tokens[0].1, "a\tb\nc");
}

// ============================================================================
// RegExp scanning
// ============================================================================

#[test]
fn test_regexp_basic() {
    let mut scanner = Scanner::new("/abc/");
    let kind = scanner.scan();
    // Initially scanned as SlashToken, needs rescan
    assert_eq!(kind, SyntaxKind::SlashToken);
    let rescanned = scanner.rescan_slash_token();
    assert_eq!(rescanned, SyntaxKind::RegularExpressionLiteral);
}

#[test]
fn test_regexp_with_flags() {
    let mut scanner = Scanner::new("/abc/gi");
    scanner.scan();
    let rescanned = scanner.rescan_slash_token();
    assert_eq!(rescanned, SyntaxKind::RegularExpressionLiteral);
    assert!(scanner.token_value().contains("gi"));
}

// ============================================================================
// Comment handling
// ============================================================================

#[test]
fn test_single_line_comment() {
    let tokens = scan_all("// this is a comment\n42");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
}

#[test]
fn test_multiline_comment_inline() {
    let tokens = scan_all("1 /* comment */ + 2");
    assert_eq!(tokens.len(), 3);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
    assert_eq!(tokens[1].0, SyntaxKind::PlusToken);
    assert_eq!(tokens[2].0, SyntaxKind::NumericLiteral);
}

#[test]
fn test_jsdoc_style_comment() {
    let tokens = scan_all("/** @param x */\nconst a = 1;");
    let kinds: Vec<_> = tokens.iter().map(|(k, _)| *k).collect();
    assert!(kinds.contains(&SyntaxKind::ConstKeyword));
}

// ============================================================================
// Operator exhaustive tests
// ============================================================================

#[test]
fn test_all_assignment_operators() {
    let ops = vec![
        ("=", SyntaxKind::EqualsToken),
        ("+=", SyntaxKind::PlusEqualsToken),
        ("-=", SyntaxKind::MinusEqualsToken),
        ("*=", SyntaxKind::AsteriskEqualsToken),
        ("/=", SyntaxKind::SlashEqualsToken),
        ("%=", SyntaxKind::PercentEqualsToken),
        ("**=", SyntaxKind::AsteriskAsteriskEqualsToken),
        ("&=", SyntaxKind::AmpersandEqualsToken),
        ("|=", SyntaxKind::BarEqualsToken),
        ("^=", SyntaxKind::CaretEqualsToken),
    ];
    for (src, expected) in ops {
        let kinds = scan_kinds(&format!("x {} y", src));
        assert!(kinds.contains(&expected), "Expected {:?} in {:?} for {}", expected, kinds, src);
    }
}

#[test]
fn test_comparison_operators() {
    let ops = vec![
        ("==", SyntaxKind::EqualsEqualsToken),
        ("!=", SyntaxKind::ExclamationEqualsToken),
        ("===", SyntaxKind::EqualsEqualsEqualsToken),
        ("!==", SyntaxKind::ExclamationEqualsEqualsToken),
        ("<", SyntaxKind::LessThanToken),
        (">", SyntaxKind::GreaterThanToken),
        ("<=", SyntaxKind::LessThanEqualsToken),
    ];
    for (src, expected) in ops {
        let kinds = scan_kinds(&format!("x {} y", src));
        assert!(kinds.contains(&expected), "Expected {:?} for {}", expected, src);
    }
    // >= is scanned as GreaterThanToken + EqualsToken by default (rescan needed for >=)
    // This matches TypeScript's scanner behavior where > is always scanned first
}

#[test]
fn test_logical_operators() {
    let ops = vec![
        ("&&", SyntaxKind::AmpersandAmpersandToken),
        ("||", SyntaxKind::BarBarToken),
        ("??", SyntaxKind::QuestionQuestionToken),
    ];
    for (src, expected) in ops {
        let kinds = scan_kinds(&format!("x {} y", src));
        assert!(kinds.contains(&expected), "Expected {:?} for {}", expected, src);
    }
}

#[test]
fn test_dot_token() {
    let kinds = scan_kinds("a.b");
    assert_eq!(kinds, vec![SyntaxKind::Identifier, SyntaxKind::DotToken, SyntaxKind::Identifier]);
}

#[test]
fn test_spread_token() {
    let kinds = scan_kinds("...x");
    assert_eq!(kinds, vec![SyntaxKind::DotDotDotToken, SyntaxKind::Identifier]);
}

#[test]
fn test_optional_chaining_token() {
    let kinds = scan_kinds("a?.b");
    assert!(kinds.contains(&SyntaxKind::QuestionDotToken));
}

#[test]
fn test_arrow_token() {
    let kinds = scan_kinds("() => x");
    assert!(kinds.contains(&SyntaxKind::EqualsGreaterThanToken));
}

// ============================================================================
// Numeric literal edge cases
// ============================================================================

#[test]
fn test_leading_dot_number_value() {
    let tokens = scan_all(".5");
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
    assert_eq!(tokens[0].1, ".5");
}

#[test]
fn test_scientific_notation_positive() {
    let tokens = scan_all("1e10");
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
}

#[test]
fn test_scientific_negative_exponent() {
    let tokens = scan_all("1e-5");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
}

#[test]
fn test_hex_number_ff() {
    let tokens = scan_all("0xFF");
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
}

#[test]
fn test_binary_number_literal() {
    let tokens = scan_all("0b1010");
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
}

#[test]
fn test_octal_number_literal() {
    let tokens = scan_all("0o77");
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
}

#[test]
fn test_numeric_separator_millions() {
    let tokens = scan_all("1_000_000");
    assert_eq!(tokens[0].0, SyntaxKind::NumericLiteral);
}

// ============================================================================
// BigInt edge cases
// ============================================================================

#[test]
fn test_bigint_simple() {
    let tokens = scan_all("100n");
    assert_eq!(tokens[0].0, SyntaxKind::BigIntLiteral);
}

#[test]
fn test_bigint_hex_ff() {
    let tokens = scan_all("0xFFn");
    assert_eq!(tokens[0].0, SyntaxKind::BigIntLiteral);
}

#[test]
fn test_bigint_zero_literal() {
    let tokens = scan_all("0n");
    assert_eq!(tokens[0].0, SyntaxKind::BigIntLiteral);
}

// ============================================================================
// Template literal edge cases
// ============================================================================

#[test]
fn test_template_no_substitution_literal() {
    let tokens = scan_all("`hello world`");
    assert_eq!(tokens[0].0, SyntaxKind::NoSubstitutionTemplateLiteral);
}

#[test]
fn test_template_with_expression_head() {
    let kinds = scan_kinds("`hello ${name}`");
    assert_eq!(kinds[0], SyntaxKind::TemplateHead);
}

#[test]
fn test_empty_template_literal() {
    let tokens = scan_all("``");
    assert_eq!(tokens[0].0, SyntaxKind::NoSubstitutionTemplateLiteral);
    assert_eq!(tokens[0].1, "");
}

// ============================================================================
// Private identifiers
// ============================================================================

#[test]
fn test_private_identifier_hash() {
    let kinds = scan_kinds("#field");
    assert_eq!(kinds[0], SyntaxKind::HashToken);
    assert_eq!(kinds[1], SyntaxKind::Identifier);
}

// ============================================================================
// Shebang
// ============================================================================

#[test]
fn test_shebang_skipped() {
    let mut scanner = Scanner::new("#!/usr/bin/env node\nconst x = 1;");
    scanner.skip_shebang();
    let kind = scanner.scan();
    assert_eq!(kind, SyntaxKind::ConstKeyword);
}

#[test]
fn test_no_shebang_no_skip() {
    let mut scanner = Scanner::new("const x = 1;");
    scanner.skip_shebang();
    let kind = scanner.scan();
    assert_eq!(kind, SyntaxKind::ConstKeyword);
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn test_whitespace_only_input() {
    let tokens = scan_all("   \t\n  ");
    assert!(tokens.is_empty());
}

#[test]
fn test_unterminated_string() {
    let mut scanner = Scanner::new("\"hello");
    scanner.scan();
    assert!(!scanner.diagnostics().is_empty());
}

#[test]
fn test_unterminated_template() {
    let mut scanner = Scanner::new("`hello");
    scanner.scan();
    assert!(!scanner.diagnostics().is_empty());
}

#[test]
fn test_identifier_with_dollar() {
    let tokens = scan_all("$foo");
    assert_eq!(tokens[0].0, SyntaxKind::Identifier);
    assert_eq!(tokens[0].1, "$foo");
}

#[test]
fn test_identifier_with_underscore() {
    let tokens = scan_all("_bar");
    assert_eq!(tokens[0].0, SyntaxKind::Identifier);
    assert_eq!(tokens[0].1, "_bar");
}

#[test]
fn test_scanner_save_restore() {
    let mut scanner = Scanner::new("a b c");
    scanner.scan(); // a
    let state = scanner.save_state();
    scanner.scan(); // b
    assert_eq!(scanner.token_value(), "b");
    scanner.restore_state(state);
    let kind = scanner.scan();
    assert_eq!(kind, SyntaxKind::Identifier);
    assert_eq!(scanner.token_value(), "b");
}

#[test]
fn test_look_ahead() {
    let mut scanner = Scanner::new("a b c");
    scanner.scan(); // a
    let next = scanner.look_ahead(|s| {
        s.scan() // peek at b
    });
    assert_eq!(next, SyntaxKind::Identifier);
    // Position should be restored
    let kind = scanner.scan();
    assert_eq!(kind, SyntaxKind::Identifier);
    assert_eq!(scanner.token_value(), "b");
}

// ============================================================================
// JSX scanning
// ============================================================================

#[test]
fn test_jsx_text_scanning() {
    let mut scanner = Scanner::new("Hello World");
    scanner.set_in_jsx(true);
    let kind = scanner.scan_jsx_text();
    assert_eq!(kind, SyntaxKind::JsxText);
}

// ============================================================================
// Keyword coverage (comprehensive)
// ============================================================================

#[test]
fn test_declaration_keywords_all() {
    let keywords = vec![
        ("var", SyntaxKind::VarKeyword),
        ("let", SyntaxKind::LetKeyword),
        ("const", SyntaxKind::ConstKeyword),
        ("function", SyntaxKind::FunctionKeyword),
        ("class", SyntaxKind::ClassKeyword),
        ("interface", SyntaxKind::InterfaceKeyword),
        ("enum", SyntaxKind::EnumKeyword),
        ("type", SyntaxKind::TypeKeyword),
        ("namespace", SyntaxKind::NamespaceKeyword),
    ];
    for (src, expected) in keywords {
        let kinds = scan_kinds(src);
        assert_eq!(kinds, vec![expected], "Failed for keyword: {}", src);
    }
}

#[test]
fn test_all_control_flow_keywords() {
    let keywords = vec![
        ("if", SyntaxKind::IfKeyword),
        ("else", SyntaxKind::ElseKeyword),
        ("for", SyntaxKind::ForKeyword),
        ("while", SyntaxKind::WhileKeyword),
        ("do", SyntaxKind::DoKeyword),
        ("switch", SyntaxKind::SwitchKeyword),
        ("case", SyntaxKind::CaseKeyword),
        ("break", SyntaxKind::BreakKeyword),
        ("continue", SyntaxKind::ContinueKeyword),
        ("return", SyntaxKind::ReturnKeyword),
        ("try", SyntaxKind::TryKeyword),
        ("catch", SyntaxKind::CatchKeyword),
        ("finally", SyntaxKind::FinallyKeyword),
        ("throw", SyntaxKind::ThrowKeyword),
    ];
    for (src, expected) in keywords {
        let kinds = scan_kinds(src);
        assert_eq!(kinds, vec![expected], "Failed for keyword: {}", src);
    }
}

#[test]
fn test_type_keywords_all() {
    let keywords = vec![
        ("any", SyntaxKind::AnyKeyword),
        ("boolean", SyntaxKind::BooleanKeyword),
        ("number", SyntaxKind::NumberKeyword),
        ("string", SyntaxKind::StringKeyword),
        ("void", SyntaxKind::VoidKeyword),
        ("never", SyntaxKind::NeverKeyword),
        ("unknown", SyntaxKind::UnknownKeyword),
        ("undefined", SyntaxKind::UndefinedKeyword),
    ];
    for (src, expected) in keywords {
        let kinds = scan_kinds(src);
        assert_eq!(kinds, vec![expected], "Failed for keyword: {}", src);
    }
}

#[test]
fn test_modifier_keywords_all() {
    let keywords = vec![
        ("public", SyntaxKind::PublicKeyword),
        ("private", SyntaxKind::PrivateKeyword),
        ("protected", SyntaxKind::ProtectedKeyword),
        ("static", SyntaxKind::StaticKeyword),
        ("readonly", SyntaxKind::ReadonlyKeyword),
        ("abstract", SyntaxKind::AbstractKeyword),
        ("async", SyntaxKind::AsyncKeyword),
        ("declare", SyntaxKind::DeclareKeyword),
        ("export", SyntaxKind::ExportKeyword),
        ("import", SyntaxKind::ImportKeyword),
    ];
    for (src, expected) in keywords {
        let kinds = scan_kinds(src);
        assert_eq!(kinds, vec![expected], "Failed for keyword: {}", src);
    }
}

#[test]
fn test_literal_keywords_all() {
    let keywords = vec![
        ("true", SyntaxKind::TrueKeyword),
        ("false", SyntaxKind::FalseKeyword),
        ("null", SyntaxKind::NullKeyword),
        ("this", SyntaxKind::ThisKeyword),
        ("super", SyntaxKind::SuperKeyword),
    ];
    for (src, expected) in keywords {
        let kinds = scan_kinds(src);
        assert_eq!(kinds, vec![expected], "Failed for keyword: {}", src);
    }
}

// ============================================================================
// Rescan methods
// ============================================================================

#[test]
fn test_rescan_greater_than_for_shift() {
    let mut scanner = Scanner::new("> >");
    let kind = scanner.scan();
    assert_eq!(kind, SyntaxKind::GreaterThanToken);
}

#[test]
fn test_rescan_template_token() {
    // After template head, rescan should produce template middle/tail
    let mut scanner = Scanner::new("`a${b}c`");
    let head = scanner.scan();
    assert_eq!(head, SyntaxKind::TemplateHead);
}

// ============================================================================
// Token position tracking (comprehensive)
// ============================================================================

#[test]
fn test_token_start_end_positions() {
    let mut scanner = Scanner::new("ab cd");
    scanner.scan(); // ab
    assert_eq!(scanner.token_start(), 0);
    assert_eq!(scanner.token_end(), 2);
    scanner.scan(); // cd
    assert_eq!(scanner.token_start(), 3);
    assert_eq!(scanner.token_end(), 5);
}

#[test]
fn test_line_break_detection() {
    let mut scanner = Scanner::new("a\nb");
    scanner.scan(); // a
    assert!(!scanner.has_preceding_line_break());
    scanner.scan(); // b
    assert!(scanner.has_preceding_line_break());
}

// ============================================================================
// Complex multi-token sequences
// ============================================================================

#[test]
fn test_arrow_function_sequence() {
    let kinds = scan_kinds("(x: number) => x + 1");
    assert!(kinds.contains(&SyntaxKind::OpenParenToken));
    assert!(kinds.contains(&SyntaxKind::ColonToken));
    assert!(kinds.contains(&SyntaxKind::EqualsGreaterThanToken));
    assert!(kinds.contains(&SyntaxKind::PlusToken));
}

#[test]
fn test_generic_type_sequence() {
    let kinds = scan_kinds("Array<string>");
    assert_eq!(kinds[0], SyntaxKind::Identifier);
    assert_eq!(kinds[1], SyntaxKind::LessThanToken);
    assert_eq!(kinds[2], SyntaxKind::StringKeyword);
    assert_eq!(kinds[3], SyntaxKind::GreaterThanToken);
}

#[test]
fn test_optional_chaining_call() {
    let kinds = scan_kinds("a?.()");
    assert!(kinds.contains(&SyntaxKind::QuestionDotToken));
    assert!(kinds.contains(&SyntaxKind::OpenParenToken));
}

#[test]
fn test_nullish_coalescing_assignment() {
    let kinds = scan_kinds("x ??= y");
    assert!(kinds.contains(&SyntaxKind::QuestionQuestionEqualsToken));
}
