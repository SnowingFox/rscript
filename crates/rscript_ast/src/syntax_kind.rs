//! SyntaxKind enum - all token and node kinds in the TypeScript AST.
//!
//! This is a faithful port of TypeScript's SyntaxKind enum with all 300+ variants.

/// The kind of a syntax token or node in the AST.
/// This is a 1:1 port of TypeScript's SyntaxKind enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u16)]
pub enum SyntaxKind {
    // ========================================================================
    // Tokens
    // ========================================================================
    Unknown = 0,
    EndOfFileToken = 1,

    // Trivia
    SingleLineCommentTrivia = 2,
    MultiLineCommentTrivia = 3,
    NewLineTrivia = 4,
    WhitespaceTrivia = 5,
    ShebangTrivia = 6,
    ConflictMarkerTrivia = 7,

    // Literals
    NumericLiteral = 8,
    BigIntLiteral = 9,
    StringLiteral = 10,
    JsxText = 11,
    JsxTextAllWhiteSpaces = 12,
    RegularExpressionLiteral = 13,
    NoSubstitutionTemplateLiteral = 14,

    // Pseudo-literals (template)
    TemplateHead = 15,
    TemplateMiddle = 16,
    TemplateTail = 17,

    // Punctuation
    OpenBraceToken = 18,
    CloseBraceToken = 19,
    OpenParenToken = 20,
    CloseParenToken = 21,
    OpenBracketToken = 22,
    CloseBracketToken = 23,
    DotToken = 24,
    DotDotDotToken = 25,
    SemicolonToken = 26,
    CommaToken = 27,
    QuestionDotToken = 28,
    LessThanToken = 29,
    LessThanSlashToken = 30,
    GreaterThanToken = 31,
    LessThanEqualsToken = 32,
    GreaterThanEqualsToken = 33,
    EqualsEqualsToken = 34,
    ExclamationEqualsToken = 35,
    EqualsEqualsEqualsToken = 36,
    ExclamationEqualsEqualsToken = 37,
    EqualsGreaterThanToken = 38,
    PlusToken = 39,
    MinusToken = 40,
    AsteriskToken = 41,
    AsteriskAsteriskToken = 42,
    SlashToken = 43,
    PercentToken = 44,
    PlusPlusToken = 45,
    MinusMinusToken = 46,
    LessThanLessThanToken = 47,
    GreaterThanGreaterThanToken = 48,
    GreaterThanGreaterThanGreaterThanToken = 49,
    AmpersandToken = 50,
    BarToken = 51,
    CaretToken = 52,
    ExclamationToken = 53,
    TildeToken = 54,
    AmpersandAmpersandToken = 55,
    BarBarToken = 56,
    QuestionToken = 57,
    ColonToken = 58,
    AtToken = 59,
    QuestionQuestionToken = 60,
    BacktickToken = 61,
    HashToken = 62,

    // Assignments
    EqualsToken = 63,
    PlusEqualsToken = 64,
    MinusEqualsToken = 65,
    AsteriskEqualsToken = 66,
    AsteriskAsteriskEqualsToken = 67,
    SlashEqualsToken = 68,
    PercentEqualsToken = 69,
    LessThanLessThanEqualsToken = 70,
    GreaterThanGreaterThanEqualsToken = 71,
    GreaterThanGreaterThanGreaterThanEqualsToken = 72,
    AmpersandEqualsToken = 73,
    BarEqualsToken = 74,
    CaretEqualsToken = 75,
    BarBarEqualsToken = 76,
    AmpersandAmpersandEqualsToken = 77,
    QuestionQuestionEqualsToken = 78,

    // Identifiers and keywords
    Identifier = 79,

    // Reserved words
    BreakKeyword = 80,
    CaseKeyword = 81,
    CatchKeyword = 82,
    ClassKeyword = 83,
    ConstKeyword = 84,
    ContinueKeyword = 85,
    DebuggerKeyword = 86,
    DefaultKeyword = 87,
    DeleteKeyword = 88,
    DoKeyword = 89,
    ElseKeyword = 90,
    EnumKeyword = 91,
    ExportKeyword = 92,
    ExtendsKeyword = 93,
    FalseKeyword = 94,
    FinallyKeyword = 95,
    ForKeyword = 96,
    FunctionKeyword = 97,
    IfKeyword = 98,
    ImportKeyword = 99,
    InKeyword = 100,
    InstanceOfKeyword = 101,
    NewKeyword = 102,
    NullKeyword = 103,
    ReturnKeyword = 104,
    SuperKeyword = 105,
    SwitchKeyword = 106,
    ThisKeyword = 107,
    ThrowKeyword = 108,
    TrueKeyword = 109,
    TryKeyword = 110,
    TypeOfKeyword = 111,
    VarKeyword = 112,
    VoidKeyword = 113,
    WhileKeyword = 114,
    WithKeyword = 115,

