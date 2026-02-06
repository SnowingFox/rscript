//! The type checker implementation.
//!
//! This is a port of TypeScript's checker.ts - the core type-checking engine.
//! It implements type resolution, structural type checking, call resolution,
//! generic instantiation, assignment checking, and basic type narrowing.

use crate::types::{TypeTable, TypeKind, Signature, SignatureParameter, IndexInfo};
use rscript_ast::node::*;
use rscript_ast::syntax_kind::SyntaxKind;
use rscript_ast::types::*;
use rscript_binder::Binder;
use rscript_diagnostics::{DiagnosticCollection, Diagnostic, messages};
use rustc_hash::FxHashSet;
use std::collections::HashMap;

/// Maximum recursion depth for type stringification to prevent stack overflow.
const MAX_TYPE_TO_STRING_DEPTH: u32 = 20;

/// The type checker resolves types and reports type errors.
pub struct Checker {
    /// The type table (type arena).
    pub type_table: TypeTable,
    /// The binder with symbol information.
    binder: Binder,
    /// Accumulated diagnostics.
    diagnostics: DiagnosticCollection,
    /// Whether strict null checks are enabled.
    strict_null_checks: bool,
    /// Whether no implicit any is enabled.
    no_implicit_any: bool,
    /// Map of declared identifier names to their resolved types.
    /// Populated during checking as declarations are encountered.
    declared_types: HashMap<String, TypeId>,
    /// RegExp object type (lazily created).
    regexp_type: Option<TypeId>,
    /// Memoization cache for type assignability checks.
    /// Prevents infinite recursion on circular types and gives O(1) for
    /// repeated checks on the same (source, target) pair.
    assignability_cache: HashMap<(TypeId, TypeId), bool>,
}

impl Checker {
    pub fn new(binder: Binder) -> Self {
        Self {
            type_table: TypeTable::new(),
            binder,
            diagnostics: DiagnosticCollection::new(),
            strict_null_checks: true,
            no_implicit_any: false,
            declared_types: HashMap::new(),
            regexp_type: None,
            assignability_cache: HashMap::new(),
        }
    }

    pub fn with_options(binder: Binder, strict_null_checks: bool, no_implicit_any: bool) -> Self {
        Self {
            type_table: TypeTable::new(),
            binder,
            diagnostics: DiagnosticCollection::new(),
            strict_null_checks,
            no_implicit_any,
            declared_types: HashMap::new(),
            regexp_type: None,
            assignability_cache: HashMap::new(),
        }
    }

    /// Register a declared name with its resolved type.
    fn register_type(&mut self, name: &str, type_id: TypeId) {
        if !name.is_empty() {
            self.declared_types.insert(name.to_string(), type_id);
        }
    }

    /// Look up a declared name's type.
    fn get_declared_type(&self, name: &str) -> Option<TypeId> {
        self.declared_types.get(name).copied()
    }

    /// Get or create the RegExp type.
    fn get_regexp_type(&mut self) -> TypeId {
        if let Some(id) = self.regexp_type {
            return id;
        }
        let id = self.type_table.add_type(
            TypeFlags::OBJECT,
            TypeKind::ObjectType {
                object_flags: ObjectFlags::empty(),
                members: vec![
                    ("source".to_string(), self.type_table.string_type),
                    ("flags".to_string(), self.type_table.string_type),
                    ("global".to_string(), self.type_table.boolean_type),
                    ("ignoreCase".to_string(), self.type_table.boolean_type),
                    ("multiline".to_string(), self.type_table.boolean_type),
                    ("lastIndex".to_string(), self.type_table.number_type),
                ],
                call_signatures: vec![],
                construct_signatures: vec![],
                index_infos: vec![],
            },
        );
        self.regexp_type = Some(id);
        id
    }

    /// Check a source file for type errors.
    pub fn check_source_file(&mut self, source_file: &SourceFile<'_>) {
        for statement in source_file.statements.iter() {
            self.check_statement(statement);
        }
    }

    pub fn diagnostics(&self) -> &DiagnosticCollection { &self.diagnostics }
    pub fn take_diagnostics(&mut self) -> DiagnosticCollection { std::mem::take(&mut self.diagnostics) }

    fn error(&mut self, msg: &rscript_diagnostics::DiagnosticMessage, args: &[&str]) {
        self.diagnostics.add(Diagnostic::new(msg, args));
    }

    // ========================================================================
    // Statement checking
    // ========================================================================

    fn check_statement(&mut self, stmt: &Statement<'_>) {
        match stmt {
            Statement::VariableStatement(n) => self.check_variable_statement(n),
            Statement::ExpressionStatement(n) => { self.check_expression(n.expression); }
            Statement::ReturnStatement(n) => {
                if let Some(expr) = n.expression {
                    self.check_expression(expr);
                }
            }
            Statement::IfStatement(n) => self.check_if_statement(n),
            Statement::Block(n) => {
                for s in n.statements.iter() { self.check_statement(s); }
            }
            Statement::FunctionDeclaration(n) => self.check_function_declaration(n),
            Statement::ClassDeclaration(n) => self.check_class_declaration(n),
            Statement::ForStatement(n) => self.check_for_statement(n),
            Statement::ForInStatement(n) => {
                self.check_expression(n.expression);
                self.check_statement(n.statement);
            }
            Statement::ForOfStatement(n) => {
                self.check_expression(n.expression);
                self.check_statement(n.statement);
            }
            Statement::WhileStatement(n) => {
                self.check_expression(n.expression);
                self.check_statement(n.statement);
            }
            Statement::DoStatement(n) => {
                self.check_statement(n.statement);
                self.check_expression(n.expression);
            }
            Statement::SwitchStatement(n) => self.check_switch_statement(n),
            Statement::ThrowStatement(n) => { self.check_expression(n.expression); }
            Statement::TryStatement(n) => self.check_try_statement(n),
            Statement::EnumDeclaration(n) => self.check_enum_declaration(n),
            Statement::TypeAliasDeclaration(n) => self.check_type_alias_declaration(n),
            Statement::InterfaceDeclaration(n) => self.check_interface_declaration(n),
            _ => {}
        }
    }

    fn check_variable_statement(&mut self, node: &VariableStatement<'_>) {
        let is_const = node.declaration_list.data.flags.contains(NodeFlags::CONST);
        for decl in node.declaration_list.declarations.iter() {
            self.check_variable_declaration_with_const(decl, is_const);
        }
    }

    fn check_variable_declaration(&mut self, decl: &VariableDeclaration<'_>) {
        self.check_variable_declaration_with_const(decl, false);
    }

    fn check_variable_declaration_with_const(&mut self, decl: &VariableDeclaration<'_>, is_const: bool) {
        let declared_type = if decl.type_annotation.is_some() {
            self.get_type_from_type_annotation(decl.type_annotation)
        } else {
            None
        };

        // Extract the variable name for type registration
        let var_name = match &decl.name {
            BindingName::Identifier(id) => Some(id.text_name.clone()),
            _ => None,
        };

        if let Some(init) = decl.initializer {
            let init_type = self.check_expression(init);

            if let Some(declared) = declared_type {
                // Check that initializer is assignable to declared type
                if !self.is_type_assignable_to(init_type, declared) {
                    let source_name = self.type_to_string(init_type);
                    let target_name = self.type_to_string(declared);
                    self.error(
                        &messages::TYPE_0_IS_NOT_ASSIGNABLE_TO_TYPE_1,
                        &[&source_name, &target_name],
                    );
                }
                // Register the declared type
                if let Some(ref name) = var_name {
                    self.register_type(name, declared);
                }
            } else {
                // Infer type from initializer
                // For `const` declarations without type annotation, narrow to literal types
                let inferred = if is_const && declared_type.is_none() {
                    self.narrow_to_literal(init, init_type)
                } else {
                    init_type
                };
                if let Some(ref name) = var_name {
                    self.register_type(name, inferred);
                }
            }
        } else if let Some(declared) = declared_type {
            // No initializer, but has type annotation
            if let Some(ref name) = var_name {
                self.register_type(name, declared);
            }
        } else if self.no_implicit_any {
            // Variable without type annotation or initializer implicitly has 'any' type
            if let Some(ref name) = var_name {
                self.error(
                    &messages::VARIABLE_0_IMPLICITLY_HAS_AN_0_TYPE,
                    &[name, "any"],
                );
                self.register_type(name, self.type_table.any_type);
            }
        } else {
            // Register as any
            if let Some(ref name) = var_name {
                self.register_type(name, self.type_table.any_type);
            }
        }
    }

    fn check_function_declaration(&mut self, node: &FunctionDeclaration<'_>) {
        // Build parameter types for the function signature
        let params: Vec<SignatureParameter> = node.parameters.iter().map(|p| {
            if let Some(init) = p.initializer {
                self.check_expression(init);
            }
            let param_type = self.get_type_from_type_annotation(p.type_annotation);
            let param_name = match &p.name {
                BindingName::Identifier(id) => id.text_name.clone(),
                _ => String::new(),
            };
            // Register parameter in declared_types for body checking
            let resolved_type = param_type.unwrap_or(self.type_table.any_type);
            if !param_name.is_empty() {
                self.register_type(&param_name, resolved_type);
            }
            SignatureParameter {
                name: param_name,
                type_id: resolved_type,
                optional: p.question_token.is_some(),
            }
        }).collect();

        let return_type = self.get_type_from_type_annotation(node.return_type)
            .unwrap_or(self.type_table.void_type);

        // Register the function type
        if let Some(ref name) = node.name {
            let sig = Signature {
                type_parameters: vec![],
                parameters: params,
                return_type,
                min_argument_count: node.parameters.iter()
                    .filter(|p| p.question_token.is_none() && p.initializer.is_none() && p.dot_dot_dot_token.is_none())
                    .count() as u32,
                has_rest_parameter: node.parameters.iter().any(|p| p.dot_dot_dot_token.is_some()),
            };
            let func_type = self.type_table.add_type(
                TypeFlags::OBJECT,
                TypeKind::ObjectType {
                    object_flags: ObjectFlags::ANONYMOUS,
                    members: vec![],
                    call_signatures: vec![sig],
                    construct_signatures: vec![],
                    index_infos: vec![],
                },
            );
            self.register_type(&name.text_name, func_type);
        }

        // Check body
        if let Some(ref body) = node.body {
            for stmt in body.statements.iter() {
                self.check_statement(stmt);
            }
            // Check return type compatibility
            let declared_return = self.get_type_from_type_annotation(node.return_type);
            if let Some(declared_return) = declared_return {
                let ret_type = self.type_table.get(declared_return);
                if !ret_type.flags.contains(TypeFlags::VOID)
                    && !ret_type.flags.contains(TypeFlags::UNDEFINED)
                    && !ret_type.flags.contains(TypeFlags::ANY)
                {
                    let has_return = self.body_has_return(body);
                    if !has_return {
                        self.error(
                            &messages::A_FUNCTION_WHOSE_DECLARED_TYPE_IS_NEITHER_UNDEFINED_NOR_VOID_MUST_RETURN_A_VALUE,
                            &[],
                        );
                    }
                }
            }
        }
    }

    fn body_has_return(&self, body: &Block<'_>) -> bool {
        for stmt in body.statements.iter() {
            match stmt {
                Statement::ReturnStatement(r) => {
                    if r.expression.is_some() { return true; }
                }
                Statement::IfStatement(n) => {
                    if let Statement::Block(b) = n.then_statement {
                        if self.body_has_return(b) { return true; }
                    }
                    if let Some(Statement::Block(b)) = n.else_statement {
                        if self.body_has_return(b) { return true; }
                    }
                }
                Statement::Block(b) => {
                    if self.body_has_return(b) { return true; }
                }
                _ => {}
            }
        }
        false
    }

