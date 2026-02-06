//! rscript_printer: AST to text output.
//!
//! Converts AST nodes back into formatted TypeScript/JavaScript text.
//! The printer handles all AST node types and produces correctly formatted output.

use rscript_ast::node::*;
use rscript_ast::syntax_kind::SyntaxKind;
use rscript_ast::types::ModifierFlags;
use rscript_core::intern::StringInterner;

/// Options for the printer.
pub struct PrinterOptions {
    /// Whether to strip TypeScript type annotations (emit JS).
    pub strip_types: bool,
    /// Indentation string.
    pub indent_str: String,
    /// Newline string.
    pub new_line: String,
    /// Whether to emit a trailing newline.
    pub trailing_newline: bool,
}

impl Default for PrinterOptions {
    fn default() -> Self {
        Self {
            strip_types: false,
            indent_str: "    ".to_string(),
            new_line: "\n".to_string(),
            trailing_newline: true,
        }
    }
}

/// The printer converts AST nodes to text.
pub struct Printer<'i> {
    output: String,
    indent_level: u32,
    options: PrinterOptions,
    interner: &'i StringInterner,
}

impl<'i> Printer<'i> {
    pub fn new(interner: &'i StringInterner) -> Self {
        Self {
            output: String::with_capacity(4096),
            indent_level: 0,
            options: PrinterOptions::default(),
            interner,
        }
    }

    pub fn with_options(interner: &'i StringInterner, options: PrinterOptions) -> Self {
        Self {
            output: String::with_capacity(4096),
            indent_level: 0,
            options,
            interner,
        }
    }

    fn resolve(&self, s: rscript_core::intern::InternedString) -> &str {
        self.interner.resolve(s)
    }

    /// Print a source file to a string.
    pub fn print_source_file(&mut self, source_file: &SourceFile<'_>) -> String {
        self.output.clear();
        for (i, stmt) in source_file.statements.iter().enumerate() {
            if i > 0 { self.write_newline(); }
            self.write_indent();
            self.print_statement(stmt);
        }
        if self.options.trailing_newline && !self.output.is_empty() {
            self.write_newline();
        }
        self.output.clone()
    }

    // ========================================================================
    // Statement printing
    // ========================================================================

    fn print_statement(&mut self, stmt: &Statement<'_>) {
        match stmt {
            Statement::EmptyStatement(_) => self.write(";"),
            Statement::ExpressionStatement(n) => {
                self.print_expression(n.expression);
                self.write(";");
            }
            Statement::VariableStatement(n) => self.print_variable_statement(n),
            Statement::ReturnStatement(n) => {
                self.write("return");
                if let Some(expr) = n.expression {
                    self.write(" ");
                    self.print_expression(expr);
                }
                self.write(";");
            }
            Statement::IfStatement(n) => self.print_if_statement(n),
            Statement::Block(n) => self.print_block(n),
            Statement::FunctionDeclaration(n) => self.print_function_declaration(n),
            Statement::ClassDeclaration(n) => self.print_class_declaration(n),
            Statement::InterfaceDeclaration(n) => self.print_interface_declaration(n),
            Statement::TypeAliasDeclaration(n) => self.print_type_alias(n),
            Statement::EnumDeclaration(n) => self.print_enum_declaration(n),
            Statement::ForStatement(n) => self.print_for_statement(n),
            Statement::ForInStatement(n) => self.print_for_in_statement(n),
            Statement::ForOfStatement(n) => self.print_for_of_statement(n),
            Statement::WhileStatement(n) => {
                self.write("while (");
                self.print_expression(n.expression);
                self.write(") ");
                self.print_statement(n.statement);
            }
            Statement::DoStatement(n) => {
                self.write("do ");
                self.print_statement(n.statement);
                self.write(" while (");
                self.print_expression(n.expression);
                self.write(");");
            }
            Statement::SwitchStatement(n) => self.print_switch_statement(n),
            Statement::ThrowStatement(n) => {
                self.write("throw ");
                self.print_expression(n.expression);
                self.write(";");
            }
            Statement::TryStatement(n) => self.print_try_statement(n),
            Statement::BreakStatement(n) => {
                self.write("break");
                if let Some(ref label) = n.label {
                    self.write(" ");
                    self.print_identifier(label);
                }
                self.write(";");
            }
            Statement::ContinueStatement(n) => {
                self.write("continue");
                if let Some(ref label) = n.label {
                    self.write(" ");
                    self.print_identifier(label);
                }
                self.write(";");
            }
            Statement::LabeledStatement(n) => {
                self.print_identifier(&n.label);
                self.write(": ");
                self.print_statement(n.statement);
            }
            Statement::WithStatement(n) => {
                self.write("with (");
                self.print_expression(n.expression);
                self.write(") ");
                self.print_statement(n.statement);
            }
            Statement::DebuggerStatement(_) => self.write("debugger;"),
            Statement::ImportDeclaration(n) => self.print_import_declaration(n),
            Statement::ExportDeclaration(n) => self.print_export_declaration(n),
            Statement::ExportAssignment(n) => {
                if n.is_export_equals {
                    self.write("export = ");
                } else {
                    self.write("export default ");
                }
                self.print_expression(n.expression);
                self.write(";");
            }
            Statement::ModuleDeclaration(n) => self.print_module_declaration(n),
            Statement::ImportEqualsDeclaration(n) => {
                self.print_modifier_flags(n.data.modifier_flags);
                self.write("import ");
                if !self.options.strip_types && n.is_type_only { self.write("type "); }
                self.print_identifier(&n.name);
                self.write(" = ");
                match &n.module_reference {
                    ModuleReference::ExternalModuleReference(ext) => {
                        self.write("require(");
                        self.print_expression(ext.expression);
                        self.write(")");
                    }
                    ModuleReference::EntityName(name) => self.print_entity_name(name),
                }
                self.write(";");
            }
            Statement::NamespaceExportDeclaration(n) => {
                self.write("export as namespace ");
                self.print_identifier(&n.name);
                self.write(";");
            }
            Statement::MissingDeclaration(_) => {}
        }
    }

    fn print_variable_statement(&mut self, node: &VariableStatement<'_>) {
        self.print_modifier_flags(node.data.modifier_flags);
        let keyword = if node.declaration_list.data.flags.contains(rscript_ast::types::NodeFlags::CONST) {
            "const"
        } else if node.declaration_list.data.flags.contains(rscript_ast::types::NodeFlags::LET) {
            "let"
        } else {
            "var"
        };
        self.write(keyword);
        self.write(" ");
        for (i, decl) in node.declaration_list.declarations.iter().enumerate() {
            if i > 0 { self.write(", "); }
            self.print_binding_name(&decl.name);
            if !self.options.strip_types {
                if let Some(ty) = decl.type_annotation {
                    self.write(": ");
                    self.print_type_node(ty);
                }
            }
            if let Some(init) = decl.initializer {
                self.write(" = ");
                self.print_expression(init);
            }
        }
        self.write(";");
    }