    // Strict mode reserved words
    ImplementsKeyword = 116,
    InterfaceKeyword = 117,
    LetKeyword = 118,
    PackageKeyword = 119,
    PrivateKeyword = 120,
    ProtectedKeyword = 121,
    PublicKeyword = 122,
    StaticKeyword = 123,
    YieldKeyword = 124,

    // Contextual keywords
    AbstractKeyword = 125,
    AccessorKeyword = 126,
    AsKeyword = 127,
    AssertsKeyword = 128,
    AssertKeyword = 129,
    AnyKeyword = 130,
    AsyncKeyword = 131,
    AwaitKeyword = 132,
    BooleanKeyword = 133,
    ConstructorKeyword = 134,
    DeclareKeyword = 135,
    GetKeyword = 136,
    InferKeyword = 137,
    IntrinsicKeyword = 138,
    IsKeyword = 139,
    KeyOfKeyword = 140,
    ModuleKeyword = 141,
    NamespaceKeyword = 142,
    NeverKeyword = 143,
    OutKeyword = 144,
    ReadonlyKeyword = 145,
    RequireKeyword = 146,
    NumberKeyword = 147,
    ObjectKeyword = 148,
    SatisfiesKeyword = 149,
    SetKeyword = 150,
    StringKeyword = 151,
    SymbolKeyword = 152,
    TypeKeyword = 153,
    UndefinedKeyword = 154,
    UniqueKeyword = 155,
    UnknownKeyword = 156,
    UsingKeyword = 157,
    FromKeyword = 158,
    GlobalKeyword = 159,
    BigIntKeyword = 160,
    OverrideKeyword = 161,
    OfKeyword = 162,

    // ========================================================================
    // Nodes (Parsed)
    // ========================================================================

    // Names
    QualifiedName = 163,
    ComputedPropertyName = 164,

    // Signature elements
    TypeParameter = 165,
    Parameter = 166,
    Decorator = 167,

    // Type members
    PropertySignature = 168,
    PropertyDeclaration = 169,
    MethodSignature = 170,
    MethodDeclaration = 171,
    ClassStaticBlockDeclaration = 172,
    Constructor = 173,
    GetAccessor = 174,
    SetAccessor = 175,
    CallSignature = 176,
    ConstructSignature = 177,
    IndexSignature = 178,

    // Types
    TypePredicate = 179,
    TypeReference = 180,
    FunctionType = 181,
    ConstructorType = 182,
    TypeQuery = 183,
    TypeLiteral = 184,
    ArrayType = 185,
    TupleType = 186,
    OptionalType = 187,
    RestType = 188,
    UnionType = 189,
    IntersectionType = 190,
    ConditionalType = 191,
    InferType = 192,
    ParenthesizedType = 193,
    ThisType = 194,
    TypeOperator = 195,
    IndexedAccessType = 196,
    MappedType = 197,
    LiteralType = 198,
    NamedTupleMember = 199,
    TemplateLiteralType = 200,
    TemplateLiteralTypeSpan = 201,
    ImportType = 202,

    // Binding patterns
    ObjectBindingPattern = 203,
    ArrayBindingPattern = 204,
    BindingElement = 205,

    // Expressions
    ArrayLiteralExpression = 206,
    ObjectLiteralExpression = 207,
    PropertyAccessExpression = 208,
    ElementAccessExpression = 209,
    CallExpression = 210,
    NewExpression = 211,
    TaggedTemplateExpression = 212,
    TypeAssertionExpression = 213,
    ParenthesizedExpression = 214,
    FunctionExpression = 215,
    ArrowFunction = 216,
    DeleteExpression = 217,
    TypeOfExpression = 218,
    VoidExpression = 219,
    AwaitExpression = 220,
    PrefixUnaryExpression = 221,
    PostfixUnaryExpression = 222,
    BinaryExpression = 223,
    ConditionalExpression = 224,
    TemplateExpression = 225,
    YieldExpression = 226,
    SpreadElement = 227,
    ClassExpression = 228,
    OmittedExpression = 229,
    ExpressionWithTypeArguments = 230,
    AsExpression = 231,
    NonNullExpression = 232,
    MetaProperty = 233,
    SyntheticExpression = 234,
    SatisfiesExpression = 235,

    // Element
    TemplateSpan = 236,
    SemicolonClassElement = 237,

