//! rscript_diagnostics: Diagnostic messages and error reporting infrastructure.
//!
//! This module defines all diagnostic messages used by the TypeScript compiler,
//! ported from TypeScript's `diagnosticMessages.json`. Diagnostics carry
//! structured information about errors, warnings, and suggestions.

use rscript_core::text::TextSpan;
use std::fmt;

/// Diagnostic category, matching TypeScript's DiagnosticCategory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticCategory {
    Warning,
    Error,
    Suggestion,
    Message,
}

impl fmt::Display for DiagnosticCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagnosticCategory::Warning => write!(f, "warning"),
            DiagnosticCategory::Error => write!(f, "error"),
            DiagnosticCategory::Suggestion => write!(f, "suggestion"),
            DiagnosticCategory::Message => write!(f, "message"),
        }
    }
}

/// A diagnostic message template with a code and category.
/// This corresponds to a single entry in `diagnosticMessages.json`.
#[derive(Debug, Clone)]
pub struct DiagnosticMessage {
    /// The diagnostic error code (e.g., 1002, 2304).
    pub code: u32,
    /// The category of this diagnostic.
    pub category: DiagnosticCategory,
    /// The message template string. May contain `{0}`, `{1}`, etc. placeholders.
    pub message: &'static str,
}

/// A realized diagnostic with location information and resolved message text.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// The file path where this diagnostic occurred, if any.
    pub file: Option<String>,
    /// The source text span where this diagnostic occurred, if any.
    pub span: Option<TextSpan>,
    /// The diagnostic message template.
    pub message_text: String,
    /// The diagnostic error code.
    pub code: u32,
    /// The category.
    pub category: DiagnosticCategory,
    /// Related diagnostics.
    pub related_information: Vec<Diagnostic>,
}

impl Diagnostic {
    /// Create a new diagnostic without location info (global diagnostic).
    pub fn new(message: &DiagnosticMessage, args: &[&str]) -> Self {
        Self {
            file: None,
            span: None,
            message_text: format_message(message.message, args),
            code: message.code,
            category: message.category,
            related_information: Vec::new(),
        }
    }

    /// Create a new diagnostic with file and span info.
    pub fn with_location(
        file: String,
        span: TextSpan,
        message: &DiagnosticMessage,
        args: &[&str],
    ) -> Self {
        Self {
            file: Some(file),
            span: Some(span),
            message_text: format_message(message.message, args),
            code: message.code,
            category: message.category,
            related_information: Vec::new(),
        }
    }

    /// Add related diagnostic information.
    pub fn with_related(mut self, related: Diagnostic) -> Self {
        self.related_information.push(related);
        self
    }

    /// Whether this is an error diagnostic.
    pub fn is_error(&self) -> bool {
        self.category == DiagnosticCategory::Error
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref file) = self.file {
            write!(f, "{}", file)?;
            if let Some(span) = self.span {
                write!(f, "({})", span.start)?;
            }
            write!(f, ": ")?;
        }
        write!(
            f,
            "{} TS{}: {}",
            self.category, self.code, self.message_text
        )
    }
}

/// Format a diagnostic message template by replacing `{0}`, `{1}`, etc. with arguments.
pub fn format_message(template: &str, args: &[&str]) -> String {
    let mut result = template.to_string();
    for (i, arg) in args.iter().enumerate() {
        result = result.replace(&format!("{{{}}}", i), arg);
    }
    result
}

/// A collection of diagnostics accumulated during compilation.
#[derive(Debug, Clone, Default)]
pub struct DiagnosticCollection {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticCollection {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    pub fn add(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.category == DiagnosticCategory::Error)
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.category == DiagnosticCategory::Error)
            .count()
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    pub fn extend(&mut self, other: DiagnosticCollection) {
        self.diagnostics.extend(other.diagnostics);
    }

    pub fn extend_from_slice(&mut self, diagnostics: &[Diagnostic]) {
        self.diagnostics.extend_from_slice(diagnostics);
    }

    pub fn clear(&mut self) {
        self.diagnostics.clear();
    }

    /// Sort diagnostics by file and position.
    pub fn sort(&mut self) {
        self.diagnostics.sort_by(|a, b| {
            let file_cmp = a.file.cmp(&b.file);
            if file_cmp != std::cmp::Ordering::Equal {
                return file_cmp;
            }
            let a_pos = a.span.map(|s| s.start).unwrap_or(0);
            let b_pos = b.span.map(|s| s.start).unwrap_or(0);
            a_pos.cmp(&b_pos)
        });
    }
}

// ============================================================================
// Diagnostic Messages - ported from TypeScript's diagnosticMessages.json
// ============================================================================

pub mod messages {
    use super::*;

    macro_rules! diag {
        ($code:expr, Error, $msg:expr) => {
            DiagnosticMessage { code: $code, category: DiagnosticCategory::Error, message: $msg }
        };
        ($code:expr, Warning, $msg:expr) => {
            DiagnosticMessage { code: $code, category: DiagnosticCategory::Warning, message: $msg }
        };
        ($code:expr, Suggestion, $msg:expr) => {
            DiagnosticMessage { code: $code, category: DiagnosticCategory::Suggestion, message: $msg }
        };
        ($code:expr, Message, $msg:expr) => {
            DiagnosticMessage { code: $code, category: DiagnosticCategory::Message, message: $msg }
        };
    }

