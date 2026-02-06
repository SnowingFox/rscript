//! AST node definitions for the TypeScript compiler.
//!
//! Every AST node type is defined here, closely matching TypeScript's AST node
//! interfaces. Nodes reference child nodes via arena-allocated references.

use crate::syntax_kind::SyntaxKind;
use crate::types::*;
use rscript_core::intern::InternedString;
use rscript_core::text::TextRange;

// ============================================================================
// Core Node Wrapper
// ============================================================================

/// Common data shared by all AST nodes.
#[derive(Debug, Clone)]
pub struct NodeData {
    /// The kind of this node.
    pub kind: SyntaxKind,
    /// Source position range.
    pub range: TextRange,
    /// Node flags.
    pub flags: NodeFlags,
    /// Modifier flags (for declarations).
    pub modifier_flags: ModifierFlags,
    /// Unique node ID (assigned during binding).
    pub id: NodeId,
    /// Associated symbol (set during binding).
    pub symbol: Option<SymbolId>,
}

impl NodeData {
    pub fn new(kind: SyntaxKind, pos: u32, end: u32) -> Self {
        Self {
            kind,
            range: TextRange::new(pos, end),
            flags: NodeFlags::NONE,
            modifier_flags: ModifierFlags::NONE,
            id: NodeId::INVALID,
            symbol: None,
        }
    }
}

/// A list of nodes, allocated in the arena.
pub type NodeList<'a, T> = &'a [T];

/// An optional arena-allocated node.
pub type OptionalNode<'a, T> = Option<&'a T>;

// ============================================================================
// Source File
// ============================================================================

#[derive(Debug)]
pub struct SourceFile<'a> {
    pub data: NodeData,
    pub statements: NodeList<'a, Statement<'a>>,
    pub end_of_file_token: Token,
    pub file_name: String,
    pub text: String,
    pub language_variant: LanguageVariant,
    pub script_kind: ScriptKind,
    /// Whether this file is a declaration file (.d.ts).
    pub is_declaration_file: bool,
    /// Whether this file has no default lib directive.
    pub has_no_default_lib: bool,
}

/// Language variant (standard vs JSX).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageVariant {
    Standard,
    JSX,
}

/// The kind of script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptKind {
    Unknown,
    JS,
    JSX,
    TS,
    TSX,
    External,
    JSON,
    Deferred,
}

// ============================================================================
// Token
// ============================================================================

/// A simple token with kind and range.
#[derive(Debug, Clone)]
pub struct Token {
    pub data: NodeData,
}

impl Token {
    pub fn new(kind: SyntaxKind, pos: u32, end: u32) -> Self {
        Self {
            data: NodeData::new(kind, pos, end),
        }
    }
}

// ============================================================================
// Identifier
// ============================================================================

#[derive(Debug, Clone)]
pub struct Identifier {
    pub data: NodeData,
    /// The interned text of this identifier.
    pub text: InternedString,
    /// The actual text of this identifier as a plain string.
    pub text_name: String,
    /// Original keyword kind if this identifier is an escaped keyword.
    pub original_keyword_kind: Option<SyntaxKind>,
}

// ============================================================================
// Type Nodes
// ============================================================================