    fn check_class_declaration(&mut self, node: &ClassDeclaration<'_>) {
        // Check heritage
        if let Some(heritage) = node.heritage_clauses {
            for clause in heritage.iter() {
                for ty in clause.types.iter() {
                    self.check_expression(ty.expression);
                }
            }
        }

        // Collect class members and build a class type
        let mut class_members: Vec<(String, TypeId)> = Vec::new();
        let mut construct_sigs: Vec<Signature> = Vec::new();

        for member in node.members.iter() {
            match member {
                ClassElement::PropertyDeclaration(p) => {
                    let prop_name = self.property_name_text(&p.name);
                    let prop_type = if let Some(init) = p.initializer {
                        let init_type = self.check_expression(init);
                        if let Some(declared) = self.get_type_from_type_annotation(p.type_annotation) {
                            if !self.is_type_assignable_to(init_type, declared) {
                                let src = self.type_to_string(init_type);
                                let tgt = self.type_to_string(declared);
                                self.error(&messages::TYPE_0_IS_NOT_ASSIGNABLE_TO_TYPE_1, &[&src, &tgt]);
                            }
                            declared
                        } else {
                            init_type
                        }
                    } else {
                        self.get_type_from_type_annotation(p.type_annotation)
                            .unwrap_or(self.type_table.any_type)
                    };
                    class_members.push((prop_name, prop_type));
                }
                ClassElement::MethodDeclaration(m) => {
                    let method_name = self.property_name_text(&m.name);
                    let method_params: Vec<SignatureParameter> = m.parameters.iter().map(|p| {
                        if let Some(init) = p.initializer { self.check_expression(init); }
                        let ptype = self.get_type_from_type_annotation(p.type_annotation)
                            .unwrap_or(self.type_table.any_type);
                        let pname = match &p.name {
                            BindingName::Identifier(id) => id.text_name.clone(),
                            _ => String::new(),
                        };
                        self.register_type(&pname, ptype);
                        SignatureParameter { name: pname, type_id: ptype, optional: p.question_token.is_some() }
                    }).collect();
                    if let Some(ref body) = m.body {
                        for s in body.statements.iter() { self.check_statement(s); }
                    }
                    let return_type = self.get_type_from_type_annotation(m.return_type)
                        .unwrap_or(self.type_table.any_type);
                    let sig = Signature {
                        type_parameters: vec![],
                        parameters: method_params,
                        return_type,
                        min_argument_count: m.parameters.iter()
                            .filter(|p| p.question_token.is_none() && p.initializer.is_none())
                            .count() as u32,
                        has_rest_parameter: m.parameters.iter().any(|p| p.dot_dot_dot_token.is_some()),
                    };
                    let method_type = self.type_table.add_type(
                        TypeFlags::OBJECT,
                        TypeKind::ObjectType {
                            object_flags: ObjectFlags::ANONYMOUS,
                            members: vec![],
                            call_signatures: vec![sig],
                            construct_signatures: vec![],
                            index_infos: vec![],
                        },
                    );
                    class_members.push((method_name, method_type));
                }
                ClassElement::Constructor(c) => {
                    let ctor_params: Vec<SignatureParameter> = c.parameters.iter().map(|p| {
                        if let Some(init) = p.initializer { self.check_expression(init); }
                        let ptype = self.get_type_from_type_annotation(p.type_annotation)
                            .unwrap_or(self.type_table.any_type);
                        let pname = match &p.name {
                            BindingName::Identifier(id) => id.text_name.clone(),
                            _ => String::new(),
                        };
                        self.register_type(&pname, ptype);
                        SignatureParameter { name: pname, type_id: ptype, optional: p.question_token.is_some() }
                    }).collect();
                    if let Some(ref body) = c.body {
                        for s in body.statements.iter() { self.check_statement(s); }
                    }
                    construct_sigs.push(Signature {
                        type_parameters: vec![],
                        parameters: ctor_params,
                        return_type: self.type_table.any_type, // instance type
                        min_argument_count: c.parameters.iter()
                            .filter(|p| p.question_token.is_none() && p.initializer.is_none())
                            .count() as u32,
                        has_rest_parameter: c.parameters.iter().any(|p| p.dot_dot_dot_token.is_some()),
                    });
                }
                ClassElement::GetAccessor(g) => {
                    let prop_name = self.property_name_text(&g.name);
                    if let Some(ref body) = g.body {
                        for s in body.statements.iter() { self.check_statement(s); }
                    }
                    let ret = self.get_type_from_type_annotation(g.return_type)
                        .unwrap_or(self.type_table.any_type);
                    class_members.push((prop_name, ret));
                }
                ClassElement::SetAccessor(s) => {
                    let prop_name = self.property_name_text(&s.name);
                    if let Some(ref body) = s.body {
                        for stmt in body.statements.iter() { self.check_statement(stmt); }
                    }
                    class_members.push((prop_name, self.type_table.any_type));
                }
                _ => {}
            }
        }

        // Build the instance type first, then set construct signature return types
        if let Some(ref name) = node.name {
            let instance_type = self.type_table.add_type(
                TypeFlags::OBJECT,
                TypeKind::ObjectType {
                    object_flags: ObjectFlags::INTERFACE,
                    members: class_members.clone(),
                    call_signatures: vec![],
                    construct_signatures: vec![],
                    index_infos: vec![],
                },
            );

            // Update construct signatures to return the instance type
            for sig in &mut construct_sigs {
                sig.return_type = instance_type;
            }

            // If no explicit constructor, add a default one
            if construct_sigs.is_empty() {
                construct_sigs.push(Signature {
                    type_parameters: vec![],
                    parameters: vec![],
                    return_type: instance_type,
                    min_argument_count: 0,
                    has_rest_parameter: false,
                });
            }

            let class_type = self.type_table.add_type(
                TypeFlags::OBJECT,
                TypeKind::ObjectType {
                    object_flags: ObjectFlags::empty(),
                    members: class_members,
                    call_signatures: vec![],
                    construct_signatures: construct_sigs,
                    index_infos: vec![],
                },
            );
            self.register_type(&name.text_name, class_type);
        }
    }

    fn check_if_statement(&mut self, node: &IfStatement<'_>) {
        self.check_expression(node.expression);
        self.check_statement(node.then_statement);
        if let Some(else_stmt) = node.else_statement {
            self.check_statement(else_stmt);
        }
    }

    fn check_for_statement(&mut self, node: &ForStatement<'_>) {
        if let Some(ref init) = node.initializer {
            match init {
                ForInitializer::VariableDeclarationList(list) => {
                    for decl in list.declarations.iter() {
                        self.check_variable_declaration(decl);
                    }
                }
                ForInitializer::Expression(expr) => { self.check_expression(expr); }
            }
        }
        if let Some(cond) = node.condition { self.check_expression(cond); }
        if let Some(incr) = node.incrementor { self.check_expression(incr); }
        self.check_statement(node.statement);
    }

    fn check_switch_statement(&mut self, node: &SwitchStatement<'_>) {
        let switch_type = self.check_expression(node.expression);
        for clause in node.case_block.clauses.iter() {
            match clause {
                CaseOrDefaultClause::CaseClause(c) => {
                    let case_type = self.check_expression(c.expression);
                    // Check comparability
                    let _ = (switch_type, case_type);
                    for s in c.statements.iter() { self.check_statement(s); }
                }
                CaseOrDefaultClause::DefaultClause(d) => {
                    for s in d.statements.iter() { self.check_statement(s); }
                }
            }
        }
    }

    fn check_try_statement(&mut self, node: &TryStatement<'_>) {
        for s in node.try_block.statements.iter() { self.check_statement(s); }
        if let Some(ref catch) = node.catch_clause {
            for s in catch.block.statements.iter() { self.check_statement(s); }
        }
        if let Some(ref finally) = node.finally_block {
            for s in finally.statements.iter() { self.check_statement(s); }
        }
    }

    fn check_enum_declaration(&mut self, node: &EnumDeclaration<'_>) {
        for (_auto_index, member) in node.members.iter().enumerate() {
            if let Some(init) = member.initializer {
                let init_type = self.check_expression(init);
                let ty = self.type_table.get(init_type);
                if !ty.flags.intersects(TypeFlags::NUMBER_LIKE) && !ty.flags.contains(TypeFlags::STRING_LIKE) {
                    // Enum initializer must be number or string
                }
            }
        }
    }

    /// Resolve a type alias declaration:  `type Name = UnderlyingType;`
    ///
    /// This registers the alias name so that later `TypeReference` lookups
    /// (e.g. `let x: Name`) resolve to the underlying type.
    fn check_type_alias_declaration(&mut self, node: &TypeAliasDeclaration<'_>) {
        let name = node.name.text_name.clone();
        let resolved = self.get_type_from_type_node(node.type_node);
        self.register_type(&name, resolved);
    }

    /// Resolve an interface declaration and register it as a proper ObjectType
    /// so that subsequent `TypeReference` lookups produce the right shape.
    fn check_interface_declaration(&mut self, node: &InterfaceDeclaration<'_>) {
        let name = node.name.text_name.clone();

        // If this interface was already registered (declaration merging),
        // we merge members into the existing object type.
        let existing = self.get_declared_type(&name);

        let mut members: Vec<(String, TypeId)> = Vec::new();
        let mut call_signatures: Vec<Signature> = Vec::new();
        let mut construct_signatures: Vec<Signature> = Vec::new();
        let mut index_infos: Vec<IndexInfo> = Vec::new();

        // Pull existing members when merging declarations
        if let Some(existing_id) = existing {
            if let TypeKind::ObjectType {
                members: ref existing_members,
                call_signatures: ref existing_calls,
                construct_signatures: ref existing_constructs,
                index_infos: ref existing_indexes,
                ..
            } = self.type_table.get(existing_id).kind
            {
                members = existing_members.clone();
                call_signatures = existing_calls.clone();
                construct_signatures = existing_constructs.clone();
                index_infos = existing_indexes.clone();
            }
        }

        // Process each member of the interface
        for member in node.members.iter() {
            match member {
                TypeElement::PropertySignature(prop) => {
                    let prop_name = self.get_property_name_text(&prop.name);
                    let prop_type = self
                        .get_type_from_type_annotation(prop.type_annotation)
                        .unwrap_or(self.type_table.any_type);

                    // If the property is optional (has `?`), wrap in union with undefined
                    let final_type = if prop.question_token.is_some() {
                        self.create_union_type(vec![prop_type, self.type_table.undefined_type])
                    } else {
                        prop_type
                    };

                    // Check for duplicate — overwrite if the name already exists (merge semantics)
                    if let Some(entry) = members.iter_mut().find(|(n, _)| n == &prop_name) {
                        entry.1 = final_type;
                    } else {
                        members.push((prop_name, final_type));
                    }
                }
                TypeElement::MethodSignature(method) => {
                    let method_name = self.get_property_name_text(&method.name);
                    let method_type = self.build_method_type(
                        method.parameters,
                        method.return_type,
                        method.question_token.is_some(),
                    );

                    if let Some(entry) = members.iter_mut().find(|(n, _)| n == &method_name) {
                        entry.1 = method_type;
                    } else {
                        members.push((method_name, method_type));
                    }
                }
                TypeElement::CallSignature(call) => {
                    let sig = self.build_signature(call.parameters, call.return_type);
                    call_signatures.push(sig);
                }
                TypeElement::ConstructSignature(ctor) => {
                    let sig = self.build_signature(ctor.parameters, ctor.return_type);
                    construct_signatures.push(sig);
                }
                TypeElement::IndexSignature(idx) => {
                    // Index signature: [key: KeyType]: ValueType
                    let key_type = if let Some(first_param) = idx.parameters.iter().next() {
                        self.get_type_from_type_annotation(first_param.type_annotation)
                            .unwrap_or(self.type_table.string_type)
                    } else {
                        self.type_table.string_type
                    };
                    let value_type = self
                        .get_type_from_type_annotation(idx.type_annotation)
                        .unwrap_or(self.type_table.any_type);
                    index_infos.push(IndexInfo {
                        key_type,
                        type_id: value_type,
                        is_readonly: false,
                    });
                }
            }
        }

        // Resolve heritage clauses (extends)
        if let Some(heritage) = node.heritage_clauses {
            for clause in heritage.iter() {
                for expr_with_args in clause.types.iter() {
                    // Resolve the base type from the expression
                    let base_type_id = self.check_expression(expr_with_args.expression);
                    // Merge members from the base type into this interface
                    let base_members: Vec<(String, TypeId)> =
                        if let TypeKind::ObjectType { members: ref bm, .. } =
                            self.type_table.get(base_type_id).kind
                        {
                            bm.clone()
                        } else {
                            vec![]
                        };
                    for (base_name, base_tid) in base_members {
                        // Only add if not already overridden
                        if !members.iter().any(|(n, _)| n == &base_name) {
                            members.push((base_name, base_tid));
                        }
                    }
                }
            }
        }

        let interface_type = self.type_table.add_type(
            TypeFlags::OBJECT,
            TypeKind::ObjectType {
                object_flags: ObjectFlags::INTERFACE,
                members,
                call_signatures,
                construct_signatures,
                index_infos,
            },
        );
        self.register_type(&name, interface_type);
    }