    // ========================================================================
    // Scanner errors (1000-1099)
    // ========================================================================
    pub const UNTERMINATED_STRING_LITERAL: DiagnosticMessage = diag!(1002, Error, "Unterminated string literal.");
    pub const IDENTIFIER_EXPECTED: DiagnosticMessage = diag!(1003, Error, "Identifier expected.");
    pub const _0_EXPECTED: DiagnosticMessage = diag!(1005, Error, "'{0}' expected.");
    pub const A_FILE_CANNOT_HAVE_A_REFERENCE_TO_ITSELF: DiagnosticMessage = diag!(1006, Error, "A file cannot have a reference to itself.");
    pub const THE_PARSER_EXPECTED_TO_FIND_A_0_TO_MATCH_THE_1_TOKEN_HERE: DiagnosticMessage = diag!(1007, Error, "The parser expected to find a '{0}' to match the '{1}' token here.");
    pub const TRAILING_COMMA_NOT_ALLOWED: DiagnosticMessage = diag!(1009, Error, "Trailing comma not allowed.");
    pub const ASTERISK_SLASH_EXPECTED: DiagnosticMessage = diag!(1010, Error, "'*/' expected.");
    pub const AN_ELEMENT_ACCESS_EXPRESSION_SHOULD_TAKE_AN_ARGUMENT: DiagnosticMessage = diag!(1011, Error, "An element access expression should take an argument.");
    pub const UNEXPECTED_TOKEN: DiagnosticMessage = diag!(1012, Error, "Unexpected token.");
    pub const A_REST_PARAMETER_MUST_BE_LAST: DiagnosticMessage = diag!(1014, Error, "A rest parameter must be last in a parameter list.");
    pub const PARAMETER_CANNOT_HAVE_QUESTION_MARK_AND_INITIALIZER: DiagnosticMessage = diag!(1015, Error, "Parameter cannot have question mark and initializer.");
    pub const A_REQUIRED_PARAMETER_CANNOT_FOLLOW_AN_OPTIONAL_PARAMETER: DiagnosticMessage = diag!(1016, Error, "A required parameter cannot follow an optional parameter.");
    pub const AN_INDEX_SIGNATURE_CANNOT_HAVE_A_REST_PARAMETER: DiagnosticMessage = diag!(1017, Error, "An index signature cannot have a rest parameter.");
    pub const AN_INDEX_SIGNATURE_PARAMETER_CANNOT_HAVE_AN_ACCESSIBILITY_MODIFIER: DiagnosticMessage = diag!(1018, Error, "An index signature parameter cannot have an accessibility modifier.");
    pub const AN_INDEX_SIGNATURE_PARAMETER_CANNOT_HAVE_A_QUESTION_MARK: DiagnosticMessage = diag!(1019, Error, "An index signature parameter cannot have a question mark.");
    pub const AN_INDEX_SIGNATURE_PARAMETER_CANNOT_HAVE_AN_INITIALIZER: DiagnosticMessage = diag!(1020, Error, "An index signature parameter cannot have an initializer.");
    pub const AN_INDEX_SIGNATURE_MUST_HAVE_A_TYPE_ANNOTATION: DiagnosticMessage = diag!(1021, Error, "An index signature must have a type annotation.");
    pub const AN_INDEX_SIGNATURE_PARAMETER_MUST_HAVE_A_TYPE_ANNOTATION: DiagnosticMessage = diag!(1022, Error, "An index signature parameter must have a type annotation.");
    pub const READONLY_MODIFIER_CAN_ONLY_APPEAR_ON_A_PROPERTY_DECLARATION_OR_INDEX_SIGNATURE: DiagnosticMessage = diag!(1024, Error, "'readonly' modifier can only appear on a property declaration or index signature.");
    pub const AN_INDEX_SIGNATURE_CANNOT_HAVE_A_TRAILING_COMMA: DiagnosticMessage = diag!(1025, Error, "An index signature cannot have a trailing comma.");
    pub const ACCESSIBILITY_MODIFIER_ALREADY_SEEN: DiagnosticMessage = diag!(1028, Error, "Accessibility modifier already seen.");
    pub const _0_MODIFIER_MUST_PRECEDE_1_MODIFIER: DiagnosticMessage = diag!(1029, Error, "'{0}' modifier must precede '{1}' modifier.");
    pub const _0_MODIFIER_ALREADY_SEEN: DiagnosticMessage = diag!(1030, Error, "'{0}' modifier already seen.");
    pub const _0_MODIFIER_CANNOT_APPEAR_ON_A_CLASS_ELEMENT: DiagnosticMessage = diag!(1031, Error, "'{0}' modifier cannot appear on a class element.");
    pub const SUPER_MUST_BE_FOLLOWED_BY_AN_ARGUMENT_LIST_OR_MEMBER_ACCESS: DiagnosticMessage = diag!(1034, Error, "'super' must be followed by an argument list or member access.");
    pub const ONLY_AMBIENT_MODULES_CAN_USE_QUOTED_NAMES: DiagnosticMessage = diag!(1035, Error, "Only ambient modules can use quoted names.");
    pub const STATEMENTS_ARE_NOT_ALLOWED_IN_AMBIENT_CONTEXTS: DiagnosticMessage = diag!(1036, Error, "Statements are not allowed in ambient contexts.");
    pub const A_DECLARE_MODIFIER_CANNOT_BE_USED_IN_AN_ALREADY_AMBIENT_CONTEXT: DiagnosticMessage = diag!(1038, Error, "A 'declare' modifier cannot be used in an already ambient context.");
    pub const INITIALIZERS_ARE_NOT_ALLOWED_IN_AMBIENT_CONTEXTS: DiagnosticMessage = diag!(1039, Error, "Initializers are not allowed in ambient contexts.");
    pub const _0_MODIFIER_CANNOT_BE_USED_HERE: DiagnosticMessage = diag!(1042, Error, "'{0}' modifier cannot be used here.");
    pub const _0_MODIFIER_CANNOT_APPEAR_ON_A_MODULE_OR_NAMESPACE_ELEMENT: DiagnosticMessage = diag!(1044, Error, "'{0}' modifier cannot appear on a module or namespace element.");
    pub const TOP_LEVEL_DECLARATIONS_IN_D_TS_FILES_MUST_START_WITH_DECLARE_OR_EXPORT: DiagnosticMessage = diag!(1046, Error, "Top-level declarations in .d.ts files must start with either a 'declare' or 'export' modifier.");
    pub const A_REST_PARAMETER_CANNOT_BE_OPTIONAL: DiagnosticMessage = diag!(1047, Error, "A rest parameter cannot be optional.");
    pub const A_REST_PARAMETER_CANNOT_HAVE_AN_INITIALIZER: DiagnosticMessage = diag!(1048, Error, "A rest parameter cannot have an initializer.");
    pub const A_SET_ACCESSOR_MUST_HAVE_EXACTLY_ONE_PARAMETER: DiagnosticMessage = diag!(1049, Error, "A 'set' accessor must have exactly one parameter.");
    pub const A_SET_ACCESSOR_CANNOT_HAVE_AN_OPTIONAL_PARAMETER: DiagnosticMessage = diag!(1051, Error, "A 'set' accessor cannot have an optional parameter.");
    pub const A_SET_ACCESSOR_PARAMETER_CANNOT_HAVE_AN_INITIALIZER: DiagnosticMessage = diag!(1052, Error, "A 'set' accessor parameter cannot have an initializer.");
    pub const A_SET_ACCESSOR_CANNOT_HAVE_REST_PARAMETER: DiagnosticMessage = diag!(1053, Error, "A 'set' accessor cannot have rest parameter.");
    pub const A_GET_ACCESSOR_CANNOT_HAVE_PARAMETERS: DiagnosticMessage = diag!(1054, Error, "A 'get' accessor cannot have parameters.");
    pub const TYPE_0_IS_NOT_A_VALID_ASYNC_FUNCTION_RETURN_TYPE: DiagnosticMessage = diag!(1055, Error, "Type '{0}' is not a valid async function return type in ES5/ES3 because it does not refer to a Promise-compatible constructor value.");
    pub const ACCESSORS_ARE_ONLY_AVAILABLE_WHEN_TARGETING_ECMASCRIPT_5_AND_HIGHER: DiagnosticMessage = diag!(1056, Error, "Accessors are only available when targeting ECMAScript 5 and higher.");
    pub const THE_RETURN_TYPE_OF_A_GET_ACCESSOR_MUST_BE_ASSIGNABLE: DiagnosticMessage = diag!(1058, Error, "The return type of a 'get' accessor must be assignable to its 'set' accessor type.");
    pub const A_GET_ACCESSOR_MUST_RETURN_A_VALUE: DiagnosticMessage = diag!(1059, Error, "A 'get' accessor must return a value.");
    pub const ENUM_MEMBER_MUST_HAVE_INITIALIZER: DiagnosticMessage = diag!(1061, Error, "Enum member must have initializer.");
    pub const AN_EXPORT_ASSIGNMENT_CANNOT_BE_USED_IN_A_NAMESPACE: DiagnosticMessage = diag!(1063, Error, "An export assignment cannot be used in a namespace.");
    pub const IN_AMBIENT_ENUM_DECLARATIONS_MEMBER_INITIALIZER_MUST_BE_CONSTANT_EXPRESSION: DiagnosticMessage = diag!(1066, Error, "In ambient enum declarations member initializer must be constant expression.");
    pub const UNEXPECTED_TOKEN_A_CONSTRUCTOR_METHOD_ACCESSOR_OR_PROPERTY_WAS_EXPECTED: DiagnosticMessage = diag!(1068, Error, "Unexpected token. A constructor, method, accessor, or property was expected.");
    pub const _0_MODIFIER_CANNOT_APPEAR_ON_A_TYPE_MEMBER: DiagnosticMessage = diag!(1070, Error, "'{0}' modifier cannot appear on a type member.");
    pub const _0_MODIFIER_CANNOT_APPEAR_ON_AN_INDEX_SIGNATURE: DiagnosticMessage = diag!(1071, Error, "'{0}' modifier cannot appear on an index signature.");
    pub const A_0_MODIFIER_CANNOT_BE_USED_WITH_AN_IMPORT_DECLARATION: DiagnosticMessage = diag!(1079, Error, "A '{0}' modifier cannot be used with an import declaration.");
    pub const INVALID_REFERENCE_DIRECTIVE_SYNTAX: DiagnosticMessage = diag!(1084, Error, "Invalid 'reference' directive syntax.");
    pub const OCTAL_LITERALS_ARE_NOT_AVAILABLE_WHEN_TARGETING_ES5_AND_HIGHER: DiagnosticMessage = diag!(1085, Error, "Octal literals are not available when targeting ECMAScript 5 and higher. Use the syntax '{0}'.");

