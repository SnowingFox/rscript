//! The binder implementation.
//!
//! Walks the AST and builds symbol tables, scopes, and flow graphs.
//! Handles:
//! - Symbol creation for all declarations
//! - Scope management (block scopes, function scopes)
//! - Symbol resolution via scope chain
//! - Function/var hoisting
//! - Class and enum member binding
//! - Parameter binding
//! - Import/export binding
//! - Declaration merging (interfaces, namespaces)

use crate::scope::Scope;
use crate::symbol::{Symbol, SymbolTable};
use rscript_ast::node::*;
use rscript_ast::types::*;
use rscript_core::intern::InternedString;
use rscript_diagnostics::DiagnosticCollection;

/// Flow node kinds for control flow analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowNodeKind {
    Start,
    BranchLabel,
    LoopLabel,
    Assignment,
    TrueCondition,
    FalseCondition,
    Narrowing,
    Call,
    ReduceLabel,
    Unreachable,
}

/// A flow node in the control flow graph.
#[derive(Debug)]
pub struct FlowNode {
    pub kind: FlowNodeKind,
    pub id: u32,
    /// Predecessors in the flow graph.
    pub antecedents: Vec<u32>,
    /// Associated AST node.
    pub node: Option<NodeId>,
}

/// The binder creates symbols and links declarations.
pub struct Binder {
    /// All symbols created during binding.
    symbols: Vec<Symbol>,
    /// The current scope.
    current_scope: Option<Box<Scope>>,
    /// The global symbol table.
    pub globals: SymbolTable,
    /// Next symbol ID to assign.
    next_symbol_id: u32,
    /// File-level symbol for the source file.
    pub file_symbol: Option<SymbolId>,
    /// Flow nodes for control flow analysis.
    flow_nodes: Vec<FlowNode>,
    /// Current flow node.
    current_flow: u32,
    /// Next flow node ID.
    next_flow_id: u32,
    /// Diagnostics from binding.
    diagnostics: DiagnosticCollection,
    /// Whether we're in a strict mode context.
    #[allow(dead_code)]
    in_strict_mode: bool,
    /// Nesting depth for scope tracking.
    scope_depth: u32,
}

impl Binder {
    /// Maximum scope chain traversal depth to guard against cycles.
    const MAX_SCOPE_DEPTH: u32 = 500;

    pub fn new() -> Self {
        let mut binder = Self {
            symbols: Vec::new(),
            current_scope: Some(Box::new(Scope::new(None))),
            globals: SymbolTable::new(),
            next_symbol_id: 0,
            file_symbol: None,
            flow_nodes: Vec::new(),
            current_flow: 0,
            next_flow_id: 0,
            diagnostics: DiagnosticCollection::new(),
            in_strict_mode: false,
            scope_depth: 0,
        };
        // Create the start flow node
        binder.create_flow_node(FlowNodeKind::Start, None);
        binder
    }

    /// Take diagnostics from the binder.
    pub fn take_diagnostics(&mut self) -> DiagnosticCollection {
        std::mem::take(&mut self.diagnostics)
    }

    /// Get the flow nodes.
    pub fn flow_nodes(&self) -> &[FlowNode] {
        &self.flow_nodes
    }

    /// Get all symbols created by this binder.
    pub fn get_symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    /// Get a symbol by ID.
    pub fn get_symbol(&self, id: SymbolId) -> Option<&Symbol> {
        self.symbols.get(id.index())
    }

    // ========================================================================
    // Source file binding
    // ========================================================================

    /// Bind a source file, creating symbols for all declarations.
    pub fn bind_source_file(&mut self, source_file: &SourceFile<'_>) {
        // Detect strict mode from "use strict" directive
        if let Some(Statement::ExpressionStatement(expr_stmt)) = source_file.statements.first() {
            if let Expression::StringLiteral(s) = expr_stmt.expression {
                // Check if it's "use strict" (simplified - we'd need interned string lookup)
                let _ = s;
                // self.in_strict_mode = true;
            }
        }

        // Hoist function declarations and var declarations
        self.hoist_declarations(source_file.statements);

        // Bind all statements
        for statement in source_file.statements.iter() {
            self.bind_statement(statement);
        }
    }

    // ========================================================================
    // Hoisting
    // ========================================================================