    fn print_binding_name(&mut self, name: &BindingName<'_>) {
        match name {
            BindingName::Identifier(id) => self.print_identifier(id),
            BindingName::ObjectBindingPattern(p) => {
                self.write("{ ");
                for (i, elem) in p.elements.iter().enumerate() {
                    if i > 0 { self.write(", "); }
                    self.print_binding_element(elem);
                }
                self.write(" }");
            }
            BindingName::ArrayBindingPattern(p) => {
                self.write("[");
                for (i, elem) in p.elements.iter().enumerate() {
                    if i > 0 { self.write(", "); }
                    match elem {
                        ArrayBindingElement::BindingElement(e) => self.print_binding_element(e),
                        ArrayBindingElement::OmittedExpression(_) => {}
                    }
                }
                self.write("]");
            }
        }
    }

    fn print_binding_element(&mut self, elem: &BindingElement<'_>) {
        if elem.dot_dot_dot_token.is_some() { self.write("..."); }
        if let Some(ref prop_name) = elem.property_name {
            self.print_property_name(prop_name);
            self.write(": ");
        }
        self.print_binding_name(&elem.name);
        if let Some(init) = elem.initializer {
            self.write(" = ");
            self.print_expression(init);
        }
    }

    fn print_if_statement(&mut self, node: &IfStatement<'_>) {
        self.write("if (");
        self.print_expression(node.expression);
        self.write(") ");
        self.print_statement(node.then_statement);
        if let Some(else_stmt) = node.else_statement {
            self.write(" else ");
            self.print_statement(else_stmt);
        }
    }

    fn print_block(&mut self, node: &Block<'_>) {
        self.write("{");
        if !node.statements.is_empty() {
            self.increase_indent();
            for stmt in node.statements.iter() {
                self.write_newline();
                self.write_indent();
                self.print_statement(stmt);
            }
            self.decrease_indent();
            self.write_newline();
            self.write_indent();
        }
        self.write("}");
    }

    fn print_function_declaration(&mut self, node: &FunctionDeclaration<'_>) {
        let mf = node.data.modifier_flags;
        if !self.options.strip_types && mf.contains(ModifierFlags::AMBIENT) { self.write("declare "); }
        if mf.contains(ModifierFlags::EXPORT) { self.write("export "); }
        if mf.contains(ModifierFlags::DEFAULT) { self.write("default "); }
        if mf.contains(ModifierFlags::ASYNC) { self.write("async "); }
        self.write("function");
        if node.asterisk_token.is_some() { self.write("*"); }
        if let Some(ref name) = node.name {
            self.write(" ");
            self.print_identifier(name);
        }
        if !self.options.strip_types {
            self.print_optional_type_parameters(node.type_parameters);
        }
        self.write("(");
        self.print_parameters(node.parameters);
        self.write(")");
        if !self.options.strip_types {
            if let Some(ret) = node.return_type {
                self.write(": ");
                self.print_type_node(ret);
            }
        }
        if let Some(ref body) = node.body {
            self.write(" ");
            self.print_block(body);
        } else {
            self.write(";");
        }
    }

    fn print_class_declaration(&mut self, node: &ClassDeclaration<'_>) {
        let mf = node.data.modifier_flags;
        if !self.options.strip_types && mf.contains(ModifierFlags::AMBIENT) { self.write("declare "); }
        if mf.contains(ModifierFlags::EXPORT) { self.write("export "); }
        if mf.contains(ModifierFlags::DEFAULT) { self.write("default "); }
        if mf.contains(ModifierFlags::ABSTRACT) { self.write("abstract "); }
        self.write("class");
        if let Some(ref name) = node.name {
            self.write(" ");
            self.print_identifier(name);
        }
        if !self.options.strip_types {
            self.print_optional_type_parameters(node.type_parameters);
        }
        if let Some(heritage) = node.heritage_clauses {
            for clause in heritage.iter() {
                match clause.token {
                    SyntaxKind::ExtendsKeyword => self.write(" extends "),
                    SyntaxKind::ImplementsKeyword => {
                        if self.options.strip_types { continue; }
                        self.write(" implements ");
                    }
                    _ => self.write(" "),
                }
                for (i, ty) in clause.types.iter().enumerate() {
                    if i > 0 { self.write(", "); }
                    self.print_expression(ty.expression);
                    if !self.options.strip_types {
                        if let Some(type_args) = ty.type_arguments {
                            self.write("<");
                            for (j, arg) in type_args.iter().enumerate() {
                                if j > 0 { self.write(", "); }
                                self.print_type_node(arg);
                            }
                            self.write(">");
                        }
                    }
                }
            }
        }
        self.write(" {");
        if !node.members.is_empty() {
            self.increase_indent();
            for member in node.members.iter() {
                self.write_newline();
                self.write_indent();
                self.print_class_element(member);
            }
            self.decrease_indent();
            self.write_newline();
            self.write_indent();
        }
        self.write("}");
    }