    // ========================================================================
    // Parser errors (1100-1199)
    // ========================================================================
    pub const EXPRESSION_EXPECTED: DiagnosticMessage = diag!(1109, Error, "Expression expected.");
    pub const TYPE_EXPECTED: DiagnosticMessage = diag!(1110, Error, "Type expected.");
    pub const A_DEFAULT_CLAUSE_CANNOT_APPEAR_MORE_THAN_ONCE_IN_A_SWITCH_STATEMENT: DiagnosticMessage = diag!(1113, Error, "A 'default' clause cannot appear more than once in a 'switch' statement.");
    pub const DUPLICATE_LABEL_0: DiagnosticMessage = diag!(1114, Error, "Duplicate label '{0}'.");
    pub const A_CONTINUE_STATEMENT_CAN_ONLY_JUMP_TO_AN_ENCLOSING_ITERATION_STATEMENT: DiagnosticMessage = diag!(1115, Error, "A 'continue' statement can only jump to a label of an enclosing iteration statement.");
    pub const A_BREAK_STATEMENT_CAN_ONLY_JUMP_TO_AN_ENCLOSING_STATEMENT: DiagnosticMessage = diag!(1116, Error, "A 'break' statement can only jump to a label of an enclosing labeled statement.");
    pub const AN_OBJECT_MEMBER_CANNOT_BE_DECLARED_OPTIONAL: DiagnosticMessage = diag!(1162, Error, "An object member cannot be declared optional.");
    pub const A_YIELD_EXPRESSION_IS_ONLY_ALLOWED_IN_A_GENERATOR_BODY: DiagnosticMessage = diag!(1163, Error, "A 'yield' expression is only allowed in a generator body.");
    pub const COMPUTED_PROPERTY_NAMES_ARE_NOT_ALLOWED_IN_ENUMS: DiagnosticMessage = diag!(1164, Error, "Computed property names are not allowed in enums.");
    pub const DIGIT_EXPECTED: DiagnosticMessage = diag!(1124, Error, "Digit expected.");
    pub const HEXADECIMAL_DIGIT_EXPECTED: DiagnosticMessage = diag!(1125, Error, "Hexadecimal digit expected.");
    pub const UNEXPECTED_END_OF_TEXT: DiagnosticMessage = diag!(1126, Error, "Unexpected end of text.");
    pub const INVALID_CHARACTER: DiagnosticMessage = diag!(1127, Error, "Invalid character.");
    pub const DECLARATION_OR_STATEMENT_EXPECTED: DiagnosticMessage = diag!(1128, Error, "Declaration or statement expected.");
    pub const STATEMENT_EXPECTED: DiagnosticMessage = diag!(1145, Error, "Statement expected.");
    pub const DECLARATION_EXPECTED: DiagnosticMessage = diag!(1146, Error, "Declaration expected.");
    pub const CASE_OR_DEFAULT_EXPECTED: DiagnosticMessage = diag!(1130, Error, "'case' or 'default' expected.");
    pub const PROPERTY_OR_SIGNATURE_EXPECTED: DiagnosticMessage = diag!(1131, Error, "Property or signature expected.");
    pub const ENUM_MEMBER_EXPECTED: DiagnosticMessage = diag!(1132, Error, "Enum member expected.");
    pub const VARIABLE_DECLARATION_EXPECTED: DiagnosticMessage = diag!(1134, Error, "Variable declaration expected.");
    pub const ARGUMENT_EXPRESSION_EXPECTED: DiagnosticMessage = diag!(1135, Error, "Argument expression expected.");
    pub const PROPERTY_ASSIGNMENT_EXPECTED: DiagnosticMessage = diag!(1136, Error, "Property assignment expected.");
    pub const EXPRESSION_OR_COMMA_EXPECTED: DiagnosticMessage = diag!(1137, Error, "Expression or comma expected.");
    pub const PARAMETER_DECLARATION_EXPECTED: DiagnosticMessage = diag!(1138, Error, "Parameter declaration expected.");
    pub const TYPE_PARAMETER_DECLARATION_EXPECTED: DiagnosticMessage = diag!(1139, Error, "Type parameter declaration expected.");
    pub const TYPE_ARGUMENT_EXPECTED: DiagnosticMessage = diag!(1140, Error, "Type argument expected.");
    pub const STRING_LITERAL_EXPECTED: DiagnosticMessage = diag!(1141, Error, "String literal expected.");
    pub const LINE_BREAK_NOT_PERMITTED_HERE: DiagnosticMessage = diag!(1142, Error, "Line break not permitted here.");
    pub const OR_EXPECTED: DiagnosticMessage = diag!(1144, Error, "'{' or ';' expected.");
    pub const _0_IS_DECLARED_BUT_ITS_VALUE_IS_NEVER_READ: DiagnosticMessage = diag!(6133, Warning, "'{0}' is declared but its value is never read.");
    pub const UNTERMINATED_TEMPLATE_LITERAL: DiagnosticMessage = diag!(1160, Error, "Unterminated template literal.");
    pub const UNTERMINATED_REGULAR_EXPRESSION_LITERAL: DiagnosticMessage = diag!(1161, Error, "Unterminated regular expression literal.");

    // ========================================================================
    // Parser grammar errors (1170-1299)
    // ========================================================================
    pub const A_COMMA_EXPRESSION_IS_NOT_ALLOWED_IN_A_COMPUTED_PROPERTY_NAME: DiagnosticMessage = diag!(1171, Error, "A comma expression is not allowed in a computed property name.");
    pub const EXTENDS_CLAUSE_ALREADY_SEEN: DiagnosticMessage = diag!(1172, Error, "'extends' clause already seen.");
    pub const EXTENDS_CLAUSE_MUST_PRECEDE_IMPLEMENTS_CLAUSE: DiagnosticMessage = diag!(1173, Error, "'extends' clause must precede 'implements' clause.");
    pub const CLASSES_CAN_ONLY_EXTEND_A_SINGLE_CLASS: DiagnosticMessage = diag!(1174, Error, "Classes can only extend a single class.");
    pub const IMPLEMENTS_CLAUSE_ALREADY_SEEN: DiagnosticMessage = diag!(1175, Error, "'implements' clause already seen.");
    pub const INTERFACE_DECLARATION_CANNOT_HAVE_IMPLEMENTS_CLAUSE: DiagnosticMessage = diag!(1176, Error, "Interface declaration cannot have 'implements' clause.");
    pub const BINARY_DIGIT_EXPECTED: DiagnosticMessage = diag!(1177, Error, "Binary digit expected.");
    pub const OCTAL_DIGIT_EXPECTED: DiagnosticMessage = diag!(1178, Error, "Octal digit expected.");
    pub const AN_IMPLEMENTATION_CANNOT_BE_DECLARED_IN_AMBIENT_CONTEXTS: DiagnosticMessage = diag!(1183, Error, "An implementation cannot be declared in ambient contexts.");
    pub const AN_EXTENDED_UNICODE_ESCAPE_VALUE_MUST_BE_BETWEEN_0X0_AND_0X10FFFF: DiagnosticMessage = diag!(1198, Error, "An extended Unicode escape value must be between 0x0 and 0x10FFFF inclusive.");
    pub const UNTERMINATED_UNICODE_ESCAPE_SEQUENCE: DiagnosticMessage = diag!(1199, Error, "Unterminated Unicode escape sequence.");
    pub const LINE_TERMINATOR_NOT_PERMITTED_BEFORE_ARROW: DiagnosticMessage = diag!(1200, Error, "Line terminator not permitted before arrow.");
    pub const IMPORT_ASSIGNMENT_CANNOT_BE_USED_WHEN_TARGETING_ECMASCRIPT_MODULES: DiagnosticMessage = diag!(1202, Error, "Import assignment cannot be used when targeting ECMAScript modules.");
    pub const DECORATORS_ARE_NOT_VALID_HERE: DiagnosticMessage = diag!(1206, Error, "Decorators are not valid here.");
    pub const DECORATORS_CANNOT_BE_APPLIED_TO_MULTIPLE_GET_SET_ACCESSORS_OF_THE_SAME_NAME: DiagnosticMessage = diag!(1207, Error, "Decorators cannot be applied to multiple get/set accessors of the same name.");
    pub const ALL_DECLARATIONS_OF_AN_ABSTRACT_METHOD_MUST_BE_CONSECUTIVE: DiagnosticMessage = diag!(1227, Error, "All declarations of an abstract method must be consecutive.");
    pub const CANNOT_USE_IMPORTS_EXPORTS_OR_MODULE_AUGMENTATIONS_WHEN_MODULE_IS_NONE: DiagnosticMessage = diag!(1148, Error, "Cannot use imports, exports, or module augmentations when '--module' is 'none'.");
    pub const A_NAMESPACE_DECLARATION_CANNOT_BE_IN_A_DIFFERENT_FILE_FROM_A_CLASS_OR_FUNCTION_WITH_WHICH_IT_IS_MERGED: DiagnosticMessage = diag!(2433, Error, "A namespace declaration cannot be in a different file from a class or function with which it is merged.");
    pub const A_NAMESPACE_DECLARATION_CANNOT_BE_LOCATED_PRIOR_TO_A_CLASS_OR_FUNCTION_WITH_WHICH_IT_IS_MERGED: DiagnosticMessage = diag!(2434, Error, "A namespace declaration cannot be located prior to a class or function with which it is merged.");
    pub const _0_MODIFIER_CANNOT_APPEAR_ON_A_CONSTRUCTOR_DECLARATION: DiagnosticMessage = diag!(1089, Error, "'{0}' modifier cannot appear on a constructor declaration.");
    pub const _0_MODIFIER_CANNOT_APPEAR_ON_A_PARAMETER: DiagnosticMessage = diag!(1090, Error, "'{0}' modifier cannot appear on a parameter.");
    pub const ONLY_A_SINGLE_VARIABLE_DECLARATION_IS_ALLOWED_IN_A_FOR_IN_STATEMENT: DiagnosticMessage = diag!(1091, Error, "Only a single variable declaration is allowed in a 'for...in' statement.");
    pub const TYPE_PARAMETERS_CANNOT_APPEAR_ON_A_CONSTRUCTOR_DECLARATION: DiagnosticMessage = diag!(1092, Error, "Type parameters cannot appear on a constructor declaration.");
    pub const TYPE_ANNOTATION_CANNOT_APPEAR_ON_A_CONSTRUCTOR_DECLARATION: DiagnosticMessage = diag!(1093, Error, "Type annotation cannot appear on a constructor declaration.");
    pub const A_SET_ACCESSOR_CANNOT_HAVE_A_RETURN_TYPE_ANNOTATION: DiagnosticMessage = diag!(1095, Error, "A 'set' accessor cannot have a return type annotation.");
    pub const AN_INDEX_SIGNATURE_MUST_HAVE_EXACTLY_ONE_PARAMETER: DiagnosticMessage = diag!(1096, Error, "An index signature must have exactly one parameter.");
    pub const _0_LIST_CANNOT_BE_EMPTY: DiagnosticMessage = diag!(1097, Error, "'{0}' list cannot be empty.");
    pub const TYPE_PARAMETER_LIST_CANNOT_BE_EMPTY: DiagnosticMessage = diag!(1098, Error, "Type parameter list cannot be empty.");
    pub const TYPE_ARGUMENT_LIST_CANNOT_BE_EMPTY: DiagnosticMessage = diag!(1099, Error, "Type argument list cannot be empty.");