    // Statements
    Block = 238,
    EmptyStatement = 239,
    VariableStatement = 240,
    ExpressionStatement = 241,
    IfStatement = 242,
    DoStatement = 243,
    WhileStatement = 244,
    ForStatement = 245,
    ForInStatement = 246,
    ForOfStatement = 247,
    ContinueStatement = 248,
    BreakStatement = 249,
    ReturnStatement = 250,
    WithStatement = 251,
    SwitchStatement = 252,
    LabeledStatement = 253,
    ThrowStatement = 254,
    TryStatement = 255,
    DebuggerStatement = 256,
    VariableDeclaration = 257,
    VariableDeclarationList = 258,
    FunctionDeclaration = 259,
    ClassDeclaration = 260,
    InterfaceDeclaration = 261,
    TypeAliasDeclaration = 262,
    EnumDeclaration = 263,
    ModuleDeclaration = 264,
    ModuleBlock = 265,
    CaseBlock = 266,
    NamespaceExportDeclaration = 267,
    ImportEqualsDeclaration = 268,
    ImportDeclaration = 269,
    ImportClause = 270,
    NamespaceImport = 271,
    NamedImports = 272,
    ImportSpecifier = 273,
    ExportAssignment = 274,
    ExportDeclaration = 275,
    NamedExports = 276,
    NamespaceExport = 277,
    ExportSpecifier = 278,
    MissingDeclaration = 279,

    // Module references
    ExternalModuleReference = 280,

    // JSX
    JsxElement = 281,
    JsxSelfClosingElement = 282,
    JsxOpeningElement = 283,
    JsxClosingElement = 284,
    JsxFragment = 285,
    JsxOpeningFragment = 286,
    JsxClosingFragment = 287,
    JsxAttribute = 288,
    JsxAttributes = 289,
    JsxSpreadAttribute = 290,
    JsxExpression = 291,
    JsxNamespacedName = 292,

    // Clauses
    CaseClause = 293,
    DefaultClause = 294,
    HeritageClause = 295,
    CatchClause = 296,

    // Assert / Import attributes
    AssertClause = 297,
    AssertEntry = 298,
    ImportAttributes = 299,
    ImportAttribute = 300,

    // Property assignments
    PropertyAssignment = 301,
    ShorthandPropertyAssignment = 302,
    SpreadAssignment = 303,

    // Enum member
    EnumMember = 304,

    // Unparsed
    UnparsedPrologue = 305,
    UnparsedPrepend = 306,
    UnparsedText = 307,
    UnparsedInternalText = 308,
    UnparsedSyntheticReference = 309,

    // Top-level
    SourceFile = 310,
    Bundle = 311,
    UnparsedSource = 312,
    InputFiles = 313,

    // JSDoc
    JSDocTypeExpression = 314,
    JSDocNameReference = 315,
    JSDocMemberName = 316,
    JSDocAllType = 317,
    JSDocUnknownType = 318,
    JSDocNullableType = 319,
    JSDocNonNullableType = 320,
    JSDocOptionalType = 321,
    JSDocFunctionType = 322,
    JSDocVariadicType = 323,
    JSDocNamepathType = 324,
    JSDocComment = 325,
    JSDocText = 326,
    JSDocTypeLiteral = 327,
    JSDocSignature = 328,
    JSDocLink = 329,
    JSDocLinkCode = 330,
    JSDocLinkPlain = 331,
    JSDocTag = 332,
    JSDocAugmentsTag = 333,
    JSDocImplementsTag = 334,
    JSDocAuthorTag = 335,
    JSDocDeprecatedTag = 336,
    JSDocClassTag = 337,
    JSDocPublicTag = 338,
    JSDocPrivateTag = 339,
    JSDocProtectedTag = 340,
    JSDocReadonlyTag = 341,
    JSDocOverrideTag = 342,
    JSDocCallbackTag = 343,
    JSDocOverloadTag = 344,
    JSDocEnumTag = 345,
    JSDocParameterTag = 346,
    JSDocReturnTag = 347,
    JSDocThisTag = 348,
    JSDocTypeTag = 349,
    JSDocTemplateTag = 350,
    JSDocTypedefTag = 351,
    JSDocSeeTag = 352,
    JSDocPropertyTag = 353,
    JSDocThrowsTag = 354,
    JSDocSatisfiesTag = 355,
    JSDocImportTag = 356,

    // Synthesized
    SyntaxList = 357,
    NotEmittedStatement = 358,
    PartiallyEmittedExpression = 359,
    CommaListExpression = 360,
    SyntheticReferenceExpression = 361,

    // Transformation
    Count = 362,

}