#[derive(Debug)]
pub enum TypeNode<'a> {
    KeywordType(KeywordTypeNode),
    TypeReference(TypeReferenceNode<'a>),
    FunctionType(FunctionTypeNode<'a>),
    ConstructorType(ConstructorTypeNode<'a>),
    TypeQuery(TypeQueryNode<'a>),
    TypeLiteral(TypeLiteralNode<'a>),
    ArrayType(ArrayTypeNode<'a>),
    TupleType(TupleTypeNode<'a>),
    OptionalType(OptionalTypeNode<'a>),
    RestType(RestTypeNode<'a>),
    UnionType(UnionTypeNode<'a>),
    IntersectionType(IntersectionTypeNode<'a>),
    ConditionalType(ConditionalTypeNode<'a>),
    InferType(InferTypeNode<'a>),
    ParenthesizedType(ParenthesizedTypeNode<'a>),
    ThisType(ThisTypeNode),
    TypeOperator(TypeOperatorNode<'a>),
    IndexedAccessType(IndexedAccessTypeNode<'a>),
    MappedType(MappedTypeNode<'a>),
    LiteralType(LiteralTypeNode<'a>),
    NamedTupleMember(NamedTupleMemberNode<'a>),
    TemplateLiteralType(TemplateLiteralTypeNode<'a>),
    ImportType(ImportTypeNode<'a>),
    TypePredicate(TypePredicateNode<'a>),
    ExpressionWithTypeArguments(ExpressionWithTypeArgumentsNode<'a>),
}

#[derive(Debug)]
pub struct KeywordTypeNode {
    pub data: NodeData,
}

#[derive(Debug)]
pub struct TypeReferenceNode<'a> {
    pub data: NodeData,
    pub type_name: EntityName<'a>,
    pub type_arguments: Option<NodeList<'a, TypeNode<'a>>>,
}

#[derive(Debug)]
pub struct FunctionTypeNode<'a> {
    pub data: NodeData,
    pub type_parameters: Option<NodeList<'a, TypeParameterDeclaration<'a>>>,
    pub parameters: NodeList<'a, ParameterDeclaration<'a>>,
    pub return_type: OptionalNode<'a, TypeNode<'a>>,
}

#[derive(Debug)]
pub struct ConstructorTypeNode<'a> {
    pub data: NodeData,
    pub type_parameters: Option<NodeList<'a, TypeParameterDeclaration<'a>>>,
    pub parameters: NodeList<'a, ParameterDeclaration<'a>>,
    pub return_type: OptionalNode<'a, TypeNode<'a>>,
}

#[derive(Debug)]
pub struct TypeQueryNode<'a> {
    pub data: NodeData,
    pub expr_name: EntityName<'a>,
    pub type_arguments: Option<NodeList<'a, TypeNode<'a>>>,
}

#[derive(Debug)]
pub struct TypeLiteralNode<'a> {
    pub data: NodeData,
    pub members: NodeList<'a, TypeElement<'a>>,
}

#[derive(Debug)]
pub struct ArrayTypeNode<'a> {
    pub data: NodeData,
    pub element_type: &'a TypeNode<'a>,
}

#[derive(Debug)]
pub struct TupleTypeNode<'a> {
    pub data: NodeData,
    pub elements: NodeList<'a, TypeNode<'a>>,
}

#[derive(Debug)]
pub struct OptionalTypeNode<'a> {
    pub data: NodeData,
    pub type_node: &'a TypeNode<'a>,
}

#[derive(Debug)]
pub struct RestTypeNode<'a> {
    pub data: NodeData,
    pub type_node: &'a TypeNode<'a>,
}

#[derive(Debug)]
pub struct UnionTypeNode<'a> {
    pub data: NodeData,
    pub types: NodeList<'a, TypeNode<'a>>,
}

#[derive(Debug)]
pub struct IntersectionTypeNode<'a> {
    pub data: NodeData,
    pub types: NodeList<'a, TypeNode<'a>>,
}

#[derive(Debug)]
pub struct ConditionalTypeNode<'a> {
    pub data: NodeData,
    pub check_type: &'a TypeNode<'a>,
    pub extends_type: &'a TypeNode<'a>,
    pub true_type: &'a TypeNode<'a>,
    pub false_type: &'a TypeNode<'a>,
}

#[derive(Debug)]
pub struct InferTypeNode<'a> {
    pub data: NodeData,
    pub type_parameter: &'a TypeParameterDeclaration<'a>,
}

#[derive(Debug)]
pub struct ParenthesizedTypeNode<'a> {
    pub data: NodeData,
    pub type_node: &'a TypeNode<'a>,
}

#[derive(Debug)]
pub struct ThisTypeNode {
    pub data: NodeData,
}

#[derive(Debug)]
pub struct TypeOperatorNode<'a> {
    pub data: NodeData,
    pub operator: SyntaxKind, // KeyOfKeyword, UniqueKeyword, ReadonlyKeyword
    pub type_node: &'a TypeNode<'a>,
}

#[derive(Debug)]
pub struct IndexedAccessTypeNode<'a> {
    pub data: NodeData,
    pub object_type: &'a TypeNode<'a>,
    pub index_type: &'a TypeNode<'a>,
}

#[derive(Debug)]
pub struct MappedTypeNode<'a> {
    pub data: NodeData,
    pub readonly_token: Option<Token>,
    pub type_parameter: &'a TypeParameterDeclaration<'a>,
    pub name_type: OptionalNode<'a, TypeNode<'a>>,
    pub question_token: Option<Token>,
    pub type_node: OptionalNode<'a, TypeNode<'a>>,
    pub members: Option<NodeList<'a, TypeElement<'a>>>,
}

#[derive(Debug)]
pub struct LiteralTypeNode<'a> {
    pub data: NodeData,
    pub literal: &'a Expression<'a>,
}

#[derive(Debug)]
pub struct NamedTupleMemberNode<'a> {
    pub data: NodeData,
    pub dot_dot_dot_token: Option<Token>,
    pub name: Identifier,
    pub question_token: Option<Token>,
    pub type_node: &'a TypeNode<'a>,
}

#[derive(Debug)]
pub struct TemplateLiteralTypeNode<'a> {
    pub data: NodeData,
    pub head: Token, // TemplateHead
    pub template_spans: NodeList<'a, TemplateLiteralTypeSpan<'a>>,
}

#[derive(Debug)]
pub struct TemplateLiteralTypeSpan<'a> {
    pub data: NodeData,
    pub type_node: &'a TypeNode<'a>,
    pub literal: Token,
}

#[derive(Debug)]
pub struct ImportTypeNode<'a> {
    pub data: NodeData,
    pub is_type_of: bool,
    pub argument: &'a TypeNode<'a>,
    pub assertions: Option<&'a ImportTypeAssertionContainer<'a>>,
    pub qualifier: Option<EntityName<'a>>,
    pub type_arguments: Option<NodeList<'a, TypeNode<'a>>>,
}

#[derive(Debug)]
pub struct ImportTypeAssertionContainer<'a> {
    pub data: NodeData,
    pub assert_clause: &'a AssertClause<'a>,
    pub multi_line: bool,
}