    // ========================================================================
    // Strict mode errors (1100-1109)
    // ========================================================================
    pub const INVALID_USE_OF_0_IN_STRICT_MODE: DiagnosticMessage = diag!(1100, Error, "Invalid use of '{0}' in strict mode.");
    pub const WITH_STATEMENTS_ARE_NOT_ALLOWED_IN_STRICT_MODE: DiagnosticMessage = diag!(1101, Error, "'with' statements are not allowed in strict mode.");
    pub const DELETE_CANNOT_BE_CALLED_ON_AN_IDENTIFIER_IN_STRICT_MODE: DiagnosticMessage = diag!(1102, Error, "'delete' cannot be called on an identifier in strict mode.");
    pub const FOR_AWAIT_LOOPS_ARE_ONLY_ALLOWED_WITHIN_ASYNC_FUNCTIONS: DiagnosticMessage = diag!(1103, Error, "'for await' loops are only allowed within async functions and at the top levels of modules.");
    pub const A_CONTINUE_STATEMENT_CAN_ONLY_BE_USED_WITHIN_AN_ENCLOSING_ITERATION_STATEMENT: DiagnosticMessage = diag!(1104, Error, "A 'continue' statement can only be used within an enclosing iteration statement.");
    pub const A_BREAK_STATEMENT_CAN_ONLY_BE_USED_WITHIN_AN_ENCLOSING_ITERATION_OR_SWITCH_STATEMENT: DiagnosticMessage = diag!(1105, Error, "A 'break' statement can only be used within an enclosing iteration statement or a switch statement.");
    pub const THE_LEFT_HAND_SIDE_OF_A_FOR_OF_STATEMENT_CANNOT_USE_A_TYPE_ANNOTATION: DiagnosticMessage = diag!(1106, Error, "The left-hand side of a 'for...of' statement cannot use a type annotation.");
    pub const EXPORT_ASSIGNMENT_IS_NOT_SUPPORTED_WHEN_MODULE_FLAG_IS_SYSTEM: DiagnosticMessage = diag!(1218, Error, "Export assignment is not supported when '--module' flag is 'system'.");