    fn print_class_element(&mut self, elem: &ClassElement<'_>) {
        match elem {
            ClassElement::PropertyDeclaration(p) => {
                self.print_modifier_flags(p.data.modifier_flags);
                self.print_property_name(&p.name);
                if p.question_token.is_some() { self.write("?"); }
                if p.exclamation_token.is_some() { self.write("!"); }
                if !self.options.strip_types {
                    if let Some(ty) = p.type_annotation {
                        self.write(": ");
                        self.print_type_node(ty);
                    }
                }
                if let Some(init) = p.initializer {
                    self.write(" = ");
                    self.print_expression(init);
                }
                self.write(";");
            }
            ClassElement::MethodDeclaration(m) => {
                self.print_modifier_flags(m.data.modifier_flags);
                if m.asterisk_token.is_some() { self.write("*"); }
                self.print_property_name(&m.name);
                if m.question_token.is_some() { self.write("?"); }
                if !self.options.strip_types {
                    self.print_optional_type_parameters(m.type_parameters);
                }
                self.write("(");
                self.print_parameters(m.parameters);
                self.write(")");
                if !self.options.strip_types {
                    if let Some(ret) = m.return_type {
                        self.write(": ");
                        self.print_type_node(ret);
                    }
                }
                if let Some(ref body) = m.body {
                    self.write(" ");
                    self.print_block(body);
                } else {
                    self.write(";");
                }
            }
            ClassElement::Constructor(c) => {
                self.print_modifier_flags(c.data.modifier_flags);
                self.write("constructor(");
                self.print_parameters(c.parameters);
                self.write(")");
                if let Some(ref body) = c.body {
                    self.write(" ");
                    self.print_block(body);
                } else {
                    self.write(";");
                }
            }
            ClassElement::GetAccessor(g) => {
                self.print_modifier_flags(g.data.modifier_flags);
                self.write("get ");
                self.print_property_name(&g.name);
                self.write("()");
                if !self.options.strip_types {
                    if let Some(ret) = g.return_type {
                        self.write(": ");
                        self.print_type_node(ret);
                    }
                }
                if let Some(ref body) = g.body {
                    self.write(" ");
                    self.print_block(body);
                } else {
                    self.write(";");
                }
            }
            ClassElement::SetAccessor(s) => {
                self.print_modifier_flags(s.data.modifier_flags);
                self.write("set ");
                self.print_property_name(&s.name);
                self.write("(");
                self.print_parameters(s.parameters);
                self.write(")");
                if let Some(ref body) = s.body {
                    self.write(" ");
                    self.print_block(body);
                } else {
                    self.write(";");
                }
            }
            ClassElement::IndexSignature(idx) => {
                let mf = idx.data.modifier_flags;
                if mf.contains(ModifierFlags::READONLY) { self.write("readonly "); }
                self.write("[");
                self.print_parameters(idx.parameters);
                self.write("]");
                if let Some(ty) = idx.type_annotation {
                    self.write(": ");
                    self.print_type_node(ty);
                }
                self.write(";");
            }
            ClassElement::SemicolonClassElement(_) => self.write(";"),
            ClassElement::ClassStaticBlockDeclaration(sb) => {
                self.write("static ");
                self.print_block(&sb.body);
            }
        }
    }

    fn print_interface_declaration(&mut self, node: &InterfaceDeclaration<'_>) {
        if self.options.strip_types { return; }
        let mf = node.data.modifier_flags;
        if mf.contains(ModifierFlags::AMBIENT) { self.write("declare "); }
        if mf.contains(ModifierFlags::EXPORT) { self.write("export "); }
        self.write("interface ");
        self.print_identifier(&node.name);
        self.print_optional_type_parameters(node.type_parameters);
        if let Some(heritage) = node.heritage_clauses {
            for clause in heritage.iter() {
                self.write(" extends ");
                for (i, ty) in clause.types.iter().enumerate() {
                    if i > 0 { self.write(", "); }
                    self.print_expression(ty.expression);
                }
            }
        }
        self.write(" {");
        if !node.members.is_empty() {
            self.increase_indent();
            for member in node.members.iter() {
                self.write_newline();
                self.write_indent();
                self.print_type_element(member);
            }
            self.decrease_indent();
            self.write_newline();
            self.write_indent();
        }
        self.write("}");
    }

    fn print_type_element(&mut self, elem: &TypeElement<'_>) {
        match elem {
            TypeElement::PropertySignature(p) => {
                let mf = p.data.modifier_flags;
                if mf.contains(ModifierFlags::READONLY) { self.write("readonly "); }
                self.print_property_name(&p.name);
                if p.question_token.is_some() { self.write("?"); }
                if let Some(ty) = p.type_annotation {
                    self.write(": ");
                    self.print_type_node(ty);
                }
                self.write(";");
            }
            TypeElement::MethodSignature(m) => {
                self.print_property_name(&m.name);
                if m.question_token.is_some() { self.write("?"); }
                self.print_optional_type_parameters(m.type_parameters);
                self.write("(");
                self.print_parameters(m.parameters);
                self.write(")");
                if let Some(ret) = m.return_type {
                    self.write(": ");
                    self.print_type_node(ret);
                }
                self.write(";");
            }
            TypeElement::CallSignature(c) => {
                self.print_optional_type_parameters(c.type_parameters);
                self.write("(");
                self.print_parameters(c.parameters);
                self.write(")");
                if let Some(ret) = c.return_type {
                    self.write(": ");
                    self.print_type_node(ret);
                }
                self.write(";");
            }
            TypeElement::ConstructSignature(c) => {
                self.write("new ");
                self.print_optional_type_parameters(c.type_parameters);
                self.write("(");
                self.print_parameters(c.parameters);
                self.write(")");
                if let Some(ret) = c.return_type {
                    self.write(": ");
                    self.print_type_node(ret);
                }
                self.write(";");
            }
            TypeElement::IndexSignature(idx) => {
                let mf = idx.data.modifier_flags;
                if mf.contains(ModifierFlags::READONLY) { self.write("readonly "); }
                self.write("[");
                self.print_parameters(idx.parameters);
                self.write("]");
                if let Some(ty) = idx.type_annotation {
                    self.write(": ");
                    self.print_type_node(ty);
                }
                self.write(";");
            }
        }
    }

    fn print_type_alias(&mut self, node: &TypeAliasDeclaration<'_>) {
        if self.options.strip_types { return; }
        let mf = node.data.modifier_flags;
        if mf.contains(ModifierFlags::AMBIENT) { self.write("declare "); }
        if mf.contains(ModifierFlags::EXPORT) { self.write("export "); }
        self.write("type ");
        self.print_identifier(&node.name);
        self.print_optional_type_parameters(node.type_parameters);
        self.write(" = ");
        self.print_type_node(node.type_node);
        self.write(";");
    }

    fn print_enum_declaration(&mut self, node: &EnumDeclaration<'_>) {
        let mf = node.data.modifier_flags;
        if mf.contains(ModifierFlags::AMBIENT) { self.write("declare "); }
        if mf.contains(ModifierFlags::EXPORT) { self.write("export "); }
        if mf.contains(ModifierFlags::CONST) { self.write("const "); }
        self.write("enum ");
        self.print_identifier(&node.name);
        self.write(" {");
        if !node.members.is_empty() {
            self.increase_indent();
            for (i, member) in node.members.iter().enumerate() {
                self.write_newline();
                self.write_indent();
                self.print_property_name(&member.name);
                if let Some(init) = member.initializer {
                    self.write(" = ");
                    self.print_expression(init);
                }
                if i < node.members.len() - 1 { self.write(","); }
            }
            self.decrease_indent();
            self.write_newline();
            self.write_indent();
        }
        self.write("}");
    }