// Marker constants for SyntaxKind ranges.
// These can't be enum variants because Rust doesn't allow duplicate discriminants.
impl SyntaxKind {
    pub const FIRST_ASSIGNMENT: SyntaxKind = SyntaxKind::EqualsToken;
    pub const LAST_ASSIGNMENT: SyntaxKind = SyntaxKind::QuestionQuestionEqualsToken;
    pub const FIRST_COMPOUND_ASSIGNMENT: SyntaxKind = SyntaxKind::PlusEqualsToken;
    pub const LAST_COMPOUND_ASSIGNMENT: SyntaxKind = SyntaxKind::QuestionQuestionEqualsToken;
    pub const FIRST_RESERVED_WORD: SyntaxKind = SyntaxKind::BreakKeyword;
    pub const LAST_RESERVED_WORD: SyntaxKind = SyntaxKind::WithKeyword;
    pub const FIRST_KEYWORD: SyntaxKind = SyntaxKind::BreakKeyword;
    pub const LAST_KEYWORD: SyntaxKind = SyntaxKind::OfKeyword;
    pub const FIRST_FUTURE_RESERVED_WORD: SyntaxKind = SyntaxKind::ImplementsKeyword;
    pub const LAST_FUTURE_RESERVED_WORD: SyntaxKind = SyntaxKind::YieldKeyword;
    pub const FIRST_TYPE_NODE: SyntaxKind = SyntaxKind::TypePredicate;
    pub const LAST_TYPE_NODE: SyntaxKind = SyntaxKind::ImportType;
    pub const FIRST_PUNCTUATION: SyntaxKind = SyntaxKind::OpenBraceToken;
    pub const LAST_PUNCTUATION: SyntaxKind = SyntaxKind::QuestionQuestionEqualsToken;
    pub const FIRST_TOKEN: SyntaxKind = SyntaxKind::Unknown;
    pub const LAST_TOKEN: SyntaxKind = SyntaxKind::OfKeyword;
    pub const FIRST_TRIVIA_TOKEN: SyntaxKind = SyntaxKind::SingleLineCommentTrivia;
    pub const LAST_TRIVIA_TOKEN: SyntaxKind = SyntaxKind::ConflictMarkerTrivia;
    pub const FIRST_LITERAL_TOKEN: SyntaxKind = SyntaxKind::NumericLiteral;
    pub const LAST_LITERAL_TOKEN: SyntaxKind = SyntaxKind::NoSubstitutionTemplateLiteral;
    pub const FIRST_TEMPLATE_TOKEN: SyntaxKind = SyntaxKind::NoSubstitutionTemplateLiteral;
    pub const LAST_TEMPLATE_TOKEN: SyntaxKind = SyntaxKind::TemplateTail;
    pub const FIRST_BINARY_OPERATOR: SyntaxKind = SyntaxKind::LessThanToken;
    pub const LAST_BINARY_OPERATOR: SyntaxKind = SyntaxKind::QuestionQuestionEqualsToken;
    pub const FIRST_STATEMENT: SyntaxKind = SyntaxKind::VariableStatement;
    pub const LAST_STATEMENT: SyntaxKind = SyntaxKind::VariableDeclarationList;
    pub const FIRST_NODE: SyntaxKind = SyntaxKind::QualifiedName;
    pub const FIRST_JSDOC_NODE: SyntaxKind = SyntaxKind::JSDocTypeExpression;
    pub const LAST_JSDOC_NODE: SyntaxKind = SyntaxKind::JSDocImportTag;
    pub const FIRST_JSDOC_TAG_NODE: SyntaxKind = SyntaxKind::JSDocTag;
    pub const LAST_JSDOC_TAG_NODE: SyntaxKind = SyntaxKind::JSDocImportTag;
}

impl SyntaxKind {
    /// Whether this kind represents a keyword.
    #[inline]
    pub fn is_keyword(self) -> bool {
        let v = self as u16;
        v >= SyntaxKind::BreakKeyword as u16 && v <= SyntaxKind::OfKeyword as u16
    }

    /// Whether this kind represents a punctuation token.
    #[inline]
    pub fn is_punctuation(self) -> bool {
        let v = self as u16;
        v >= SyntaxKind::OpenBraceToken as u16 && v <= SyntaxKind::QuestionQuestionEqualsToken as u16
    }

    /// Whether this kind represents a literal token.
    #[inline]
    pub fn is_literal(self) -> bool {
        let v = self as u16;
        v >= SyntaxKind::NumericLiteral as u16
            && v <= SyntaxKind::NoSubstitutionTemplateLiteral as u16
    }

    /// Whether this kind represents a template token.
    #[inline]
    pub fn is_template(self) -> bool {
        let v = self as u16;
        v >= SyntaxKind::NoSubstitutionTemplateLiteral as u16
            && v <= SyntaxKind::TemplateTail as u16
    }

    /// Whether this kind represents trivia.
    #[inline]
    pub fn is_trivia(self) -> bool {
        let v = self as u16;
        v >= SyntaxKind::SingleLineCommentTrivia as u16
            && v <= SyntaxKind::ConflictMarkerTrivia as u16
    }

    /// Whether this kind represents an assignment operator.
    #[inline]
    pub fn is_assignment_operator(self) -> bool {
        let v = self as u16;
        v >= SyntaxKind::EqualsToken as u16
            && v <= SyntaxKind::QuestionQuestionEqualsToken as u16
    }

    /// Whether this kind represents a compound assignment operator.
    #[inline]
    pub fn is_compound_assignment(self) -> bool {
        let v = self as u16;
        v >= SyntaxKind::PlusEqualsToken as u16
            && v <= SyntaxKind::QuestionQuestionEqualsToken as u16
    }