    // ========================================================================
    // Semantic errors (2000-2999)
    // ========================================================================
    pub const DUPLICATE_IDENTIFIER_0: DiagnosticMessage = diag!(2300, Error, "Duplicate identifier '{0}'.");
    pub const AN_INTERFACE_CAN_ONLY_EXTEND_AN_IDENTIFIER_ENTITY_NAME_EXPRESSION: DiagnosticMessage = diag!(2301, Error, "An interface can only extend an identifier/qualified-name with optional type arguments.");
    pub const CANNOT_FIND_NAME_0: DiagnosticMessage = diag!(2304, Error, "Cannot find name '{0}'.");
    pub const MODULE_0_HAS_NO_EXPORTED_MEMBER_1: DiagnosticMessage = diag!(2305, Error, "Module '{0}' has no exported member '{1}'.");
    pub const FILE_0_IS_NOT_A_MODULE: DiagnosticMessage = diag!(2306, Error, "File '{0}' is not a module.");
    pub const CANNOT_FIND_MODULE_0: DiagnosticMessage = diag!(2307, Error, "Cannot find module '{0}' or its corresponding type declarations.");
    pub const MODULE_0_HAS_ALREADY_EXPORTED_A_MEMBER_NAMED_1: DiagnosticMessage = diag!(2308, Error, "Module {0} has already exported a member named '{1}'.");
    pub const AN_EXPORT_ASSIGNMENT_CANNOT_BE_USED_IN_A_MODULE_WITH_OTHER_EXPORTED_ELEMENTS: DiagnosticMessage = diag!(2309, Error, "An export assignment cannot be used in a module with other exported elements.");
    pub const GENERIC_TYPE_0_REQUIRES_1_TYPE_ARGUMENT_S: DiagnosticMessage = diag!(2314, Error, "Generic type '{0}' requires {1} type argument(s).");
    pub const TYPE_0_IS_NOT_GENERIC: DiagnosticMessage = diag!(2315, Error, "Type '{0}' is not generic.");
    pub const TYPE_0_IS_NOT_ASSIGNABLE_TO_TYPE_1: DiagnosticMessage = diag!(2322, Error, "Type '{0}' is not assignable to type '{1}'.");
    pub const TYPE_0_IS_NOT_ASSIGNABLE_TO_TYPE_1_WITH_EXACTOPTIONALPROPERTYTYPES: DiagnosticMessage = diag!(2375, Error, "Type '{0}' is not assignable to type '{1}' with 'exactOptionalPropertyTypes: true'. Consider adding 'undefined' to the type of the target.");
    pub const PROPERTY_0_IS_MISSING_IN_TYPE_1: DiagnosticMessage = diag!(2324, Error, "Property '{0}' is missing in type '{1}'.");
    pub const INDEX_SIGNATURE_IS_MISSING_IN_TYPE_0: DiagnosticMessage = diag!(2329, Error, "Index signature is missing in type '{0}'.");
    pub const THIS_CANNOT_BE_REFERENCED_IN_CURRENT_LOCATION: DiagnosticMessage = diag!(2332, Error, "'this' cannot be referenced in current location.");
    pub const SUPER_CAN_ONLY_BE_REFERENCED_IN_A_DERIVED_CLASS: DiagnosticMessage = diag!(2335, Error, "'super' can only be referenced in a derived class.");
    pub const SUPER_CANNOT_BE_REFERENCED_IN_CONSTRUCTOR_ARGUMENTS: DiagnosticMessage = diag!(2336, Error, "Super calls are not permitted outside constructors or in nested functions inside constructors.");
    pub const SUPER_PROPERTY_ACCESS_IS_PERMITTED_ONLY_IN_A_CONSTRUCTOR: DiagnosticMessage = diag!(2338, Error, "'super' property access is permitted only in a constructor, member function, or member accessor of a derived class.");
    pub const PROPERTY_0_DOES_NOT_EXIST_ON_TYPE_1: DiagnosticMessage = diag!(2339, Error, "Property '{0}' does not exist on type '{1}'.");
    pub const EACH_MEMBER_OF_THE_UNION_TYPE_0_HAS_SIGNATURES: DiagnosticMessage = diag!(2349, Error, "This expression is not callable.");
    pub const CANNOT_INVOKE_AN_EXPRESSION_WHOSE_TYPE_LACKS_A_CALL_SIGNATURE: DiagnosticMessage = diag!(2349, Error, "This expression is not callable.");
    pub const THIS_EXPRESSION_IS_NOT_CONSTRUCTABLE: DiagnosticMessage = diag!(2351, Error, "This expression is not constructable.");
    pub const A_FUNCTION_WHOSE_DECLARED_TYPE_IS_NEITHER_UNDEFINED_NOR_VOID_MUST_RETURN_A_VALUE: DiagnosticMessage = diag!(2355, Error, "A function whose declared type is neither 'undefined', 'void', nor 'any' must return a value.");
    pub const THE_LEFT_HAND_SIDE_OF_AN_ARITHMETIC_OPERATION_MUST_BE_OF_TYPE_ANY_NUMBER_BIGINT_OR_AN_ENUM_TYPE: DiagnosticMessage = diag!(2362, Error, "The left-hand side of an arithmetic operation must be of type 'any', 'number', 'bigint' or an enum type.");
    pub const THE_RIGHT_HAND_SIDE_OF_AN_ARITHMETIC_OPERATION_MUST_BE_OF_TYPE_ANY_NUMBER_BIGINT_OR_AN_ENUM_TYPE: DiagnosticMessage = diag!(2363, Error, "The right-hand side of an arithmetic operation must be of type 'any', 'number', 'bigint' or an enum type.");
    pub const THE_LEFT_HAND_SIDE_OF_AN_ASSIGNMENT_EXPRESSION_MUST_BE_A_VARIABLE: DiagnosticMessage = diag!(2364, Error, "The left-hand side of an assignment expression must be a variable or a property access.");
    pub const OPERATOR_0_CANNOT_BE_APPLIED_TO_TYPES_1_AND_2: DiagnosticMessage = diag!(2365, Error, "Operator '{0}' cannot be applied to types '{1}' and '{2}'.");
    pub const A_SUPER_CALL_MUST_BE_THE_FIRST_STATEMENT_IN_THE_CONSTRUCTOR: DiagnosticMessage = diag!(2376, Error, "A 'super' call must be the first statement in the constructor to refer to 'super' or 'this' when a derived class contains initialized properties, parameter properties, or private identifiers.");
    pub const CONSTRUCTORS_FOR_DERIVED_CLASSES_MUST_CONTAIN_A_SUPER_CALL: DiagnosticMessage = diag!(2377, Error, "Constructors for derived classes must contain a 'super' call.");
    pub const A_GET_ACCESSOR_MUST_RETURN_A_VALUE_2: DiagnosticMessage = diag!(2378, Error, "A 'get' accessor must return a value.");
    pub const GETTER_AND_SETTER_ACCESSORS_DO_NOT_AGREE_IN_VISIBILITY: DiagnosticMessage = diag!(2379, Error, "Getter and setter accessors do not agree in visibility.");
    pub const OVERLOAD_SIGNATURES_MUST_ALL_BE_AMBIENT_OR_NON_AMBIENT: DiagnosticMessage = diag!(2384, Error, "Overload signatures must all be ambient or non-ambient.");
    pub const OVERLOAD_SIGNATURES_MUST_ALL_BE_PUBLIC_PRIVATE_OR_PROTECTED: DiagnosticMessage = diag!(2385, Error, "Overload signatures must all be public, private or protected.");
    pub const OVERLOAD_SIGNATURES_MUST_ALL_BE_OPTIONAL_OR_REQUIRED: DiagnosticMessage = diag!(2386, Error, "Overload signatures must all be optional or required.");
    pub const FUNCTION_OVERLOAD_MUST_NOT_BE_STATIC: DiagnosticMessage = diag!(2387, Error, "Function overload must not be static.");
    pub const INDIVIDUAL_DECLARATIONS_IN_MERGED_DECLARATION_0_MUST_BE_ALL_EXPORTED_OR_ALL_LOCAL: DiagnosticMessage = diag!(2395, Error, "Individual declarations in merged declaration '{0}' must be all exported or all local.");
    pub const A_NAMESPACE_DECLARATION_IS_ONLY_ALLOWED_AT_THE_TOP_LEVEL_OF_A_NAMESPACE_OR_MODULE: DiagnosticMessage = diag!(2434, Error, "A namespace declaration is only allowed at the top level of a namespace or module.");
    pub const THE_TYPE_RETURNED_BY_THE_0_METHOD_OF_AN_ASYNC_ITERATOR_MUST_BE_A_PROMISE: DiagnosticMessage = diag!(2547, Error, "The type returned by the '{0}()' method of an async iterator must be a promise for a type with a 'value' property.");
    pub const TYPE_0_IS_NOT_AN_ARRAY_TYPE: DiagnosticMessage = diag!(2461, Error, "Type '{0}' is not an array type.");
    pub const REST_TYPES_MAY_ONLY_BE_CREATED_FROM_OBJECT_TYPES: DiagnosticMessage = diag!(2700, Error, "Rest types may only be created from object types.");