    fn print_for_statement(&mut self, node: &ForStatement<'_>) {
        self.write("for (");
        if let Some(ref init) = node.initializer {
            self.print_for_initializer(init);
        }
        self.write("; ");
        if let Some(cond) = node.condition { self.print_expression(cond); }
        self.write("; ");
        if let Some(incr) = node.incrementor { self.print_expression(incr); }
        self.write(") ");
        self.print_statement(node.statement);
    }

    fn print_for_in_statement(&mut self, node: &ForInStatement<'_>) {
        self.write("for (");
        self.print_for_initializer(&node.initializer);
        self.write(" in ");
        self.print_expression(node.expression);
        self.write(") ");
        self.print_statement(node.statement);
    }

    fn print_for_of_statement(&mut self, node: &ForOfStatement<'_>) {
        self.write("for ");
        if node.await_modifier.is_some() { self.write("await "); }
        self.write("(");
        self.print_for_initializer(&node.initializer);
        self.write(" of ");
        self.print_expression(node.expression);
        self.write(") ");
        self.print_statement(node.statement);
    }

    fn print_for_initializer(&mut self, init: &ForInitializer<'_>) {
        match init {
            ForInitializer::VariableDeclarationList(list) => self.print_variable_declaration_list(list),
            ForInitializer::Expression(expr) => self.print_expression(expr),
        }
    }

    fn print_variable_declaration_list(&mut self, list: &VariableDeclarationList<'_>) {
        let keyword = if list.data.flags.contains(rscript_ast::types::NodeFlags::CONST) {
            "const"
        } else if list.data.flags.contains(rscript_ast::types::NodeFlags::LET) {
            "let"
        } else {
            "var"
        };
        self.write(keyword);
        self.write(" ");
        for (i, decl) in list.declarations.iter().enumerate() {
            if i > 0 { self.write(", "); }
            self.print_binding_name(&decl.name);
            if !self.options.strip_types {
                if let Some(ty) = decl.type_annotation {
                    self.write(": ");
                    self.print_type_node(ty);
                }
            }
            if let Some(init) = decl.initializer {
                self.write(" = ");
                self.print_expression(init);
            }
        }
    }

    fn print_switch_statement(&mut self, node: &SwitchStatement<'_>) {
        self.write("switch (");
        self.print_expression(node.expression);
        self.write(") {");
        self.increase_indent();
        for clause in node.case_block.clauses.iter() {
            self.write_newline();
            self.write_indent();
            match clause {
                CaseOrDefaultClause::CaseClause(c) => {
                    self.write("case ");
                    self.print_expression(c.expression);
                    self.write(":");
                    self.increase_indent();
                    for s in c.statements.iter() {
                        self.write_newline();
                        self.write_indent();
                        self.print_statement(s);
                    }
                    self.decrease_indent();
                }
                CaseOrDefaultClause::DefaultClause(d) => {
                    self.write("default:");
                    self.increase_indent();
                    for s in d.statements.iter() {
                        self.write_newline();
                        self.write_indent();
                        self.print_statement(s);
                    }
                    self.decrease_indent();
                }
            }
        }
        self.decrease_indent();
        self.write_newline();
        self.write_indent();
        self.write("}");
    }

    fn print_try_statement(&mut self, node: &TryStatement<'_>) {
        self.write("try ");
        self.print_block(&node.try_block);
        if let Some(ref catch) = node.catch_clause {
            self.write(" catch");
            if let Some(ref var) = catch.variable_declaration {
                self.write(" (");
                self.print_binding_name(&var.name);
                self.write(")");
            }
            self.write(" ");
            self.print_block(&catch.block);
        }
        if let Some(ref finally) = node.finally_block {
            self.write(" finally ");
            self.print_block(finally);
        }
    }

    fn print_import_declaration(&mut self, node: &ImportDeclaration<'_>) {
        let is_type_only = node.import_clause.as_ref().is_some_and(|c| c.is_type_only);
        if self.options.strip_types && is_type_only { return; }
        self.write("import ");
        if !self.options.strip_types && is_type_only { self.write("type "); }
        if let Some(ref clause) = node.import_clause {
            if let Some(ref name) = clause.name {
                self.print_identifier(name);
                if clause.named_bindings.is_some() { self.write(", "); }
            }
            if let Some(ref bindings) = clause.named_bindings {
                match bindings {
                    NamedImportBindings::NamespaceImport(ns) => {
                        self.write("* as ");
                        self.print_identifier(&ns.name);
                    }
                    NamedImportBindings::NamedImports(named) => {
                        self.write("{ ");
                        for (i, spec) in named.elements.iter().enumerate() {
                            if i > 0 { self.write(", "); }
                            if !self.options.strip_types && spec.is_type_only { self.write("type "); }
                            if let Some(ref prop) = spec.property_name {
                                self.print_identifier(prop);
                                self.write(" as ");
                            }
                            self.print_identifier(&spec.name);
                        }
                        self.write(" }");
                    }
                }
            }
            self.write(" from ");
        }
        self.print_expression(node.module_specifier);
        self.write(";");
    }

    fn print_export_declaration(&mut self, node: &ExportDeclaration<'_>) {
        if self.options.strip_types && node.is_type_only { return; }
        self.write("export ");
        if !self.options.strip_types && node.is_type_only { self.write("type "); }
        if let Some(ref clause) = node.export_clause {
            match clause {
                NamedExportBindings::NamespaceExport(ns) => {
                    self.write("* as ");
                    self.print_identifier(&ns.name);
                }
                NamedExportBindings::NamedExports(named) => {
                    self.write("{ ");
                    for (i, spec) in named.elements.iter().enumerate() {
                        if i > 0 { self.write(", "); }
                        if !self.options.strip_types && spec.is_type_only { self.write("type "); }
                        self.print_identifier(&spec.name);
                        if let Some(ref prop) = spec.property_name {
                            self.write(" as ");
                            self.print_identifier(prop);
                        }
                    }
                    self.write(" }");
                }
            }
        } else {
            self.write("*");
        }
        if let Some(module_spec) = node.module_specifier {
            self.write(" from ");
            self.print_expression(module_spec);
        }
        self.write(";");
    }

    fn print_module_declaration(&mut self, node: &ModuleDeclaration<'_>) {
        let mf = node.data.modifier_flags;
        if mf.contains(ModifierFlags::AMBIENT) { self.write("declare "); }
        if mf.contains(ModifierFlags::EXPORT) { self.write("export "); }
        // Determine module/namespace/global from kind
        match &node.name {
            ModuleName::StringLiteral(_) => self.write("module "),
            ModuleName::Identifier(_) => {
                // Check the syntax kind for GlobalKeyword
                if node.data.kind == SyntaxKind::ModuleDeclaration {
                    self.write("namespace ");
                } else {
                    self.write("module ");
                }
            }
        }
        self.print_module_name(&node.name);
        if let Some(ref body) = node.body {
            self.write(" ");
            self.print_module_body(body);
        }
    }