#[derive(Debug)]
pub struct TypePredicateNode<'a> {
    pub data: NodeData,
    pub asserts_modifier: Option<Token>,
    pub parameter_name: TypePredicateParameterName,
    pub type_node: OptionalNode<'a, TypeNode<'a>>,
}

#[derive(Debug)]
pub enum TypePredicateParameterName {
    Identifier(Identifier),
    ThisType(ThisTypeNode),
}

#[derive(Debug)]
pub struct ExpressionWithTypeArgumentsNode<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
    pub type_arguments: Option<NodeList<'a, TypeNode<'a>>>,
}

// ============================================================================
// Declarations
// ============================================================================

#[derive(Debug)]
pub struct TypeParameterDeclaration<'a> {
    pub data: NodeData,
    pub name: Identifier,
    pub constraint: OptionalNode<'a, TypeNode<'a>>,
    pub default: OptionalNode<'a, TypeNode<'a>>,
}

#[derive(Debug)]
pub struct ParameterDeclaration<'a> {
    pub data: NodeData,
    pub dot_dot_dot_token: Option<Token>,
    pub name: BindingName<'a>,
    pub question_token: Option<Token>,
    pub type_annotation: OptionalNode<'a, TypeNode<'a>>,
    pub initializer: OptionalNode<'a, Expression<'a>>,
}

// ============================================================================
// Names
// ============================================================================

#[derive(Debug)]
pub enum EntityName<'a> {
    Identifier(Identifier),
    QualifiedName(&'a QualifiedName<'a>),
}

#[derive(Debug)]
pub struct QualifiedName<'a> {
    pub data: NodeData,
    pub left: EntityName<'a>,
    pub right: Identifier,
}

#[derive(Debug)]
pub enum BindingName<'a> {
    Identifier(Identifier),
    ObjectBindingPattern(&'a ObjectBindingPattern<'a>),
    ArrayBindingPattern(&'a ArrayBindingPattern<'a>),
}

#[derive(Debug)]
pub struct ComputedPropertyName<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
}

#[derive(Debug)]
pub enum PropertyName<'a> {
    Identifier(Identifier),
    StringLiteral(Token),
    NumericLiteral(Token),
    ComputedPropertyName(&'a ComputedPropertyName<'a>),
    PrivateIdentifier(Identifier),
}

// ============================================================================
// Binding Patterns
// ============================================================================

#[derive(Debug)]
pub struct ObjectBindingPattern<'a> {
    pub data: NodeData,
    pub elements: NodeList<'a, BindingElement<'a>>,
}

#[derive(Debug)]
pub struct ArrayBindingPattern<'a> {
    pub data: NodeData,
    pub elements: NodeList<'a, ArrayBindingElement<'a>>,
}

#[derive(Debug)]
pub enum ArrayBindingElement<'a> {
    BindingElement(BindingElement<'a>),
    OmittedExpression(NodeData),
}

#[derive(Debug)]
pub struct BindingElement<'a> {
    pub data: NodeData,
    pub dot_dot_dot_token: Option<Token>,
    pub property_name: Option<PropertyName<'a>>,
    pub name: BindingName<'a>,
    pub initializer: OptionalNode<'a, Expression<'a>>,
}

// ============================================================================
// Type Elements (Interface/Object type members)
// ============================================================================

#[derive(Debug)]
pub enum TypeElement<'a> {
    PropertySignature(PropertySignatureNode<'a>),
    MethodSignature(MethodSignatureNode<'a>),
    CallSignature(CallSignatureNode<'a>),
    ConstructSignature(ConstructSignatureNode<'a>),
    IndexSignature(IndexSignatureNode<'a>),
}

#[derive(Debug)]
pub struct PropertySignatureNode<'a> {
    pub data: NodeData,
    pub name: PropertyName<'a>,
    pub question_token: Option<Token>,
    pub type_annotation: OptionalNode<'a, TypeNode<'a>>,
}

#[derive(Debug)]
pub struct MethodSignatureNode<'a> {
    pub data: NodeData,
    pub name: PropertyName<'a>,
    pub question_token: Option<Token>,
    pub type_parameters: Option<NodeList<'a, TypeParameterDeclaration<'a>>>,
    pub parameters: NodeList<'a, ParameterDeclaration<'a>>,
    pub return_type: OptionalNode<'a, TypeNode<'a>>,
}

#[derive(Debug)]
pub struct CallSignatureNode<'a> {
    pub data: NodeData,
    pub type_parameters: Option<NodeList<'a, TypeParameterDeclaration<'a>>>,
    pub parameters: NodeList<'a, ParameterDeclaration<'a>>,
    pub return_type: OptionalNode<'a, TypeNode<'a>>,
}

#[derive(Debug)]
pub struct ConstructSignatureNode<'a> {
    pub data: NodeData,
    pub type_parameters: Option<NodeList<'a, TypeParameterDeclaration<'a>>>,
    pub parameters: NodeList<'a, ParameterDeclaration<'a>>,
    pub return_type: OptionalNode<'a, TypeNode<'a>>,
}

#[derive(Debug)]
pub struct IndexSignatureNode<'a> {
    pub data: NodeData,
    pub parameters: NodeList<'a, ParameterDeclaration<'a>>,
    pub type_annotation: OptionalNode<'a, TypeNode<'a>>,
}

