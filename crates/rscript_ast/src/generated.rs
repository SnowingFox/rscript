//! Auto-generated helpers and utilities for AST nodes.
//!
//! This module contains generated code for AST node utilities
//! such as node kind checks, casts, etc.

use crate::node::*;
use crate::syntax_kind::SyntaxKind;

/// Helper to get the NodeData from any statement.
impl<'a> Statement<'a> {
    pub fn data(&self) -> &NodeData {
        match self {
            Statement::VariableStatement(n) => &n.data,
            Statement::FunctionDeclaration(n) => &n.data,
            Statement::ClassDeclaration(n) => &n.data,
            Statement::InterfaceDeclaration(n) => &n.data,
            Statement::TypeAliasDeclaration(n) => &n.data,
            Statement::EnumDeclaration(n) => &n.data,
            Statement::ModuleDeclaration(n) => &n.data,
            Statement::ImportDeclaration(n) => &n.data,
            Statement::ImportEqualsDeclaration(n) => &n.data,
            Statement::ExportDeclaration(n) => &n.data,
            Statement::ExportAssignment(n) => &n.data,
            Statement::NamespaceExportDeclaration(n) => &n.data,
            Statement::Block(n) => &n.data,
            Statement::EmptyStatement(d) => d,
            Statement::ExpressionStatement(n) => &n.data,
            Statement::IfStatement(n) => &n.data,
            Statement::DoStatement(n) => &n.data,
            Statement::WhileStatement(n) => &n.data,
            Statement::ForStatement(n) => &n.data,
            Statement::ForInStatement(n) => &n.data,
            Statement::ForOfStatement(n) => &n.data,
            Statement::ContinueStatement(n) => &n.data,
            Statement::BreakStatement(n) => &n.data,
            Statement::ReturnStatement(n) => &n.data,
            Statement::WithStatement(n) => &n.data,
            Statement::SwitchStatement(n) => &n.data,
            Statement::LabeledStatement(n) => &n.data,
            Statement::ThrowStatement(n) => &n.data,
            Statement::TryStatement(n) => &n.data,
            Statement::DebuggerStatement(d) => d,
            Statement::MissingDeclaration(d) => d,
        }
    }

    pub fn kind(&self) -> SyntaxKind {
        self.data().kind
    }

    pub fn pos(&self) -> u32 {
        self.data().range.pos
    }

    pub fn end(&self) -> u32 {
        self.data().range.end
    }
}

/// Helper to get the NodeData from any expression.
impl<'a> Expression<'a> {
    pub fn data(&self) -> &NodeData {
        match self {
            Expression::Identifier(n) => &n.data,
            Expression::StringLiteral(n) => &n.data,
            Expression::NumericLiteral(n) => &n.data,
            Expression::BigIntLiteral(n) => &n.data,
            Expression::RegularExpressionLiteral(n) => &n.data,
            Expression::NoSubstitutionTemplateLiteral(n) => &n.data,
            Expression::TemplateExpression(n) => &n.data,
            Expression::ArrayLiteral(n) => &n.data,
            Expression::ObjectLiteral(n) => &n.data,
            Expression::PropertyAccess(n) => &n.data,
            Expression::ElementAccess(n) => &n.data,
            Expression::Call(n) => &n.data,
            Expression::New(n) => &n.data,
            Expression::TaggedTemplate(n) => &n.data,
            Expression::TypeAssertion(n) => &n.data,
            Expression::Parenthesized(n) => &n.data,
            Expression::FunctionExpression(n) => &n.data,
            Expression::ArrowFunction(n) => &n.data,
            Expression::Delete(n) => &n.data,
            Expression::TypeOf(n) => &n.data,
            Expression::Void(n) => &n.data,
            Expression::Await(n) => &n.data,
            Expression::PrefixUnary(n) => &n.data,
            Expression::PostfixUnary(n) => &n.data,
            Expression::Binary(n) => &n.data,
            Expression::Conditional(n) => &n.data,
            Expression::Yield(n) => &n.data,
            Expression::Spread(n) => &n.data,
            Expression::ClassExpression(n) => &n.data,
            Expression::OmittedExpression(d) => d,
            Expression::As(n) => &n.data,
            Expression::NonNull(n) => &n.data,
            Expression::MetaProperty(n) => &n.data,
            Expression::Satisfies(n) => &n.data,
            Expression::ThisKeyword(d) => d,
            Expression::SuperKeyword(d) => d,
            Expression::NullKeyword(d) => d,
            Expression::TrueKeyword(d) => d,
            Expression::FalseKeyword(d) => d,
        }
    }

    pub fn kind(&self) -> SyntaxKind {
        self.data().kind
    }
}

/// Helper to get the NodeData from any type node.
impl<'a> TypeNode<'a> {
    pub fn data(&self) -> &NodeData {
        match self {
            TypeNode::KeywordType(n) => &n.data,
            TypeNode::TypeReference(n) => &n.data,
            TypeNode::FunctionType(n) => &n.data,
            TypeNode::ConstructorType(n) => &n.data,
            TypeNode::TypeQuery(n) => &n.data,
            TypeNode::TypeLiteral(n) => &n.data,
            TypeNode::ArrayType(n) => &n.data,
            TypeNode::TupleType(n) => &n.data,
            TypeNode::OptionalType(n) => &n.data,
            TypeNode::RestType(n) => &n.data,
            TypeNode::UnionType(n) => &n.data,
            TypeNode::IntersectionType(n) => &n.data,
            TypeNode::ConditionalType(n) => &n.data,
            TypeNode::InferType(n) => &n.data,
            TypeNode::ParenthesizedType(n) => &n.data,
            TypeNode::ThisType(n) => &n.data,
            TypeNode::TypeOperator(n) => &n.data,
            TypeNode::IndexedAccessType(n) => &n.data,
            TypeNode::MappedType(n) => &n.data,
            TypeNode::LiteralType(n) => &n.data,
            TypeNode::NamedTupleMember(n) => &n.data,
            TypeNode::TemplateLiteralType(n) => &n.data,
            TypeNode::ImportType(n) => &n.data,
            TypeNode::TypePredicate(n) => &n.data,
            TypeNode::ExpressionWithTypeArguments(n) => &n.data,
        }
    }
}