    fn print_module_name(&mut self, name: &ModuleName) {
        match name {
            ModuleName::Identifier(id) => self.print_identifier(id),
            ModuleName::StringLiteral(s) => {
                self.write("\"");
                let text = self.resolve(s.text);
                self.write_owned(text.to_string());
                self.write("\"");
            }
        }
    }

    fn print_module_body(&mut self, body: &ModuleBody<'_>) {
        match body {
            ModuleBody::ModuleBlock(block) => {
                self.write("{");
                if !block.statements.is_empty() {
                    self.increase_indent();
                    for stmt in block.statements.iter() {
                        self.write_newline();
                        self.write_indent();
                        self.print_statement(stmt);
                    }
                    self.decrease_indent();
                    self.write_newline();
                    self.write_indent();
                }
                self.write("}");
            }
            ModuleBody::ModuleDeclaration(decl) => self.print_module_declaration(decl),
        }
    }

    // ========================================================================
    // Expression printing
    // ========================================================================

    fn print_expression(&mut self, expr: &Expression<'_>) {
        match expr {
            Expression::Identifier(id) => self.print_identifier(id),
            Expression::NumericLiteral(n) => {
                let text = self.resolve(n.text);
                self.write_owned(text.to_string());
            }
            Expression::StringLiteral(n) => {
                let quote = if n.is_single_quote { "'" } else { "\"" };
                self.write(quote);
                let text = self.resolve(n.text);
                self.write_owned(text.to_string());
                self.write(quote);
            }
            Expression::BigIntLiteral(n) => {
                let text = self.resolve(n.text);
                self.write_owned(text.to_string());
            }
            Expression::NoSubstitutionTemplateLiteral(n) => {
                self.write("`");
                let text = self.resolve(n.text);
                self.write_owned(text.to_string());
                self.write("`");
            }
            Expression::TemplateExpression(n) => {
                self.write("`");
                // Head is a Token - we need to extract its text from the source
                // For now, use an empty string placeholder
                self.write(""); // head text
                for span in n.template_spans.iter() {
                    self.write("${");
                    self.print_expression(span.expression);
                    self.write("}");
                    // literal text from span
                }
                self.write("`");
            }
            Expression::RegularExpressionLiteral(n) => {
                let text = self.resolve(n.text);
                self.write_owned(text.to_string());
            }
            Expression::TrueKeyword(_) => self.write("true"),
            Expression::FalseKeyword(_) => self.write("false"),
            Expression::NullKeyword(_) => self.write("null"),
            Expression::ThisKeyword(_) => self.write("this"),
            Expression::SuperKeyword(_) => self.write("super"),
            Expression::Binary(n) => {
                self.print_expression(n.left);
                self.write(" ");
                self.write(operator_to_string(n.operator_token.data.kind));
                self.write(" ");
                self.print_expression(n.right);
            }
            Expression::PrefixUnary(n) => {
                let op_str = operator_to_string(n.operator);
                self.write(op_str);
                if matches!(n.operator, SyntaxKind::TypeOfKeyword | SyntaxKind::VoidKeyword | SyntaxKind::DeleteKeyword) {
                    self.write(" ");
                }
                self.print_expression(n.operand);
            }
            Expression::PostfixUnary(n) => {
                self.print_expression(n.operand);
                self.write(operator_to_string(n.operator));
            }
            Expression::Conditional(n) => {
                self.print_expression(n.condition);
                self.write(" ? ");
                self.print_expression(n.when_true);
                self.write(" : ");
                self.print_expression(n.when_false);
            }
            Expression::Call(n) => {
                self.print_expression(n.expression);
                if !self.options.strip_types {
                    if let Some(type_args) = n.type_arguments {
                        self.write("<");
                        for (i, ty) in type_args.iter().enumerate() {
                            if i > 0 { self.write(", "); }
                            self.print_type_node(ty);
                        }
                        self.write(">");
                    }
                }
                self.write("(");
                for (i, arg) in n.arguments.iter().enumerate() {
                    if i > 0 { self.write(", "); }
                    self.print_expression(arg);
                }
                self.write(")");
            }
            Expression::New(n) => {
                self.write("new ");
                self.print_expression(n.expression);
                if let Some(args) = n.arguments {
                    self.write("(");
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 { self.write(", "); }
                        self.print_expression(arg);
                    }
                    self.write(")");
                }
            }
            Expression::PropertyAccess(n) => {
                self.print_expression(n.expression);
                if n.question_dot_token.is_some() { self.write("?."); }
                else { self.write("."); }
                self.print_member_name(&n.name);
            }
            Expression::ElementAccess(n) => {
                self.print_expression(n.expression);
                if n.question_dot_token.is_some() { self.write("?."); }
                self.write("[");
                self.print_expression(n.argument_expression);
                self.write("]");
            }
            Expression::Parenthesized(n) => {
                self.write("(");
                self.print_expression(n.expression);
                self.write(")");
            }
            Expression::ArrowFunction(n) => {
                let mf = n.data.modifier_flags;
                if mf.contains(ModifierFlags::ASYNC) { self.write("async "); }
                if !self.options.strip_types {
                    self.print_optional_type_parameters(n.type_parameters);
                }
                self.write("(");
                self.print_parameters(n.parameters);
                self.write(")");
                if !self.options.strip_types {
                    if let Some(ret) = n.return_type {
                        self.write(": ");
                        self.print_type_node(ret);
                    }
                }
                self.write(" => ");
                match &n.body {
                    ArrowFunctionBody::Block(block) => self.print_block(block),
                    ArrowFunctionBody::Expression(expr) => self.print_expression(expr),
                }
            }
            Expression::FunctionExpression(n) => {
                let mf = n.data.modifier_flags;
                if mf.contains(ModifierFlags::ASYNC) { self.write("async "); }
                self.write("function");
                if n.asterisk_token.is_some() { self.write("*"); }
                if let Some(ref name) = n.name {
                    self.write(" ");
                    self.print_identifier(name);
                }
                if !self.options.strip_types {
                    self.print_optional_type_parameters(n.type_parameters);
                }
                self.write("(");
                self.print_parameters(n.parameters);
                self.write(")");
                if !self.options.strip_types {
                    if let Some(ret) = n.return_type {
                        self.write(": ");
                        self.print_type_node(ret);
                    }
                }
                self.write(" ");
                self.print_block(n.body);
            }
            Expression::ArrayLiteral(n) => {
                self.write("[");
                for (i, elem) in n.elements.iter().enumerate() {
                    if i > 0 { self.write(", "); }
                    self.print_expression(elem);
                }
                self.write("]");
            }
            Expression::ObjectLiteral(n) => self.print_object_literal(n),
            Expression::Spread(n) => {
                self.write("...");
                self.print_expression(n.expression);
            }
            Expression::As(n) => {
                self.print_expression(n.expression);
                if !self.options.strip_types {
                    self.write(" as ");
                    self.print_type_node(n.type_node);
                }
            }
            Expression::Satisfies(n) => {
                self.print_expression(n.expression);
                if !self.options.strip_types {
                    self.write(" satisfies ");
                    self.print_type_node(n.type_node);
                }
            }
            Expression::TypeAssertion(n) => {
                if !self.options.strip_types {
                    self.write("<");
                    self.print_type_node(n.type_node);
                    self.write(">");
                }
                self.print_expression(n.expression);
            }
            Expression::NonNull(n) => {
                self.print_expression(n.expression);
                if !self.options.strip_types { self.write("!"); }
            }
            Expression::Await(n) => {
                self.write("await ");
                self.print_expression(n.expression);
            }
            Expression::Yield(n) => {
                self.write("yield");
                if n.asterisk_token.is_some() { self.write("*"); }
                if let Some(expr) = n.expression {
                    self.write(" ");
                    self.print_expression(expr);
                }
            }
            Expression::TypeOf(n) => {
                self.write("typeof ");
                self.print_expression(n.expression);
            }
            Expression::Delete(n) => {
                self.write("delete ");
                self.print_expression(n.expression);
            }
            Expression::Void(n) => {
                self.write("void ");
                self.print_expression(n.expression);
            }
            Expression::TaggedTemplate(n) => {
                self.print_expression(n.tag);
                self.print_expression(n.template);
            }
            Expression::ClassExpression(n) => {
                self.write("class");
                if let Some(ref name) = n.name {
                    self.write(" ");
                    self.print_identifier(name);
                }
                self.write(" {");
                if !n.members.is_empty() {
                    self.increase_indent();
                    for member in n.members.iter() {
                        self.write_newline();
                        self.write_indent();
                        self.print_class_element(member);
                    }
                    self.decrease_indent();
                    self.write_newline();
                    self.write_indent();
                }
                self.write("}");
            }
            Expression::MetaProperty(n) => {
                match n.keyword_token {
                    SyntaxKind::NewKeyword => self.write("new."),
                    SyntaxKind::ImportKeyword => self.write("import."),
                    _ => {}
                }
                self.print_identifier(&n.name);
            }
            Expression::OmittedExpression(_) => {}
        }
    }

    fn print_object_literal(&mut self, node: &ObjectLiteralExpression<'_>) {
        if node.properties.is_empty() {
            self.write("{}");
            return;
        }
        self.write("{");
        self.increase_indent();
        for (i, prop) in node.properties.iter().enumerate() {
            self.write_newline();
            self.write_indent();
            match prop {
                ObjectLiteralElement::PropertyAssignment(p) => {
                    self.print_property_name(&p.name);
                    self.write(": ");
                    self.print_expression(p.initializer);
                }
                ObjectLiteralElement::ShorthandPropertyAssignment(p) => {
                    self.print_identifier(&p.name);
                    if let Some(init) = p.object_assignment_initializer {
                        self.write(" = ");
                        self.print_expression(init);
                    }
                }
                ObjectLiteralElement::SpreadAssignment(p) => {
                    self.write("...");
                    self.print_expression(p.expression);
                }
                ObjectLiteralElement::MethodDeclaration(m) => {
                    if m.asterisk_token.is_some() { self.write("*"); }
                    self.print_property_name(&m.name);
                    self.write("(");
                    self.print_parameters(m.parameters);
                    self.write(")");
                    if let Some(ref body) = m.body {
                        self.write(" ");
                        self.print_block(body);
                    }
                }
                ObjectLiteralElement::GetAccessor(g) => {
                    self.write("get ");
                    self.print_property_name(&g.name);
                    self.write("()");
                    if let Some(ref body) = g.body {
                        self.write(" ");
                        self.print_block(body);
                    }
                }
                ObjectLiteralElement::SetAccessor(s) => {
                    self.write("set ");
                    self.print_property_name(&s.name);
                    self.write("(");
                    self.print_parameters(s.parameters);
                    self.write(")");
                    if let Some(ref body) = s.body {
                        self.write(" ");
                        self.print_block(body);
                    }
                }
            }
            if i < node.properties.len() - 1 { self.write(","); }
        }
        self.decrease_indent();
        self.write_newline();
        self.write_indent();
        self.write("}");
    }

    // ========================================================================
    // Type node printing
    // ========================================================================

    fn print_type_node(&mut self, ty: &TypeNode<'_>) {
        match ty {
            TypeNode::KeywordType(n) => self.write(keyword_to_string(n.data.kind)),
            TypeNode::TypeReference(n) => {
                self.print_entity_name(&n.type_name);
                if let Some(type_args) = n.type_arguments {
                    self.write("<");
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 { self.write(", "); }
                        self.print_type_node(arg);
                    }
                    self.write(">");
                }
            }
            TypeNode::ArrayType(n) => {
                self.print_type_node(n.element_type);
                self.write("[]");
            }
            TypeNode::TupleType(n) => {
                self.write("[");
                for (i, elem) in n.elements.iter().enumerate() {
                    if i > 0 { self.write(", "); }
                    self.print_type_node(elem);
                }
                self.write("]");
            }
            TypeNode::UnionType(n) => {
                for (i, t) in n.types.iter().enumerate() {
                    if i > 0 { self.write(" | "); }
                    self.print_type_node(t);
                }
            }
            TypeNode::IntersectionType(n) => {
                for (i, t) in n.types.iter().enumerate() {
                    if i > 0 { self.write(" & "); }
                    self.print_type_node(t);
                }
            }
            TypeNode::FunctionType(n) => {
                self.print_optional_type_parameters(n.type_parameters);
                self.write("(");
                self.print_parameters(n.parameters);
                self.write(") => ");
                if let Some(ret) = n.return_type {
                    self.print_type_node(ret);
                } else {
                    self.write("void");
                }
            }
            TypeNode::ConstructorType(n) => {
                self.write("new ");
                self.print_optional_type_parameters(n.type_parameters);
                self.write("(");
                self.print_parameters(n.parameters);
                self.write(") => ");
                if let Some(ret) = n.return_type {
                    self.print_type_node(ret);
                } else {
                    self.write("void");
                }
            }
            TypeNode::ConditionalType(n) => {
                self.print_type_node(n.check_type);
                self.write(" extends ");
                self.print_type_node(n.extends_type);
                self.write(" ? ");
                self.print_type_node(n.true_type);
                self.write(" : ");
                self.print_type_node(n.false_type);
            }
            TypeNode::IndexedAccessType(n) => {
                self.print_type_node(n.object_type);
                self.write("[");
                self.print_type_node(n.index_type);
                self.write("]");
            }
            TypeNode::MappedType(n) => {
                self.write("{ ");
                if let Some(ref readonly) = n.readonly_token {
                    match readonly.data.kind {
                        SyntaxKind::PlusToken => self.write("+readonly "),
                        SyntaxKind::MinusToken => self.write("-readonly "),
                        _ => self.write("readonly "),
                    }
                }
                self.write("[");
                self.print_identifier(&n.type_parameter.name);
                if let Some(constraint) = n.type_parameter.constraint {
                    self.write(" in ");
                    self.print_type_node(constraint);
                }
                if let Some(name_type) = n.name_type {
                    self.write(" as ");
                    self.print_type_node(name_type);
                }
                self.write("]");
                if let Some(ref question) = n.question_token {
                    match question.data.kind {
                        SyntaxKind::PlusToken => self.write("+?"),
                        SyntaxKind::MinusToken => self.write("-?"),
                        _ => self.write("?"),
                    }
                }
                if let Some(ty) = n.type_node {
                    self.write(": ");
                    self.print_type_node(ty);
                }
                self.write(" }");
            }
            TypeNode::TypeLiteral(n) => {
                self.write("{ ");
                for (i, member) in n.members.iter().enumerate() {
                    if i > 0 { self.write(" "); }
                    self.print_type_element(member);
                }
                if !n.members.is_empty() { self.write(" "); }
                self.write("}");
            }
            TypeNode::ParenthesizedType(n) => {
                self.write("(");
                self.print_type_node(n.type_node);
                self.write(")");
            }
            TypeNode::TypeOperator(n) => {
                match n.operator {
                    SyntaxKind::KeyOfKeyword => self.write("keyof "),
                    SyntaxKind::ReadonlyKeyword => self.write("readonly "),
                    SyntaxKind::UniqueKeyword => self.write("unique "),
                    _ => {}
                }
                self.print_type_node(n.type_node);
            }
            TypeNode::LiteralType(n) => self.print_expression(n.literal),
            TypeNode::TypeQuery(n) => {
                self.write("typeof ");
                self.print_entity_name(&n.expr_name);
            }
            TypeNode::ThisType(_) => self.write("this"),
            TypeNode::InferType(n) => {
                self.write("infer ");
                self.print_identifier(&n.type_parameter.name);
            }
            TypeNode::TemplateLiteralType(_n) => {
                // Simplified: template literal types need source text
                self.write("`...`");
            }
            TypeNode::TypePredicate(n) => {
                if n.asserts_modifier.is_some() { self.write("asserts "); }
                match &n.parameter_name {
                    TypePredicateParameterName::Identifier(id) => self.print_identifier(id),
                    TypePredicateParameterName::ThisType(_) => self.write("this"),
                }
                if let Some(ty) = n.type_node {
                    self.write(" is ");
                    self.print_type_node(ty);
                }
            }
            TypeNode::ImportType(n) => {
                self.write("import(");
                self.print_type_node(n.argument);
                self.write(")");
                if let Some(ref qualifier) = n.qualifier {
                    self.write(".");
                    self.print_entity_name(qualifier);
                }
            }
            TypeNode::OptionalType(n) => {
                self.print_type_node(n.type_node);
                self.write("?");
            }
            TypeNode::RestType(n) => {
                self.write("...");
                self.print_type_node(n.type_node);
            }
            TypeNode::NamedTupleMember(n) => {
                if n.dot_dot_dot_token.is_some() { self.write("..."); }
                self.print_identifier(&n.name);
                if n.question_token.is_some() { self.write("?"); }
                self.write(": ");
                self.print_type_node(n.type_node);
            }
            TypeNode::ExpressionWithTypeArguments(n) => {
                self.print_expression(n.expression);
                if let Some(type_args) = n.type_arguments {
                    self.write("<");
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 { self.write(", "); }
                        self.print_type_node(arg);
                    }
                    self.write(">");
                }
            }
        }
    }

    // ========================================================================
    // Helper printing functions
    // ========================================================================

    fn print_identifier(&mut self, id: &Identifier) {
        let text = self.resolve(id.text);
        self.write_owned(text.to_string());
    }

    fn print_entity_name(&mut self, name: &EntityName<'_>) {
        match name {
            EntityName::Identifier(id) => self.print_identifier(id),
            EntityName::QualifiedName(q) => {
                self.print_entity_name(&q.left);
                self.write(".");
                self.print_identifier(&q.right);
            }
        }
    }

    fn print_property_name(&mut self, name: &PropertyName<'_>) {
        match name {
            PropertyName::Identifier(id) => self.print_identifier(id),
            PropertyName::StringLiteral(tok) => {
                // Token-based string literal - we'd need source text
                self.write("\"\"");
                let _ = tok;
            }
            PropertyName::NumericLiteral(tok) => {
                self.write("0");
                let _ = tok;
            }
            PropertyName::ComputedPropertyName(c) => {
                self.write("[");
                self.print_expression(c.expression);
                self.write("]");
            }
            PropertyName::PrivateIdentifier(id) => {
                self.write("#");
                self.print_identifier(id);
            }
        }
    }

    fn print_member_name(&mut self, name: &MemberName) {
        match name {
            MemberName::Identifier(id) => self.print_identifier(id),
            MemberName::PrivateIdentifier(id) => {
                self.write("#");
                self.print_identifier(id);
            }
        }
    }

    fn print_parameters(&mut self, params: &[ParameterDeclaration<'_>]) {
        for (i, param) in params.iter().enumerate() {
            if i > 0 { self.write(", "); }
            self.print_modifier_flags(param.data.modifier_flags);
            if param.dot_dot_dot_token.is_some() { self.write("..."); }
            self.print_binding_name(&param.name);
            if param.question_token.is_some() { self.write("?"); }
            if !self.options.strip_types {
                if let Some(ty) = param.type_annotation {
                    self.write(": ");
                    self.print_type_node(ty);
                }
            }
            if let Some(init) = param.initializer {
                self.write(" = ");
                self.print_expression(init);
            }
        }
    }

    fn print_optional_type_parameters(&mut self, type_params: Option<&[TypeParameterDeclaration<'_>]>) {
        if let Some(params) = type_params {
            if !params.is_empty() {
                self.write("<");
                for (i, tp) in params.iter().enumerate() {
                    if i > 0 { self.write(", "); }
                    self.print_identifier(&tp.name);
                    if let Some(constraint) = tp.constraint {
                        self.write(" extends ");
                        self.print_type_node(constraint);
                    }
                    if let Some(default) = tp.default {
                        self.write(" = ");
                        self.print_type_node(default);
                    }
                }
                self.write(">");
            }
        }
    }

    fn print_modifier_flags(&mut self, flags: ModifierFlags) {
        if flags.is_empty() || flags == ModifierFlags::NONE { return; }
        if flags.contains(ModifierFlags::EXPORT) { self.write("export "); }
        if !self.options.strip_types && flags.contains(ModifierFlags::AMBIENT) { self.write("declare "); }
        if flags.contains(ModifierFlags::ABSTRACT) { self.write("abstract "); }
        if flags.contains(ModifierFlags::STATIC) { self.write("static "); }
        if !self.options.strip_types && flags.contains(ModifierFlags::READONLY) { self.write("readonly "); }
        if flags.contains(ModifierFlags::OVERRIDE) { self.write("override "); }
        if flags.contains(ModifierFlags::ACCESSOR) { self.write("accessor "); }
        if !self.options.strip_types && flags.contains(ModifierFlags::PUBLIC) { self.write("public "); }
        if !self.options.strip_types && flags.contains(ModifierFlags::PRIVATE) { self.write("private "); }
        if !self.options.strip_types && flags.contains(ModifierFlags::PROTECTED) { self.write("protected "); }
        if flags.contains(ModifierFlags::ASYNC) { self.write("async "); }
    }

    // ========================================================================
    // Core write helpers
    // ========================================================================

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn write_owned(&mut self, s: String) {
        self.output.push_str(&s);
    }

    fn write_newline(&mut self) {
        self.output.push_str(&self.options.new_line);
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent_level {
            self.output.push_str(&self.options.indent_str);
        }
    }

    fn increase_indent(&mut self) {
        self.indent_level += 1;
    }

    fn decrease_indent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }
}