// ============================================================================
// Expressions
// ============================================================================

#[derive(Debug)]
pub enum Expression<'a> {
    Identifier(Identifier),
    StringLiteral(StringLiteral),
    NumericLiteral(NumericLiteral),
    BigIntLiteral(BigIntLiteral),
    RegularExpressionLiteral(RegularExpressionLiteral),
    NoSubstitutionTemplateLiteral(NoSubstitutionTemplateLiteral),
    TemplateExpression(TemplateExpression<'a>),
    ArrayLiteral(ArrayLiteralExpression<'a>),
    ObjectLiteral(ObjectLiteralExpression<'a>),
    PropertyAccess(PropertyAccessExpression<'a>),
    ElementAccess(ElementAccessExpression<'a>),
    Call(CallExpression<'a>),
    New(NewExpression<'a>),
    TaggedTemplate(TaggedTemplateExpression<'a>),
    TypeAssertion(TypeAssertionExpression<'a>),
    Parenthesized(ParenthesizedExpression<'a>),
    FunctionExpression(FunctionExpression<'a>),
    ArrowFunction(ArrowFunction<'a>),
    Delete(DeleteExpression<'a>),
    TypeOf(TypeOfExpression<'a>),
    Void(VoidExpression<'a>),
    Await(AwaitExpression<'a>),
    PrefixUnary(PrefixUnaryExpression<'a>),
    PostfixUnary(PostfixUnaryExpression<'a>),
    Binary(BinaryExpression<'a>),
    Conditional(ConditionalExpression<'a>),
    Yield(YieldExpression<'a>),
    Spread(SpreadElement<'a>),
    ClassExpression(ClassExpression<'a>),
    OmittedExpression(NodeData),
    As(AsExpression<'a>),
    NonNull(NonNullExpression<'a>),
    MetaProperty(MetaPropertyExpression),
    Satisfies(SatisfiesExpression<'a>),
    // Keyword expressions
    ThisKeyword(NodeData),
    SuperKeyword(NodeData),
    NullKeyword(NodeData),
    TrueKeyword(NodeData),
    FalseKeyword(NodeData),
}

// -- Literal Expressions --

#[derive(Debug, Clone)]
pub struct StringLiteral {
    pub data: NodeData,
    pub text: InternedString,
    pub is_single_quote: bool,
}

#[derive(Debug, Clone)]
pub struct NumericLiteral {
    pub data: NodeData,
    pub text: InternedString,
    pub numeric_literal_flags: TokenFlags,
}

#[derive(Debug, Clone)]
pub struct BigIntLiteral {
    pub data: NodeData,
    pub text: InternedString,
}

#[derive(Debug, Clone)]
pub struct RegularExpressionLiteral {
    pub data: NodeData,
    pub text: InternedString,
}

#[derive(Debug, Clone)]
pub struct NoSubstitutionTemplateLiteral {
    pub data: NodeData,
    pub text: InternedString,
    pub raw_text: Option<InternedString>,
}

#[derive(Debug)]
pub struct TemplateExpression<'a> {
    pub data: NodeData,
    pub head: Token,
    pub template_spans: NodeList<'a, TemplateSpan<'a>>,
}

#[derive(Debug)]
pub struct TemplateSpan<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
    pub literal: Token,
}

// -- Compound Expressions --

#[derive(Debug)]
pub struct ArrayLiteralExpression<'a> {
    pub data: NodeData,
    pub elements: NodeList<'a, Expression<'a>>,
    pub multi_line: bool,
}

#[derive(Debug)]
pub struct ObjectLiteralExpression<'a> {
    pub data: NodeData,
    pub properties: NodeList<'a, ObjectLiteralElement<'a>>,
    pub multi_line: bool,
}

#[derive(Debug)]
pub enum ObjectLiteralElement<'a> {
    PropertyAssignment(PropertyAssignment<'a>),
    ShorthandPropertyAssignment(ShorthandPropertyAssignment<'a>),
    SpreadAssignment(SpreadAssignment<'a>),
    MethodDeclaration(MethodDeclaration<'a>),
    GetAccessor(GetAccessorDeclaration<'a>),
    SetAccessor(SetAccessorDeclaration<'a>),
}

#[derive(Debug)]
pub struct PropertyAssignment<'a> {
    pub data: NodeData,
    pub name: PropertyName<'a>,
    pub initializer: &'a Expression<'a>,
}

#[derive(Debug)]
pub struct ShorthandPropertyAssignment<'a> {
    pub data: NodeData,
    pub name: Identifier,
    pub object_assignment_initializer: OptionalNode<'a, Expression<'a>>,
}

#[derive(Debug)]
pub struct SpreadAssignment<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
}

#[derive(Debug)]
pub struct PropertyAccessExpression<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
    pub question_dot_token: Option<Token>,
    pub name: MemberName,
}

#[derive(Debug)]
pub enum MemberName {
    Identifier(Identifier),
    PrivateIdentifier(Identifier),
}

#[derive(Debug)]
pub struct ElementAccessExpression<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
    pub question_dot_token: Option<Token>,
    pub argument_expression: &'a Expression<'a>,
}

#[derive(Debug)]
pub struct CallExpression<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
    pub question_dot_token: Option<Token>,
    pub type_arguments: Option<NodeList<'a, TypeNode<'a>>>,
    pub arguments: NodeList<'a, Expression<'a>>,
}

#[derive(Debug)]
pub struct NewExpression<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
    pub type_arguments: Option<NodeList<'a, TypeNode<'a>>>,
    pub arguments: Option<NodeList<'a, Expression<'a>>>,
}

#[derive(Debug)]
pub struct TaggedTemplateExpression<'a> {
    pub data: NodeData,
    pub tag: &'a Expression<'a>,
    pub type_arguments: Option<NodeList<'a, TypeNode<'a>>>,
    pub template: &'a Expression<'a>, // NoSubstitutionTemplateLiteral or TemplateExpression
}

#[derive(Debug)]
pub struct TypeAssertionExpression<'a> {
    pub data: NodeData,
    pub type_node: &'a TypeNode<'a>,
    pub expression: &'a Expression<'a>,
}

#[derive(Debug)]
pub struct ParenthesizedExpression<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
}

#[derive(Debug)]
pub struct FunctionExpression<'a> {
    pub data: NodeData,
    pub name: Option<Identifier>,
    pub asterisk_token: Option<Token>,
    pub type_parameters: Option<NodeList<'a, TypeParameterDeclaration<'a>>>,
    pub parameters: NodeList<'a, ParameterDeclaration<'a>>,
    pub return_type: OptionalNode<'a, TypeNode<'a>>,
    pub body: &'a Block<'a>,
}

#[derive(Debug)]
pub struct ArrowFunction<'a> {
    pub data: NodeData,
    pub type_parameters: Option<NodeList<'a, TypeParameterDeclaration<'a>>>,
    pub parameters: NodeList<'a, ParameterDeclaration<'a>>,
    pub return_type: OptionalNode<'a, TypeNode<'a>>,
    pub equals_greater_than_token: Token,
    pub body: ArrowFunctionBody<'a>,
}

#[derive(Debug)]
pub enum ArrowFunctionBody<'a> {
    Block(&'a Block<'a>),
    Expression(&'a Expression<'a>),
}

#[derive(Debug)]
pub struct DeleteExpression<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
}

#[derive(Debug)]
pub struct TypeOfExpression<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
}

#[derive(Debug)]
pub struct VoidExpression<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
}

#[derive(Debug)]
pub struct AwaitExpression<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
}

#[derive(Debug)]
pub struct PrefixUnaryExpression<'a> {
    pub data: NodeData,
    pub operator: SyntaxKind,
    pub operand: &'a Expression<'a>,
}

#[derive(Debug)]
pub struct PostfixUnaryExpression<'a> {
    pub data: NodeData,
    pub operand: &'a Expression<'a>,
    pub operator: SyntaxKind,
}

#[derive(Debug)]
pub struct BinaryExpression<'a> {
    pub data: NodeData,
    pub left: &'a Expression<'a>,
    pub operator_token: Token,
    pub right: &'a Expression<'a>,
}

#[derive(Debug)]
pub struct ConditionalExpression<'a> {
    pub data: NodeData,
    pub condition: &'a Expression<'a>,
    pub question_token: Token,
    pub when_true: &'a Expression<'a>,
    pub colon_token: Token,
    pub when_false: &'a Expression<'a>,
}

#[derive(Debug)]
pub struct YieldExpression<'a> {
    pub data: NodeData,
    pub asterisk_token: Option<Token>,
    pub expression: OptionalNode<'a, Expression<'a>>,
}

#[derive(Debug)]
pub struct SpreadElement<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
}

#[derive(Debug)]
pub struct AsExpression<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
    pub type_node: &'a TypeNode<'a>,
}

#[derive(Debug)]
pub struct NonNullExpression<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
}

#[derive(Debug)]
pub struct MetaPropertyExpression {
    pub data: NodeData,
    pub keyword_token: SyntaxKind, // NewKeyword or ImportKeyword
    pub name: Identifier,
}

#[derive(Debug)]
pub struct SatisfiesExpression<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
    pub type_node: &'a TypeNode<'a>,
}