    /// Whether this kind represents a modifier keyword.
    #[inline]
    pub fn is_modifier_kind(self) -> bool {
        matches!(
            self,
            SyntaxKind::AbstractKeyword
                | SyntaxKind::AccessorKeyword
                | SyntaxKind::AsyncKeyword
                | SyntaxKind::ConstKeyword
                | SyntaxKind::DeclareKeyword
                | SyntaxKind::DefaultKeyword
                | SyntaxKind::ExportKeyword
                | SyntaxKind::InKeyword
                | SyntaxKind::OutKeyword
                | SyntaxKind::OverrideKeyword
                | SyntaxKind::PrivateKeyword
                | SyntaxKind::ProtectedKeyword
                | SyntaxKind::PublicKeyword
                | SyntaxKind::ReadonlyKeyword
                | SyntaxKind::StaticKeyword
        )
    }

    /// Whether this kind represents a type node.
    #[inline]
    pub fn is_type_node(self) -> bool {
        let v = self as u16;
        v >= SyntaxKind::TypePredicate as u16 && v <= SyntaxKind::ImportType as u16
    }

    /// Whether this kind represents a statement.
    #[inline]
    pub fn is_statement(self) -> bool {
        matches!(
            self,
            SyntaxKind::VariableStatement
                | SyntaxKind::EmptyStatement
                | SyntaxKind::ExpressionStatement
                | SyntaxKind::IfStatement
                | SyntaxKind::DoStatement
                | SyntaxKind::WhileStatement
                | SyntaxKind::ForStatement
                | SyntaxKind::ForInStatement
                | SyntaxKind::ForOfStatement
                | SyntaxKind::ContinueStatement
                | SyntaxKind::BreakStatement
                | SyntaxKind::ReturnStatement
                | SyntaxKind::WithStatement
                | SyntaxKind::SwitchStatement
                | SyntaxKind::LabeledStatement
                | SyntaxKind::ThrowStatement
                | SyntaxKind::TryStatement
                | SyntaxKind::DebuggerStatement
                | SyntaxKind::Block
                | SyntaxKind::FunctionDeclaration
                | SyntaxKind::ClassDeclaration
                | SyntaxKind::InterfaceDeclaration
                | SyntaxKind::TypeAliasDeclaration
                | SyntaxKind::EnumDeclaration
                | SyntaxKind::ModuleDeclaration
                | SyntaxKind::ImportDeclaration
                | SyntaxKind::ImportEqualsDeclaration
                | SyntaxKind::ExportDeclaration
                | SyntaxKind::ExportAssignment
                | SyntaxKind::NamespaceExportDeclaration
        )
    }

    /// Whether this kind represents a declaration.
    #[inline]
    pub fn is_declaration(self) -> bool {
        matches!(
            self,
            SyntaxKind::VariableDeclaration
                | SyntaxKind::Parameter
                | SyntaxKind::PropertyDeclaration
                | SyntaxKind::PropertySignature
                | SyntaxKind::BindingElement
                | SyntaxKind::FunctionDeclaration
                | SyntaxKind::FunctionExpression
                | SyntaxKind::ArrowFunction
                | SyntaxKind::ClassDeclaration
                | SyntaxKind::ClassExpression
                | SyntaxKind::InterfaceDeclaration
                | SyntaxKind::TypeAliasDeclaration
                | SyntaxKind::EnumDeclaration
                | SyntaxKind::EnumMember
                | SyntaxKind::ModuleDeclaration
                | SyntaxKind::ImportEqualsDeclaration
                | SyntaxKind::ImportClause
                | SyntaxKind::NamespaceImport
                | SyntaxKind::ImportSpecifier
                | SyntaxKind::ExportSpecifier
                | SyntaxKind::MethodDeclaration
                | SyntaxKind::MethodSignature
                | SyntaxKind::Constructor
                | SyntaxKind::GetAccessor
                | SyntaxKind::SetAccessor
                | SyntaxKind::TypeParameter
                | SyntaxKind::IndexSignature
        )
    }