    /// Hoist function and var declarations in the current scope.
    fn hoist_declarations(&mut self, statements: &[Statement<'_>]) {
        for stmt in statements {
            match stmt {
                Statement::FunctionDeclaration(n) => {
                    // Function declarations are hoisted entirely
                    if let Some(ref name) = n.name {
                        self.declare_symbol_with_text(name.text, name.text_name.clone(), SymbolFlags::FUNCTION, n.data.id);
                    }
                }
                Statement::VariableStatement(n) => {
                    // Only `var` is hoisted (not let/const)
                    if !n.declaration_list.data.flags.contains(NodeFlags::LET)
                        && !n.declaration_list.data.flags.contains(NodeFlags::CONST)
                    {
                        for decl in n.declaration_list.declarations.iter() {
                            self.hoist_binding_name(&decl.name, decl.data.id);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn hoist_binding_name(&mut self, name: &BindingName<'_>, node_id: NodeId) {
        match name {
            BindingName::Identifier(id) => {
                self.declare_symbol_with_text(id.text, id.text_name.clone(), SymbolFlags::FUNCTION_SCOPED_VARIABLE, node_id);
            }
            BindingName::ObjectBindingPattern(pattern) => {
                for elem in pattern.elements.iter() {
                    self.hoist_binding_name(&elem.name, node_id);
                }
            }
            BindingName::ArrayBindingPattern(pattern) => {
                for elem in pattern.elements.iter() {
                    if let ArrayBindingElement::BindingElement(e) = elem {
                        self.hoist_binding_name(&e.name, node_id);
                    }
                }
            }
        }
    }

    // ========================================================================
    // Statement binding
    // ========================================================================

    fn bind_statement(&mut self, stmt: &Statement<'_>) {
        match stmt {
            Statement::VariableStatement(n) => self.bind_variable_statement(n),
            Statement::FunctionDeclaration(n) => self.bind_function_declaration(n),
            Statement::ClassDeclaration(n) => self.bind_class_declaration(n),
            Statement::InterfaceDeclaration(n) => self.bind_interface_declaration(n),
            Statement::TypeAliasDeclaration(n) => self.bind_type_alias_declaration(n),
            Statement::EnumDeclaration(n) => self.bind_enum_declaration(n),
            Statement::ModuleDeclaration(n) => self.bind_module_declaration(n),
            Statement::ImportDeclaration(n) => self.bind_import_declaration(n),
            Statement::ExportDeclaration(n) => self.bind_export_declaration(n),
            Statement::ExportAssignment(n) => self.bind_export_assignment(n),
            Statement::Block(n) => {
                self.push_block_scope();
                for s in n.statements.iter() {
                    self.bind_statement(s);
                }
                self.pop_scope();
            }
            Statement::IfStatement(n) => self.bind_if_statement(n),
            Statement::WhileStatement(n) => self.bind_while_statement(n),
            Statement::DoStatement(n) => self.bind_do_statement(n),
            Statement::ForStatement(n) => self.bind_for_statement(n),
            Statement::ForInStatement(n) => self.bind_for_in_statement(n),
            Statement::ForOfStatement(n) => self.bind_for_of_statement(n),
            Statement::SwitchStatement(n) => self.bind_switch_statement(n),
            Statement::TryStatement(n) => self.bind_try_statement(n),
            Statement::LabeledStatement(n) => {
                self.bind_statement(n.statement);
            }
            Statement::ReturnStatement(n) => {
                if let Some(expr) = n.expression {
                    self.bind_expression(expr);
                }
            }
            Statement::ThrowStatement(n) => {
                self.bind_expression(n.expression);
            }
            Statement::ExpressionStatement(n) => {
                self.bind_expression(n.expression);
            }
            Statement::WithStatement(n) => {
                self.bind_expression(n.expression);
                self.bind_statement(n.statement);
            }
            _ => {}
        }
    }

    fn bind_variable_statement(&mut self, node: &VariableStatement<'_>) {
        let is_block_scoped = node.declaration_list.data.flags.contains(NodeFlags::LET)
            || node.declaration_list.data.flags.contains(NodeFlags::CONST);

        for decl in node.declaration_list.declarations.iter() {
            let flags = if is_block_scoped {
                SymbolFlags::BLOCK_SCOPED_VARIABLE
            } else {
                SymbolFlags::FUNCTION_SCOPED_VARIABLE
            };

            // Always bind the variable name (both var and let/const)
            self.bind_binding_name(&decl.name, flags, decl.data.id);

            if let Some(init) = decl.initializer {
                self.bind_expression(init);
                // Create flow node for assignment
                self.create_flow_node(FlowNodeKind::Assignment, Some(decl.data.id));
            }
        }
    }

    fn bind_binding_name(&mut self, name: &BindingName<'_>, flags: SymbolFlags, node_id: NodeId) {
        match name {
            BindingName::Identifier(id) => {
                self.declare_symbol_with_text(id.text, id.text_name.clone(), flags, node_id);
            }
            BindingName::ObjectBindingPattern(pattern) => {
                for elem in pattern.elements.iter() {
                    self.bind_binding_name(&elem.name, flags, node_id);
                }
            }
            BindingName::ArrayBindingPattern(pattern) => {
                for elem in pattern.elements.iter() {
                    if let ArrayBindingElement::BindingElement(e) = elem {
                        self.bind_binding_name(&e.name, flags, node_id);
                    }
                }
            }
        }
    }

    fn bind_function_declaration(&mut self, node: &FunctionDeclaration<'_>) {
        // Function itself was hoisted; now bind its body
        if let Some(ref name) = node.name {
            // Symbol already created during hoisting; check for re-declaration
            let _ = name;
        }

        // Bind parameters in a new scope
        self.push_function_scope(node.data.id);
        for param in node.parameters.iter() {
            self.bind_parameter(param);
        }
        if let Some(ref body) = node.body {
            for s in body.statements.iter() {
                self.bind_statement(s);
            }
        }
        self.pop_scope();
    }

    fn bind_class_declaration(&mut self, node: &ClassDeclaration<'_>) {
        if let Some(ref name) = node.name {
            self.declare_symbol_with_text(name.text, name.text_name.clone(), SymbolFlags::CLASS, node.data.id);
        }

        // Create a scope for class members
        self.push_block_scope();

        // Bind heritage clauses
        if let Some(heritage) = node.heritage_clauses {
            for clause in heritage.iter() {
                for ty in clause.types.iter() {
                    self.bind_expression(ty.expression);
                }
            }
        }

        // Bind class members
        for member in node.members.iter() {
            self.bind_class_element(member);
        }

        self.pop_scope();
    }

    /// Convert AST modifier flags to symbol visibility flags.
    fn visibility_flags(modifier_flags: ModifierFlags) -> SymbolFlags {
        let mut flags = SymbolFlags::empty();
        if modifier_flags.contains(ModifierFlags::PRIVATE) {
            flags |= SymbolFlags::PRIVATE;
        }
        if modifier_flags.contains(ModifierFlags::PROTECTED) {
            flags |= SymbolFlags::PROTECTED;
        }
        if modifier_flags.contains(ModifierFlags::STATIC) {
            flags |= SymbolFlags::STATIC;
        }
        flags
    }

    fn bind_class_element(&mut self, elem: &ClassElement<'_>) {
        match elem {
            ClassElement::PropertyDeclaration(n) => {
                // Create symbol for the property name with visibility
                let vis = Self::visibility_flags(n.data.modifier_flags);
                self.declare_property_name_symbol(&n.name, SymbolFlags::PROPERTY | vis, n.data.id);
                if let Some(init) = n.initializer {
                    self.bind_expression(init);
                }
            }
            ClassElement::MethodDeclaration(n) => {
                // Create symbol for the method name with visibility
                let vis = Self::visibility_flags(n.data.modifier_flags);
                self.declare_property_name_symbol(&n.name, SymbolFlags::METHOD | vis, n.data.id);
                self.push_function_scope(n.data.id);
                for param in n.parameters.iter() {
                    self.bind_parameter(param);
                }
                if let Some(ref body) = n.body {
                    for s in body.statements.iter() {
                        self.bind_statement(s);
                    }
                }
                self.pop_scope();
            }
            ClassElement::Constructor(n) => {
                self.push_function_scope(n.data.id);
                for param in n.parameters.iter() {
                    self.bind_parameter(param);
                    // Constructor parameter properties
                    if param.data.modifier_flags.intersects(
                        ModifierFlags::PUBLIC | ModifierFlags::PRIVATE
                        | ModifierFlags::PROTECTED | ModifierFlags::READONLY
                    ) {
                        if let BindingName::Identifier(ref id) = param.name {
                            self.declare_symbol_with_text(id.text, id.text_name.clone(), SymbolFlags::PROPERTY, param.data.id);
                        }
                    }
                }
                if let Some(ref body) = n.body {
                    for s in body.statements.iter() {
                        self.bind_statement(s);
                    }
                }
                self.pop_scope();
            }
            ClassElement::GetAccessor(n) => {
                self.push_function_scope(n.data.id);
                for param in n.parameters.iter() {
                    self.bind_parameter(param);
                }
                if let Some(ref body) = n.body {
                    for s in body.statements.iter() {
                        self.bind_statement(s);
                    }
                }
                self.pop_scope();
            }
            ClassElement::SetAccessor(n) => {
                self.push_function_scope(n.data.id);
                for param in n.parameters.iter() {
                    self.bind_parameter(param);
                }
                if let Some(ref body) = n.body {
                    for s in body.statements.iter() {
                        self.bind_statement(s);
                    }
                }
                self.pop_scope();
            }
            ClassElement::ClassStaticBlockDeclaration(n) => {
                self.push_block_scope();
                for s in n.body.statements.iter() {
                    self.bind_statement(s);
                }
                self.pop_scope();
            }
            ClassElement::IndexSignature(_) | ClassElement::SemicolonClassElement(_) => {}
        }
    }

    fn bind_interface_declaration(&mut self, node: &InterfaceDeclaration<'_>) {
        // Declaration merging: check if symbol already exists
        if let Some(scope) = &self.current_scope {
            if let Some(existing_id) = scope.locals.get(&node.name.text) {
                // Merge: add this declaration to existing symbol
                if let Some(symbol) = self.symbols.get_mut(existing_id.index()) {
                    if symbol.flags.contains(SymbolFlags::INTERFACE) {
                        symbol.declarations.push(node.data.id);
                        return;
                    }
                }
            }
        }
        self.declare_symbol_with_text(node.name.text, node.name.text_name.clone(), SymbolFlags::INTERFACE, node.data.id);
    }

    fn bind_type_alias_declaration(&mut self, node: &TypeAliasDeclaration<'_>) {
        self.declare_symbol_with_text(node.name.text, node.name.text_name.clone(), SymbolFlags::TYPE_ALIAS, node.data.id);
    }

    fn bind_enum_declaration(&mut self, node: &EnumDeclaration<'_>) {
        let enum_symbol = self.declare_symbol_with_text(node.name.text, node.name.text_name.clone(), SymbolFlags::REGULAR_ENUM, node.data.id);

        // Bind enum members
        for member in node.members.iter() {
            if let PropertyName::Identifier(ref id) = member.name {
                let member_symbol = self.declare_symbol_with_text(id.text, id.text_name.clone(), SymbolFlags::ENUM_MEMBER, member.data.id);
                // Set parent
                if let Some(sym) = self.symbols.get_mut(member_symbol.index()) {
                    sym.parent = Some(enum_symbol);
                }
            }
            if let Some(init) = member.initializer {
                self.bind_expression(init);
            }
        }
    }

    fn bind_module_declaration(&mut self, node: &ModuleDeclaration<'_>) {
        let sym_id = if let ModuleName::Identifier(ref name) = node.name {
            Some(self.declare_symbol_with_text(name.text, name.text_name.clone(), SymbolFlags::VALUE_MODULE, node.data.id))
        } else {
            None
        };

        if let Some(ref body) = node.body {
            match body {
                ModuleBody::ModuleBlock(block) => {
                    self.push_block_scope();
                    for s in block.statements.iter() {
                        self.bind_statement(s);
                        // Track exported declarations in the namespace symbol's exports table
                        if let Some(sid) = sym_id {
                            self.collect_namespace_export(s, sid);
                        }
                    }
                    self.pop_scope();
                }
                ModuleBody::ModuleDeclaration(inner) => {
                    self.bind_module_declaration(inner);
                }
            }
        }
    }

    /// If a statement is an export declaration, record its symbols in the
    /// namespace symbol's `exports` table.
    fn collect_namespace_export(&mut self, stmt: &Statement<'_>, ns_symbol: SymbolId) {
        match stmt {
            Statement::ExportDeclaration(ed) => {
                if let Some(NamedExportBindings::NamedExports(ref named)) = ed.export_clause {
                    for spec in named.elements.iter() {
                        let name_text = spec.name.text_name.clone();
                        let interned = spec.name.text;
                        // Create an alias symbol for the export
                        let export_sym = self.declare_symbol(interned, SymbolFlags::EXPORT_VALUE, ed.data.id);
                        if let Some(ns) = self.symbols.get_mut(ns_symbol.index()) {
                            if ns.exports.is_none() {
                                ns.exports = Some(SymbolTable::new());
                            }
                            if let Some(ref mut exports) = ns.exports {
                                exports.set(interned, export_sym);
                            }
                            if !name_text.is_empty() {
                                // Also set by name_text for lookup
                            }
                        }
                    }
                }
            }
            // Track exported function/class/variable declarations
            Statement::FunctionDeclaration(n) => {
                if n.data.modifier_flags.contains(ModifierFlags::EXPORT) {
                    if let Some(ref name) = n.name {
                        let resolved = self.resolve_name(&name.text_name);
                        if let Some(sym_id) = resolved {
                            if let Some(ns) = self.symbols.get_mut(ns_symbol.index()) {
                                if ns.exports.is_none() { ns.exports = Some(SymbolTable::new()); }
                                if let Some(ref mut exports) = ns.exports {
                                    exports.set(name.text, sym_id);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn bind_import_declaration(&mut self, node: &ImportDeclaration<'_>) {
        if let Some(ref clause) = node.import_clause {
            // Default import
            if let Some(ref name) = clause.name {
                self.declare_symbol_with_text(name.text, name.text_name.clone(), SymbolFlags::ALIAS, node.data.id);
            }

            // Named imports
            if let Some(ref bindings) = clause.named_bindings {
                match bindings {
                    NamedImportBindings::NamespaceImport(ns) => {
                        self.declare_symbol_with_text(ns.name.text, ns.name.text_name.clone(), SymbolFlags::ALIAS, node.data.id);
                    }
                    NamedImportBindings::NamedImports(named) => {
                        for spec in named.elements.iter() {
                            self.declare_symbol_with_text(spec.name.text, spec.name.text_name.clone(), SymbolFlags::ALIAS, spec.data.id);
                        }
                    }
                }
            }
        }
    }

    fn bind_export_declaration(&mut self, node: &ExportDeclaration<'_>) {
        if let Some(ref clause) = node.export_clause {
            match clause {
                NamedExportBindings::NamespaceExport(ns) => {
                    self.declare_symbol_with_text(ns.name.text, ns.name.text_name.clone(), SymbolFlags::ALIAS, node.data.id);
                }
                NamedExportBindings::NamedExports(_named) => {
                    // Export specifiers don't create new symbols in the scope
                }
            }
        }
    }

    fn bind_export_assignment(&mut self, _node: &ExportAssignment<'_>) {
        // Export assignments don't create named symbols
    }

    fn bind_parameter(&mut self, param: &ParameterDeclaration<'_>) {
        self.bind_binding_name(&param.name, SymbolFlags::FUNCTION_SCOPED_VARIABLE, param.data.id);
        if let Some(init) = param.initializer {
            self.bind_expression(init);
        }
    }

    // ========================================================================
    // Control flow statement binding
    // ========================================================================

    fn bind_if_statement(&mut self, node: &IfStatement<'_>) {
        self.bind_expression(node.expression);

        // Create branch flow nodes
        let pre_if_flow = self.current_flow;
        let true_flow = self.create_flow_node(FlowNodeKind::TrueCondition, None);
        self.current_flow = true_flow;
        self.bind_statement(node.then_statement);
        let post_then_flow = self.current_flow;

        if let Some(else_stmt) = node.else_statement {
            self.current_flow = pre_if_flow;
            let false_flow = self.create_flow_node(FlowNodeKind::FalseCondition, None);
            self.current_flow = false_flow;
            self.bind_statement(else_stmt);
            let post_else_flow = self.current_flow;

            // Merge
            let merge = self.create_flow_node(FlowNodeKind::BranchLabel, None);
            if let Some(node) = self.flow_nodes.get_mut(merge as usize) {
                node.antecedents.push(post_then_flow);
                node.antecedents.push(post_else_flow);
            }
            self.current_flow = merge;
        } else {
            let merge = self.create_flow_node(FlowNodeKind::BranchLabel, None);
            if let Some(node) = self.flow_nodes.get_mut(merge as usize) {
                node.antecedents.push(post_then_flow);
                node.antecedents.push(pre_if_flow);
            }
            self.current_flow = merge;
        }
    }

    fn bind_while_statement(&mut self, node: &WhileStatement<'_>) {
        let loop_label = self.create_flow_node(FlowNodeKind::LoopLabel, None);
        self.current_flow = loop_label;
        self.bind_expression(node.expression);
        let true_flow = self.create_flow_node(FlowNodeKind::TrueCondition, None);
        self.current_flow = true_flow;
        self.bind_statement(node.statement);
        // Loop back
        if let Some(node) = self.flow_nodes.get_mut(loop_label as usize) {
            node.antecedents.push(self.current_flow);
        }
        let false_flow = self.create_flow_node(FlowNodeKind::FalseCondition, None);
        self.current_flow = false_flow;
    }

    fn bind_do_statement(&mut self, node: &DoStatement<'_>) {
        let loop_label = self.create_flow_node(FlowNodeKind::LoopLabel, None);
        self.current_flow = loop_label;
        self.bind_statement(node.statement);
        self.bind_expression(node.expression);
        if let Some(n) = self.flow_nodes.get_mut(loop_label as usize) {
            n.antecedents.push(self.current_flow);
        }
    }

    fn bind_for_statement(&mut self, node: &ForStatement<'_>) {
        self.push_block_scope();
        if let Some(ref init) = node.initializer {
            match init {
                ForInitializer::VariableDeclarationList(list) => {
                    let is_block = list.data.flags.contains(NodeFlags::LET)
                        || list.data.flags.contains(NodeFlags::CONST);
                    for decl in list.declarations.iter() {
                        if is_block {
                            self.bind_binding_name(&decl.name, SymbolFlags::BLOCK_SCOPED_VARIABLE, decl.data.id);
                        }
                        if let Some(init_expr) = decl.initializer {
                            self.bind_expression(init_expr);
                        }
                    }
                }
                ForInitializer::Expression(expr) => self.bind_expression(expr),
            }
        }
        let loop_label = self.create_flow_node(FlowNodeKind::LoopLabel, None);
        self.current_flow = loop_label;
        if let Some(cond) = node.condition {
            self.bind_expression(cond);
        }
        self.bind_statement(node.statement);
        if let Some(incr) = node.incrementor {
            self.bind_expression(incr);
        }
        if let Some(n) = self.flow_nodes.get_mut(loop_label as usize) {
            n.antecedents.push(self.current_flow);
        }
        self.pop_scope();
    }

    fn bind_for_in_statement(&mut self, node: &ForInStatement<'_>) {
        self.push_block_scope();
        match &node.initializer {
            ForInitializer::VariableDeclarationList(list) => {
                for decl in list.declarations.iter() {
                    self.bind_binding_name(&decl.name, SymbolFlags::BLOCK_SCOPED_VARIABLE, decl.data.id);
                }
            }
            ForInitializer::Expression(expr) => self.bind_expression(expr),
        }
        self.bind_expression(node.expression);
        self.bind_statement(node.statement);
        self.pop_scope();
    }

    fn bind_for_of_statement(&mut self, node: &ForOfStatement<'_>) {
        self.push_block_scope();
        match &node.initializer {
            ForInitializer::VariableDeclarationList(list) => {
                for decl in list.declarations.iter() {
                    self.bind_binding_name(&decl.name, SymbolFlags::BLOCK_SCOPED_VARIABLE, decl.data.id);
                }
            }
            ForInitializer::Expression(expr) => self.bind_expression(expr),
        }
        self.bind_expression(node.expression);
        self.bind_statement(node.statement);
        self.pop_scope();
    }

    fn bind_switch_statement(&mut self, node: &SwitchStatement<'_>) {
        self.bind_expression(node.expression);
        for clause in node.case_block.clauses.iter() {
            match clause {
                CaseOrDefaultClause::CaseClause(c) => {
                    self.bind_expression(c.expression);
                    for s in c.statements.iter() {
                        self.bind_statement(s);
                    }
                }
                CaseOrDefaultClause::DefaultClause(d) => {
                    for s in d.statements.iter() {
                        self.bind_statement(s);
                    }
                }
            }
        }
    }

    fn bind_try_statement(&mut self, node: &TryStatement<'_>) {
        self.push_block_scope();
        for s in node.try_block.statements.iter() {
            self.bind_statement(s);
        }
        self.pop_scope();

        if let Some(ref catch) = node.catch_clause {
            self.push_block_scope();
            if let Some(ref var_decl) = catch.variable_declaration {
                self.bind_binding_name(&var_decl.name, SymbolFlags::BLOCK_SCOPED_VARIABLE, var_decl.data.id);
            }
            for s in catch.block.statements.iter() {
                self.bind_statement(s);
            }
            self.pop_scope();
        }

        if let Some(ref finally) = node.finally_block {
            self.push_block_scope();
            for s in finally.statements.iter() {
                self.bind_statement(s);
            }
            self.pop_scope();
        }
    }

    // ========================================================================
    // Expression binding
    // ========================================================================

    fn bind_expression(&mut self, expr: &Expression<'_>) {
        match expr {
            Expression::Binary(n) => {
                self.bind_expression(n.left);
                self.bind_expression(n.right);
                if n.operator_token.data.kind.is_assignment_operator() {
                    self.create_flow_node(FlowNodeKind::Assignment, None);
                }
            }
            Expression::Call(n) => {
                self.bind_expression(n.expression);
                for arg in n.arguments.iter() {
                    self.bind_expression(arg);
                }
                self.create_flow_node(FlowNodeKind::Call, None);
            }
            Expression::New(n) => {
                self.bind_expression(n.expression);
                if let Some(args) = n.arguments {
                    for arg in args.iter() {
                        self.bind_expression(arg);
                    }
                }
            }
            Expression::PropertyAccess(n) => {
                self.bind_expression(n.expression);
            }
            Expression::ElementAccess(n) => {
                self.bind_expression(n.expression);
                self.bind_expression(n.argument_expression);
            }
            Expression::Conditional(n) => {
                self.bind_expression(n.condition);
                let pre_flow = self.current_flow;
                let true_flow = self.create_flow_node(FlowNodeKind::TrueCondition, None);
                self.current_flow = true_flow;
                self.bind_expression(n.when_true);
                let post_true = self.current_flow;
                self.current_flow = pre_flow;
                let false_flow = self.create_flow_node(FlowNodeKind::FalseCondition, None);
                self.current_flow = false_flow;
                self.bind_expression(n.when_false);
                let post_false = self.current_flow;
                let merge = self.create_flow_node(FlowNodeKind::BranchLabel, None);
                if let Some(node) = self.flow_nodes.get_mut(merge as usize) {
                    node.antecedents.push(post_true);
                    node.antecedents.push(post_false);
                }
                self.current_flow = merge;
            }
            Expression::ArrowFunction(n) => {
                self.push_function_scope(n.data.id);
                for param in n.parameters.iter() {
                    self.bind_parameter(param);
                }
                match &n.body {
                    ArrowFunctionBody::Block(block) => {
                        for s in block.statements.iter() {
                            self.bind_statement(s);
                        }
                    }
                    ArrowFunctionBody::Expression(e) => {
                        self.bind_expression(e);
                    }
                }
                self.pop_scope();
            }
            Expression::FunctionExpression(n) => {
                self.push_function_scope(n.data.id);
                if let Some(ref name) = n.name {
                    self.declare_symbol_with_text(name.text, name.text_name.clone(), SymbolFlags::FUNCTION, n.data.id);
                }
                for param in n.parameters.iter() {
                    self.bind_parameter(param);
                }
                for s in n.body.statements.iter() {
                    self.bind_statement(s);
                }
                self.pop_scope();
            }
            Expression::ClassExpression(n) => {
                if let Some(ref name) = n.name {
                    self.declare_symbol_with_text(name.text, name.text_name.clone(), SymbolFlags::CLASS, n.data.id);
                }
                self.push_block_scope();
                for member in n.members.iter() {
                    self.bind_class_element(member);
                }
                self.pop_scope();
            }
            Expression::ArrayLiteral(n) => {
                for elem in n.elements.iter() {
                    self.bind_expression(elem);
                }
            }
            Expression::ObjectLiteral(n) => {
                for prop in n.properties.iter() {
                    match prop {
                        ObjectLiteralElement::PropertyAssignment(p) => {
                            self.bind_expression(p.initializer);
                        }
                        ObjectLiteralElement::ShorthandPropertyAssignment(p) => {
                            if let Some(init) = p.object_assignment_initializer {
                                self.bind_expression(init);
                            }
                        }
                        ObjectLiteralElement::SpreadAssignment(p) => {
                            self.bind_expression(p.expression);
                        }
                        ObjectLiteralElement::MethodDeclaration(m) => {
                            self.push_function_scope(m.data.id);
                            for param in m.parameters.iter() {
                                self.bind_parameter(param);
                            }
                            if let Some(ref body) = m.body {
                                for s in body.statements.iter() {
                                    self.bind_statement(s);
                                }
                            }
                            self.pop_scope();
                        }
                        ObjectLiteralElement::GetAccessor(g) => {
                            self.push_function_scope(g.data.id);
                            if let Some(ref body) = g.body {
                                for s in body.statements.iter() { self.bind_statement(s); }
                            }
                            self.pop_scope();
                        }
                        ObjectLiteralElement::SetAccessor(s_decl) => {
                            self.push_function_scope(s_decl.data.id);
                            for param in s_decl.parameters.iter() { self.bind_parameter(param); }
                            if let Some(ref body) = s_decl.body {
                                for s in body.statements.iter() { self.bind_statement(s); }
                            }
                            self.pop_scope();
                        }
                    }
                }
            }
            Expression::TemplateExpression(n) => {
                for span in n.template_spans.iter() {
                    self.bind_expression(span.expression);
                }
            }
            Expression::TaggedTemplate(n) => {
                self.bind_expression(n.tag);
                self.bind_expression(n.template);
            }
            Expression::Spread(n) => self.bind_expression(n.expression),
            Expression::Parenthesized(n) => self.bind_expression(n.expression),
            Expression::Await(n) => self.bind_expression(n.expression),
            Expression::Yield(n) => {
                if let Some(e) = n.expression { self.bind_expression(e); }
            }
            Expression::PrefixUnary(n) => self.bind_expression(n.operand),
            Expression::PostfixUnary(n) => self.bind_expression(n.operand),
            Expression::TypeOf(n) => self.bind_expression(n.expression),
            Expression::Delete(n) => self.bind_expression(n.expression),
            Expression::Void(n) => self.bind_expression(n.expression),
            Expression::As(n) => self.bind_expression(n.expression),
            Expression::Satisfies(n) => self.bind_expression(n.expression),
            Expression::NonNull(n) => self.bind_expression(n.expression),
            Expression::TypeAssertion(n) => self.bind_expression(n.expression),
            _ => {}
        }
    }

    // ========================================================================
    // Symbol resolution
    // ========================================================================

    /// Resolve a name in the current scope chain.
    pub fn resolve_symbol(&self, name: &InternedString) -> Option<SymbolId> {
        let mut scope = self.current_scope.as_ref();
        let mut depth = 0u32;
        while let Some(s) = scope {
            if let Some(id) = s.locals.get(name) {
                return Some(id);
            }
            depth += 1;
            if depth > Self::MAX_SCOPE_DEPTH {
                break;
            }
            scope = s.parent.as_ref();
        }
        // Check globals
        self.globals.get(name)
    }

    /// Resolve a name by its text string in the current scope chain.
    pub fn resolve_name(&self, name: &str) -> Option<SymbolId> {
        let mut scope = self.current_scope.as_ref();
        let mut depth = 0u32;
        while let Some(s) = scope {
            if let Some(&id) = s.names.get(name) {
                return Some(id);
            }
            depth += 1;
            if depth > Self::MAX_SCOPE_DEPTH {
                break;
            }
            scope = s.parent.as_ref();
        }
        None
    }

    /// Resolve a name starting from a specific symbol's members.
    pub fn resolve_member(&self, container: SymbolId, name: &InternedString) -> Option<SymbolId> {
        if let Some(symbol) = self.get_symbol(container) {
            if let Some(ref members) = symbol.members {
                return members.get(name);
            }
        }
        None
    }

    // ========================================================================
    // Symbol and scope management
    // ========================================================================

    /// Create a symbol from a PropertyName node (used for class/interface members).
    fn declare_property_name_symbol(&mut self, name: &PropertyName<'_>, flags: SymbolFlags, node_id: NodeId) {
        match name {
            PropertyName::Identifier(id) => {
                self.declare_symbol_with_text(id.text, id.text_name.clone(), flags, node_id);
            }
            PropertyName::PrivateIdentifier(id) => {
                self.declare_symbol_with_text(id.text, id.text_name.clone(), flags, node_id);
            }
            PropertyName::StringLiteral(_) | PropertyName::NumericLiteral(_) | PropertyName::ComputedPropertyName(_) => {
                // Computed/literal property names don't create named symbols in the scope
            }
        }
    }

    fn declare_symbol(&mut self, name: InternedString, flags: SymbolFlags, declaration: NodeId) -> SymbolId {
        self.declare_symbol_with_text(name, String::new(), flags, declaration)
    }

    fn declare_symbol_with_text(&mut self, name: InternedString, name_text: String, flags: SymbolFlags, declaration: NodeId) -> SymbolId {
        // Check for existing symbol in current scope for merging
        if !name_text.is_empty() {
            if let Some(scope) = &self.current_scope {
                if let Some(&existing_id) = scope.names.get(&name_text) {
                    let (can_merge, is_duplicate_block_scoped) = if let Some(existing) = self.symbols.get(existing_id.index()) {
                        let can_merge =
                            // Interface declaration merging
                            (existing.flags.contains(SymbolFlags::INTERFACE) && flags.contains(SymbolFlags::INTERFACE))
                            // Namespace declaration merging
                            || (existing.flags.contains(SymbolFlags::VALUE_MODULE) && flags.contains(SymbolFlags::VALUE_MODULE))
                            // Function overload merging
                            || (existing.flags.contains(SymbolFlags::FUNCTION) && flags.contains(SymbolFlags::FUNCTION))
                            // Enum declaration merging
                            || (existing.flags.intersects(SymbolFlags::ENUM) && flags.intersects(SymbolFlags::ENUM));

                        // Detect duplicate block-scoped declarations (TDZ errors)
                        let is_dup = existing.flags.contains(SymbolFlags::BLOCK_SCOPED_VARIABLE)
                            && flags.contains(SymbolFlags::BLOCK_SCOPED_VARIABLE);

                        (can_merge, is_dup)
                    } else {
                        (false, false)
                    };

                    if is_duplicate_block_scoped {
                        self.diagnostics.add(rscript_diagnostics::Diagnostic::new(
                            &rscript_diagnostics::messages::DUPLICATE_IDENTIFIER_0,
                            &[&name_text],
                        ));
                        // Return existing symbol anyway
                        return existing_id;
                    }

                    if can_merge {
                        if let Some(existing) = self.symbols.get_mut(existing_id.index()) {
                            existing.flags |= flags;
                            existing.declarations.push(declaration);
                        }
                        return existing_id;
                    }
                }
            }
        }

        let id = SymbolId(self.next_symbol_id);
        self.next_symbol_id += 1;

        let mut symbol = Symbol::with_name_text(id, name, name_text.clone(), flags);
        symbol.declarations.push(declaration);
        symbol.value_declaration = Some(declaration);
        self.symbols.push(symbol);

        if let Some(scope) = &mut self.current_scope {
            scope.locals.set(name, id);
            if !name_text.is_empty() {
                scope.names.insert(name_text, id);
            }
        }

        id
    }

    fn push_block_scope(&mut self) {
        let parent = self.current_scope.take();
        self.current_scope = Some(Box::new(Scope::new(parent)));
        self.scope_depth += 1;
    }

    fn push_function_scope(&mut self, container_node: NodeId) {
        let parent = self.current_scope.take();
        self.current_scope = Some(Box::new(Scope::new(parent)));
        self.scope_depth += 1;
        // Create flow node for function start
        self.create_flow_node(FlowNodeKind::Start, Some(container_node));
        let _ = container_node;
    }

    fn pop_scope(&mut self) {
        if let Some(scope) = self.current_scope.take() {
            self.current_scope = scope.parent;
            if self.scope_depth > 0 {
                self.scope_depth -= 1;
            }
        }
    }

    fn create_flow_node(&mut self, kind: FlowNodeKind, node: Option<NodeId>) -> u32 {
        let id = self.next_flow_id;
        self.next_flow_id += 1;
        self.flow_nodes.push(FlowNode {
            kind,
            id,
            antecedents: if self.current_flow > 0 || !self.flow_nodes.is_empty() {
                vec![self.current_flow]
            } else {
                vec![]
            },
            node,
        });
        self.current_flow = id;
        id
    }

    /// Get a mutable symbol by its ID.
    pub fn get_symbol_mut(&mut self, id: SymbolId) -> Option<&mut Symbol> {
        self.symbols.get_mut(id.index())
    }

    /// Get all symbols.
    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    /// Get number of symbols.
    pub fn symbol_count(&self) -> usize {
        self.symbols.len()
    }

    /// Get the current scope depth.
    pub fn scope_depth(&self) -> u32 {
        self.scope_depth
    }
}

impl Default for Binder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binder_creation() {
        let binder = Binder::new();
        assert_eq!(binder.symbol_count(), 0);
        assert_eq!(binder.scope_depth(), 0);
    }

    #[test]
    fn test_declare_symbol() {
        let mut binder = Binder::new();
        let name = InternedString::dummy();
        let id = binder.declare_symbol(name, SymbolFlags::FUNCTION, NodeId::INVALID);
        assert_eq!(binder.symbol_count(), 1);
        let sym = binder.get_symbol(id).unwrap();
        assert!(sym.flags.contains(SymbolFlags::FUNCTION));
    }

    #[test]
    fn test_scope_management() {
        let mut binder = Binder::new();
        assert_eq!(binder.scope_depth(), 0);
        binder.push_block_scope();
        assert_eq!(binder.scope_depth(), 1);
        binder.push_block_scope();
        assert_eq!(binder.scope_depth(), 2);
        binder.pop_scope();
        assert_eq!(binder.scope_depth(), 1);
        binder.pop_scope();
        assert_eq!(binder.scope_depth(), 0);
    }

    #[test]
    fn test_resolve_symbol() {
        let mut binder = Binder::new();
        let name = InternedString::dummy();
        binder.declare_symbol(name, SymbolFlags::FUNCTION, NodeId::INVALID);
        let resolved = binder.resolve_symbol(&name);
        assert!(resolved.is_some());
    }

    #[test]
    fn test_flow_nodes() {
        let binder = Binder::new();
        assert_eq!(binder.flow_nodes().len(), 1); // Start node
        assert_eq!(binder.flow_nodes()[0].kind, FlowNodeKind::Start);
    }
}