// ============================================================================
// Statements
// ============================================================================

#[derive(Debug)]
pub enum Statement<'a> {
    VariableStatement(VariableStatement<'a>),
    FunctionDeclaration(FunctionDeclaration<'a>),
    ClassDeclaration(ClassDeclaration<'a>),
    InterfaceDeclaration(InterfaceDeclaration<'a>),
    TypeAliasDeclaration(TypeAliasDeclaration<'a>),
    EnumDeclaration(EnumDeclaration<'a>),
    ModuleDeclaration(ModuleDeclaration<'a>),
    ImportDeclaration(ImportDeclaration<'a>),
    ImportEqualsDeclaration(ImportEqualsDeclaration<'a>),
    ExportDeclaration(ExportDeclaration<'a>),
    ExportAssignment(ExportAssignment<'a>),
    NamespaceExportDeclaration(NamespaceExportDeclaration),
    Block(Block<'a>),
    EmptyStatement(NodeData),
    ExpressionStatement(ExpressionStatement<'a>),
    IfStatement(IfStatement<'a>),
    DoStatement(DoStatement<'a>),
    WhileStatement(WhileStatement<'a>),
    ForStatement(ForStatement<'a>),
    ForInStatement(ForInStatement<'a>),
    ForOfStatement(ForOfStatement<'a>),
    ContinueStatement(ContinueStatement),
    BreakStatement(BreakStatement),
    ReturnStatement(ReturnStatement<'a>),
    WithStatement(WithStatement<'a>),
    SwitchStatement(SwitchStatement<'a>),
    LabeledStatement(LabeledStatement<'a>),
    ThrowStatement(ThrowStatement<'a>),
    TryStatement(TryStatement<'a>),
    DebuggerStatement(NodeData),
    MissingDeclaration(NodeData),
}

#[derive(Debug)]
pub struct Block<'a> {
    pub data: NodeData,
    pub statements: NodeList<'a, Statement<'a>>,
    pub multi_line: bool,
}

#[derive(Debug)]
pub struct VariableStatement<'a> {
    pub data: NodeData,
    pub declaration_list: VariableDeclarationList<'a>,
}

#[derive(Debug)]
pub struct VariableDeclarationList<'a> {
    pub data: NodeData,
    pub declarations: NodeList<'a, VariableDeclaration<'a>>,
}

#[derive(Debug)]
pub struct VariableDeclaration<'a> {
    pub data: NodeData,
    pub name: BindingName<'a>,
    pub exclamation_token: Option<Token>,
    pub type_annotation: OptionalNode<'a, TypeNode<'a>>,
    pub initializer: OptionalNode<'a, Expression<'a>>,
}

#[derive(Debug)]
pub struct ExpressionStatement<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
}

#[derive(Debug)]
pub struct IfStatement<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
    pub then_statement: &'a Statement<'a>,
    pub else_statement: OptionalNode<'a, Statement<'a>>,
}

#[derive(Debug)]
pub struct DoStatement<'a> {
    pub data: NodeData,
    pub statement: &'a Statement<'a>,
    pub expression: &'a Expression<'a>,
}

#[derive(Debug)]
pub struct WhileStatement<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
    pub statement: &'a Statement<'a>,
}

#[derive(Debug)]
pub struct ForStatement<'a> {
    pub data: NodeData,
    pub initializer: Option<ForInitializer<'a>>,
    pub condition: OptionalNode<'a, Expression<'a>>,
    pub incrementor: OptionalNode<'a, Expression<'a>>,
    pub statement: &'a Statement<'a>,
}

#[derive(Debug)]
pub enum ForInitializer<'a> {
    VariableDeclarationList(VariableDeclarationList<'a>),
    Expression(&'a Expression<'a>),
}

#[derive(Debug)]
pub struct ForInStatement<'a> {
    pub data: NodeData,
    pub initializer: ForInitializer<'a>,
    pub expression: &'a Expression<'a>,
    pub statement: &'a Statement<'a>,
}

#[derive(Debug)]
pub struct ForOfStatement<'a> {
    pub data: NodeData,
    pub await_modifier: Option<Token>,
    pub initializer: ForInitializer<'a>,
    pub expression: &'a Expression<'a>,
    pub statement: &'a Statement<'a>,
}

#[derive(Debug)]
pub struct ContinueStatement {
    pub data: NodeData,
    pub label: Option<Identifier>,
}

#[derive(Debug)]
pub struct BreakStatement {
    pub data: NodeData,
    pub label: Option<Identifier>,
}

#[derive(Debug)]
pub struct ReturnStatement<'a> {
    pub data: NodeData,
    pub expression: OptionalNode<'a, Expression<'a>>,
}

#[derive(Debug)]
pub struct WithStatement<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
    pub statement: &'a Statement<'a>,
}

#[derive(Debug)]
pub struct SwitchStatement<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
    pub case_block: CaseBlock<'a>,
}

#[derive(Debug)]
pub struct CaseBlock<'a> {
    pub data: NodeData,
    pub clauses: NodeList<'a, CaseOrDefaultClause<'a>>,
}

#[derive(Debug)]
pub enum CaseOrDefaultClause<'a> {
    CaseClause(CaseClause<'a>),
    DefaultClause(DefaultClause<'a>),
}

#[derive(Debug)]
pub struct CaseClause<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
    pub statements: NodeList<'a, Statement<'a>>,
}

#[derive(Debug)]
pub struct DefaultClause<'a> {
    pub data: NodeData,
    pub statements: NodeList<'a, Statement<'a>>,
}

#[derive(Debug)]
pub struct LabeledStatement<'a> {
    pub data: NodeData,
    pub label: Identifier,
    pub statement: &'a Statement<'a>,
}

#[derive(Debug)]
pub struct ThrowStatement<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
}

#[derive(Debug)]
pub struct TryStatement<'a> {
    pub data: NodeData,
    pub try_block: Block<'a>,
    pub catch_clause: Option<CatchClause<'a>>,
    pub finally_block: Option<Block<'a>>,
}

#[derive(Debug)]
pub struct CatchClause<'a> {
    pub data: NodeData,
    pub variable_declaration: Option<VariableDeclaration<'a>>,
    pub block: Block<'a>,
}

// ============================================================================
// Declarations
// ============================================================================

#[derive(Debug)]
pub struct FunctionDeclaration<'a> {
    pub data: NodeData,
    pub name: Option<Identifier>,
    pub asterisk_token: Option<Token>,
    pub type_parameters: Option<NodeList<'a, TypeParameterDeclaration<'a>>>,
    pub parameters: NodeList<'a, ParameterDeclaration<'a>>,
    pub return_type: OptionalNode<'a, TypeNode<'a>>,
    pub body: Option<Block<'a>>,
}

#[derive(Debug)]
pub struct ClassDeclaration<'a> {
    pub data: NodeData,
    pub name: Option<Identifier>,
    pub type_parameters: Option<NodeList<'a, TypeParameterDeclaration<'a>>>,
    pub heritage_clauses: Option<NodeList<'a, HeritageClause<'a>>>,
    pub members: NodeList<'a, ClassElement<'a>>,
}

#[derive(Debug)]
pub struct ClassExpression<'a> {
    pub data: NodeData,
    pub name: Option<Identifier>,
    pub type_parameters: Option<NodeList<'a, TypeParameterDeclaration<'a>>>,
    pub heritage_clauses: Option<NodeList<'a, HeritageClause<'a>>>,
    pub members: NodeList<'a, ClassElement<'a>>,
}

#[derive(Debug)]
pub struct HeritageClause<'a> {
    pub data: NodeData,
    pub token: SyntaxKind, // ExtendsKeyword or ImplementsKeyword
    pub types: NodeList<'a, ExpressionWithTypeArgumentsNode<'a>>,
}