    /// Get the keyword text for a keyword kind, or None if not a keyword.
    pub fn keyword_text(self) -> Option<&'static str> {
        match self {
            SyntaxKind::BreakKeyword => Some("break"),
            SyntaxKind::CaseKeyword => Some("case"),
            SyntaxKind::CatchKeyword => Some("catch"),
            SyntaxKind::ClassKeyword => Some("class"),
            SyntaxKind::ConstKeyword => Some("const"),
            SyntaxKind::ContinueKeyword => Some("continue"),
            SyntaxKind::DebuggerKeyword => Some("debugger"),
            SyntaxKind::DefaultKeyword => Some("default"),
            SyntaxKind::DeleteKeyword => Some("delete"),
            SyntaxKind::DoKeyword => Some("do"),
            SyntaxKind::ElseKeyword => Some("else"),
            SyntaxKind::EnumKeyword => Some("enum"),
            SyntaxKind::ExportKeyword => Some("export"),
            SyntaxKind::ExtendsKeyword => Some("extends"),
            SyntaxKind::FalseKeyword => Some("false"),
            SyntaxKind::FinallyKeyword => Some("finally"),
            SyntaxKind::ForKeyword => Some("for"),
            SyntaxKind::FunctionKeyword => Some("function"),
            SyntaxKind::IfKeyword => Some("if"),
            SyntaxKind::ImportKeyword => Some("import"),
            SyntaxKind::InKeyword => Some("in"),
            SyntaxKind::InstanceOfKeyword => Some("instanceof"),
            SyntaxKind::NewKeyword => Some("new"),
            SyntaxKind::NullKeyword => Some("null"),
            SyntaxKind::ReturnKeyword => Some("return"),
            SyntaxKind::SuperKeyword => Some("super"),
            SyntaxKind::SwitchKeyword => Some("switch"),
            SyntaxKind::ThisKeyword => Some("this"),
            SyntaxKind::ThrowKeyword => Some("throw"),
            SyntaxKind::TrueKeyword => Some("true"),
            SyntaxKind::TryKeyword => Some("try"),
            SyntaxKind::TypeOfKeyword => Some("typeof"),
            SyntaxKind::VarKeyword => Some("var"),
            SyntaxKind::VoidKeyword => Some("void"),
            SyntaxKind::WhileKeyword => Some("while"),
            SyntaxKind::WithKeyword => Some("with"),
            SyntaxKind::ImplementsKeyword => Some("implements"),
            SyntaxKind::InterfaceKeyword => Some("interface"),
            SyntaxKind::LetKeyword => Some("let"),
            SyntaxKind::PackageKeyword => Some("package"),
            SyntaxKind::PrivateKeyword => Some("private"),
            SyntaxKind::ProtectedKeyword => Some("protected"),
            SyntaxKind::PublicKeyword => Some("public"),
            SyntaxKind::StaticKeyword => Some("static"),
            SyntaxKind::YieldKeyword => Some("yield"),
            SyntaxKind::AbstractKeyword => Some("abstract"),
            SyntaxKind::AccessorKeyword => Some("accessor"),
            SyntaxKind::AsKeyword => Some("as"),
            SyntaxKind::AssertsKeyword => Some("asserts"),
            SyntaxKind::AssertKeyword => Some("assert"),
            SyntaxKind::AnyKeyword => Some("any"),
            SyntaxKind::AsyncKeyword => Some("async"),
            SyntaxKind::AwaitKeyword => Some("await"),
            SyntaxKind::BooleanKeyword => Some("boolean"),
            SyntaxKind::ConstructorKeyword => Some("constructor"),
            SyntaxKind::DeclareKeyword => Some("declare"),
            SyntaxKind::GetKeyword => Some("get"),
            SyntaxKind::InferKeyword => Some("infer"),
            SyntaxKind::IntrinsicKeyword => Some("intrinsic"),
            SyntaxKind::IsKeyword => Some("is"),
            SyntaxKind::KeyOfKeyword => Some("keyof"),
            SyntaxKind::ModuleKeyword => Some("module"),
            SyntaxKind::NamespaceKeyword => Some("namespace"),
            SyntaxKind::NeverKeyword => Some("never"),
            SyntaxKind::OutKeyword => Some("out"),
            SyntaxKind::ReadonlyKeyword => Some("readonly"),
            SyntaxKind::RequireKeyword => Some("require"),
            SyntaxKind::NumberKeyword => Some("number"),
            SyntaxKind::ObjectKeyword => Some("object"),
            SyntaxKind::SatisfiesKeyword => Some("satisfies"),
            SyntaxKind::SetKeyword => Some("set"),
            SyntaxKind::StringKeyword => Some("string"),
            SyntaxKind::SymbolKeyword => Some("symbol"),
            SyntaxKind::TypeKeyword => Some("type"),
            SyntaxKind::UndefinedKeyword => Some("undefined"),
            SyntaxKind::UniqueKeyword => Some("unique"),
            SyntaxKind::UnknownKeyword => Some("unknown"),
            SyntaxKind::UsingKeyword => Some("using"),
            SyntaxKind::FromKeyword => Some("from"),
            SyntaxKind::GlobalKeyword => Some("global"),
            SyntaxKind::BigIntKeyword => Some("bigint"),
            SyntaxKind::OverrideKeyword => Some("override"),
            SyntaxKind::OfKeyword => Some("of"),
            _ => None,
        }
    }

    /// Look up a keyword SyntaxKind from text.
    pub fn from_keyword(text: &str) -> Option<SyntaxKind> {
        match text {
            "break" => Some(SyntaxKind::BreakKeyword),
            "case" => Some(SyntaxKind::CaseKeyword),
            "catch" => Some(SyntaxKind::CatchKeyword),
            "class" => Some(SyntaxKind::ClassKeyword),
            "const" => Some(SyntaxKind::ConstKeyword),
            "continue" => Some(SyntaxKind::ContinueKeyword),
            "debugger" => Some(SyntaxKind::DebuggerKeyword),
            "default" => Some(SyntaxKind::DefaultKeyword),
            "delete" => Some(SyntaxKind::DeleteKeyword),
            "do" => Some(SyntaxKind::DoKeyword),
            "else" => Some(SyntaxKind::ElseKeyword),
            "enum" => Some(SyntaxKind::EnumKeyword),
            "export" => Some(SyntaxKind::ExportKeyword),
            "extends" => Some(SyntaxKind::ExtendsKeyword),
            "false" => Some(SyntaxKind::FalseKeyword),
            "finally" => Some(SyntaxKind::FinallyKeyword),
            "for" => Some(SyntaxKind::ForKeyword),
            "function" => Some(SyntaxKind::FunctionKeyword),
            "if" => Some(SyntaxKind::IfKeyword),
            "import" => Some(SyntaxKind::ImportKeyword),
            "in" => Some(SyntaxKind::InKeyword),
            "instanceof" => Some(SyntaxKind::InstanceOfKeyword),
            "new" => Some(SyntaxKind::NewKeyword),
            "null" => Some(SyntaxKind::NullKeyword),
            "return" => Some(SyntaxKind::ReturnKeyword),
            "super" => Some(SyntaxKind::SuperKeyword),
            "switch" => Some(SyntaxKind::SwitchKeyword),
            "this" => Some(SyntaxKind::ThisKeyword),
            "throw" => Some(SyntaxKind::ThrowKeyword),
            "true" => Some(SyntaxKind::TrueKeyword),
            "try" => Some(SyntaxKind::TryKeyword),
            "typeof" => Some(SyntaxKind::TypeOfKeyword),
            "var" => Some(SyntaxKind::VarKeyword),
            "void" => Some(SyntaxKind::VoidKeyword),
            "while" => Some(SyntaxKind::WhileKeyword),
            "with" => Some(SyntaxKind::WithKeyword),
            "implements" => Some(SyntaxKind::ImplementsKeyword),
            "interface" => Some(SyntaxKind::InterfaceKeyword),
            "let" => Some(SyntaxKind::LetKeyword),
            "package" => Some(SyntaxKind::PackageKeyword),
            "private" => Some(SyntaxKind::PrivateKeyword),
            "protected" => Some(SyntaxKind::ProtectedKeyword),
            "public" => Some(SyntaxKind::PublicKeyword),
            "static" => Some(SyntaxKind::StaticKeyword),
            "yield" => Some(SyntaxKind::YieldKeyword),
            "abstract" => Some(SyntaxKind::AbstractKeyword),
            "accessor" => Some(SyntaxKind::AccessorKeyword),
            "as" => Some(SyntaxKind::AsKeyword),
            "asserts" => Some(SyntaxKind::AssertsKeyword),
            "assert" => Some(SyntaxKind::AssertKeyword),
            "any" => Some(SyntaxKind::AnyKeyword),
            "async" => Some(SyntaxKind::AsyncKeyword),
            "await" => Some(SyntaxKind::AwaitKeyword),
            "boolean" => Some(SyntaxKind::BooleanKeyword),
            "constructor" => Some(SyntaxKind::ConstructorKeyword),
            "declare" => Some(SyntaxKind::DeclareKeyword),
            "get" => Some(SyntaxKind::GetKeyword),
            "infer" => Some(SyntaxKind::InferKeyword),
            "intrinsic" => Some(SyntaxKind::IntrinsicKeyword),
            "is" => Some(SyntaxKind::IsKeyword),
            "keyof" => Some(SyntaxKind::KeyOfKeyword),
            "module" => Some(SyntaxKind::ModuleKeyword),
            "namespace" => Some(SyntaxKind::NamespaceKeyword),
            "never" => Some(SyntaxKind::NeverKeyword),
            "out" => Some(SyntaxKind::OutKeyword),
            "readonly" => Some(SyntaxKind::ReadonlyKeyword),
            "require" => Some(SyntaxKind::RequireKeyword),
            "number" => Some(SyntaxKind::NumberKeyword),
            "object" => Some(SyntaxKind::ObjectKeyword),
            "satisfies" => Some(SyntaxKind::SatisfiesKeyword),
            "set" => Some(SyntaxKind::SetKeyword),
            "string" => Some(SyntaxKind::StringKeyword),
            "symbol" => Some(SyntaxKind::SymbolKeyword),
            "type" => Some(SyntaxKind::TypeKeyword),
            "undefined" => Some(SyntaxKind::UndefinedKeyword),
            "unique" => Some(SyntaxKind::UniqueKeyword),
            "unknown" => Some(SyntaxKind::UnknownKeyword),
            "using" => Some(SyntaxKind::UsingKeyword),
            "from" => Some(SyntaxKind::FromKeyword),
            "global" => Some(SyntaxKind::GlobalKeyword),
            "bigint" => Some(SyntaxKind::BigIntKeyword),
            "override" => Some(SyntaxKind::OverrideKeyword),
            "of" => Some(SyntaxKind::OfKeyword),
            _ => None,
        }
    }

    /// Get the punctuation text for a punctuation kind, or None.
    pub fn punctuation_text(self) -> Option<&'static str> {
        match self {
            SyntaxKind::OpenBraceToken => Some("{"),
            SyntaxKind::CloseBraceToken => Some("}"),
            SyntaxKind::OpenParenToken => Some("("),
            SyntaxKind::CloseParenToken => Some(")"),
            SyntaxKind::OpenBracketToken => Some("["),
            SyntaxKind::CloseBracketToken => Some("]"),
            SyntaxKind::DotToken => Some("."),
            SyntaxKind::DotDotDotToken => Some("..."),
            SyntaxKind::SemicolonToken => Some(";"),
            SyntaxKind::CommaToken => Some(","),
            SyntaxKind::QuestionDotToken => Some("?."),
            SyntaxKind::LessThanToken => Some("<"),
            SyntaxKind::LessThanSlashToken => Some("</"),
            SyntaxKind::GreaterThanToken => Some(">"),
            SyntaxKind::LessThanEqualsToken => Some("<="),
            SyntaxKind::GreaterThanEqualsToken => Some(">="),
            SyntaxKind::EqualsEqualsToken => Some("=="),
            SyntaxKind::ExclamationEqualsToken => Some("!="),
            SyntaxKind::EqualsEqualsEqualsToken => Some("==="),
            SyntaxKind::ExclamationEqualsEqualsToken => Some("!=="),
            SyntaxKind::EqualsGreaterThanToken => Some("=>"),
            SyntaxKind::PlusToken => Some("+"),
            SyntaxKind::MinusToken => Some("-"),
            SyntaxKind::AsteriskToken => Some("*"),
            SyntaxKind::AsteriskAsteriskToken => Some("**"),
            SyntaxKind::SlashToken => Some("/"),
            SyntaxKind::PercentToken => Some("%"),
            SyntaxKind::PlusPlusToken => Some("++"),
            SyntaxKind::MinusMinusToken => Some("--"),
            SyntaxKind::LessThanLessThanToken => Some("<<"),
            SyntaxKind::GreaterThanGreaterThanToken => Some(">>"),
            SyntaxKind::GreaterThanGreaterThanGreaterThanToken => Some(">>>"),
            SyntaxKind::AmpersandToken => Some("&"),
            SyntaxKind::BarToken => Some("|"),
            SyntaxKind::CaretToken => Some("^"),
            SyntaxKind::ExclamationToken => Some("!"),
            SyntaxKind::TildeToken => Some("~"),
            SyntaxKind::AmpersandAmpersandToken => Some("&&"),
            SyntaxKind::BarBarToken => Some("||"),
            SyntaxKind::QuestionToken => Some("?"),
            SyntaxKind::ColonToken => Some(":"),
            SyntaxKind::AtToken => Some("@"),
            SyntaxKind::QuestionQuestionToken => Some("??"),
            SyntaxKind::BacktickToken => Some("`"),
            SyntaxKind::HashToken => Some("#"),
            SyntaxKind::EqualsToken => Some("="),
            SyntaxKind::PlusEqualsToken => Some("+="),
            SyntaxKind::MinusEqualsToken => Some("-="),
            SyntaxKind::AsteriskEqualsToken => Some("*="),
            SyntaxKind::AsteriskAsteriskEqualsToken => Some("**="),
            SyntaxKind::SlashEqualsToken => Some("/="),
            SyntaxKind::PercentEqualsToken => Some("%="),
            SyntaxKind::LessThanLessThanEqualsToken => Some("<<="),
            SyntaxKind::GreaterThanGreaterThanEqualsToken => Some(">>="),
            SyntaxKind::GreaterThanGreaterThanGreaterThanEqualsToken => Some(">>>="),
            SyntaxKind::AmpersandEqualsToken => Some("&="),
            SyntaxKind::BarEqualsToken => Some("|="),
            SyntaxKind::CaretEqualsToken => Some("^="),
            SyntaxKind::BarBarEqualsToken => Some("||="),
            SyntaxKind::AmpersandAmpersandEqualsToken => Some("&&="),
            SyntaxKind::QuestionQuestionEqualsToken => Some("??="),
            _ => None,
        }
    }
}

impl std::fmt::Display for SyntaxKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