// Free functions for operator/keyword to string conversion
fn operator_to_string(kind: SyntaxKind) -> &'static str {
    match kind {
        SyntaxKind::PlusToken => "+",
        SyntaxKind::MinusToken => "-",
        SyntaxKind::AsteriskToken => "*",
        SyntaxKind::SlashToken => "/",
        SyntaxKind::PercentToken => "%",
        SyntaxKind::AsteriskAsteriskToken => "**",
        SyntaxKind::AmpersandToken => "&",
        SyntaxKind::BarToken => "|",
        SyntaxKind::CaretToken => "^",
        SyntaxKind::TildeToken => "~",
        SyntaxKind::LessThanLessThanToken => "<<",
        SyntaxKind::GreaterThanGreaterThanToken => ">>",
        SyntaxKind::GreaterThanGreaterThanGreaterThanToken => ">>>",
        SyntaxKind::EqualsToken => "=",
        SyntaxKind::PlusEqualsToken => "+=",
        SyntaxKind::MinusEqualsToken => "-=",
        SyntaxKind::AsteriskEqualsToken => "*=",
        SyntaxKind::SlashEqualsToken => "/=",
        SyntaxKind::PercentEqualsToken => "%=",
        SyntaxKind::AsteriskAsteriskEqualsToken => "**=",
        SyntaxKind::AmpersandEqualsToken => "&=",
        SyntaxKind::BarEqualsToken => "|=",
        SyntaxKind::CaretEqualsToken => "^=",
        SyntaxKind::LessThanLessThanEqualsToken => "<<=",
        SyntaxKind::GreaterThanGreaterThanEqualsToken => ">>=",
        SyntaxKind::GreaterThanGreaterThanGreaterThanEqualsToken => ">>>=",
        SyntaxKind::AmpersandAmpersandToken => "&&",
        SyntaxKind::BarBarToken => "||",
        SyntaxKind::QuestionQuestionToken => "??",
        SyntaxKind::AmpersandAmpersandEqualsToken => "&&=",
        SyntaxKind::BarBarEqualsToken => "||=",
        SyntaxKind::QuestionQuestionEqualsToken => "??=",
        SyntaxKind::ExclamationToken => "!",
        SyntaxKind::PlusPlusToken => "++",
        SyntaxKind::MinusMinusToken => "--",
        SyntaxKind::EqualsEqualsToken => "==",
        SyntaxKind::ExclamationEqualsToken => "!=",
        SyntaxKind::EqualsEqualsEqualsToken => "===",
        SyntaxKind::ExclamationEqualsEqualsToken => "!==",
        SyntaxKind::LessThanToken => "<",
        SyntaxKind::GreaterThanToken => ">",
        SyntaxKind::LessThanEqualsToken => "<=",
        SyntaxKind::GreaterThanEqualsToken => ">=",
        SyntaxKind::InstanceOfKeyword => "instanceof",
        SyntaxKind::InKeyword => "in",
        SyntaxKind::CommaToken => ",",
        SyntaxKind::TypeOfKeyword => "typeof",
        SyntaxKind::VoidKeyword => "void",
        SyntaxKind::DeleteKeyword => "delete",
        _ => "?",
    }
}