#[derive(Debug)]
pub enum ClassElement<'a> {
    PropertyDeclaration(PropertyDeclarationNode<'a>),
    MethodDeclaration(MethodDeclaration<'a>),
    Constructor(ConstructorDeclaration<'a>),
    GetAccessor(GetAccessorDeclaration<'a>),
    SetAccessor(SetAccessorDeclaration<'a>),
    IndexSignature(IndexSignatureNode<'a>),
    SemicolonClassElement(NodeData),
    ClassStaticBlockDeclaration(ClassStaticBlockDeclaration<'a>),
}

#[derive(Debug)]
pub struct PropertyDeclarationNode<'a> {
    pub data: NodeData,
    pub name: PropertyName<'a>,
    pub question_token: Option<Token>,
    pub exclamation_token: Option<Token>,
    pub type_annotation: OptionalNode<'a, TypeNode<'a>>,
    pub initializer: OptionalNode<'a, Expression<'a>>,
}

#[derive(Debug)]
pub struct MethodDeclaration<'a> {
    pub data: NodeData,
    pub name: PropertyName<'a>,
    pub question_token: Option<Token>,
    pub asterisk_token: Option<Token>,
    pub type_parameters: Option<NodeList<'a, TypeParameterDeclaration<'a>>>,
    pub parameters: NodeList<'a, ParameterDeclaration<'a>>,
    pub return_type: OptionalNode<'a, TypeNode<'a>>,
    pub body: Option<Block<'a>>,
}

#[derive(Debug)]
pub struct ConstructorDeclaration<'a> {
    pub data: NodeData,
    pub type_parameters: Option<NodeList<'a, TypeParameterDeclaration<'a>>>,
    pub parameters: NodeList<'a, ParameterDeclaration<'a>>,
    pub body: Option<Block<'a>>,
}

#[derive(Debug)]
pub struct GetAccessorDeclaration<'a> {
    pub data: NodeData,
    pub name: PropertyName<'a>,
    pub type_parameters: Option<NodeList<'a, TypeParameterDeclaration<'a>>>,
    pub parameters: NodeList<'a, ParameterDeclaration<'a>>,
    pub return_type: OptionalNode<'a, TypeNode<'a>>,
    pub body: Option<Block<'a>>,
}

#[derive(Debug)]
pub struct SetAccessorDeclaration<'a> {
    pub data: NodeData,
    pub name: PropertyName<'a>,
    pub type_parameters: Option<NodeList<'a, TypeParameterDeclaration<'a>>>,
    pub parameters: NodeList<'a, ParameterDeclaration<'a>>,
    pub body: Option<Block<'a>>,
}

#[derive(Debug)]
pub struct ClassStaticBlockDeclaration<'a> {
    pub data: NodeData,
    pub body: Block<'a>,
}