    // ========================================================================
    // More semantic errors (2400-2599)
    // ========================================================================
    pub const THE_LEFT_HAND_SIDE_OF_AN_INSTANCEOF_EXPRESSION_MUST_BE_OF_TYPE_ANY: DiagnosticMessage = diag!(2358, Error, "The left-hand side of an 'instanceof' expression must be of type 'any', an object type or a type parameter.");
    pub const THE_RIGHT_HAND_SIDE_OF_AN_INSTANCEOF_EXPRESSION_MUST_BE_OF_TYPE_ANY: DiagnosticMessage = diag!(2359, Error, "The right-hand side of an 'instanceof' expression must be of type 'any' or of a type assignable to the 'Function' interface type.");
    pub const THE_LEFT_HAND_SIDE_OF_AN_IN_EXPRESSION_MUST_BE_A_PRIVATE_IDENTIFIER: DiagnosticMessage = diag!(2360, Error, "The left-hand side of an 'in' expression must be a private identifier or of type 'string', 'number', or 'symbol'.");
    pub const THE_RIGHT_HAND_SIDE_OF_AN_IN_EXPRESSION_MUST_NOT_BE_A_PRIMITIVE: DiagnosticMessage = diag!(2361, Error, "The right-hand side of an 'in' expression must not be a primitive.");
    pub const ARGUMENT_OF_TYPE_0_IS_NOT_ASSIGNABLE_TO_PARAMETER_OF_TYPE_1: DiagnosticMessage = diag!(2345, Error, "Argument of type '{0}' is not assignable to parameter of type '{1}'.");
    pub const RETURN_TYPE_OF_PUBLIC_METHOD_FROM_EXPORTED_CLASS_HAS_OR_IS_USING_NAME_0: DiagnosticMessage = diag!(4055, Error, "Return type of public method from exported class has or is using name '{0}' from external module {1} but cannot be named.");
    pub const VARIABLE_0_IS_USED_BEFORE_BEING_ASSIGNED: DiagnosticMessage = diag!(2454, Error, "Variable '{0}' is used before being assigned.");
    pub const TYPE_OF_AWAIT_OPERAND_MUST_EITHER_BE_A_VALID_PROMISE: DiagnosticMessage = diag!(2770, Error, "Type of 'await' operand must either be a valid promise or must not contain a callable 'then' member.");
    pub const TYPE_0_CAN_ONLY_BE_ITERATED_THROUGH_WHEN_USING_DOWNLEVEL_ITERATION: DiagnosticMessage = diag!(2802, Error, "Type '{0}' can only be iterated through when using the '--downlevelIteration' flag or with a '--target' of 'es2015' or higher.");
    pub const OBJECT_IS_POSSIBLY_NULL: DiagnosticMessage = diag!(2531, Error, "Object is possibly 'null'.");
    pub const OBJECT_IS_POSSIBLY_UNDEFINED: DiagnosticMessage = diag!(2532, Error, "Object is possibly 'undefined'.");
    pub const OBJECT_IS_POSSIBLY_NULL_OR_UNDEFINED: DiagnosticMessage = diag!(2533, Error, "Object is possibly 'null' or 'undefined'.");
    pub const A_FUNCTION_THAT_IS_CALLED_WITH_THE_NEW_KEYWORD_CANNOT_HAVE_A_THIS_TYPE_THAT_IS_VOID: DiagnosticMessage = diag!(2679, Error, "A function that is called with the 'new' keyword cannot have a 'this' type that is 'void'.");
    pub const EXPECTED_0_ARGUMENTS_BUT_GOT_1: DiagnosticMessage = diag!(2554, Error, "Expected {0} arguments, but got {1}.");
    pub const EXPECTED_AT_LEAST_0_ARGUMENTS_BUT_GOT_1: DiagnosticMessage = diag!(2555, Error, "Expected at least {0} arguments, but got {1}.");
    pub const EXPECTED_0_TYPE_ARGUMENTS_BUT_GOT_1: DiagnosticMessage = diag!(2558, Error, "Expected {0} type arguments, but got {1}.");
    pub const TYPE_0_HAS_NO_PROPERTIES_IN_COMMON_WITH_TYPE_1: DiagnosticMessage = diag!(2559, Error, "Type '{0}' has no properties in common with type '{1}'.");
    pub const VALUE_OF_TYPE_0_HAS_NO_PROPERTIES_IN_COMMON_WITH_TYPE_1: DiagnosticMessage = diag!(2560, Error, "Value of type '{0}' has no properties in common with type '{1}'. Did you mean to call it?");
    pub const BASE_CONSTRUCTORS_MUST_ALL_HAVE_THE_SAME_RETURN_TYPE: DiagnosticMessage = diag!(2510, Error, "Base constructors must all have the same return type.");
    pub const CANNOT_ASSIGN_TO_0_BECAUSE_IT_IS_A_READ_ONLY_PROPERTY: DiagnosticMessage = diag!(2540, Error, "Cannot assign to '{0}' because it is a read-only property.");
    pub const _0_INDEX_TYPE_1_IS_NOT_ASSIGNABLE_TO_2_INDEX_TYPE_3: DiagnosticMessage = diag!(2413, Error, "'{0}' index type '{1}' is not assignable to '{2}' index type '{3}'.");
    pub const CLASS_0_INCORRECTLY_EXTENDS_BASE_CLASS_1: DiagnosticMessage = diag!(2415, Error, "Class '{0}' incorrectly extends base class '{1}'.");
    pub const CLASS_0_INCORRECTLY_IMPLEMENTS_INTERFACE_1: DiagnosticMessage = diag!(2420, Error, "Class '{0}' incorrectly implements interface '{1}'.");
    pub const A_CLASS_CAN_ONLY_IMPLEMENT_AN_OBJECT_TYPE_OR_INTERSECTION_OF_OBJECT_TYPES: DiagnosticMessage = diag!(2422, Error, "A class can only implement an object type or intersection of object types with statically known members.");
    pub const CLASS_STATIC_SIDE_0_INCORRECTLY_EXTENDS_BASE_CLASS_STATIC_SIDE_1: DiagnosticMessage = diag!(2417, Error, "Class static side '{0}' incorrectly extends base class static side '{1}'.");
    pub const TYPE_NAME_0_IN_EXTENDS_CLAUSE_DOES_NOT_REFERENCE_CONSTRUCTOR_FUNCTION_FOR_0: DiagnosticMessage = diag!(2419, Error, "Type name '{0}' in extends clause does not reference constructor function for '{0}'.");
    pub const CANNOT_ASSIGN_TO_0_BECAUSE_IT_IS_NOT_A_VARIABLE: DiagnosticMessage = diag!(2539, Error, "Cannot assign to '{0}' because it is not a variable.");
    pub const CANNOT_ASSIGN_TO_0_BECAUSE_IT_IS_A_CONSTANT: DiagnosticMessage = diag!(2588, Error, "Cannot assign to '{0}' because it is a constant.");
    pub const THE_OPERAND_OF_AN_INCREMENT_OR_DECREMENT_OPERATOR_MUST_BE_A_VARIABLE_OR_A_PROPERTY_ACCESS: DiagnosticMessage = diag!(2357, Error, "The operand of an increment or decrement operator must be a variable or a property access.");
    pub const NO_OVERLOAD_MATCHES_THIS_CALL: DiagnosticMessage = diag!(2769, Error, "No overload matches this call.");
    pub const PROPERTY_0_IS_MISSING_IN_TYPE_1_BUT_REQUIRED_IN_TYPE_2: DiagnosticMessage = diag!(2741, Error, "Property '{0}' is missing in type '{1}' but required in type '{2}'.");
    pub const TYPE_0_HAS_NO_CALL_SIGNATURES: DiagnosticMessage = diag!(2757, Error, "Type '{0}' has no call signatures.");
    pub const TYPE_0_HAS_NO_CONSTRUCT_SIGNATURES: DiagnosticMessage = diag!(2761, Error, "Type '{0}' has no construct signatures.");
    pub const CANNOT_FIND_NAME_0_DID_YOU_MEAN_1: DiagnosticMessage = diag!(2552, Error, "Cannot find name '{0}'. Did you mean '{1}'?");
    pub const CANNOT_FIND_NAME_0_DID_YOU_MEAN_THE_INSTANCE_MEMBER_THIS_0: DiagnosticMessage = diag!(2663, Error, "Cannot find name '{0}'. Did you mean the instance member 'this.{0}'?");
    pub const PROPERTY_0_HAS_NO_INITIALIZER_AND_IS_NOT_DEFINITELY_ASSIGNED_IN_THE_CONSTRUCTOR: DiagnosticMessage = diag!(2564, Error, "Property '{0}' has no initializer and is not definitely assigned in the constructor.");
    pub const TYPE_ALIAS_0_CIRCULARLY_REFERENCES_ITSELF: DiagnosticMessage = diag!(2456, Error, "Type alias '{0}' circularly references itself.");
    pub const AN_ENUM_MEMBER_CANNOT_HAVE_A_NUMERIC_NAME: DiagnosticMessage = diag!(2452, Error, "An enum member cannot have a numeric name.");

    // ========================================================================
    // Additional semantic errors (2600+)
    // ========================================================================
    pub const JSX_ELEMENT_0_HAS_NO_CORRESPONDING_CLOSING_TAG: DiagnosticMessage = diag!(17008, Error, "JSX element '{0}' has no corresponding closing tag.");
    pub const EXPECTED_CORRESPONDING_JSX_CLOSING_TAG_FOR_0: DiagnosticMessage = diag!(17002, Error, "Expected corresponding JSX closing tag for '{0}'.");
    pub const JSX_EXPRESSIONS_MUST_HAVE_ONE_PARENT_ELEMENT: DiagnosticMessage = diag!(2657, Error, "JSX expressions must have one parent element.");
    pub const CANNOT_USE_JSX_UNLESS_THE_JSX_FLAG_IS_PROVIDED: DiagnosticMessage = diag!(17004, Error, "Cannot use JSX unless the '--jsx' flag is provided.");
    pub const THE_RETURN_TYPE_OF_A_JSX_ELEMENT_CONSTRUCTOR_MUST_RETURN_AN_OBJECT_TYPE: DiagnosticMessage = diag!(2601, Error, "The return type of a JSX element constructor must return an object type.");

    // ========================================================================
    // Module errors (2700-2799)
    // ========================================================================
    pub const CANNOT_FIND_MODULE_0_DID_YOU_MEAN_TO_SET_THE_MODULE_RESOLUTION_OPTION_TO_NODENEXT: DiagnosticMessage = diag!(2792, Error, "Cannot find module '{0}'. Did you mean to set the 'moduleResolution' option to 'nodenext', or to add aliases to the 'paths' option?");
    pub const THE_CURRENT_FILE_IS_A_COMMONJS_MODULE_WHOSE_IMPORTS_WILL_PRODUCE_REQUIRE_CALLS: DiagnosticMessage = diag!(1479, Error, "The current file is a CommonJS module whose imports will produce 'require' calls; however, the referenced file is an ECMAScript module and cannot be imported with 'require'.");
    pub const ESM_SYNTAX_IS_NOT_ALLOWED_IN_A_COMMONJS_MODULE_WHEN_MODULE_IS_SET_TO_PRESERVE: DiagnosticMessage = diag!(1293, Error, "ESM syntax is not allowed in a CommonJS module when 'module' is set to 'preserve'.");

