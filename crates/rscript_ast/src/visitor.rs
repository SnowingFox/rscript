//! AST visitor trait for traversing the syntax tree.
//!
//! Provides both a `AstVisitor` trait for customizable traversal and
//! a `for_each_child` function for generic iteration over all children.

use crate::node::*;

/// A visitor that traverses the AST. Implement this trait to perform
/// operations on each node kind. Default implementations walk into children.
pub trait AstVisitor<'a> {
    fn visit_source_file(&mut self, node: &SourceFile<'a>) {
        for stmt in node.statements.iter() {
            self.visit_statement(stmt);
        }
    }

    fn visit_statement(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::VariableStatement(n) => self.visit_variable_statement(n),
            Statement::FunctionDeclaration(n) => self.visit_function_declaration(n),
            Statement::ClassDeclaration(n) => self.visit_class_declaration(n),
            Statement::InterfaceDeclaration(n) => self.visit_interface_declaration(n),
            Statement::TypeAliasDeclaration(n) => self.visit_type_alias_declaration(n),
            Statement::EnumDeclaration(n) => self.visit_enum_declaration(n),
            Statement::ModuleDeclaration(n) => self.visit_module_declaration(n),
            Statement::ImportDeclaration(n) => self.visit_import_declaration(n),
            Statement::ImportEqualsDeclaration(n) => self.visit_import_equals_declaration(n),
            Statement::ExportDeclaration(n) => self.visit_export_declaration(n),
            Statement::ExportAssignment(n) => self.visit_export_assignment(n),
            Statement::NamespaceExportDeclaration(n) => self.visit_namespace_export_declaration(n),
            Statement::Block(n) => self.visit_block(n),
            Statement::EmptyStatement(_) => {}
            Statement::ExpressionStatement(n) => self.visit_expression_statement(n),
            Statement::IfStatement(n) => self.visit_if_statement(n),
            Statement::DoStatement(n) => self.visit_do_statement(n),
            Statement::WhileStatement(n) => self.visit_while_statement(n),
            Statement::ForStatement(n) => self.visit_for_statement(n),
            Statement::ForInStatement(n) => self.visit_for_in_statement(n),
            Statement::ForOfStatement(n) => self.visit_for_of_statement(n),
            Statement::ContinueStatement(_) => {}
            Statement::BreakStatement(_) => {}
            Statement::ReturnStatement(n) => self.visit_return_statement(n),
            Statement::WithStatement(n) => self.visit_with_statement(n),
            Statement::SwitchStatement(n) => self.visit_switch_statement(n),
            Statement::LabeledStatement(n) => self.visit_labeled_statement(n),
            Statement::ThrowStatement(n) => self.visit_throw_statement(n),
            Statement::TryStatement(n) => self.visit_try_statement(n),
            Statement::DebuggerStatement(_) => {}
            Statement::MissingDeclaration(_) => {}
        }
    }

    // -- Statements --

    fn visit_variable_statement(&mut self, node: &VariableStatement<'a>) {
        for decl in node.declaration_list.declarations.iter() {
            self.visit_variable_declaration(decl);
        }
    }

    fn visit_variable_declaration(&mut self, node: &VariableDeclaration<'a>) {
        self.visit_binding_name(&node.name);
        if let Some(ty) = node.type_annotation {
            self.visit_type_node(ty);
        }
        if let Some(init) = node.initializer {
            self.visit_expression(init);
        }
    }

    fn visit_function_declaration(&mut self, node: &FunctionDeclaration<'a>) {
        if let Some(ref type_params) = node.type_parameters {
            for tp in type_params.iter() {
                self.visit_type_parameter(tp);
            }
        }
        for param in node.parameters.iter() {
            self.visit_parameter(param);
        }
        if let Some(ret) = node.return_type {
            self.visit_type_node(ret);
        }
        if let Some(ref body) = node.body {
            self.visit_block(body);
        }
    }

    fn visit_class_declaration(&mut self, node: &ClassDeclaration<'a>) {
        if let Some(ref type_params) = node.type_parameters {
            for tp in type_params.iter() {
                self.visit_type_parameter(tp);
            }
        }
        if let Some(ref heritage) = node.heritage_clauses {
            for clause in heritage.iter() {
                self.visit_heritage_clause(clause);
            }
        }
        for member in node.members.iter() {
            self.visit_class_element(member);
        }
    }

    fn visit_interface_declaration(&mut self, node: &InterfaceDeclaration<'a>) {
        if let Some(ref type_params) = node.type_parameters {
            for tp in type_params.iter() {
                self.visit_type_parameter(tp);
            }
        }
        if let Some(ref heritage) = node.heritage_clauses {
            for clause in heritage.iter() {
                self.visit_heritage_clause(clause);
            }
        }
        for member in node.members.iter() {
            self.visit_type_element(member);
        }
    }

    fn visit_type_alias_declaration(&mut self, node: &TypeAliasDeclaration<'a>) {
        if let Some(ref type_params) = node.type_parameters {
            for tp in type_params.iter() {
                self.visit_type_parameter(tp);
            }
        }
        self.visit_type_node(node.type_node);
    }

    fn visit_enum_declaration(&mut self, node: &EnumDeclaration<'a>) {
        for member in node.members.iter() {
            self.visit_enum_member(member);
        }
    }

    fn visit_enum_member(&mut self, node: &EnumMemberNode<'a>) {
        if let Some(init) = node.initializer {
            self.visit_expression(init);
        }
    }

    fn visit_module_declaration(&mut self, node: &ModuleDeclaration<'a>) {
        if let Some(ref body) = node.body {
            match body {
                ModuleBody::ModuleBlock(block) => {
                    for stmt in block.statements.iter() {
                        self.visit_statement(stmt);
                    }
                }
                ModuleBody::ModuleDeclaration(inner) => {
                    self.visit_module_declaration(inner);
                }
            }
        }
    }

    fn visit_import_declaration(&mut self, node: &ImportDeclaration<'a>) {
        if let Some(ref clause) = node.import_clause {
            self.visit_import_clause(clause);
        }
        self.visit_expression(node.module_specifier);
    }

    fn visit_import_clause(&mut self, node: &ImportClause<'a>) {
        if let Some(ref bindings) = node.named_bindings {
            match bindings {
                NamedImportBindings::NamespaceImport(_) => {}
                NamedImportBindings::NamedImports(named) => {
                    for spec in named.elements.iter() {
                        self.visit_import_specifier(spec);
                    }
                }
            }
        }
    }

    fn visit_import_specifier(&mut self, _node: &ImportSpecifier) {}

    fn visit_import_equals_declaration(&mut self, node: &ImportEqualsDeclaration<'a>) {
        match &node.module_reference {
            ModuleReference::ExternalModuleReference(ext) => {
                self.visit_expression(ext.expression);
            }
            ModuleReference::EntityName(_) => {}
        }
    }

    fn visit_export_declaration(&mut self, node: &ExportDeclaration<'a>) {
        if let Some(ref clause) = node.export_clause {
            match clause {
                NamedExportBindings::NamespaceExport(_) => {}
                NamedExportBindings::NamedExports(named) => {
                    for spec in named.elements.iter() {
                        self.visit_export_specifier(spec);
                    }
                }
            }
        }
        if let Some(module_specifier) = node.module_specifier {
            self.visit_expression(module_specifier);
        }
    }

    fn visit_export_specifier(&mut self, _node: &ExportSpecifier) {}

    fn visit_export_assignment(&mut self, node: &ExportAssignment<'a>) {
        self.visit_expression(node.expression);
    }

    fn visit_namespace_export_declaration(&mut self, _node: &NamespaceExportDeclaration) {}

    fn visit_block(&mut self, node: &Block<'a>) {
        for stmt in node.statements.iter() {
            self.visit_statement(stmt);
        }
    }

    fn visit_expression_statement(&mut self, node: &ExpressionStatement<'a>) {
        self.visit_expression(node.expression);
    }

    fn visit_if_statement(&mut self, node: &IfStatement<'a>) {
        self.visit_expression(node.expression);
        self.visit_statement(node.then_statement);
        if let Some(else_stmt) = node.else_statement {
            self.visit_statement(else_stmt);
        }
    }

    fn visit_do_statement(&mut self, node: &DoStatement<'a>) {
        self.visit_statement(node.statement);
        self.visit_expression(node.expression);
    }

    fn visit_while_statement(&mut self, node: &WhileStatement<'a>) {
        self.visit_expression(node.expression);
        self.visit_statement(node.statement);
    }

    fn visit_for_statement(&mut self, node: &ForStatement<'a>) {
        if let Some(ref init) = node.initializer {
            self.visit_for_initializer(init);
        }
        if let Some(cond) = node.condition {
            self.visit_expression(cond);
        }
        if let Some(incr) = node.incrementor {
            self.visit_expression(incr);
        }
        self.visit_statement(node.statement);
    }

    fn visit_for_in_statement(&mut self, node: &ForInStatement<'a>) {
        self.visit_for_initializer(&node.initializer);
        self.visit_expression(node.expression);
        self.visit_statement(node.statement);
    }

    fn visit_for_of_statement(&mut self, node: &ForOfStatement<'a>) {
        self.visit_for_initializer(&node.initializer);
        self.visit_expression(node.expression);
        self.visit_statement(node.statement);
    }

    fn visit_for_initializer(&mut self, init: &ForInitializer<'a>) {
        match init {
            ForInitializer::VariableDeclarationList(list) => {
                for decl in list.declarations.iter() {
                    self.visit_variable_declaration(decl);
                }
            }
            ForInitializer::Expression(expr) => self.visit_expression(expr),
        }
    }

    fn visit_return_statement(&mut self, node: &ReturnStatement<'a>) {
        if let Some(expr) = node.expression {
            self.visit_expression(expr);
        }
    }

    fn visit_with_statement(&mut self, node: &WithStatement<'a>) {
        self.visit_expression(node.expression);
        self.visit_statement(node.statement);
    }

    fn visit_switch_statement(&mut self, node: &SwitchStatement<'a>) {
        self.visit_expression(node.expression);
        for clause in node.case_block.clauses.iter() {
            match clause {
                CaseOrDefaultClause::CaseClause(c) => {
                    self.visit_expression(c.expression);
                    for stmt in c.statements.iter() {
                        self.visit_statement(stmt);
                    }
                }
                CaseOrDefaultClause::DefaultClause(d) => {
                    for stmt in d.statements.iter() {
                        self.visit_statement(stmt);
                    }
                }
            }
        }
    }

    fn visit_labeled_statement(&mut self, node: &LabeledStatement<'a>) {
        self.visit_statement(node.statement);
    }

    fn visit_throw_statement(&mut self, node: &ThrowStatement<'a>) {
        self.visit_expression(node.expression);
    }

    fn visit_try_statement(&mut self, node: &TryStatement<'a>) {
        self.visit_block(&node.try_block);
        if let Some(ref catch) = node.catch_clause {
            if let Some(ref var_decl) = catch.variable_declaration {
                self.visit_variable_declaration(var_decl);
            }
            self.visit_block(&catch.block);
        }
        if let Some(ref finally) = node.finally_block {
            self.visit_block(finally);
        }
    }

    // -- Expressions --

    fn visit_expression(&mut self, expr: &Expression<'a>) {
        match expr {
            Expression::Identifier(_) => {}
            Expression::StringLiteral(_) => {}
            Expression::NumericLiteral(_) => {}
            Expression::BigIntLiteral(_) => {}
            Expression::RegularExpressionLiteral(_) => {}
            Expression::NoSubstitutionTemplateLiteral(_) => {}
            Expression::TemplateExpression(n) => self.visit_template_expression(n),
            Expression::ArrayLiteral(n) => self.visit_array_literal(n),
            Expression::ObjectLiteral(n) => self.visit_object_literal(n),
            Expression::PropertyAccess(n) => self.visit_property_access(n),
            Expression::ElementAccess(n) => self.visit_element_access(n),
            Expression::Call(n) => self.visit_call_expression(n),
            Expression::New(n) => self.visit_new_expression(n),
            Expression::TaggedTemplate(n) => self.visit_tagged_template(n),
            Expression::TypeAssertion(n) => self.visit_type_assertion(n),
            Expression::Parenthesized(n) => self.visit_expression(n.expression),
            Expression::FunctionExpression(n) => self.visit_function_expression(n),
            Expression::ArrowFunction(n) => self.visit_arrow_function(n),
            Expression::Delete(n) => self.visit_expression(n.expression),
            Expression::TypeOf(n) => self.visit_expression(n.expression),
            Expression::Void(n) => self.visit_expression(n.expression),
            Expression::Await(n) => self.visit_expression(n.expression),
            Expression::PrefixUnary(n) => self.visit_expression(n.operand),
            Expression::PostfixUnary(n) => self.visit_expression(n.operand),
            Expression::Binary(n) => self.visit_binary_expression(n),
            Expression::Conditional(n) => self.visit_conditional_expression(n),
            Expression::Yield(n) => {
                if let Some(expr) = n.expression {
                    self.visit_expression(expr);
                }
            }
            Expression::Spread(n) => self.visit_expression(n.expression),
            Expression::ClassExpression(n) => self.visit_class_expression(n),
            Expression::OmittedExpression(_) => {}
            Expression::As(n) => {
                self.visit_expression(n.expression);
                self.visit_type_node(n.type_node);
            }
            Expression::NonNull(n) => self.visit_expression(n.expression),
            Expression::MetaProperty(_) => {}
            Expression::Satisfies(n) => {
                self.visit_expression(n.expression);
                self.visit_type_node(n.type_node);
            }
            Expression::ThisKeyword(_) => {}
            Expression::SuperKeyword(_) => {}
            Expression::NullKeyword(_) => {}
            Expression::TrueKeyword(_) => {}
            Expression::FalseKeyword(_) => {}
        }
    }

    fn visit_template_expression(&mut self, node: &TemplateExpression<'a>) {
        for span in node.template_spans.iter() {
            self.visit_expression(span.expression);
        }
    }

    fn visit_array_literal(&mut self, node: &ArrayLiteralExpression<'a>) {
        for elem in node.elements.iter() {
            self.visit_expression(elem);
        }
    }

    fn visit_object_literal(&mut self, node: &ObjectLiteralExpression<'a>) {
        for prop in node.properties.iter() {
            self.visit_object_literal_element(prop);
        }
    }

    fn visit_object_literal_element(&mut self, elem: &ObjectLiteralElement<'a>) {
        match elem {
            ObjectLiteralElement::PropertyAssignment(n) => {
                self.visit_expression(n.initializer);
            }
            ObjectLiteralElement::ShorthandPropertyAssignment(n) => {
                if let Some(init) = n.object_assignment_initializer {
                    self.visit_expression(init);
                }
            }
            ObjectLiteralElement::SpreadAssignment(n) => {
                self.visit_expression(n.expression);
            }
            ObjectLiteralElement::MethodDeclaration(n) => {
                self.visit_method_declaration(n);
            }
            ObjectLiteralElement::GetAccessor(n) => {
                if let Some(ret) = n.return_type {
                    self.visit_type_node(ret);
                }
                if let Some(ref body) = n.body {
                    self.visit_block(body);
                }
            }
            ObjectLiteralElement::SetAccessor(n) => {
                for param in n.parameters.iter() {
                    self.visit_parameter(param);
                }
                if let Some(ref body) = n.body {
                    self.visit_block(body);
                }
            }
        }
    }

    fn visit_property_access(&mut self, node: &PropertyAccessExpression<'a>) {
        self.visit_expression(node.expression);
    }

    fn visit_element_access(&mut self, node: &ElementAccessExpression<'a>) {
        self.visit_expression(node.expression);
        self.visit_expression(node.argument_expression);
    }

    fn visit_call_expression(&mut self, node: &CallExpression<'a>) {
        self.visit_expression(node.expression);
        if let Some(ref type_args) = node.type_arguments {
            for ta in type_args.iter() {
                self.visit_type_node(ta);
            }
        }
        for arg in node.arguments.iter() {
            self.visit_expression(arg);
        }
    }

    fn visit_new_expression(&mut self, node: &NewExpression<'a>) {
        self.visit_expression(node.expression);
        if let Some(ref type_args) = node.type_arguments {
            for ta in type_args.iter() {
                self.visit_type_node(ta);
            }
        }
        if let Some(ref args) = node.arguments {
            for arg in args.iter() {
                self.visit_expression(arg);
            }
        }
    }

    fn visit_tagged_template(&mut self, node: &TaggedTemplateExpression<'a>) {
        self.visit_expression(node.tag);
        self.visit_expression(node.template);
    }

    fn visit_type_assertion(&mut self, node: &TypeAssertionExpression<'a>) {
        self.visit_type_node(node.type_node);
        self.visit_expression(node.expression);
    }

    fn visit_function_expression(&mut self, node: &FunctionExpression<'a>) {
        if let Some(ref type_params) = node.type_parameters {
            for tp in type_params.iter() {
                self.visit_type_parameter(tp);
            }
        }
        for param in node.parameters.iter() {
            self.visit_parameter(param);
        }
        if let Some(ret) = node.return_type {
            self.visit_type_node(ret);
        }
        self.visit_block(node.body);
    }

    fn visit_arrow_function(&mut self, node: &ArrowFunction<'a>) {
        if let Some(ref type_params) = node.type_parameters {
            for tp in type_params.iter() {
                self.visit_type_parameter(tp);
            }
        }
        for param in node.parameters.iter() {
            self.visit_parameter(param);
        }
        if let Some(ret) = node.return_type {
            self.visit_type_node(ret);
        }
        match &node.body {
            ArrowFunctionBody::Block(block) => self.visit_block(block),
            ArrowFunctionBody::Expression(expr) => self.visit_expression(expr),
        }
    }

    fn visit_binary_expression(&mut self, node: &BinaryExpression<'a>) {
        self.visit_expression(node.left);
        self.visit_expression(node.right);
    }

    fn visit_conditional_expression(&mut self, node: &ConditionalExpression<'a>) {
        self.visit_expression(node.condition);
        self.visit_expression(node.when_true);
        self.visit_expression(node.when_false);
    }

    fn visit_class_expression(&mut self, node: &ClassExpression<'a>) {
        if let Some(ref type_params) = node.type_parameters {
            for tp in type_params.iter() {
                self.visit_type_parameter(tp);
            }
        }
        if let Some(ref heritage) = node.heritage_clauses {
            for clause in heritage.iter() {
                self.visit_heritage_clause(clause);
            }
        }
        for member in node.members.iter() {
            self.visit_class_element(member);
        }
    }

    // -- Type Nodes --

    fn visit_type_node(&mut self, ty: &TypeNode<'a>) {
        match ty {
            TypeNode::KeywordType(_) => {}
            TypeNode::TypeReference(n) => {
                if let Some(ref type_args) = n.type_arguments {
                    for ta in type_args.iter() {
                        self.visit_type_node(ta);
                    }
                }
            }
            TypeNode::FunctionType(n) => {
                if let Some(ref tps) = n.type_parameters {
                    for tp in tps.iter() {
                        self.visit_type_parameter(tp);
                    }
                }
                for param in n.parameters.iter() {
                    self.visit_parameter(param);
                }
                if let Some(ret) = n.return_type {
                    self.visit_type_node(ret);
                }
            }
            TypeNode::ConstructorType(n) => {
                if let Some(ref tps) = n.type_parameters {
                    for tp in tps.iter() {
                        self.visit_type_parameter(tp);
                    }
                }
                for param in n.parameters.iter() {
                    self.visit_parameter(param);
                }
                if let Some(ret) = n.return_type {
                    self.visit_type_node(ret);
                }
            }
            TypeNode::TypeQuery(n) => {
                if let Some(ref type_args) = n.type_arguments {
                    for ta in type_args.iter() {
                        self.visit_type_node(ta);
                    }
                }
            }
            TypeNode::TypeLiteral(n) => {
                for member in n.members.iter() {
                    self.visit_type_element(member);
                }
            }
            TypeNode::ArrayType(n) => self.visit_type_node(n.element_type),
            TypeNode::TupleType(n) => {
                for elem in n.elements.iter() {
                    self.visit_type_node(elem);
                }
            }
            TypeNode::OptionalType(n) => self.visit_type_node(n.type_node),
            TypeNode::RestType(n) => self.visit_type_node(n.type_node),
            TypeNode::UnionType(n) => {
                for t in n.types.iter() {
                    self.visit_type_node(t);
                }
            }
            TypeNode::IntersectionType(n) => {
                for t in n.types.iter() {
                    self.visit_type_node(t);
                }
            }
            TypeNode::ConditionalType(n) => {
                self.visit_type_node(n.check_type);
                self.visit_type_node(n.extends_type);
                self.visit_type_node(n.true_type);
                self.visit_type_node(n.false_type);
            }
            TypeNode::InferType(n) => {
                self.visit_type_parameter(n.type_parameter);
            }
            TypeNode::ParenthesizedType(n) => self.visit_type_node(n.type_node),
            TypeNode::ThisType(_) => {}
            TypeNode::TypeOperator(n) => self.visit_type_node(n.type_node),
            TypeNode::IndexedAccessType(n) => {
                self.visit_type_node(n.object_type);
                self.visit_type_node(n.index_type);
            }
            TypeNode::MappedType(n) => {
                self.visit_type_parameter(n.type_parameter);
                if let Some(name_type) = n.name_type {
                    self.visit_type_node(name_type);
                }
                if let Some(type_node) = n.type_node {
                    self.visit_type_node(type_node);
                }
            }
            TypeNode::LiteralType(n) => {
                self.visit_expression(n.literal);
            }
            TypeNode::NamedTupleMember(n) => {
                self.visit_type_node(n.type_node);
            }
            TypeNode::TemplateLiteralType(n) => {
                for span in n.template_spans.iter() {
                    self.visit_type_node(span.type_node);
                }
            }
            TypeNode::ImportType(n) => {
                self.visit_type_node(n.argument);
                if let Some(ref type_args) = n.type_arguments {
                    for ta in type_args.iter() {
                        self.visit_type_node(ta);
                    }
                }
            }
            TypeNode::TypePredicate(n) => {
                if let Some(type_node) = n.type_node {
                    self.visit_type_node(type_node);
                }
            }
            TypeNode::ExpressionWithTypeArguments(n) => {
                self.visit_expression(n.expression);
                if let Some(ref type_args) = n.type_arguments {
                    for ta in type_args.iter() {
                        self.visit_type_node(ta);
                    }
                }
            }
        }
    }

    // -- Declarations and helpers --

    fn visit_type_parameter(&mut self, node: &TypeParameterDeclaration<'a>) {
        if let Some(constraint) = node.constraint {
            self.visit_type_node(constraint);
        }
        if let Some(default) = node.default {
            self.visit_type_node(default);
        }
    }

    fn visit_parameter(&mut self, node: &ParameterDeclaration<'a>) {
        self.visit_binding_name(&node.name);
        if let Some(ty) = node.type_annotation {
            self.visit_type_node(ty);
        }
        if let Some(init) = node.initializer {
            self.visit_expression(init);
        }
    }

    fn visit_binding_name(&mut self, name: &BindingName<'a>) {
        match name {
            BindingName::Identifier(_) => {}
            BindingName::ObjectBindingPattern(pattern) => {
                for elem in pattern.elements.iter() {
                    self.visit_binding_element(elem);
                }
            }
            BindingName::ArrayBindingPattern(pattern) => {
                for elem in pattern.elements.iter() {
                    match elem {
                        ArrayBindingElement::BindingElement(e) => {
                            self.visit_binding_element(e);
                        }
                        ArrayBindingElement::OmittedExpression(_) => {}
                    }
                }
            }
        }
    }

    fn visit_binding_element(&mut self, node: &BindingElement<'a>) {
        self.visit_binding_name(&node.name);
        if let Some(init) = node.initializer {
            self.visit_expression(init);
        }
    }

    fn visit_heritage_clause(&mut self, node: &HeritageClause<'a>) {
        for ty in node.types.iter() {
            self.visit_expression(ty.expression);
            if let Some(ref type_args) = ty.type_arguments {
                for ta in type_args.iter() {
                    self.visit_type_node(ta);
                }
            }
        }
    }

    fn visit_class_element(&mut self, elem: &ClassElement<'a>) {
        match elem {
            ClassElement::PropertyDeclaration(n) => {
                if let Some(ty) = n.type_annotation {
                    self.visit_type_node(ty);
                }
                if let Some(init) = n.initializer {
                    self.visit_expression(init);
                }
            }
            ClassElement::MethodDeclaration(n) => self.visit_method_declaration(n),
            ClassElement::Constructor(n) => {
                for param in n.parameters.iter() {
                    self.visit_parameter(param);
                }
                if let Some(ref body) = n.body {
                    self.visit_block(body);
                }
            }
            ClassElement::GetAccessor(n) => {
                for param in n.parameters.iter() {
                    self.visit_parameter(param);
                }
                if let Some(ret) = n.return_type {
                    self.visit_type_node(ret);
                }
                if let Some(ref body) = n.body {
                    self.visit_block(body);
                }
            }
            ClassElement::SetAccessor(n) => {
                for param in n.parameters.iter() {
                    self.visit_parameter(param);
                }
                if let Some(ref body) = n.body {
                    self.visit_block(body);
                }
            }
            ClassElement::IndexSignature(n) => {
                for param in n.parameters.iter() {
                    self.visit_parameter(param);
                }
                if let Some(ty) = n.type_annotation {
                    self.visit_type_node(ty);
                }
            }
            ClassElement::SemicolonClassElement(_) => {}
            ClassElement::ClassStaticBlockDeclaration(n) => {
                self.visit_block(&n.body);
            }
        }
    }

    fn visit_method_declaration(&mut self, node: &MethodDeclaration<'a>) {
        if let Some(ref type_params) = node.type_parameters {
            for tp in type_params.iter() {
                self.visit_type_parameter(tp);
            }
        }
        for param in node.parameters.iter() {
            self.visit_parameter(param);
        }
        if let Some(ret) = node.return_type {
            self.visit_type_node(ret);
        }
        if let Some(ref body) = node.body {
            self.visit_block(body);
        }
    }

    fn visit_type_element(&mut self, elem: &TypeElement<'a>) {
        match elem {
            TypeElement::PropertySignature(n) => {
                if let Some(ty) = n.type_annotation {
                    self.visit_type_node(ty);
                }
            }
            TypeElement::MethodSignature(n) => {
                if let Some(ref tps) = n.type_parameters {
                    for tp in tps.iter() {
                        self.visit_type_parameter(tp);
                    }
                }
                for param in n.parameters.iter() {
                    self.visit_parameter(param);
                }
                if let Some(ret) = n.return_type {
                    self.visit_type_node(ret);
                }
            }
            TypeElement::CallSignature(n) => {
                if let Some(ref tps) = n.type_parameters {
                    for tp in tps.iter() {
                        self.visit_type_parameter(tp);
                    }
                }
                for param in n.parameters.iter() {
                    self.visit_parameter(param);
                }
                if let Some(ret) = n.return_type {
                    self.visit_type_node(ret);
                }
            }
            TypeElement::ConstructSignature(n) => {
                if let Some(ref tps) = n.type_parameters {
                    for tp in tps.iter() {
                        self.visit_type_parameter(tp);
                    }
                }
                for param in n.parameters.iter() {
                    self.visit_parameter(param);
                }
                if let Some(ret) = n.return_type {
                    self.visit_type_node(ret);
                }
            }
            TypeElement::IndexSignature(n) => {
                for param in n.parameters.iter() {
                    self.visit_parameter(param);
                }
                if let Some(ty) = n.type_annotation {
                    self.visit_type_node(ty);
                }
            }
        }
    }
}