#[derive(Debug)]
pub struct InterfaceDeclaration<'a> {
    pub data: NodeData,
    pub name: Identifier,
    pub type_parameters: Option<NodeList<'a, TypeParameterDeclaration<'a>>>,
    pub heritage_clauses: Option<NodeList<'a, HeritageClause<'a>>>,
    pub members: NodeList<'a, TypeElement<'a>>,
}

#[derive(Debug)]
pub struct TypeAliasDeclaration<'a> {
    pub data: NodeData,
    pub name: Identifier,
    pub type_parameters: Option<NodeList<'a, TypeParameterDeclaration<'a>>>,
    pub type_node: &'a TypeNode<'a>,
}

#[derive(Debug)]
pub struct EnumDeclaration<'a> {
    pub data: NodeData,
    pub name: Identifier,
    pub members: NodeList<'a, EnumMemberNode<'a>>,
}

#[derive(Debug)]
pub struct EnumMemberNode<'a> {
    pub data: NodeData,
    pub name: PropertyName<'a>,
    pub initializer: OptionalNode<'a, Expression<'a>>,
}

#[derive(Debug)]
pub struct ModuleDeclaration<'a> {
    pub data: NodeData,
    pub name: ModuleName,
    pub body: Option<ModuleBody<'a>>,
}