    // ========================================================================
    // Suggestion diagnostics (6000+)
    // ========================================================================
    pub const VARIABLE_0_IMPLICITLY_HAS_AN_0_TYPE: DiagnosticMessage = diag!(7005, Error, "Variable '{0}' implicitly has an '{1}' type.");
    pub const PARAMETER_0_IMPLICITLY_HAS_AN_0_TYPE: DiagnosticMessage = diag!(7006, Error, "Parameter '{0}' implicitly has an '{1}' type.");
    pub const MEMBER_0_IMPLICITLY_HAS_AN_0_TYPE: DiagnosticMessage = diag!(7008, Error, "Member '{0}' implicitly has an '{1}' type.");
    pub const NEW_EXPRESSION_WHOSE_TARGET_LACKS_A_CONSTRUCT_SIGNATURE_IMPLICITLY_HAS_AN_ANY_TYPE: DiagnosticMessage = diag!(7009, Error, "'new' expression, whose target lacks a construct signature, implicitly has an 'any' type.");
    pub const _0_WHICH_LACKS_RETURN_TYPE_ANNOTATION_IMPLICITLY_HAS_AN_1_RETURN_TYPE: DiagnosticMessage = diag!(7010, Error, "'{0}', which lacks return-type annotation, implicitly has an '{1}' return type.");
    pub const FUNCTION_EXPRESSION_WHICH_LACKS_RETURN_TYPE_ANNOTATION_IMPLICITLY_HAS_AN_0_RETURN_TYPE: DiagnosticMessage = diag!(7011, Error, "Function expression, which lacks return-type annotation, implicitly has an '{0}' return type.");
    pub const THIS_IMPLICITLY_HAS_TYPE_ANY_BECAUSE_IT_DOES_NOT_HAVE_A_TYPE_ANNOTATION: DiagnosticMessage = diag!(2683, Error, "'this' implicitly has type 'any' because it does not have a type annotation.");
    pub const ELEMENT_IMPLICITLY_HAS_AN_ANY_TYPE_BECAUSE_EXPRESSION_OF_TYPE_0_CANT_BE_USED_TO_INDEX_TYPE_1: DiagnosticMessage = diag!(7053, Error, "Element implicitly has an 'any' type because expression of type '{0}' can't be used to index type '{1}'.");
    pub const ELEMENT_IMPLICITLY_HAS_AN_ANY_TYPE_BECAUSE_TYPE_0_HAS_NO_INDEX_SIGNATURE: DiagnosticMessage = diag!(7017, Error, "Element implicitly has an 'any' type because type '{0}' has no index signature.");
    pub const OBJECT_LITERAL_MAY_ONLY_SPECIFY_KNOWN_PROPERTIES_AND_0_DOES_NOT_EXIST_IN_TYPE_1: DiagnosticMessage = diag!(2353, Error, "Object literal may only specify known properties, and '{0}' does not exist in type '{1}'.");
    pub const NO_OVERLOAD_EXPECTS_0_ARGUMENTS: DiagnosticMessage = diag!(2575, Error, "No overload expects {0} arguments, but overloads do exist that expect either {1} or {2} arguments.");
    pub const _0_IS_DEFINED_AS_AN_ACCESSOR_IN_CLASS_1_BUT_IS_OVERRIDDEN_HERE_IN_2_AS_AN_INSTANCE_PROPERTY: DiagnosticMessage = diag!(2610, Error, "'{0}' is defined as an accessor in class '{1}', but is overridden here in '{2}' as an instance property.");
    pub const _0_IS_DEFINED_AS_A_PROPERTY_IN_CLASS_1_BUT_IS_OVERRIDDEN_HERE_IN_2_AS_AN_ACCESSOR: DiagnosticMessage = diag!(2611, Error, "'{0}' is defined as a property in class '{1}', but is overridden here in '{2}' as an accessor.");
    pub const PROPERTY_0_IN_TYPE_1_IS_NOT_ASSIGNABLE_TO_THE_SAME_PROPERTY_IN_BASE_TYPE_2: DiagnosticMessage = diag!(2416, Error, "Property '{0}' in type '{1}' is not assignable to the same property in base type '{2}'.");

    // ========================================================================
    // Declaration emit errors (4000-4099)
    // ========================================================================
    pub const TYPE_OF_PROPERTY_0_CIRCULARLY_REFERENCES_ITSELF_IN_MAPPED_TYPE_1: DiagnosticMessage = diag!(2615, Error, "Type of property '{0}' circularly references itself in mapped type '{1}'.");
    pub const RETURN_TYPE_OF_PUBLIC_STATIC_METHOD_FROM_EXPORTED_CLASS: DiagnosticMessage = diag!(4060, Error, "Return type of public static method from exported class has or is using private name '{0}'.");
    pub const RETURN_TYPE_OF_PUBLIC_METHOD_FROM_EXPORTED_CLASS: DiagnosticMessage = diag!(4056, Error, "Return type of public method from exported class has or is using private name '{0}'.");
    pub const PROPERTY_0_OF_EXPORTED_CLASS_EXPRESSION_MAY_NOT_BE_PRIVATE: DiagnosticMessage = diag!(4094, Error, "Property '{0}' of exported class expression may not be private or protected.");
    pub const THE_INFERRED_TYPE_OF_0_CANNOT_BE_NAMED_WITHOUT_A_REFERENCE_TO_1: DiagnosticMessage = diag!(9010, Error, "The inferred type of '{0}' cannot be named without a reference to '{1}'. This is likely not portable. A type annotation is necessary.");

    // ========================================================================
    // Configuration errors (5000-5099)
    // ========================================================================
    pub const UNKNOWN_COMPILER_OPTION_0: DiagnosticMessage = diag!(5023, Error, "Unknown compiler option '{0}'.");
    pub const COMPILER_OPTION_0_REQUIRES_A_VALUE_OF_TYPE_1: DiagnosticMessage = diag!(5024, Error, "Compiler option '{0}' requires a value of type {1}.");
    pub const COULD_NOT_WRITE_FILE_0_COLON_1: DiagnosticMessage = diag!(5033, Error, "Could not write file '{0}': {1}.");
    pub const OPTION_PROJECT_CANNOT_BE_MIXED_WITH_SOURCE_FILES: DiagnosticMessage = diag!(5042, Error, "Option 'project' cannot be mixed with source files on a command line.");
    pub const OPTION_ISOLATEDMODULES_CAN_ONLY_BE_USED_WHEN_MODULE_IS_PROVIDED: DiagnosticMessage = diag!(5047, Error, "Option 'isolatedModules' can only be used when either option '--module' is provided or option 'target' is 'ES2015' or higher.");
    pub const OPTION_0_CAN_ONLY_BE_USED_WHEN_MODULE_IS_SET_TO_COMMONJS: DiagnosticMessage = diag!(5071, Error, "Option '{0}' can only be used when 'module' is set to 'preserve' or to 'es2015' or later.");
    pub const CANNOT_FIND_A_TSCONFIG_JSON_FILE_AT_THE_SPECIFIED_DIRECTORY_0: DiagnosticMessage = diag!(5057, Error, "Cannot find a tsconfig.json file at the specified directory: '{0}'.");
    pub const THE_FILES_LIST_IN_CONFIG_FILE_0_IS_EMPTY: DiagnosticMessage = diag!(18002, Error, "The 'files' list in config file '{0}' is empty.");
    pub const NO_INPUTS_WERE_FOUND_IN_CONFIG_FILE_0: DiagnosticMessage = diag!(18003, Error, "No inputs were found in config file '{0}'. Specified 'include' paths were '{1}' and 'exclude' paths were '{2}'.");
    pub const FILE_0_NOT_FOUND: DiagnosticMessage = diag!(6053, Error, "File '{0}' not found.");
    pub const FILE_0_HAS_AN_UNSUPPORTED_EXTENSION: DiagnosticMessage = diag!(6054, Error, "File '{0}' has an unsupported extension. The only supported extensions are {1}.");