    /// Extract a textual name from a `PropertyName` node.
    fn get_property_name_text(&self, name: &PropertyName<'_>) -> String {
        match name {
            PropertyName::Identifier(id) => id.text_name.clone(),
            PropertyName::StringLiteral(_tok) => {
                // Token doesn't carry source text; fall back to empty for now.
                // A full implementation would resolve from the source map.
                String::new()
            }
            PropertyName::NumericLiteral(_tok) => String::new(),
            PropertyName::PrivateIdentifier(id) => id.text_name.clone(),
            PropertyName::ComputedPropertyName(_) => "[computed]".to_string(),
        }
    }

    /// Build a `Signature` from parameter + return-type AST nodes.
    fn build_signature(
        &mut self,
        parameters: &[ParameterDeclaration<'_>],
        return_type: Option<&TypeNode<'_>>,
    ) -> Signature {
        let mut min_args: u32 = 0;
        let mut has_rest = false;
        let params: Vec<SignatureParameter> = parameters
            .iter()
            .map(|p| {
                let param_name = match &p.name {
                    BindingName::Identifier(id) => id.text_name.clone(),
                    _ => String::new(),
                };
                let param_type = self
                    .get_type_from_type_annotation(p.type_annotation)
                    .unwrap_or(self.type_table.any_type);
                let optional = p.question_token.is_some() || p.initializer.is_some();
                if p.dot_dot_dot_token.is_some() {
                    has_rest = true;
                }
                if !optional && p.dot_dot_dot_token.is_none() {
                    min_args += 1;
                }
                SignatureParameter {
                    name: param_name,
                    type_id: param_type,
                    optional,
                }
            })
            .collect();

        let ret = return_type
            .map(|rt| self.get_type_from_type_node(rt))
            .unwrap_or(self.type_table.void_type);

        Signature {
            type_parameters: vec![],
            parameters: params,
            return_type: ret,
            min_argument_count: min_args,
            has_rest_parameter: has_rest,
        }
    }

    /// Build a function-typed TypeId for a method signature.
    fn build_method_type(
        &mut self,
        parameters: &[ParameterDeclaration<'_>],
        return_type: Option<&TypeNode<'_>>,
        _optional: bool,
    ) -> TypeId {
        let sig = self.build_signature(parameters, return_type);
        self.type_table.add_type(
            TypeFlags::OBJECT,
            TypeKind::ObjectType {
                object_flags: ObjectFlags::ANONYMOUS,
                members: vec![],
                call_signatures: vec![sig],
                construct_signatures: vec![],
                index_infos: vec![],
            },
        )
    }

    // ========================================================================
    // Expression checking
    // ========================================================================

    fn check_expression(&mut self, expr: &Expression<'_>) -> TypeId {
        match expr {
            Expression::Identifier(id) => {
                self.check_identifier(id)
            }
            Expression::NumericLiteral(_) => self.type_table.number_type,
            Expression::StringLiteral(_) => self.type_table.string_type,
            Expression::BigIntLiteral(_) => self.type_table.bigint_type,
            Expression::NoSubstitutionTemplateLiteral(_) => self.type_table.string_type,
            Expression::TemplateExpression(n) => {
                for span in n.template_spans.iter() { self.check_expression(span.expression); }
                self.type_table.string_type
            }
            Expression::RegularExpressionLiteral(_) => {
                self.get_regexp_type()
            }
            Expression::TrueKeyword(_) => self.type_table.true_type,
            Expression::FalseKeyword(_) => self.type_table.false_type,
            Expression::NullKeyword(_) => self.type_table.null_type,
            Expression::ThisKeyword(_) => {
                // In a class context, `this` is the instance type
                // Simplified: return any (would need class context tracking)
                self.type_table.any_type
            }
            Expression::SuperKeyword(_) => {
                // super refers to the base class
                self.type_table.any_type
            }

            Expression::Binary(n) => self.check_binary_expression(n),
            Expression::Call(n) => self.check_call_expression(n),
            Expression::New(n) => self.check_new_expression(n),
            Expression::PropertyAccess(n) => self.check_property_access(n),
            Expression::ElementAccess(n) => self.check_element_access(n),
            Expression::Conditional(n) => self.check_conditional_expression(n),
            Expression::Parenthesized(n) => self.check_expression(n.expression),
            Expression::PrefixUnary(n) => self.check_prefix_unary(n),
            Expression::PostfixUnary(n) => self.check_postfix_unary(n),
            Expression::TypeOf(n) => { self.check_expression(n.expression); self.type_table.string_type }
            Expression::Delete(n) => { self.check_expression(n.expression); self.type_table.boolean_type }
            Expression::Void(n) => { self.check_expression(n.expression); self.type_table.undefined_type }
            Expression::Await(n) => self.check_await_expression(n),
            Expression::Yield(n) => {
                if let Some(e) = n.expression {
                    self.check_expression(e)
                } else {
                    self.type_table.undefined_type
                }
            }
            Expression::Spread(n) => self.check_expression(n.expression),
            Expression::As(n) => {
                self.check_expression(n.expression);
                self.get_type_from_type_node(n.type_node)
            }
            Expression::Satisfies(n) => {
                let expr_type = self.check_expression(n.expression);
                let target_type = self.get_type_from_type_node(n.type_node);
                if !self.is_type_assignable_to(expr_type, target_type) {
                    let src = self.type_to_string(expr_type);
                    let tgt = self.type_to_string(target_type);
                    self.error(&messages::TYPE_0_IS_NOT_ASSIGNABLE_TO_TYPE_1, &[&src, &tgt]);
                }
                expr_type
            }
            Expression::NonNull(n) => {
                let inner = self.check_expression(n.expression);
                // Remove null and undefined from union
                self.get_non_nullable_type(inner)
            }
            Expression::TypeAssertion(n) => {
                self.check_expression(n.expression);
                self.get_type_from_type_node(n.type_node)
            }
            Expression::ArrowFunction(n) => self.check_arrow_function(n),
            Expression::FunctionExpression(n) => self.check_function_expression(n),
            Expression::ArrayLiteral(n) => self.check_array_literal(n),
            Expression::ObjectLiteral(n) => self.check_object_literal(n),
            Expression::ClassExpression(n) => { self.check_class_expression(n); self.type_table.any_type }
            Expression::TaggedTemplate(n) => {
                self.check_expression(n.tag);
                self.check_expression(n.template);
                self.type_table.any_type
            }
            Expression::MetaProperty(_) => self.type_table.any_type,
            Expression::OmittedExpression(_) => self.type_table.undefined_type,
        }
    }

    /// Well-known globals that don't require declaration.
    const BUILTIN_GLOBALS: &'static [&'static str] = &[
        "console", "setTimeout", "setInterval", "clearTimeout", "clearInterval",
        "parseInt", "parseFloat", "isNaN", "isFinite", "NaN", "Infinity",
        "undefined", "JSON", "Math", "Date", "Object", "Array", "String",
        "Number", "Boolean", "Symbol", "Map", "Set", "WeakMap", "WeakSet",
        "Promise", "Proxy", "Reflect", "Error", "TypeError", "RangeError",
        "SyntaxError", "ReferenceError", "URIError", "EvalError",
        "RegExp", "Function", "ArrayBuffer", "DataView",
        "Int8Array", "Uint8Array", "Int16Array", "Uint16Array",
        "Int32Array", "Uint32Array", "Float32Array", "Float64Array",
        "BigInt64Array", "BigUint64Array",
        "globalThis", "window", "document", "navigator",
        "process", "require", "module", "exports", "__dirname", "__filename",
        "fetch", "URL", "URLSearchParams", "Headers", "Request", "Response",
        "TextEncoder", "TextDecoder", "AbortController", "AbortSignal",
        "queueMicrotask", "structuredClone", "atob", "btoa",
        "alert", "confirm", "prompt",
        "eval", "encodeURI", "decodeURI", "encodeURIComponent", "decodeURIComponent",
    ];

    fn check_identifier(&mut self, id: &Identifier) -> TypeId {
        let name = &id.text_name;

        // Empty names are synthetic/dummy identifiers
        if name.is_empty() {
            return self.type_table.any_type;
        }

        // 1. Check declared_types (from checking phase)
        if let Some(type_id) = self.get_declared_type(name) {
            return type_id;
        }

        // 2. Check binder's symbol table
        if let Some(symbol_id) = self.binder.resolve_name(name) {
            // Determine type from symbol flags
            if let Some(symbol) = self.binder.get_symbol(symbol_id) {
                let flags = symbol.flags;
                if flags.contains(SymbolFlags::FUNCTION) {
                    return self.type_table.any_type; // function - type registered later
                }
                if flags.contains(SymbolFlags::CLASS) {
                    return self.type_table.any_type; // class - type registered later
                }
                if flags.intersects(SymbolFlags::FUNCTION_SCOPED_VARIABLE | SymbolFlags::BLOCK_SCOPED_VARIABLE) {
                    return self.type_table.any_type; // var type registered during check
                }
                if flags.contains(SymbolFlags::ENUM_MEMBER) {
                    return self.type_table.number_type;
                }
                if flags.contains(SymbolFlags::REGULAR_ENUM) {
                    return self.type_table.number_type;
                }
                if flags.contains(SymbolFlags::ALIAS) {
                    return self.type_table.any_type;
                }
            }
            return self.type_table.any_type;
        }

        // 3. Check built-in globals
        if Self::BUILTIN_GLOBALS.contains(&name.as_str()) {
            return self.type_table.any_type;
        }

        // 4. Check common keywords used as identifiers
        if name == "arguments" {
            return self.type_table.any_type;
        }

        // Not found - report error
        self.error(
            &messages::CANNOT_FIND_NAME_0,
            &[name],
        );
        // Use any_type for error recovery (prevents cascading errors)
        self.type_table.any_type
    }

    fn check_binary_expression(&mut self, node: &BinaryExpression<'_>) -> TypeId {
        let left_type = self.check_expression(node.left);
        let right_type = self.check_expression(node.right);

        match node.operator_token.data.kind {
            SyntaxKind::PlusToken => {
                let left = self.type_table.get(left_type);
                let right = self.type_table.get(right_type);
                if left.flags.contains(TypeFlags::STRING_LIKE) || right.flags.contains(TypeFlags::STRING_LIKE) {
                    self.type_table.string_type
                } else if left.flags.intersects(TypeFlags::NUMBER_LIKE) && right.flags.intersects(TypeFlags::NUMBER_LIKE) {
                    self.type_table.number_type
                } else if left.flags.contains(TypeFlags::ANY) || right.flags.contains(TypeFlags::ANY) {
                    self.type_table.any_type
                } else {
                    // Could be string or number - return string | number
                    self.create_union_type(vec![self.type_table.string_type, self.type_table.number_type])
                }
            }
            SyntaxKind::MinusToken | SyntaxKind::AsteriskToken | SyntaxKind::SlashToken
            | SyntaxKind::PercentToken | SyntaxKind::AsteriskAsteriskToken => {
                // Arithmetic: operands must be number-like
                let left_flags = self.type_table.get(left_type).flags;
                let right_flags = self.type_table.get(right_type).flags;
                if !left_flags.intersects(TypeFlags::NUMBER_LIKE | TypeFlags::ANY)
                    && !left_flags.contains(TypeFlags::BIG_INT)
                {
                    self.error(
                        &messages::THE_LEFT_HAND_SIDE_OF_AN_ARITHMETIC_OPERATION_MUST_BE_OF_TYPE_ANY_NUMBER_BIGINT_OR_AN_ENUM_TYPE,
                        &[],
                    );
                }
                if !right_flags.intersects(TypeFlags::NUMBER_LIKE | TypeFlags::ANY)
                    && !right_flags.contains(TypeFlags::BIG_INT)
                {
                    self.error(
                        &messages::THE_RIGHT_HAND_SIDE_OF_AN_ARITHMETIC_OPERATION_MUST_BE_OF_TYPE_ANY_NUMBER_BIGINT_OR_AN_ENUM_TYPE,
                        &[],
                    );
                }
                self.type_table.number_type
            }
            // Comparison operators
            SyntaxKind::EqualsEqualsToken | SyntaxKind::ExclamationEqualsToken
            | SyntaxKind::EqualsEqualsEqualsToken | SyntaxKind::ExclamationEqualsEqualsToken
            | SyntaxKind::LessThanToken | SyntaxKind::GreaterThanToken
            | SyntaxKind::LessThanEqualsToken | SyntaxKind::GreaterThanEqualsToken => {
                self.type_table.boolean_type
            }
            // instanceof
            SyntaxKind::InstanceOfKeyword => self.type_table.boolean_type,
            // in
            SyntaxKind::InKeyword => {
                let left = self.type_table.get(left_type);
                if !left.flags.intersects(TypeFlags::STRING_LIKE | TypeFlags::NUMBER_LIKE | TypeFlags::ES_SYMBOL | TypeFlags::ANY) {
                    self.error(
                        &messages::THE_LEFT_HAND_SIDE_OF_AN_IN_EXPRESSION_MUST_BE_A_PRIVATE_IDENTIFIER,
                        &[],
                    );
                }
                self.type_table.boolean_type
            }
            // Logical operators
            SyntaxKind::AmpersandAmpersandToken => right_type,
            SyntaxKind::BarBarToken => {
                self.create_union_type(vec![left_type, right_type])
            }
            SyntaxKind::QuestionQuestionToken => {
                // Result is right_type | NonNullable<left_type>
                let non_null_left = self.get_non_nullable_type(left_type);
                self.create_union_type(vec![non_null_left, right_type])
            }
            // Assignment operators
            SyntaxKind::EqualsToken => {
                if !self.is_type_assignable_to(right_type, left_type) {
                    let src = self.type_to_string(right_type);
                    let tgt = self.type_to_string(left_type);
                    self.error(&messages::TYPE_0_IS_NOT_ASSIGNABLE_TO_TYPE_1, &[&src, &tgt]);
                }
                right_type
            }
            SyntaxKind::PlusEqualsToken | SyntaxKind::MinusEqualsToken
            | SyntaxKind::AsteriskEqualsToken | SyntaxKind::SlashEqualsToken
            | SyntaxKind::PercentEqualsToken | SyntaxKind::AsteriskAsteriskEqualsToken => {
                self.type_table.number_type
            }
            SyntaxKind::AmpersandAmpersandEqualsToken | SyntaxKind::BarBarEqualsToken
            | SyntaxKind::QuestionQuestionEqualsToken => {
                right_type
            }
            // Bitwise
            SyntaxKind::AmpersandToken | SyntaxKind::BarToken | SyntaxKind::CaretToken
            | SyntaxKind::LessThanLessThanToken | SyntaxKind::GreaterThanGreaterThanToken
            | SyntaxKind::GreaterThanGreaterThanGreaterThanToken => {
                self.type_table.number_type
            }
            SyntaxKind::CommaToken => right_type,
            _ => self.type_table.any_type,
        }
    }

    fn check_call_expression(&mut self, node: &CallExpression<'_>) -> TypeId {
        let func_type = self.check_expression(node.expression);
        let mut arg_types = Vec::new();
        for arg in node.arguments.iter() {
            arg_types.push(self.check_expression(arg));
        }

        // Try to resolve call signature
        let func_flags = self.type_table.get(func_type).flags;
        if func_flags.contains(TypeFlags::ANY) {
            return self.type_table.any_type;
        }

        // Extract lightweight signature data (param TypeIds + return type)
        // instead of cloning full Signature structs with their String-heavy fields.
        let sigs: Vec<(Vec<TypeId>, TypeId, usize, usize)> = if let TypeKind::ObjectType { call_signatures, .. } = &self.type_table.get(func_type).kind {
            call_signatures.iter().map(|sig| (
                sig.parameters.iter().map(|p| p.type_id).collect(),
                sig.return_type,
                sig.min_argument_count as usize,
                if sig.has_rest_parameter { usize::MAX } else { sig.parameters.len() },
            )).collect()
        } else {
            return self.type_table.any_type;
        };

        if sigs.is_empty() {
            self.error(&messages::CANNOT_INVOKE_AN_EXPRESSION_WHOSE_TYPE_LACKS_A_CALL_SIGNATURE, &[]);
            return self.type_table.any_type;
        }

        // Try each overload
        for (ref param_types, return_type, min_args, max_args) in &sigs {
            if self.check_call_arguments(&arg_types, param_types, *min_args, *max_args).is_some() {
                return *return_type;
            }
        }

        // No overload matched
        if sigs.len() > 1 {
            self.error(&messages::NO_OVERLOAD_MATCHES_THIS_CALL, &[]);
        }
        sigs[0].1
    }

    /// Checks that the argument types match the parameter types.
    /// Takes flattened signature data (param TypeIds, min/max args) instead of
    /// a full `&Signature` to avoid cloning String-heavy `SignatureParameter`s.
    fn check_call_arguments(
        &mut self,
        arg_types: &[TypeId],
        param_types: &[TypeId],
        min_args: usize,
        max_args: usize,
    ) -> Option<()> {
        if arg_types.len() < min_args {
            self.error(
                &messages::EXPECTED_0_ARGUMENTS_BUT_GOT_1,
                &[&min_args.to_string(), &arg_types.len().to_string()],
            );
            return None;
        }
        if arg_types.len() > max_args {
            self.error(
                &messages::EXPECTED_0_ARGUMENTS_BUT_GOT_1,
                &[&max_args.to_string(), &arg_types.len().to_string()],
            );
            return None;
        }

        // Check each argument against its parameter type
        for (i, arg_type) in arg_types.iter().enumerate() {
            if i < param_types.len() {
                let param_type = param_types[i];
                if !self.is_type_assignable_to(*arg_type, param_type) {
                    let src = self.type_to_string(*arg_type);
                    let tgt = self.type_to_string(param_type);
                    self.error(
                        &messages::ARGUMENT_OF_TYPE_0_IS_NOT_ASSIGNABLE_TO_PARAMETER_OF_TYPE_1,
                        &[&src, &tgt],
                    );
                    return None;
                }
            }
        }

        Some(())
    }

    fn check_new_expression(&mut self, node: &NewExpression<'_>) -> TypeId {
        let class_type = self.check_expression(node.expression);
        let mut arg_types = Vec::new();
        if let Some(args) = node.arguments {
            for arg in args.iter() {
                arg_types.push(self.check_expression(arg));
            }
        }

        let class_flags = self.type_table.get(class_type).flags;
        if class_flags.contains(TypeFlags::ANY) {
            return self.type_table.any_type;
        }

        // Extract lightweight construct signature data instead of cloning
        let sigs: Vec<(Vec<TypeId>, TypeId, usize, usize)> = if let TypeKind::ObjectType { construct_signatures, .. } = &self.type_table.get(class_type).kind {
            construct_signatures.iter().map(|sig| (
                sig.parameters.iter().map(|p| p.type_id).collect(),
                sig.return_type,
                sig.min_argument_count as usize,
                if sig.has_rest_parameter { usize::MAX } else { sig.parameters.len() },
            )).collect()
        } else {
            vec![]
        };

        if sigs.is_empty() {
            self.error(&messages::THIS_EXPRESSION_IS_NOT_CONSTRUCTABLE, &[]);
            return self.type_table.any_type;
        }

        // Use the return type of the first matching construct signature
        for (ref param_types, return_type, min_args, max_args) in &sigs {
            if self.check_call_arguments(&arg_types, param_types, *min_args, *max_args).is_some() {
                return *return_type;
            }
        }

        // Return the first construct signature's return type as fallback
        sigs[0].1
    }

    fn check_property_access(&mut self, node: &PropertyAccessExpression<'_>) -> TypeId {
        let obj_type = self.check_expression(node.expression);

        let obj_flags = self.type_table.get(obj_type).flags;
        if obj_flags.contains(TypeFlags::ANY) {
            return self.type_table.any_type;
        }

        // Check for null/undefined access
        if self.strict_null_checks {
            if obj_flags.contains(TypeFlags::NULL) {
                self.error(&messages::OBJECT_IS_POSSIBLY_NULL, &[]);
            }
            if obj_flags.contains(TypeFlags::UNDEFINED) {
                self.error(&messages::OBJECT_IS_POSSIBLY_UNDEFINED, &[]);
            }
        }

        // Get the property name
        let prop_name = match &node.name {
            MemberName::Identifier(id) => id.text_name.clone(),
            MemberName::PrivateIdentifier(id) => id.text_name.clone(),
        };

        // Look up the single property we need from the type table.
        // Extract just the TypeId instead of cloning the entire members vec.
        let prop_type_id = if let TypeKind::ObjectType { members, .. } = &self.type_table.get(obj_type).kind {
            members.iter()
                .find(|(name, _)| name == &prop_name)
                .map(|(_, tid)| *tid)
        } else {
            return self.type_table.any_type;
        };

        if let Some(tid) = prop_type_id {
            return tid;
        }

        // Property not found on a known object type — for now return any
        // (a full implementation would report TS2339: Property 'x' does not exist on type 'Y')
        self.type_table.any_type
    }

    fn check_element_access(&mut self, node: &ElementAccessExpression<'_>) -> TypeId {
        let obj_type = self.check_expression(node.expression);
        let index_type = self.check_expression(node.argument_expression);

        let obj_flags = self.type_table.get(obj_type).flags;
        if obj_flags.contains(TypeFlags::ANY) {
            return self.type_table.any_type;
        }

        // Extract index info pairs to release the borrow before is_type_assignable_to
        let index_pairs: Vec<(TypeId, TypeId)> = if let TypeKind::ObjectType { index_infos, .. } = &self.type_table.get(obj_type).kind {
            index_infos.iter().map(|info| (info.key_type, info.type_id)).collect()
        } else {
            vec![]
        };

        for (key_type, result_type) in index_pairs {
            if self.is_type_assignable_to(index_type, key_type) {
                return result_type;
            }
        }

        self.type_table.any_type
    }

    fn check_conditional_expression(&mut self, node: &ConditionalExpression<'_>) -> TypeId {
        self.check_expression(node.condition);
        let true_type = self.check_expression(node.when_true);
        let false_type = self.check_expression(node.when_false);
        self.create_union_type(vec![true_type, false_type])
    }

    fn check_prefix_unary(&mut self, node: &PrefixUnaryExpression<'_>) -> TypeId {
        let operand_type = self.check_expression(node.operand);
        match node.operator {
            SyntaxKind::PlusToken | SyntaxKind::MinusToken | SyntaxKind::TildeToken => {
                self.type_table.number_type
            }
            SyntaxKind::ExclamationToken => self.type_table.boolean_type,
            SyntaxKind::PlusPlusToken | SyntaxKind::MinusMinusToken => {
                let ty = self.type_table.get(operand_type);
                if !ty.flags.intersects(TypeFlags::NUMBER_LIKE | TypeFlags::ANY) {
                    self.error(
                        &messages::THE_OPERAND_OF_AN_INCREMENT_OR_DECREMENT_OPERATOR_MUST_BE_A_VARIABLE_OR_A_PROPERTY_ACCESS,
                        &[],
                    );
                }
                self.type_table.number_type
            }
            _ => self.type_table.any_type,
        }
    }

    fn check_postfix_unary(&mut self, node: &PostfixUnaryExpression<'_>) -> TypeId {
        let operand_type = self.check_expression(node.operand);
        let ty = self.type_table.get(operand_type);
        if !ty.flags.intersects(TypeFlags::NUMBER_LIKE | TypeFlags::ANY) {
            self.error(
                &messages::THE_OPERAND_OF_AN_INCREMENT_OR_DECREMENT_OPERATOR_MUST_BE_A_VARIABLE_OR_A_PROPERTY_ACCESS,
                &[],
            );
        }
        self.type_table.number_type
    }

    fn check_await_expression(&mut self, node: &AwaitExpression<'_>) -> TypeId {
        let operand_type = self.check_expression(node.expression);
        // If the operand is any, return any
        let ty = self.type_table.get(operand_type);
        if ty.flags.contains(TypeFlags::ANY) {
            return self.type_table.any_type;
        }
        // For non-any types, the awaited result is the operand type itself
        // (a real implementation would unwrap Promise<T> to T via the thenable protocol)
        operand_type
    }

    fn check_arrow_function(&mut self, node: &ArrowFunction<'_>) -> TypeId {
        for param in node.parameters.iter() {
            if let Some(init) = param.initializer { self.check_expression(init); }
        }
        match &node.body {
            ArrowFunctionBody::Block(block) => {
                for s in block.statements.iter() { self.check_statement(s); }
            }
            ArrowFunctionBody::Expression(expr) => { self.check_expression(expr); }
        }

        // Create function type
        let params: Vec<SignatureParameter> = node.parameters.iter().map(|p| {
            let param_type = self.get_type_from_type_annotation(p.type_annotation);
            SignatureParameter {
                name: String::new(), // would need interner
                type_id: param_type.unwrap_or(self.type_table.any_type),
                optional: p.question_token.is_some(),
            }
        }).collect();

        let return_type = self.get_type_from_type_annotation(node.return_type)
            .unwrap_or(self.type_table.any_type);

        let sig = Signature {
            type_parameters: vec![],
            parameters: params,
            return_type,
            min_argument_count: node.parameters.iter()
                .filter(|p| p.question_token.is_none() && p.initializer.is_none() && p.dot_dot_dot_token.is_none())
                .count() as u32,
            has_rest_parameter: node.parameters.iter().any(|p| p.dot_dot_dot_token.is_some()),
        };

        self.type_table.add_type(
            TypeFlags::OBJECT,
            TypeKind::ObjectType {
                object_flags: ObjectFlags::ANONYMOUS,
                members: vec![],
                call_signatures: vec![sig],
                construct_signatures: vec![],
                index_infos: vec![],
            },
        )
    }

    fn check_function_expression(&mut self, node: &FunctionExpression<'_>) -> TypeId {
        let params: Vec<SignatureParameter> = node.parameters.iter().map(|p| {
            if let Some(init) = p.initializer { self.check_expression(init); }
            let param_type = self.get_type_from_type_annotation(p.type_annotation);
            let param_name = match &p.name {
                BindingName::Identifier(id) => id.text_name.clone(),
                _ => String::new(),
            };
            let resolved_type = param_type.unwrap_or(self.type_table.any_type);
            if !param_name.is_empty() {
                self.register_type(&param_name, resolved_type);
            }
            SignatureParameter {
                name: param_name,
                type_id: resolved_type,
                optional: p.question_token.is_some(),
            }
        }).collect();

        for s in node.body.statements.iter() { self.check_statement(s); }

        let return_type = self.get_type_from_type_annotation(node.return_type)
            .unwrap_or(self.type_table.any_type);

        let sig = Signature {
            type_parameters: vec![],
            parameters: params,
            return_type,
            min_argument_count: node.parameters.iter()
                .filter(|p| p.question_token.is_none() && p.initializer.is_none() && p.dot_dot_dot_token.is_none())
                .count() as u32,
            has_rest_parameter: node.parameters.iter().any(|p| p.dot_dot_dot_token.is_some()),
        };

        self.type_table.add_type(
            TypeFlags::OBJECT,
            TypeKind::ObjectType {
                object_flags: ObjectFlags::ANONYMOUS,
                members: vec![],
                call_signatures: vec![sig],
                construct_signatures: vec![],
                index_infos: vec![],
            },
        )
    }

    fn check_array_literal(&mut self, node: &ArrayLiteralExpression<'_>) -> TypeId {
        let mut element_types = Vec::new();
        for elem in node.elements.iter() {
            element_types.push(self.check_expression(elem));
        }

        if element_types.is_empty() {
            // Empty array: any[] — create Array<any> object type
            return self.create_array_type(self.type_table.any_type);
        }

        // Infer array element type as union of all element types
        let elem_type = if element_types.len() == 1 {
            element_types[0]
        } else {
            self.create_union_type(element_types)
        };

        // Return T[] type as an object type with numeric index signature
        self.create_array_type(elem_type)
    }

    fn check_object_literal(&mut self, node: &ObjectLiteralExpression<'_>) -> TypeId {
        let mut members = Vec::new();
        for prop in node.properties.iter() {
            match prop {
                ObjectLiteralElement::PropertyAssignment(p) => {
                    let value_type = self.check_expression(p.initializer);
                    let prop_name = self.property_name_text(&p.name);
                    members.push((prop_name, value_type));
                }
                ObjectLiteralElement::ShorthandPropertyAssignment(p) => {
                    // Shorthand { x } is equivalent to { x: x }
                    let name = p.name.text_name.clone();
                    let value_type = if let Some(init) = p.object_assignment_initializer {
                        self.check_expression(init)
                    } else {
                        // Look up the identifier's type
                        self.check_identifier(&p.name)
                    };
                    members.push((name, value_type));
                }
                ObjectLiteralElement::SpreadAssignment(p) => {
                    self.check_expression(p.expression);
                    // Spread merges in members from the spread object
                }
                ObjectLiteralElement::MethodDeclaration(m) => {
                    // Build method type
                    let method_params: Vec<SignatureParameter> = m.parameters.iter().map(|p| {
                        if let Some(init) = p.initializer { self.check_expression(init); }
                        let param_type = self.get_type_from_type_annotation(p.type_annotation);
                        SignatureParameter {
                            name: match &p.name {
                                BindingName::Identifier(id) => id.text_name.clone(),
                                _ => String::new(),
                            },
                            type_id: param_type.unwrap_or(self.type_table.any_type),
                            optional: p.question_token.is_some(),
                        }
                    }).collect();

                    let return_type = self.get_type_from_type_annotation(m.return_type)
                        .unwrap_or(self.type_table.any_type);

                    if let Some(ref body) = m.body {
                        for s in body.statements.iter() { self.check_statement(s); }
                    }

                    let sig = Signature {
                        type_parameters: vec![],
                        parameters: method_params,
                        return_type,
                        min_argument_count: m.parameters.iter()
                            .filter(|p| p.question_token.is_none() && p.initializer.is_none())
                            .count() as u32,
                        has_rest_parameter: m.parameters.iter().any(|p| p.dot_dot_dot_token.is_some()),
                    };
                    let method_type = self.type_table.add_type(
                        TypeFlags::OBJECT,
                        TypeKind::ObjectType {
                            object_flags: ObjectFlags::ANONYMOUS,
                            members: vec![],
                            call_signatures: vec![sig],
                            construct_signatures: vec![],
                            index_infos: vec![],
                        },
                    );
                    let method_name = self.property_name_text(&m.name);
                    members.push((method_name, method_type));
                }
                _ => {}
            }
        }

        self.type_table.add_type(
            TypeFlags::OBJECT,
            TypeKind::ObjectType {
                object_flags: ObjectFlags::ANONYMOUS,
                members,
                call_signatures: vec![],
                construct_signatures: vec![],
                index_infos: vec![],
            },
        )
    }

    fn check_class_expression(&mut self, node: &ClassExpression<'_>) {
        for member in node.members.iter() {
            match member {
                ClassElement::MethodDeclaration(m) => {
                    if let Some(ref body) = m.body {
                        for s in body.statements.iter() { self.check_statement(s); }
                    }
                }
                ClassElement::PropertyDeclaration(p) => {
                    if let Some(init) = p.initializer { self.check_expression(init); }
                }
                _ => {}
            }
        }
    }

    // ========================================================================
    // Type resolution
    // ========================================================================

    fn get_type_from_type_annotation(&mut self, annotation: Option<&TypeNode<'_>>) -> Option<TypeId> {
        annotation.map(|ty| self.get_type_from_type_node(ty))
    }

    fn get_type_from_type_node(&mut self, type_node: &TypeNode<'_>) -> TypeId {
        match type_node {
            TypeNode::KeywordType(n) => self.get_type_from_keyword(n.data.kind),
            TypeNode::TypeReference(n) => {
                // Resolve the type reference through the binder/declared types
                let ref_name = match &n.type_name {
                    EntityName::Identifier(id) => id.text_name.clone(),
                    EntityName::QualifiedName(q) => {
                        // For qualified names like A.B, just use the right name for now
                        q.right.text_name.clone()
                    }
                };

                // Check well-known type names
                match ref_name.as_str() {
                    "Array" => {
                        let elem_type = if let Some(type_args) = n.type_arguments {
                            if !type_args.is_empty() {
                                self.get_type_from_type_node(&type_args[0])
                            } else {
                                self.type_table.any_type
                            }
                        } else {
                            self.type_table.any_type
                        };
                        return self.create_array_type(elem_type);
                    }
                    "ReadonlyArray" => {
                        let elem_type = if let Some(type_args) = n.type_arguments {
                            if !type_args.is_empty() {
                                self.get_type_from_type_node(&type_args[0])
                            } else {
                                self.type_table.any_type
                            }
                        } else {
                            self.type_table.any_type
                        };
                        return self.create_array_type(elem_type);
                    }
                    "Promise" => {
                        let _inner = if let Some(type_args) = n.type_arguments {
                            if !type_args.is_empty() {
                                self.get_type_from_type_node(&type_args[0])
                            } else {
                                self.type_table.any_type
                            }
                        } else {
                            self.type_table.any_type
                        };
                        // Promise<T> is an object type; full implementation would track T
                        return self.type_table.add_type(
                            TypeFlags::OBJECT,
                            TypeKind::ObjectType {
                                object_flags: ObjectFlags::empty(),
                                members: vec![],
                                call_signatures: vec![],
                                construct_signatures: vec![],
                                index_infos: vec![],
                            },
                        );
                    }
                    "Record" => return self.type_table.any_type,
                    "Partial" => {
                        if let Some(type_args) = n.type_arguments {
                            if !type_args.is_empty() {
                                let inner = self.get_type_from_type_node(&type_args[0]);
                                return self.create_partial_type(inner);
                            }
                        }
                        return self.type_table.any_type;
                    }
                    "Required" => {
                        if let Some(type_args) = n.type_arguments {
                            if !type_args.is_empty() {
                                let inner = self.get_type_from_type_node(&type_args[0]);
                                return self.create_required_type(inner);
                            }
                        }
                        return self.type_table.any_type;
                    }
                    "Readonly" => {
                        if let Some(type_args) = n.type_arguments {
                            if !type_args.is_empty() {
                                let inner = self.get_type_from_type_node(&type_args[0]);
                                return self.create_readonly_type(inner);
                            }
                        }
                        return self.type_table.any_type;
                    }
                    "Pick" => {
                        if let Some(type_args) = n.type_arguments {
                            if type_args.len() >= 2 {
                                let base = self.get_type_from_type_node(&type_args[0]);
                                let keys_type = self.get_type_from_type_node(&type_args[1]);
                                return self.create_pick_type(base, &[keys_type]);
                            }
                        }
                        return self.type_table.any_type;
                    }
                    "Omit" => {
                        if let Some(type_args) = n.type_arguments {
                            if type_args.len() >= 2 {
                                let base = self.get_type_from_type_node(&type_args[0]);
                                let keys_type = self.get_type_from_type_node(&type_args[1]);
                                return self.create_omit_type(base, &[keys_type]);
                            }
                        }
                        return self.type_table.any_type;
                    }
                    "ReturnType" => {
                        if let Some(type_args) = n.type_arguments {
                            if !type_args.is_empty() {
                                let inner = self.get_type_from_type_node(&type_args[0]);
                                return self.get_return_type_of(inner);
                            }
                        }
                        return self.type_table.any_type;
                    }
                    "Parameters" => {
                        if let Some(type_args) = n.type_arguments {
                            if !type_args.is_empty() {
                                let inner = self.get_type_from_type_node(&type_args[0]);
                                return self.get_parameters_type_of(inner);
                            }
                        }
                        return self.type_table.any_type;
                    }
                    "NonNullable" => {
                        if let Some(type_args) = n.type_arguments {
                            if !type_args.is_empty() {
                                let inner = self.get_type_from_type_node(&type_args[0]);
                                return self.get_non_nullable_type(inner);
                            }
                        }
                        return self.type_table.any_type;
                    }
                    "Exclude" | "Extract" | "InstanceType" | "ConstructorParameters"
                    | "ThisParameterType" | "OmitThisParameter" | "Uppercase" | "Lowercase"
                    | "Capitalize" | "Uncapitalize" => {
                        return self.type_table.any_type;
                    }
                    _ => {}
                }

                // Look up in declared_types
                if let Some(type_id) = self.get_declared_type(&ref_name) {
                    return type_id;
                }

                // Look up in binder
                if let Some(symbol_id) = self.binder.resolve_name(&ref_name) {
                    if let Some(symbol) = self.binder.get_symbol(symbol_id) {
                        if symbol.flags.contains(SymbolFlags::INTERFACE) || symbol.flags.contains(SymbolFlags::TYPE_ALIAS) {
                            return self.type_table.any_type; // Would need type resolution
                        }
                        if symbol.flags.contains(SymbolFlags::CLASS) {
                            return self.type_table.any_type; // Would need instance type
                        }
                        if symbol.flags.contains(SymbolFlags::REGULAR_ENUM) {
                            return self.type_table.number_type;
                        }
                    }
                }

                self.type_table.any_type
            }
            TypeNode::ArrayType(n) => {
                let elem_type = self.get_type_from_type_node(n.element_type);
                self.create_array_type(elem_type)
            }
            TypeNode::TupleType(n) => {
                let elem_types: Vec<TypeId> = n.elements.iter()
                    .map(|e| self.get_type_from_type_node(e))
                    .collect();
                self.type_table.add_type(
                    TypeFlags::OBJECT,
                    TypeKind::Tuple {
                        element_types: elem_types,
                        element_flags: vec![],
                    },
                )
            }
            TypeNode::UnionType(n) => {
                let types: Vec<TypeId> = n.types.iter()
                    .map(|t| self.get_type_from_type_node(t))
                    .collect();
                self.create_union_type(types)
            }
            TypeNode::IntersectionType(n) => {
                let types: Vec<TypeId> = n.types.iter()
                    .map(|t| self.get_type_from_type_node(t))
                    .collect();
                self.create_intersection_type(types)
            }
            TypeNode::FunctionType(n) => {
                let params: Vec<SignatureParameter> = n.parameters.iter().map(|p| {
                    let param_type = self.get_type_from_type_annotation(p.type_annotation);
                    SignatureParameter {
                        name: String::new(),
                        type_id: param_type.unwrap_or(self.type_table.any_type),
                        optional: p.question_token.is_some(),
                    }
                }).collect();
                let return_type = n.return_type
                    .map(|r| self.get_type_from_type_node(r))
                    .unwrap_or(self.type_table.any_type);

                let sig = Signature {
                    type_parameters: vec![],
                    parameters: params,
                    return_type,
                    min_argument_count: n.parameters.iter()
                        .filter(|p| p.question_token.is_none() && p.initializer.is_none())
                        .count() as u32,
                    has_rest_parameter: n.parameters.iter().any(|p| p.dot_dot_dot_token.is_some()),
                };
                self.type_table.add_type(
                    TypeFlags::OBJECT,
                    TypeKind::ObjectType {
                        object_flags: ObjectFlags::ANONYMOUS,
                        members: vec![],
                        call_signatures: vec![sig],
                        construct_signatures: vec![],
                        index_infos: vec![],
                    },
                )
            }
            TypeNode::ConditionalType(n) => {
                let check = self.get_type_from_type_node(n.check_type);
                let extends = self.get_type_from_type_node(n.extends_type);
                let true_type = self.get_type_from_type_node(n.true_type);
                let false_type = self.get_type_from_type_node(n.false_type);

                // If check and extends are concrete, evaluate immediately
                let check_flags = self.type_table.get(check).flags;
                let extends_flags = self.type_table.get(extends).flags;
                let is_concrete = |f: TypeFlags| {
                    f.intersects(TypeFlags::STRING | TypeFlags::NUMBER | TypeFlags::BOOLEAN |
                        TypeFlags::NULL | TypeFlags::UNDEFINED | TypeFlags::VOID |
                        TypeFlags::NEVER | TypeFlags::STRING_LITERAL | TypeFlags::NUMBER_LITERAL |
                        TypeFlags::BOOLEAN_LITERAL | TypeFlags::OBJECT)
                };
                if is_concrete(check_flags) && is_concrete(extends_flags) {
                    return self.evaluate_conditional_type(check, extends, true_type, false_type);
                }

                self.type_table.add_type(
                    TypeFlags::CONDITIONAL,
                    TypeKind::Conditional { check_type: check, extends_type: extends, true_type, false_type },
                )
            }
            TypeNode::IndexedAccessType(n) => {
                let obj = self.get_type_from_type_node(n.object_type);
                let idx = self.get_type_from_type_node(n.index_type);
                // Try to resolve the indexed access immediately
                let resolved = self.resolve_indexed_access(obj, idx);
                if resolved != self.type_table.any_type {
                    return resolved;
                }
                self.type_table.add_type(
                    TypeFlags::INDEXED_ACCESS,
                    TypeKind::IndexedAccess { object_type: obj, index_type: idx },
                )
            }
            TypeNode::TypeOperator(n) => {
                let operand = self.get_type_from_type_node(n.type_node);
                match n.operator {
                    SyntaxKind::KeyOfKeyword => {
                        // Extract member names from the operand type and create a union of string literal types
                        let member_names = self.get_object_member_names(operand);
                        if member_names.is_empty() {
                            // Fallback: string | number | symbol
                            self.create_union_type(vec![self.type_table.string_type, self.type_table.number_type, self.type_table.symbol_type])
                        } else {
                            let literal_types: Vec<TypeId> = member_names.iter().map(|name| {
                                self.type_table.add_type(
                                    TypeFlags::STRING_LITERAL,
                                    TypeKind::StringLiteral { value: name.clone(), regular: true },
                                )
                            }).collect();
                            self.create_union_type(literal_types)
                        }
                    }
                    SyntaxKind::ReadonlyKeyword => operand,
                    SyntaxKind::UniqueKeyword => self.type_table.symbol_type,
                    _ => operand,
                }
            }
            TypeNode::TypeLiteral(n) => {
                let mut members = Vec::new();
                let mut index_infos = Vec::new();
                for member in n.members.iter() {
                    match member {
                        TypeElement::PropertySignature(p) => {
                            let prop_type = self.get_type_from_type_annotation(p.type_annotation);
                            members.push((String::new(), prop_type.unwrap_or(self.type_table.any_type)));
                        }
                        TypeElement::IndexSignature(idx) => {
                            let idx_type = self.get_type_from_type_annotation(idx.type_annotation);
                            index_infos.push(IndexInfo {
                                key_type: self.type_table.string_type,
                                type_id: idx_type.unwrap_or(self.type_table.any_type),
                                is_readonly: false,
                            });
                        }
                        _ => {}
                    }
                }
                self.type_table.add_type(
                    TypeFlags::OBJECT,
                    TypeKind::ObjectType {
                        object_flags: ObjectFlags::ANONYMOUS,
                        members, call_signatures: vec![], construct_signatures: vec![],
                        index_infos,
                    },
                )
            }
            TypeNode::ParenthesizedType(n) => self.get_type_from_type_node(n.type_node),
            TypeNode::LiteralType(n) => {
                match n.literal {
                    Expression::TrueKeyword(_) => self.type_table.true_type,
                    Expression::FalseKeyword(_) => self.type_table.false_type,
                    Expression::NullKeyword(_) => self.type_table.null_type,
                    Expression::StringLiteral(_) => self.type_table.string_type,
                    Expression::NumericLiteral(_) => self.type_table.number_type,
                    _ => self.type_table.any_type,
                }
            }
            TypeNode::ThisType(_) => self.type_table.any_type,
            TypeNode::TypeQuery(n) => {
                // typeof expr — resolve the expression name to its declared type
                let name = match &n.expr_name {
                    EntityName::Identifier(id) => id.text_name.clone(),
                    EntityName::QualifiedName(q) => q.right.text_name.clone(),
                };
                self.get_declared_type(&name).unwrap_or(self.type_table.any_type)
            }
            TypeNode::InferType(_) => self.type_table.any_type,
            TypeNode::MappedType(_) => self.type_table.any_type,
            TypeNode::TemplateLiteralType(_) => self.type_table.string_type,
            _ => self.type_table.any_type,
        }
    }

    fn get_type_from_keyword(&self, kind: SyntaxKind) -> TypeId {
        match kind {
            SyntaxKind::StringKeyword => self.type_table.string_type,
            SyntaxKind::NumberKeyword => self.type_table.number_type,
            SyntaxKind::BooleanKeyword => self.type_table.boolean_type,
            SyntaxKind::AnyKeyword => self.type_table.any_type,
            SyntaxKind::VoidKeyword => self.type_table.void_type,
            SyntaxKind::NeverKeyword => self.type_table.never_type,
            SyntaxKind::UndefinedKeyword => self.type_table.undefined_type,
            SyntaxKind::NullKeyword => self.type_table.null_type,
            SyntaxKind::UnknownKeyword => self.type_table.unknown_type,
            SyntaxKind::ObjectKeyword => self.type_table.object_type,
            SyntaxKind::BigIntKeyword => self.type_table.bigint_type,
            SyntaxKind::SymbolKeyword => self.type_table.symbol_type,
            _ => self.type_table.any_type,
        }
    }

    // ========================================================================
    // Type utilities
    // ========================================================================

    /// Create an Array<T> type with a numeric index signature.
    fn create_array_type(&mut self, element_type: TypeId) -> TypeId {
        self.type_table.add_type(
            TypeFlags::OBJECT,
            TypeKind::ObjectType {
                object_flags: ObjectFlags::empty(),
                members: vec![
                    ("length".to_string(), self.type_table.number_type),
                ],
                call_signatures: vec![],
                construct_signatures: vec![],
                index_infos: vec![IndexInfo {
                    key_type: self.type_table.number_type,
                    type_id: element_type,
                    is_readonly: false,
                }],
            },
        )
    }

    /// Extract the text name from a PropertyName node.
    fn property_name_text(&self, name: &PropertyName<'_>) -> String {
        match name {
            PropertyName::Identifier(id) => id.text_name.clone(),
            PropertyName::StringLiteral(_) => String::new(), // would need source text
            PropertyName::NumericLiteral(_) => String::new(),
            PropertyName::ComputedPropertyName(_) => String::new(), // dynamic
            PropertyName::PrivateIdentifier(id) => id.text_name.clone(),
        }
    }

    fn create_union_type(&mut self, types: Vec<TypeId>) -> TypeId {
        // O(n) dedup using FxHashSet while preserving insertion order.
        let mut seen = FxHashSet::default();
        let unique: Vec<TypeId> = types.into_iter().filter(|t| seen.insert(*t)).collect();
        if unique.is_empty() {
            return self.type_table.never_type;
        }
        if unique.len() == 1 {
            return unique[0];
        }
        self.type_table.add_type(
            TypeFlags::UNION,
            TypeKind::Union { types: unique },
        )
    }

    fn create_intersection_type(&mut self, types: Vec<TypeId>) -> TypeId {
        let mut seen = FxHashSet::default();
        let unique: Vec<TypeId> = types.into_iter().filter(|t| seen.insert(*t)).collect();
        if unique.is_empty() {
            return self.type_table.any_type;
        }
        if unique.len() == 1 { return unique[0]; }
        self.type_table.add_type(
            TypeFlags::INTERSECTION,
            TypeKind::Intersection { types: unique },
        )
    }

    fn get_non_nullable_type(&mut self, type_id: TypeId) -> TypeId {
        let ty = self.type_table.get(type_id);
        if ty.flags.contains(TypeFlags::NULL) || ty.flags.contains(TypeFlags::UNDEFINED) {
            return self.type_table.never_type;
        }
        if let TypeKind::Union { types } = &ty.kind {
            let filtered: Vec<TypeId> = types.iter()
                .filter(|&&t| {
                    let inner = self.type_table.get(t);
                    !inner.flags.contains(TypeFlags::NULL) && !inner.flags.contains(TypeFlags::UNDEFINED)
                })
                .copied()
                .collect();
            if filtered.is_empty() {
                return self.type_table.never_type;
            }
            if filtered.len() == 1 {
                return filtered[0];
            }
            return self.create_union_type(filtered);
        }
        type_id
    }

    fn type_to_string(&self, type_id: TypeId) -> String {
        self.type_to_string_inner(type_id, 0)
    }

    fn type_to_string_inner(&self, type_id: TypeId, depth: u32) -> String {
        if depth > MAX_TYPE_TO_STRING_DEPTH {
            return "...".to_string();
        }
        let ty = self.type_table.get(type_id);
        match &ty.kind {
            TypeKind::Intrinsic { name } => name.to_string(),
            TypeKind::BooleanLiteral { value } => value.to_string(),
            TypeKind::StringLiteral { value, .. } => format!("\"{}\"", value),
            TypeKind::NumberLiteral { value } => value.to_string(),
            TypeKind::Union { types } => {
                types.iter()
                    .map(|t| self.type_to_string_inner(*t, depth + 1))
                    .collect::<Vec<_>>()
                    .join(" | ")
            }
            TypeKind::Intersection { types } => {
                types.iter()
                    .map(|t| self.type_to_string_inner(*t, depth + 1))
                    .collect::<Vec<_>>()
                    .join(" & ")
            }
            TypeKind::ObjectType { index_infos, members, call_signatures, .. } => {
                // Check if it's an array type (has number index signature)
                if let Some(idx) = index_infos.first() {
                    if self.type_table.get(idx.key_type).flags.intersects(TypeFlags::NUMBER_LIKE) {
                        return format!("{}[]", self.type_to_string_inner(idx.type_id, depth + 1));
                    }
                }
                // Check if it's a function type (has call signatures, no members)
                if !call_signatures.is_empty() && members.is_empty() {
                    let sig = &call_signatures[0];
                    let params = sig.parameters.iter()
                        .map(|p| {
                            let t = self.type_to_string_inner(p.type_id, depth + 1);
                            if p.name.is_empty() { t } else { format!("{}: {}", p.name, t) }
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    let ret = self.type_to_string_inner(sig.return_type, depth + 1);
                    return format!("({}) => {}", params, ret);
                }
                // Object type with members
                if members.is_empty() {
                    "{}".to_string()
                } else {
                    let props = members.iter()
                        .map(|(name, tid)| format!("{}: {}", name, self.type_to_string_inner(*tid, depth + 1)))
                        .collect::<Vec<_>>()
                        .join("; ");
                    format!("{{ {} }}", props)
                }
            }
            TypeKind::Tuple { element_types, .. } => {
                let elems = element_types.iter()
                    .map(|t| self.type_to_string_inner(*t, depth + 1))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", elems)
            }
            TypeKind::TypeParameter { .. } => "T".to_string(),
            _ => "any".to_string(),
        }
    }

    // ========================================================================
    // Type relationship checking
    // ========================================================================

    pub fn is_type_assignable_to(&mut self, source: TypeId, target: TypeId) -> bool {
        if source == target { return true; }

        // Check cache — O(1) for repeated checks & breaks cycles
        if let Some(&result) = self.assignability_cache.get(&(source, target)) {
            return result;
        }

        // Insert optimistic sentinel before recursing.
        // If a cycle re-enters this (source, target) pair, it returns `true`
        // (assumes assignable), which is the correct conservative answer for
        // recursive / circular types.
        self.assignability_cache.insert((source, target), true);

        let result = self.is_type_assignable_to_worker(source, target);

        // Update cache with actual result
        self.assignability_cache.insert((source, target), result);
        result
    }

    /// Core assignability logic.  Separated from the public entry point so that
    /// the cache look-up / cycle-sentinel lives in one place.
    fn is_type_assignable_to_worker(&mut self, source: TypeId, target: TypeId) -> bool {
        // Extract flags upfront (TypeFlags is Copy, no borrow held)
        let target_flags = self.type_table.get(target).flags;
        let source_flags = self.type_table.get(source).flags;

        if target_flags.contains(TypeFlags::ANY) || target_flags.contains(TypeFlags::UNKNOWN) {
            return true;
        }
        if source_flags.contains(TypeFlags::ANY) { return true; }
        if source_flags.contains(TypeFlags::NEVER) { return true; }

        // null/undefined assignability
        if !self.strict_null_checks
            && (source_flags.contains(TypeFlags::NULL) || source_flags.contains(TypeFlags::UNDEFINED)) {
                return true;
            }

        // Union target: source must be assignable to at least one constituent.
        // Clone the types vec to release the borrow before recursive calls.
        if let TypeKind::Union { types } = &self.type_table.get(target).kind {
            let target_types = types.clone();
            for t in target_types {
                if self.is_type_assignable_to(source, t) {
                    return true;
                }
            }
            return false;
        }

        // Union source: each constituent must be assignable to target
        if let TypeKind::Union { types } = &self.type_table.get(source).kind {
            let source_types = types.clone();
            return source_types.iter().all(|&t| self.is_type_assignable_to(t, target));
        }

        // Intersection source: any constituent assignable to target is sufficient
        if let TypeKind::Intersection { types } = &self.type_table.get(source).kind {
            let source_types = types.clone();
            for t in source_types {
                if self.is_type_assignable_to(t, target) { return true; }
            }
        }

        // Same primitive type
        if source_flags.intersects(TypeFlags::STRING_LIKE) && target_flags.intersects(TypeFlags::STRING_LIKE) { return true; }
        if source_flags.intersects(TypeFlags::NUMBER_LIKE) && target_flags.intersects(TypeFlags::NUMBER_LIKE) { return true; }
        if source_flags.intersects(TypeFlags::BOOLEAN_LIKE) && target_flags.intersects(TypeFlags::BOOLEAN_LIKE) { return true; }
        if source_flags.contains(TypeFlags::BIG_INT) && target_flags.contains(TypeFlags::BIG_INT) { return true; }
        if source_flags.contains(TypeFlags::ES_SYMBOL) && target_flags.contains(TypeFlags::ES_SYMBOL) { return true; }
        if source_flags.contains(TypeFlags::VOID) && target_flags.contains(TypeFlags::VOID) { return true; }

        // Structural type checking for object types.
        // Build a HashMap from source members for O(1) property lookup per
        // target member (previously O(n*m) via linear search).
        let member_pairs: Option<Vec<(TypeId, TypeId)>> = {
            let source_type = self.type_table.get(source);
            let target_type = self.type_table.get(target);
            match (&source_type.kind, &target_type.kind) {
                (
                    TypeKind::ObjectType { members: source_members, .. },
                    TypeKind::ObjectType { members: target_members, .. },
                ) => {
                    let source_map: HashMap<&str, TypeId> = source_members
                        .iter()
                        .map(|(name, tid)| (name.as_str(), *tid))
                        .collect();
                    let mut pairs = Vec::new();
                    for (target_name, target_prop_type) in target_members {
                        if let Some(&source_prop_type) = source_map.get(target_name.as_str()) {
                            pairs.push((source_prop_type, *target_prop_type));
                        } else {
                            // Missing property — not assignable
                            return false;
                        }
                    }
                    Some(pairs)
                }
                _ => None,
            }
        };

        if let Some(pairs) = member_pairs {
            for (source_prop, target_prop) in pairs {
                if !self.is_type_assignable_to(source_prop, target_prop) {
                    return false;
                }
            }
            return true;
        }

        false
    }

    // ========================================================================
    // Advanced type operations
    // ========================================================================

    /// For const declarations, narrow the inferred type to a literal type when possible.
    /// Since StringLiteral and NumericLiteral use InternedString (which requires the interner
    /// to resolve back to text), we narrow booleans directly and keep the widened type for
    /// strings/numbers. Full literal narrowing will require threading the interner through.
    fn narrow_to_literal(&mut self, expr: &Expression<'_>, inferred: TypeId) -> TypeId {
        match expr {
            Expression::TrueKeyword(_) => self.type_table.true_type,
            Expression::FalseKeyword(_) => self.type_table.false_type,
            Expression::NullKeyword(_) => self.type_table.null_type,
            _ => inferred,
        }
    }

    /// Extract member names from an object type.
    fn get_object_member_names(&self, type_id: TypeId) -> Vec<String> {
        let ty = self.type_table.get(type_id);
        match &ty.kind {
            TypeKind::ObjectType { members, .. } => {
                members.iter().map(|(name, _)| name.clone()).collect()
            }
            _ => vec![],
        }
    }

    /// Evaluate a conditional type when check and extends types are concrete.
    fn evaluate_conditional_type(&mut self, check: TypeId, extends: TypeId, true_type: TypeId, false_type: TypeId) -> TypeId {
        if self.is_type_assignable_to(check, extends) {
            true_type
        } else {
            false_type
        }
    }

    /// Resolve an indexed access type T[K] — look up property K in T.
    fn resolve_indexed_access(&self, object_type: TypeId, index_type: TypeId) -> TypeId {
        let obj = self.type_table.get(object_type);
        let idx = self.type_table.get(index_type);

        // If index is a string literal, look up the property
        if let TypeKind::StringLiteral { value, .. } = &idx.kind {
            if let TypeKind::ObjectType { members, .. } = &obj.kind {
                for (name, tid) in members {
                    if name == value {
                        return *tid;
                    }
                }
            }
        }

        // Check index signatures
        if let TypeKind::ObjectType { index_infos, .. } = &obj.kind {
            for info in index_infos {
                if self.type_table.get(info.key_type).flags.contains(self.type_table.get(index_type).flags) {
                    return info.type_id;
                }
            }
        }

        self.type_table.any_type
    }

    /// Create a literal string type.
    #[allow(dead_code)]
    fn create_string_literal_type(&mut self, value: String) -> TypeId {
        self.type_table.add_type(
            TypeFlags::STRING_LITERAL,
            TypeKind::StringLiteral { value, regular: false },
        )
    }

    /// Create a literal number type.
    #[allow(dead_code)]
    fn create_number_literal_type(&mut self, value: f64) -> TypeId {
        self.type_table.add_type(
            TypeFlags::NUMBER_LITERAL,
            TypeKind::NumberLiteral { value },
        )
    }

    /// Implement Partial<T> — make all properties optional (add undefined to each member type).
    fn create_partial_type(&mut self, type_id: TypeId) -> TypeId {
        let members = {
            let ty = self.type_table.get(type_id);
            if let TypeKind::ObjectType { members, .. } = &ty.kind {
                members.clone()
            } else {
                return type_id;
            }
        };
        let partial_members: Vec<(String, TypeId)> = members.iter().map(|(name, tid)| {
            let optional = self.create_union_type(vec![*tid, self.type_table.undefined_type]);
            (name.clone(), optional)
        }).collect();
        self.type_table.add_type(
            TypeFlags::OBJECT,
            TypeKind::ObjectType {
                object_flags: ObjectFlags::ANONYMOUS,
                members: partial_members,
                call_signatures: vec![],
                construct_signatures: vec![],
                index_infos: vec![],
            },
        )
    }

    /// Implement Required<T> — remove undefined from all member types.
    fn create_required_type(&mut self, type_id: TypeId) -> TypeId {
        let members = {
            let ty = self.type_table.get(type_id);
            if let TypeKind::ObjectType { members, .. } = &ty.kind {
                members.clone()
            } else {
                return type_id;
            }
        };
        let required_members: Vec<(String, TypeId)> = members.iter().map(|(name, tid)| {
            let non_null = self.get_non_nullable_type(*tid);
            (name.clone(), non_null)
        }).collect();
        self.type_table.add_type(
            TypeFlags::OBJECT,
            TypeKind::ObjectType {
                object_flags: ObjectFlags::ANONYMOUS,
                members: required_members,
                call_signatures: vec![],
                construct_signatures: vec![],
                index_infos: vec![],
            },
        )
    }

    /// Implement Readonly<T> — keep all members but mark object as immutable.
    fn create_readonly_type(&mut self, type_id: TypeId) -> TypeId {
        let ty = self.type_table.get(type_id);
        if let TypeKind::ObjectType { members, call_signatures, construct_signatures, index_infos, .. } = &ty.kind {
            let members = members.clone();
            let call_signatures = call_signatures.clone();
            let construct_signatures = construct_signatures.clone();
            let index_infos = index_infos.clone();
            self.type_table.add_type(
                TypeFlags::OBJECT,
                TypeKind::ObjectType {
                    object_flags: ObjectFlags::ANONYMOUS,
                    members,
                    call_signatures,
                    construct_signatures,
                    index_infos,
                },
            )
        } else {
            type_id
        }
    }

    /// Implement Pick<T, K> — select only the specified keys.
    fn create_pick_type(&mut self, type_id: TypeId, keys: &[TypeId]) -> TypeId {
        let members = {
            let ty = self.type_table.get(type_id);
            if let TypeKind::ObjectType { members, .. } = &ty.kind {
                members.clone()
            } else {
                return type_id;
            }
        };
        let key_names: Vec<String> = keys.iter().filter_map(|k| {
            if let TypeKind::StringLiteral { value, .. } = &self.type_table.get(*k).kind {
                Some(value.clone())
            } else {
                None
            }
        }).collect();
        let picked: Vec<(String, TypeId)> = members.iter()
            .filter(|(name, _)| key_names.contains(name))
            .cloned()
            .collect();
        self.type_table.add_type(
            TypeFlags::OBJECT,
            TypeKind::ObjectType {
                object_flags: ObjectFlags::ANONYMOUS,
                members: picked,
                call_signatures: vec![],
                construct_signatures: vec![],
                index_infos: vec![],
            },
        )
    }

    /// Implement Omit<T, K> — remove the specified keys.
    fn create_omit_type(&mut self, type_id: TypeId, keys: &[TypeId]) -> TypeId {
        let members = {
            let ty = self.type_table.get(type_id);
            if let TypeKind::ObjectType { members, .. } = &ty.kind {
                members.clone()
            } else {
                return type_id;
            }
        };
        let key_names: Vec<String> = keys.iter().filter_map(|k| {
            if let TypeKind::StringLiteral { value, .. } = &self.type_table.get(*k).kind {
                Some(value.clone())
            } else {
                None
            }
        }).collect();
        let omitted: Vec<(String, TypeId)> = members.iter()
            .filter(|(name, _)| !key_names.contains(name))
            .cloned()
            .collect();
        self.type_table.add_type(
            TypeFlags::OBJECT,
            TypeKind::ObjectType {
                object_flags: ObjectFlags::ANONYMOUS,
                members: omitted,
                call_signatures: vec![],
                construct_signatures: vec![],
                index_infos: vec![],
            },
        )
    }

    /// Implement ReturnType<T> — extract the return type of a function type.
    fn get_return_type_of(&self, type_id: TypeId) -> TypeId {
        let ty = self.type_table.get(type_id);
        if let TypeKind::ObjectType { call_signatures, .. } = &ty.kind {
            if let Some(sig) = call_signatures.first() {
                return sig.return_type;
            }
        }
        self.type_table.any_type
    }

    /// Implement Parameters<T> — extract the parameter types of a function type as a tuple.
    fn get_parameters_type_of(&mut self, type_id: TypeId) -> TypeId {
        let params = {
            let ty = self.type_table.get(type_id);
            if let TypeKind::ObjectType { call_signatures, .. } = &ty.kind {
                if let Some(sig) = call_signatures.first() {
                    sig.parameters.iter().map(|p| p.type_id).collect::<Vec<_>>()
                } else {
                    return self.type_table.any_type;
                }
            } else {
                return self.type_table.any_type;
            }
        };
        self.type_table.add_type(
            TypeFlags::OBJECT,
            TypeKind::Tuple {
                element_types: params,
                element_flags: vec![],
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rscript_binder::Binder;

    #[test]
    fn test_checker_creation() {
        let binder = Binder::new();
        let checker = Checker::new(binder);
        assert!(checker.diagnostics().is_empty());
    }

    #[test]
    fn test_primitive_assignability() {
        let binder = Binder::new();
        let mut checker = Checker::new(binder);
        let num = checker.type_table.number_type;
        let str_ = checker.type_table.string_type;
        assert!(checker.is_type_assignable_to(num, num));
        assert!(checker.is_type_assignable_to(str_, str_));
        assert!(!checker.is_type_assignable_to(num, str_));
    }

    #[test]
    fn test_any_assignability() {
        let binder = Binder::new();
        let mut checker = Checker::new(binder);
        let any = checker.type_table.any_type;
        let num = checker.type_table.number_type;
        assert!(checker.is_type_assignable_to(any, num));
        assert!(checker.is_type_assignable_to(num, any));
    }

    #[test]
    fn test_never_assignability() {
        let binder = Binder::new();
        let mut checker = Checker::new(binder);
        let never = checker.type_table.never_type;
        let num = checker.type_table.number_type;
        let str_ = checker.type_table.string_type;
        assert!(checker.is_type_assignable_to(never, num));
        assert!(checker.is_type_assignable_to(never, str_));
    }

    #[test]
    fn test_union_assignability() {
        let binder = Binder::new();
        let mut checker = Checker::new(binder);
        let str_ = checker.type_table.string_type;
        let num = checker.type_table.number_type;
        let bool_ = checker.type_table.boolean_type;
        let union = checker.create_union_type(vec![str_, num]);
        assert!(checker.is_type_assignable_to(str_, union));
        assert!(checker.is_type_assignable_to(num, union));
        assert!(!checker.is_type_assignable_to(bool_, union));
    }

    #[test]
    fn test_type_to_string() {
        let binder = Binder::new();
        let checker = Checker::new(binder);
        assert_eq!(checker.type_to_string(checker.type_table.string_type), "string");
        assert_eq!(checker.type_to_string(checker.type_table.number_type), "number");
        assert_eq!(checker.type_to_string(checker.type_table.boolean_type), "boolean");
        assert_eq!(checker.type_to_string(checker.type_table.void_type), "void");
        assert_eq!(checker.type_to_string(checker.type_table.any_type), "any");
    }
}