#[derive(Debug)]
pub enum ModuleName {
    Identifier(Identifier),
    StringLiteral(StringLiteral),
}

#[derive(Debug)]
pub enum ModuleBody<'a> {
    ModuleBlock(ModuleBlock<'a>),
    ModuleDeclaration(&'a ModuleDeclaration<'a>),
}

#[derive(Debug)]
pub struct ModuleBlock<'a> {
    pub data: NodeData,
    pub statements: NodeList<'a, Statement<'a>>,
}

// ============================================================================
// Import/Export
// ============================================================================

#[derive(Debug)]
pub struct ImportDeclaration<'a> {
    pub data: NodeData,
    pub import_clause: Option<ImportClause<'a>>,
    pub module_specifier: &'a Expression<'a>,
    pub attributes: Option<ImportAttributes<'a>>,
}

#[derive(Debug)]
pub struct ImportClause<'a> {
    pub data: NodeData,
    pub is_type_only: bool,
    pub name: Option<Identifier>,
    pub named_bindings: Option<NamedImportBindings<'a>>,
}

#[derive(Debug)]
pub enum NamedImportBindings<'a> {
    NamespaceImport(NamespaceImport),
    NamedImports(NamedImports<'a>),
}

#[derive(Debug)]
pub struct NamespaceImport {
    pub data: NodeData,
    pub name: Identifier,
}

#[derive(Debug)]
pub struct NamedImports<'a> {
    pub data: NodeData,
    pub elements: NodeList<'a, ImportSpecifier>,
}

#[derive(Debug)]
pub struct ImportSpecifier {
    pub data: NodeData,
    pub is_type_only: bool,
    pub property_name: Option<Identifier>,
    pub name: Identifier,
}

#[derive(Debug)]
pub struct ExportDeclaration<'a> {
    pub data: NodeData,
    pub is_type_only: bool,
    pub export_clause: Option<NamedExportBindings<'a>>,
    pub module_specifier: OptionalNode<'a, Expression<'a>>,
    pub attributes: Option<ImportAttributes<'a>>,
}

#[derive(Debug)]
pub enum NamedExportBindings<'a> {
    NamespaceExport(NamespaceExport),
    NamedExports(NamedExports<'a>),
}

#[derive(Debug)]
pub struct NamespaceExport {
    pub data: NodeData,
    pub name: Identifier,
}

#[derive(Debug)]
pub struct NamedExports<'a> {
    pub data: NodeData,
    pub elements: NodeList<'a, ExportSpecifier>,
}

#[derive(Debug)]
pub struct ExportSpecifier {
    pub data: NodeData,
    pub is_type_only: bool,
    pub property_name: Option<Identifier>,
    pub name: Identifier,
}

#[derive(Debug)]
pub struct ExportAssignment<'a> {
    pub data: NodeData,
    pub is_export_equals: bool,
    pub expression: &'a Expression<'a>,
}

#[derive(Debug)]
pub struct ImportEqualsDeclaration<'a> {
    pub data: NodeData,
    pub is_type_only: bool,
    pub name: Identifier,
    pub module_reference: ModuleReference<'a>,
}

#[derive(Debug)]
pub enum ModuleReference<'a> {
    ExternalModuleReference(ExternalModuleReference<'a>),
    EntityName(EntityName<'a>),
}

#[derive(Debug)]
pub struct ExternalModuleReference<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
}

#[derive(Debug)]
pub struct NamespaceExportDeclaration {
    pub data: NodeData,
    pub name: Identifier,
}

#[derive(Debug)]
pub struct ImportAttributes<'a> {
    pub data: NodeData,
    pub token: SyntaxKind, // AssertKeyword or WithKeyword
    pub elements: NodeList<'a, ImportAttribute<'a>>,
    pub multi_line: bool,
}

#[derive(Debug)]
pub struct ImportAttribute<'a> {
    pub data: NodeData,
    pub name: ImportAttributeName,
    pub value: &'a Expression<'a>,
}

#[derive(Debug)]
pub enum ImportAttributeName {
    Identifier(Identifier),
    StringLiteral(StringLiteral),
}

#[derive(Debug)]
pub struct AssertClause<'a> {
    pub data: NodeData,
    pub elements: NodeList<'a, AssertEntry<'a>>,
    pub multi_line: bool,
}

#[derive(Debug)]
pub struct AssertEntry<'a> {
    pub data: NodeData,
    pub name: ImportAttributeName,
    pub value: &'a Expression<'a>,
}

// ============================================================================
// Decorators
// ============================================================================

#[derive(Debug)]
pub struct Decorator<'a> {
    pub data: NodeData,
    pub expression: &'a Expression<'a>,
}