    // ========================================================================
    // async/await/generator errors
    // ========================================================================
    pub const AN_ASYNC_FUNCTION_OR_METHOD_MUST_RETURN_A_PROMISE: DiagnosticMessage = diag!(1064, Error, "The return type of an async function or method must be the global Promise<T> type.");
    pub const THE_RETURN_TYPE_OF_AN_ASYNC_FUNCTION_MUST_EITHER_BE_A_VALID_PROMISE_OR_MUST_NOT_CONTAIN_A_CALLABLE_THEN_MEMBER: DiagnosticMessage = diag!(1058, Error, "The return type of an async function must either be a valid promise or must not contain a callable 'then' member.");
    pub const RETURN_TYPE_OF_ASYNC_ARROW_FUNCTION_MUST_EITHER_BE: DiagnosticMessage = diag!(1057, Error, "The return type of an async function or method must be the global Promise<T> type. Did you mean to write 'Promise<{0}>'?");
    pub const AN_ASYNC_FUNCTION_OR_METHOD_IN_ES5_REQUIRES_THE_PROMISE_CONSTRUCTOR: DiagnosticMessage = diag!(2705, Error, "An async function or method in ES5/ES3 requires the 'Promise' constructor. Make sure you have a declaration for the 'Promise' constructor or include 'ES2015' in your '--lib' option.");
    pub const AWAIT_EXPRESSIONS_ARE_ONLY_ALLOWED_WITHIN_ASYNC_FUNCTIONS: DiagnosticMessage = diag!(1308, Error, "'await' expressions are only allowed within async functions and at the top levels of modules.");
    pub const AWAIT_EXPRESSION_IS_ONLY_ALLOWED_WITHIN_AN_ASYNC_FUNCTION: DiagnosticMessage = diag!(1103, Error, "'await' expression is only allowed within an async function.");
    pub const AWAIT_EXPRESSIONS_CANNOT_BE_USED_IN_A_PARAMETER_INITIALIZER: DiagnosticMessage = diag!(2524, Error, "'await' expressions cannot be used in a parameter initializer.");
    pub const YIELD_EXPRESSIONS_CANNOT_BE_USED_IN_A_PARAMETER_INITIALIZER: DiagnosticMessage = diag!(2523, Error, "'yield' expressions cannot be used in a parameter initializer.");

    // ========================================================================
    // Decorator errors
    // ========================================================================
    pub const UNABLE_TO_RESOLVE_SIGNATURE_OF_CLASS_DECORATOR_WHEN_CALLED_AS_AN_EXPRESSION: DiagnosticMessage = diag!(1238, Error, "Unable to resolve signature of class decorator when called as an expression.");
    pub const UNABLE_TO_RESOLVE_SIGNATURE_OF_PARAMETER_DECORATOR_WHEN_CALLED_AS_AN_EXPRESSION: DiagnosticMessage = diag!(1239, Error, "Unable to resolve signature of parameter decorator when called as an expression.");
    pub const UNABLE_TO_RESOLVE_SIGNATURE_OF_PROPERTY_DECORATOR_WHEN_CALLED_AS_AN_EXPRESSION: DiagnosticMessage = diag!(1240, Error, "Unable to resolve signature of property decorator when called as an expression.");
    pub const UNABLE_TO_RESOLVE_SIGNATURE_OF_METHOD_DECORATOR_WHEN_CALLED_AS_AN_EXPRESSION: DiagnosticMessage = diag!(1241, Error, "Unable to resolve signature of method decorator when called as an expression.");
    pub const EXPERIMENTAL_SUPPORT_FOR_DECORATORS_IS_A_FEATURE_THAT_IS_SUBJECT_TO_CHANGE: DiagnosticMessage = diag!(1219, Error, "Experimental support for decorators is a feature that is subject to change in a future release. Set the 'experimentalDecorators' option in your 'tsconfig' or 'jsconfig' to remove this warning.");

    // ========================================================================
    // Abstract class errors
    // ========================================================================
    pub const ABSTRACT_METHOD_0_IN_CLASS_1_CANNOT_BE_ACCESSED_VIA_SUPER_EXPRESSION: DiagnosticMessage = diag!(2513, Error, "Abstract method '{0}' in class '{1}' cannot be accessed via super expression.");
    pub const NON_ABSTRACT_CLASS_0_DOES_NOT_IMPLEMENT_INHERITED_ABSTRACT_MEMBER_1_FROM_CLASS_2: DiagnosticMessage = diag!(2515, Error, "Non-abstract class '{0}' does not implement inherited abstract member '{1}' from class '{2}'.");
    pub const ABSTRACT_METHODS_CAN_ONLY_APPEAR_WITHIN_AN_ABSTRACT_CLASS: DiagnosticMessage = diag!(1244, Error, "Abstract methods can only appear within an abstract class.");
    pub const ABSTRACT_METHOD_0_CANNOT_HAVE_AN_IMPLEMENTATION: DiagnosticMessage = diag!(1245, Error, "Method '{0}' cannot have an implementation because it is marked abstract.");
    pub const CANNOT_CREATE_AN_INSTANCE_OF_AN_ABSTRACT_CLASS: DiagnosticMessage = diag!(2511, Error, "Cannot create an instance of an abstract class.");

    // ========================================================================
    // Control flow errors
    // ========================================================================
    pub const NOT_ALL_CODE_PATHS_RETURN_A_VALUE: DiagnosticMessage = diag!(7030, Error, "Not all code paths return a value.");
    pub const UNREACHABLE_CODE_DETECTED: DiagnosticMessage = diag!(7027, Error, "Unreachable code detected.");
    pub const UNUSED_LABEL: DiagnosticMessage = diag!(7028, Warning, "Unused label.");
    pub const FALLTHROUGH_CASE_IN_SWITCH: DiagnosticMessage = diag!(7029, Error, "Fallthrough case in switch.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_message() {
        let msg = format_message("Type '{0}' is not assignable to type '{1}'.", &["number", "string"]);
        assert_eq!(msg, "Type 'number' is not assignable to type 'string'.");
    }

    #[test]
    fn test_format_message_no_args() {
        let msg = format_message("Unexpected token.", &[]);
        assert_eq!(msg, "Unexpected token.");
    }

    #[test]
    fn test_format_message_three_args() {
        let msg = format_message("Operator '{0}' cannot be applied to types '{1}' and '{2}'.", &["+", "string", "boolean"]);
        assert_eq!(msg, "Operator '+' cannot be applied to types 'string' and 'boolean'.");
    }

    #[test]
    fn test_diagnostic_display() {
        let diag = Diagnostic::with_location(
            "test.ts".to_string(),
            TextSpan::new(10, 5),
            &messages::CANNOT_FIND_NAME_0,
            &["foo"],
        );
        let display = format!("{}", diag);
        assert!(display.contains("test.ts"));
        assert!(display.contains("TS2304"));
        assert!(display.contains("foo"));
    }

    #[test]
    fn test_diagnostic_without_location() {
        let diag = Diagnostic::new(&messages::UNEXPECTED_TOKEN, &[]);
        assert!(diag.file.is_none());
        assert!(diag.span.is_none());
        assert_eq!(diag.code, 1012);
        assert!(diag.is_error());
    }

    #[test]
    fn test_diagnostic_collection() {
        let mut collection = DiagnosticCollection::new();
        assert!(collection.is_empty());
        assert_eq!(collection.len(), 0);

        collection.add(Diagnostic::new(&messages::UNEXPECTED_TOKEN, &[]));
        assert!(collection.has_errors());
        assert_eq!(collection.error_count(), 1);
        assert_eq!(collection.len(), 1);
    }

    #[test]
    fn test_diagnostic_collection_sort() {
        let mut collection = DiagnosticCollection::new();
        collection.add(Diagnostic::with_location(
            "b.ts".to_string(),
            TextSpan::new(10, 1),
            &messages::UNEXPECTED_TOKEN,
            &[],
        ));
        collection.add(Diagnostic::with_location(
            "a.ts".to_string(),
            TextSpan::new(5, 1),
            &messages::IDENTIFIER_EXPECTED,
            &[],
        ));
        collection.sort();
        assert_eq!(collection.diagnostics()[0].file.as_deref(), Some("a.ts"));
        assert_eq!(collection.diagnostics()[1].file.as_deref(), Some("b.ts"));
    }

    #[test]
    fn test_diagnostic_with_related() {
        let primary = Diagnostic::new(&messages::TYPE_0_IS_NOT_ASSIGNABLE_TO_TYPE_1, &["number", "string"]);
        let related = Diagnostic::new(&messages::PROPERTY_0_IS_MISSING_IN_TYPE_1_BUT_REQUIRED_IN_TYPE_2, &["x", "A", "B"]);
        let combined = primary.with_related(related);
        assert_eq!(combined.related_information.len(), 1);
    }
}