fn keyword_to_string(kind: SyntaxKind) -> &'static str {
    match kind {
        SyntaxKind::StringKeyword => "string",
        SyntaxKind::NumberKeyword => "number",
        SyntaxKind::BooleanKeyword => "boolean",
        SyntaxKind::AnyKeyword => "any",
        SyntaxKind::VoidKeyword => "void",
        SyntaxKind::NeverKeyword => "never",
        SyntaxKind::UndefinedKeyword => "undefined",
        SyntaxKind::NullKeyword => "null",
        SyntaxKind::UnknownKeyword => "unknown",
        SyntaxKind::ObjectKeyword => "object",
        SyntaxKind::BigIntKeyword => "bigint",
        SyntaxKind::SymbolKeyword => "symbol",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operator_to_string() {
        assert_eq!(operator_to_string(SyntaxKind::PlusToken), "+");
        assert_eq!(operator_to_string(SyntaxKind::AsteriskAsteriskToken), "**");
        assert_eq!(operator_to_string(SyntaxKind::QuestionQuestionToken), "??");
    }

    #[test]
    fn test_keyword_to_string() {
        assert_eq!(keyword_to_string(SyntaxKind::StringKeyword), "string");
        assert_eq!(keyword_to_string(SyntaxKind::NumberKeyword), "number");
        assert_eq!(keyword_to_string(SyntaxKind::NeverKeyword), "never");
    }
}
