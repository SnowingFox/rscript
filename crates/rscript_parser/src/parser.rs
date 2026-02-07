//! The TypeScript parser implementation.
//!
//! This is a recursive descent parser that faithfully ports TypeScript's
//! parser.ts behavior. It consumes tokens from the scanner and builds
//! an arena-allocated AST.

use bumpalo::Bump;
use rscript_ast::node::*;
use rscript_ast::syntax_kind::SyntaxKind;
use rscript_ast::types::*;
use rscript_core::intern::InternedString;
use rscript_diagnostics::DiagnosticCollection;
use rscript_scanner::Scanner;

use crate::precedence::{get_binary_operator_precedence, OperatorPrecedence};

/// Maximum recursion depth to prevent stack overflow on deeply nested input.
const MAX_RECURSION_DEPTH: u32 = 200;

/// Allocate a Vec into the arena as a slice.
///
/// Uses ManuallyDrop to prevent double-free on panic inside alloc_slice_fill_with.
fn alloc_vec_in<T>(arena: &Bump, vec: Vec<T>) -> &[T] {
    if vec.is_empty() {
        return &[];
    }
    let mut vec = std::mem::ManuallyDrop::new(vec);
    let len = vec.len();
    let ptr = vec.as_ptr();
    let slice = arena.alloc_slice_fill_with(len, |i| {
        // SAFETY: i < len, and each element is read exactly once. ManuallyDrop
        // prevents the Vec destructor from running, so elements won't be
        // double-freed even if this closure panics partway through.
        unsafe { std::ptr::read(ptr.add(i)) }
    });
    // All elements have been moved out; set len to 0 so that if ManuallyDrop
    // is ever manually dropped, it won't try to drop moved-from elements.
    unsafe { vec.set_len(0); }
    slice
}

/// The parser produces a SourceFile AST from TypeScript source text.
pub struct Parser<'a> {
    arena: &'a Bump,
    scanner: Scanner,
    file_name: String,
    source_text: String,
    context_flags: NodeFlags,
    diagnostics: DiagnosticCollection,
    /// Tracks recursion depth to prevent stack overflow on deeply nested input.
    recursion_depth: u32,
}

impl<'a> Parser<'a> {
    pub fn new(arena: &'a Bump, file_name: &str, source_text: &str) -> Self {
        let scanner = Scanner::new(source_text);
        Self {
            arena,
            scanner,
            file_name: file_name.to_string(),
            source_text: source_text.to_string(),
            context_flags: NodeFlags::NONE,
            diagnostics: DiagnosticCollection::new(),
            recursion_depth: 0,
        }
    }

    pub fn parse_source_file(mut self) -> SourceFile<'a> {
        self.scanner.skip_shebang();
        self.next_token();

        let pos = 0u32;
        let statements = self.parse_statements();
        let end = self.source_text.len() as u32;
        let end_of_file_token = Token::new(SyntaxKind::EndOfFileToken, end, end);

        let is_tsx = self.file_name.ends_with(".tsx");
        let is_jsx = self.file_name.ends_with(".jsx");
        let is_ts = self.file_name.ends_with(".ts") || is_tsx;
        let is_dts = self.file_name.ends_with(".d.ts");

        let language_variant = if is_tsx || is_jsx { LanguageVariant::JSX } else { LanguageVariant::Standard };
        let script_kind = if is_tsx { ScriptKind::TSX } else if is_jsx { ScriptKind::JSX } else if is_ts { ScriptKind::TS } else { ScriptKind::JS };

        SourceFile {
            data: NodeData::new(SyntaxKind::SourceFile, pos, end),
            statements,
            end_of_file_token,
            file_name: self.file_name,
            text: self.source_text,
            language_variant,
            script_kind,
            is_declaration_file: is_dts,
            has_no_default_lib: false,
        }
    }

    pub fn take_diagnostics(mut self) -> DiagnosticCollection {
        let scanner_diags = self.scanner.take_diagnostics();
        self.diagnostics.extend(scanner_diags);
        self.diagnostics
    }

    // ========================================================================
    // Token management
    // ========================================================================

    #[inline]
    fn current_token(&self) -> SyntaxKind { self.scanner.token() }

    #[inline]
    fn next_token(&mut self) -> SyntaxKind { self.scanner.scan() }

    #[inline]
    fn token_pos(&self) -> u32 { self.scanner.token_start() as u32 }

    #[inline]
    fn token_end(&self) -> u32 { self.scanner.token_end() as u32 }

    #[inline]
    fn token_value(&self) -> &str { self.scanner.token_value() }

    fn expect_token(&mut self, kind: SyntaxKind) -> Token {
        let pos = self.token_pos();
        let end = self.token_end();
        if self.current_token() == kind {
            let token = Token::new(kind, pos, end);
            self.next_token();
            token
        } else {
            let text = kind.punctuation_text().or_else(|| kind.keyword_text()).unwrap_or("token");
            self.diagnostics.add(rscript_diagnostics::Diagnostic::new(
                &rscript_diagnostics::messages::_0_EXPECTED,
                &[text],
            ));
            Token::new(kind, pos, pos)
        }
    }

    fn optional_token(&mut self, kind: SyntaxKind) -> Option<Token> {
        if self.current_token() == kind {
            let pos = self.token_pos();
            let end = self.token_end();
            self.next_token();
            Some(Token::new(kind, pos, end))
        } else {
            None
        }
    }

    fn parse_expected_semicolon(&mut self) {
        if self.current_token() == SyntaxKind::SemicolonToken {
            self.next_token();
        }
        // ASI: don't error if line break, close brace, or EOF
    }

    fn error(&mut self, msg: &rscript_diagnostics::DiagnosticMessage, args: &[&str]) {
        let pos = self.token_pos();
        let end = self.token_end();
        let span = rscript_core::text::TextSpan::from_bounds(pos, end);
        self.diagnostics.add(rscript_diagnostics::Diagnostic::with_location(
            self.file_name.clone(),
            span,
            msg,
            args,
        ));
    }

    /// Check if identifier text matches (without interning - using scanner token_value).
    fn is_identifier_text(&self, text: &str) -> bool {
        self.current_token() == SyntaxKind::Identifier && self.token_value() == text
    }

    // ========================================================================
    // Statement parsing
    // ========================================================================

    fn parse_statements(&mut self) -> &'a [Statement<'a>] {
        let mut statements = Vec::new();
        while self.current_token() != SyntaxKind::EndOfFileToken
            && self.current_token() != SyntaxKind::CloseBraceToken
        {
            let saved_pos = self.scanner.token_start();
            let stmt = self.parse_statement();
            statements.push(stmt);

            // Error recovery: if the parser hasn't advanced past the same position,
            // skip forward to the next statement-starting token to avoid infinite loops.
            if self.scanner.token_start() == saved_pos {
                self.skip_to_next_statement();
            }
        }
        alloc_vec_in(self.arena, statements)
    }

    /// Error recovery: skip tokens until we find one that can start a new statement.
    /// This prevents cascading errors from a single parse failure.
    fn skip_to_next_statement(&mut self) {
        while self.current_token() != SyntaxKind::EndOfFileToken {
            match self.current_token() {
                // Tokens that can start a new statement
                SyntaxKind::VarKeyword
                | SyntaxKind::LetKeyword
                | SyntaxKind::ConstKeyword
                | SyntaxKind::UsingKeyword
                | SyntaxKind::FunctionKeyword
                | SyntaxKind::ClassKeyword
                | SyntaxKind::InterfaceKeyword
                | SyntaxKind::EnumKeyword
                | SyntaxKind::TypeKeyword
                | SyntaxKind::IfKeyword
                | SyntaxKind::ForKeyword
                | SyntaxKind::WhileKeyword
                | SyntaxKind::DoKeyword
                | SyntaxKind::SwitchKeyword
                | SyntaxKind::ReturnKeyword
                | SyntaxKind::ThrowKeyword
                | SyntaxKind::TryKeyword
                | SyntaxKind::BreakKeyword
                | SyntaxKind::ContinueKeyword
                | SyntaxKind::ExportKeyword
                | SyntaxKind::ImportKeyword
                | SyntaxKind::CloseBraceToken => return,
                _ => {
                    self.next_token();
                }
            }
        }
    }

    fn parse_statement(&mut self) -> Statement<'a> {
        match self.current_token() {
            SyntaxKind::SemicolonToken => {
                let pos = self.token_pos();
                let end = self.token_end();
                self.next_token();
                Statement::EmptyStatement(NodeData::new(SyntaxKind::EmptyStatement, pos, end))
            }
            SyntaxKind::OpenBraceToken => Statement::Block(self.parse_block()),
            SyntaxKind::ConstKeyword if self.is_const_enum() => self.parse_const_enum_declaration(),
            SyntaxKind::AwaitKeyword if self.is_await_using() => self.parse_await_using_statement(),
            SyntaxKind::VarKeyword | SyntaxKind::LetKeyword | SyntaxKind::ConstKeyword | SyntaxKind::UsingKeyword => self.parse_variable_statement(),
            SyntaxKind::FunctionKeyword => self.parse_function_declaration(false),
            SyntaxKind::ClassKeyword => self.parse_class_declaration(false),
            SyntaxKind::IfKeyword => self.parse_if_statement(),
            SyntaxKind::ReturnKeyword => self.parse_return_statement(),
            SyntaxKind::WhileKeyword => self.parse_while_statement(),
            SyntaxKind::ForKeyword => self.parse_for_statement(),
            SyntaxKind::DoKeyword => self.parse_do_statement(),
            SyntaxKind::DebuggerKeyword => {
                let pos = self.token_pos();
                self.next_token();
                let end = self.token_end();
                self.parse_expected_semicolon();
                Statement::DebuggerStatement(NodeData::new(SyntaxKind::DebuggerStatement, pos, end))
            }
            SyntaxKind::InterfaceKeyword => self.parse_interface_declaration(),
            SyntaxKind::TypeKeyword => self.parse_type_alias_declaration(),
            SyntaxKind::EnumKeyword => self.parse_enum_declaration(),
            SyntaxKind::NamespaceKeyword => self.parse_module_declaration(),
            SyntaxKind::ExportKeyword => self.parse_export_declaration_or_assignment(),
            SyntaxKind::ImportKeyword => self.parse_import_declaration(),
            SyntaxKind::ThrowKeyword => self.parse_throw_statement(),
            SyntaxKind::TryKeyword => self.parse_try_statement(),
            SyntaxKind::BreakKeyword => self.parse_break_statement(),
            SyntaxKind::ContinueKeyword => self.parse_continue_statement(),
            SyntaxKind::SwitchKeyword => self.parse_switch_statement(),
            SyntaxKind::WithKeyword => self.parse_with_statement(),
            SyntaxKind::AbstractKeyword if self.is_start_of_declaration() => self.parse_declaration(),
            SyntaxKind::AsyncKeyword if self.is_start_of_declaration() => self.parse_declaration(),
            SyntaxKind::DeclareKeyword if self.is_start_of_declaration() => self.parse_declaration(),
            SyntaxKind::AtToken => self.parse_declaration(),
            SyntaxKind::Identifier if self.is_labeled_statement() => self.parse_labeled_statement(),
            SyntaxKind::Identifier if self.is_identifier_text("namespace") || self.is_identifier_text("module") => self.parse_module_declaration(),
            _ => self.parse_expression_statement(),
        }
    }

    fn is_start_of_declaration(&self) -> bool {
        // Simplified: check if next token after modifier starts a declaration
        true
    }

    /// Look ahead: `const enum` is a const enum declaration, not a variable.
    fn is_const_enum(&mut self) -> bool {
        let saved = self.scanner.save_state();
        let next = self.scanner.scan();
        let result = next == SyntaxKind::EnumKeyword;
        self.scanner.restore_state(saved);
        result
    }

    /// Look ahead: `await using` is an await-using variable declaration (TS 5.2+).
    fn is_await_using(&mut self) -> bool {
        let saved = self.scanner.save_state();
        let next = self.scanner.scan();
        let result = next == SyntaxKind::UsingKeyword;
        self.scanner.restore_state(saved);
        result
    }

    /// Parse `await using x = ...;` — skip `await`, handle as AWAIT_USING declaration.
    fn parse_await_using_statement(&mut self) -> Statement<'a> {
        let pos = self.token_pos();
        self.next_token(); // skip 'await'
        // Now at 'using'
        let mut decl_list = self.parse_variable_declaration_list();
        decl_list.data.flags = NodeFlags::AWAIT_USING;
        self.parse_expected_semicolon();
        let end = self.token_end();
        Statement::VariableStatement(VariableStatement {
            data: NodeData::new(SyntaxKind::VariableStatement, pos, end),
            declaration_list: decl_list,
        })
    }

    /// Parse `const enum Foo { ... }` — skip `const` and delegate to enum parser.
    fn parse_const_enum_declaration(&mut self) -> Statement<'a> {
        self.next_token(); // skip 'const'
        self.parse_enum_declaration()
    }

    fn is_labeled_statement(&mut self) -> bool {
        // Look ahead: identifier followed by colon
        if self.current_token() != SyntaxKind::Identifier {
            return false;
        }
        let saved = self.scanner.save_state();
        let next = self.scanner.scan();
        let result = next == SyntaxKind::ColonToken;
        self.scanner.restore_state(saved);
        result
    }

    fn parse_declaration(&mut self) -> Statement<'a> {
        // Handle decorators and modifiers before declarations
        let pos = self.token_pos();

        // Skip decorators
        while self.current_token() == SyntaxKind::AtToken {
            self.next_token();
            // Parse decorator expression
            self.parse_left_hand_side_expression();
        }

        // Skip modifiers: declare, abstract, async, export, default
        while matches!(
            self.current_token(),
            SyntaxKind::DeclareKeyword
                | SyntaxKind::AbstractKeyword
                | SyntaxKind::AsyncKeyword
                | SyntaxKind::ExportKeyword
                | SyntaxKind::DefaultKeyword
        ) {
            self.next_token();
        }

        match self.current_token() {
            SyntaxKind::FunctionKeyword => self.parse_function_declaration(false),
            SyntaxKind::ClassKeyword => self.parse_class_declaration(false),
            SyntaxKind::InterfaceKeyword => self.parse_interface_declaration(),
            SyntaxKind::EnumKeyword => self.parse_enum_declaration(),
            SyntaxKind::TypeKeyword => self.parse_type_alias_declaration(),
            SyntaxKind::VarKeyword | SyntaxKind::LetKeyword | SyntaxKind::ConstKeyword | SyntaxKind::UsingKeyword => self.parse_variable_statement(),
            _ => {
                let end = self.token_end();
                self.next_token();
                Statement::MissingDeclaration(NodeData::new(SyntaxKind::MissingDeclaration, pos, end))
            }
        }
    }

    fn parse_block(&mut self) -> Block<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::OpenBraceToken);
        let statements = self.parse_statements();
        let end = self.token_end();
        self.expect_token(SyntaxKind::CloseBraceToken);
        Block {
            data: NodeData::new(SyntaxKind::Block, pos, end),
            statements,
            multi_line: true,
        }
    }

    fn parse_variable_statement(&mut self) -> Statement<'a> {
        let pos = self.token_pos();
        let decl_list = self.parse_variable_declaration_list();
        let end = self.token_end();
        self.parse_expected_semicolon();
        Statement::VariableStatement(VariableStatement {
            data: NodeData::new(SyntaxKind::VariableStatement, pos, end),
            declaration_list: decl_list,
        })
    }

    fn parse_variable_declaration_list(&mut self) -> VariableDeclarationList<'a> {
        let pos = self.token_pos();
        let mut flags = NodeFlags::NONE;
        match self.current_token() {
            SyntaxKind::LetKeyword => flags |= NodeFlags::LET,
            SyntaxKind::ConstKeyword => flags |= NodeFlags::CONST,
            SyntaxKind::UsingKeyword => flags |= NodeFlags::USING,
            _ => {}
        }
        self.next_token();

        let mut declarations = Vec::new();
        loop {
            let decl = self.parse_variable_declaration();
            declarations.push(decl);
            if self.optional_token(SyntaxKind::CommaToken).is_none() { break; }
        }
        let end = self.token_end();
        let decl_slice = alloc_vec_in(self.arena, declarations);
        let mut data = NodeData::new(SyntaxKind::VariableDeclarationList, pos, end);
        data.flags = flags;
        VariableDeclarationList { data, declarations: decl_slice }
    }

    fn parse_variable_declaration(&mut self) -> VariableDeclaration<'a> {
        let pos = self.token_pos();
        let name = self.parse_binding_name();
        let exclamation_token = self.optional_token(SyntaxKind::ExclamationToken);
        let type_annotation = if self.optional_token(SyntaxKind::ColonToken).is_some() {
            Some(self.parse_type_and_alloc())
        } else { None };
        let initializer = if self.optional_token(SyntaxKind::EqualsToken).is_some() {
            Some(self.parse_assignment_expression_and_alloc())
        } else { None };
        let end = self.token_end();
        VariableDeclaration {
            data: NodeData::new(SyntaxKind::VariableDeclaration, pos, end),
            name, exclamation_token, type_annotation, initializer,
        }
    }

    // ========================================================================
    // Binding patterns (destructuring)
    // ========================================================================

    fn parse_binding_name(&mut self) -> BindingName<'a> {
        match self.current_token() {
            SyntaxKind::OpenBraceToken => {
                let pattern = self.parse_object_binding_pattern();
                BindingName::ObjectBindingPattern(self.arena.alloc(pattern))
            }
            SyntaxKind::OpenBracketToken => {
                let pattern = self.parse_array_binding_pattern();
                BindingName::ArrayBindingPattern(self.arena.alloc(pattern))
            }
            _ => BindingName::Identifier(self.parse_identifier()),
        }
    }

    fn parse_object_binding_pattern(&mut self) -> ObjectBindingPattern<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::OpenBraceToken);
        let mut elements = Vec::new();
        while self.current_token() != SyntaxKind::CloseBraceToken
            && self.current_token() != SyntaxKind::EndOfFileToken
        {
            elements.push(self.parse_binding_element());
            if self.optional_token(SyntaxKind::CommaToken).is_none() { break; }
        }
        let end = self.token_end();
        self.expect_token(SyntaxKind::CloseBraceToken);
        ObjectBindingPattern {
            data: NodeData::new(SyntaxKind::ObjectBindingPattern, pos, end),
            elements: alloc_vec_in(self.arena, elements),
        }
    }

    fn parse_array_binding_pattern(&mut self) -> ArrayBindingPattern<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::OpenBracketToken);
        let mut elements = Vec::new();
        while self.current_token() != SyntaxKind::CloseBracketToken
            && self.current_token() != SyntaxKind::EndOfFileToken
        {
            if self.current_token() == SyntaxKind::CommaToken {
                let epos = self.token_pos();
                let eend = self.token_end();
                elements.push(ArrayBindingElement::OmittedExpression(
                    NodeData::new(SyntaxKind::OmittedExpression, epos, eend),
                ));
            } else {
                elements.push(ArrayBindingElement::BindingElement(self.parse_binding_element()));
            }
            if self.optional_token(SyntaxKind::CommaToken).is_none() { break; }
        }
        let end = self.token_end();
        self.expect_token(SyntaxKind::CloseBracketToken);
        ArrayBindingPattern {
            data: NodeData::new(SyntaxKind::ArrayBindingPattern, pos, end),
            elements: alloc_vec_in(self.arena, elements),
        }
    }

    fn parse_binding_element(&mut self) -> BindingElement<'a> {
        let pos = self.token_pos();
        let dot_dot_dot_token = self.optional_token(SyntaxKind::DotDotDotToken);

        let name_or_property = self.parse_binding_name();

        // Check for property_name: binding_name pattern
        let (property_name, name) = if self.optional_token(SyntaxKind::ColonToken).is_some() {
            // What we parsed was the property name, now parse the actual binding
            let prop = match name_or_property {
                BindingName::Identifier(id) => Some(PropertyName::Identifier(id)),
                _ => None,
            };
            let actual_name = self.parse_binding_name();
            (prop, actual_name)
        } else {
            (None, name_or_property)
        };

        let initializer = if self.optional_token(SyntaxKind::EqualsToken).is_some() {
            Some(self.parse_assignment_expression_and_alloc())
        } else { None };

        let end = self.token_end();
        BindingElement {
            data: NodeData::new(SyntaxKind::BindingElement, pos, end),
            dot_dot_dot_token, property_name, name, initializer,
        }
    }

    fn parse_identifier(&mut self) -> Identifier {
        let pos = self.token_pos();
        let end = self.token_end();
        let text = InternedString::dummy();
        let text_name = self.scanner.token_value().to_string();
        let original_keyword_kind = if self.current_token().is_keyword() {
            Some(self.current_token())
        } else { None };

        if self.current_token() == SyntaxKind::Identifier || self.current_token().is_keyword() {
            self.next_token();
        } else {
            self.error(&rscript_diagnostics::messages::IDENTIFIER_EXPECTED, &[]);
            // CRITICAL: Always advance to prevent infinite loops in callers.
            // Without this, any loop calling parse_identifier on an unexpected
            // token would spin forever since the token position never changes.
            if self.current_token() != SyntaxKind::EndOfFileToken {
                self.next_token();
            }
        }
        Identifier { data: NodeData::new(SyntaxKind::Identifier, pos, end), text, text_name, original_keyword_kind }
    }

    // ========================================================================
    // Statement parsing continued
    // ========================================================================

    fn parse_expression_statement(&mut self) -> Statement<'a> {
        let pos = self.token_pos();
        let expr = self.parse_expression_and_alloc();
        let end = self.token_end();
        self.parse_expected_semicolon();
        Statement::ExpressionStatement(ExpressionStatement {
            data: NodeData::new(SyntaxKind::ExpressionStatement, pos, end),
            expression: expr,
        })
    }

    fn parse_if_statement(&mut self) -> Statement<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::IfKeyword);
        self.expect_token(SyntaxKind::OpenParenToken);
        let expression = self.parse_expression_and_alloc();
        self.expect_token(SyntaxKind::CloseParenToken);
        let then_statement = self.arena.alloc(self.parse_statement());
        let else_statement = if self.optional_token(SyntaxKind::ElseKeyword).is_some() {
            Some(&*self.arena.alloc(self.parse_statement()))
        } else { None };
        let end = self.token_end();
        Statement::IfStatement(IfStatement {
            data: NodeData::new(SyntaxKind::IfStatement, pos, end),
            expression, then_statement, else_statement,
        })
    }

    fn parse_return_statement(&mut self) -> Statement<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::ReturnKeyword);
        let expression = if self.current_token() != SyntaxKind::SemicolonToken
            && self.current_token() != SyntaxKind::CloseBraceToken
            && self.current_token() != SyntaxKind::EndOfFileToken
            && !self.scanner.has_preceding_line_break()
        {
            Some(self.parse_expression_and_alloc())
        } else { None };
        let end = self.token_end();
        self.parse_expected_semicolon();
        Statement::ReturnStatement(ReturnStatement {
            data: NodeData::new(SyntaxKind::ReturnStatement, pos, end), expression,
        })
    }

    fn parse_while_statement(&mut self) -> Statement<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::WhileKeyword);
        self.expect_token(SyntaxKind::OpenParenToken);
        let expression = self.parse_expression_and_alloc();
        self.expect_token(SyntaxKind::CloseParenToken);
        let statement = self.arena.alloc(self.parse_statement());
        let end = self.token_end();
        Statement::WhileStatement(WhileStatement {
            data: NodeData::new(SyntaxKind::WhileStatement, pos, end),
            expression, statement,
        })
    }

    fn parse_for_statement(&mut self) -> Statement<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::ForKeyword);

        // Check for `for await`
        let await_modifier = self.optional_token(SyntaxKind::AwaitKeyword);
        self.expect_token(SyntaxKind::OpenParenToken);

        let initializer = if self.current_token() != SyntaxKind::SemicolonToken {
            if matches!(self.current_token(), SyntaxKind::VarKeyword | SyntaxKind::LetKeyword | SyntaxKind::ConstKeyword | SyntaxKind::UsingKeyword) {
                Some(ForInitializer::VariableDeclarationList(self.parse_variable_declaration_list()))
            } else {
                Some(ForInitializer::Expression(self.parse_expression_and_alloc()))
            }
        } else { None };

        // Check for `in` or `of`
        if self.is_identifier_text("of") || self.current_token() == SyntaxKind::InKeyword {
            let is_for_of = self.is_identifier_text("of");
            self.next_token();
            let expression = self.parse_expression_and_alloc();
            self.expect_token(SyntaxKind::CloseParenToken);
            let statement = self.arena.alloc(self.parse_statement());
            let end = self.token_end();

            if is_for_of {
                return Statement::ForOfStatement(ForOfStatement {
                    data: NodeData::new(SyntaxKind::ForOfStatement, pos, end),
                    await_modifier, initializer: initializer.unwrap_or(ForInitializer::Expression(self.arena.alloc(
                        Expression::Identifier(Identifier { data: NodeData::new(SyntaxKind::Identifier, pos, pos), text: InternedString::dummy(), text_name: String::new(), original_keyword_kind: None })
                    ))),
                    expression, statement,
                });
            } else {
                return Statement::ForInStatement(ForInStatement {
                    data: NodeData::new(SyntaxKind::ForInStatement, pos, end),
                    initializer: initializer.unwrap_or(ForInitializer::Expression(self.arena.alloc(
                        Expression::Identifier(Identifier { data: NodeData::new(SyntaxKind::Identifier, pos, pos), text: InternedString::dummy(), text_name: String::new(), original_keyword_kind: None })
                    ))),
                    expression, statement,
                });
            }
        }

        // Regular for statement
        self.expect_token(SyntaxKind::SemicolonToken);
        let condition = if self.current_token() != SyntaxKind::SemicolonToken {
            Some(self.parse_expression_and_alloc())
        } else { None };
        self.expect_token(SyntaxKind::SemicolonToken);
        let incrementor = if self.current_token() != SyntaxKind::CloseParenToken {
            Some(self.parse_expression_and_alloc())
        } else { None };
        self.expect_token(SyntaxKind::CloseParenToken);
        let statement = self.arena.alloc(self.parse_statement());
        let end = self.token_end();

        Statement::ForStatement(ForStatement {
            data: NodeData::new(SyntaxKind::ForStatement, pos, end),
            initializer, condition, incrementor, statement,
        })
    }

    fn parse_do_statement(&mut self) -> Statement<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::DoKeyword);
        let statement = self.arena.alloc(self.parse_statement());
        self.expect_token(SyntaxKind::WhileKeyword);
        self.expect_token(SyntaxKind::OpenParenToken);
        let expression = self.parse_expression_and_alloc();
        self.expect_token(SyntaxKind::CloseParenToken);
        let end = self.token_end();
        self.parse_expected_semicolon();
        Statement::DoStatement(DoStatement {
            data: NodeData::new(SyntaxKind::DoStatement, pos, end),
            statement, expression,
        })
    }

    fn parse_throw_statement(&mut self) -> Statement<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::ThrowKeyword);
        // ECMAScript spec: no line terminator allowed between `throw` and expression.
        // A line break here means ASI inserts a semicolon, making `throw;` which is illegal.
        if self.scanner.has_preceding_line_break() {
            self.error(&rscript_diagnostics::messages::EXPRESSION_EXPECTED, &[]);
        }
        let expression = self.parse_expression_and_alloc();
        let end = self.token_end();
        self.parse_expected_semicolon();
        Statement::ThrowStatement(ThrowStatement {
            data: NodeData::new(SyntaxKind::ThrowStatement, pos, end), expression,
        })
    }

    fn parse_try_statement(&mut self) -> Statement<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::TryKeyword);
        let try_block = self.parse_block();
        let catch_clause = if self.current_token() == SyntaxKind::CatchKeyword {
            let catch_pos = self.token_pos();
            self.next_token();
            let variable_declaration = if self.optional_token(SyntaxKind::OpenParenToken).is_some() {
                let decl = self.parse_variable_declaration();
                self.expect_token(SyntaxKind::CloseParenToken);
                Some(decl)
            } else { None };
            let block = self.parse_block();
            let catch_end = self.token_end();
            Some(CatchClause { data: NodeData::new(SyntaxKind::CatchClause, catch_pos, catch_end), variable_declaration, block })
        } else { None };
        let finally_block = if self.optional_token(SyntaxKind::FinallyKeyword).is_some() { Some(self.parse_block()) } else { None };
        let end = self.token_end();
        Statement::TryStatement(TryStatement {
            data: NodeData::new(SyntaxKind::TryStatement, pos, end),
            try_block, catch_clause, finally_block,
        })
    }

    fn parse_break_statement(&mut self) -> Statement<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::BreakKeyword);
        let label = if self.current_token() == SyntaxKind::Identifier && !self.scanner.has_preceding_line_break() {
            Some(self.parse_identifier())
        } else { None };
        let end = self.token_end();
        self.parse_expected_semicolon();
        Statement::BreakStatement(BreakStatement { data: NodeData::new(SyntaxKind::BreakStatement, pos, end), label })
    }

    fn parse_continue_statement(&mut self) -> Statement<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::ContinueKeyword);
        let label = if self.current_token() == SyntaxKind::Identifier && !self.scanner.has_preceding_line_break() {
            Some(self.parse_identifier())
        } else { None };
        let end = self.token_end();
        self.parse_expected_semicolon();
        Statement::ContinueStatement(ContinueStatement { data: NodeData::new(SyntaxKind::ContinueStatement, pos, end), label })
    }

    fn parse_switch_statement(&mut self) -> Statement<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::SwitchKeyword);
        self.expect_token(SyntaxKind::OpenParenToken);
        let expression = self.parse_expression_and_alloc();
        self.expect_token(SyntaxKind::CloseParenToken);

        let case_pos = self.token_pos();
        self.expect_token(SyntaxKind::OpenBraceToken);
        let mut clauses = Vec::new();
        while self.current_token() != SyntaxKind::CloseBraceToken && self.current_token() != SyntaxKind::EndOfFileToken {
            if self.current_token() == SyntaxKind::CaseKeyword {
                let cpos = self.token_pos();
                self.next_token();
                let case_expr = self.parse_expression_and_alloc();
                self.expect_token(SyntaxKind::ColonToken);
                let mut stmts = Vec::new();
                while self.current_token() != SyntaxKind::CaseKeyword
                    && self.current_token() != SyntaxKind::DefaultKeyword
                    && self.current_token() != SyntaxKind::CloseBraceToken
                    && self.current_token() != SyntaxKind::EndOfFileToken
                {
                    stmts.push(self.parse_statement());
                }
                let cend = self.token_end();
                clauses.push(CaseOrDefaultClause::CaseClause(CaseClause {
                    data: NodeData::new(SyntaxKind::CaseClause, cpos, cend),
                    expression: case_expr,
                    statements: alloc_vec_in(self.arena, stmts),
                }));
            } else if self.current_token() == SyntaxKind::DefaultKeyword {
                let dpos = self.token_pos();
                self.next_token();
                self.expect_token(SyntaxKind::ColonToken);
                let mut stmts = Vec::new();
                while self.current_token() != SyntaxKind::CaseKeyword
                    && self.current_token() != SyntaxKind::DefaultKeyword
                    && self.current_token() != SyntaxKind::CloseBraceToken
                    && self.current_token() != SyntaxKind::EndOfFileToken
                {
                    stmts.push(self.parse_statement());
                }
                let dend = self.token_end();
                clauses.push(CaseOrDefaultClause::DefaultClause(DefaultClause {
                    data: NodeData::new(SyntaxKind::DefaultClause, dpos, dend),
                    statements: alloc_vec_in(self.arena, stmts),
                }));
            } else {
                self.next_token(); // error recovery
            }
        }
        let case_end = self.token_end();
        self.expect_token(SyntaxKind::CloseBraceToken);
        let end = self.token_end();

        Statement::SwitchStatement(SwitchStatement {
            data: NodeData::new(SyntaxKind::SwitchStatement, pos, end),
            expression,
            case_block: CaseBlock { data: NodeData::new(SyntaxKind::CaseBlock, case_pos, case_end), clauses: alloc_vec_in(self.arena, clauses) },
        })
    }

    fn parse_with_statement(&mut self) -> Statement<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::WithKeyword);
        self.expect_token(SyntaxKind::OpenParenToken);
        let expression = self.parse_expression_and_alloc();
        self.expect_token(SyntaxKind::CloseParenToken);
        let statement = self.arena.alloc(self.parse_statement());
        let end = self.token_end();
        Statement::WithStatement(WithStatement {
            data: NodeData::new(SyntaxKind::WithStatement, pos, end),
            expression, statement,
        })
    }

    fn parse_labeled_statement(&mut self) -> Statement<'a> {
        let pos = self.token_pos();
        let label = self.parse_identifier();
        self.expect_token(SyntaxKind::ColonToken);
        let statement = self.arena.alloc(self.parse_statement());
        let end = self.token_end();
        Statement::LabeledStatement(LabeledStatement {
            data: NodeData::new(SyntaxKind::LabeledStatement, pos, end),
            label, statement,
        })
    }

    // ========================================================================
    // Function and Class declarations
    // ========================================================================

    fn parse_function_declaration(&mut self, _is_async: bool) -> Statement<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::FunctionKeyword);
        let asterisk_token = self.optional_token(SyntaxKind::AsteriskToken);
        let name = if self.current_token() == SyntaxKind::Identifier || self.current_token().is_keyword() {
            Some(self.parse_identifier())
        } else { None };

        let type_parameters = self.try_parse_type_parameters();
        let (parameters, return_type) = self.parse_parameter_list_and_return_type();

        let body = if self.current_token() == SyntaxKind::OpenBraceToken {
            Some(self.parse_block())
        } else {
            self.parse_expected_semicolon();
            None
        };
        let end = self.token_end();
        Statement::FunctionDeclaration(FunctionDeclaration {
            data: NodeData::new(SyntaxKind::FunctionDeclaration, pos, end),
            name, asterisk_token, type_parameters, parameters, return_type, body,
        })
    }

    fn parse_class_declaration(&mut self, _is_abstract: bool) -> Statement<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::ClassKeyword);
        let name = if self.current_token() == SyntaxKind::Identifier || self.current_token().is_keyword() {
            Some(self.parse_identifier())
        } else { None };
        let type_parameters = self.try_parse_type_parameters();
        let heritage_clauses = self.parse_heritage_clauses();
        let members = self.parse_class_members();
        let end = self.token_end();
        Statement::ClassDeclaration(ClassDeclaration {
            data: NodeData::new(SyntaxKind::ClassDeclaration, pos, end),
            name, type_parameters, heritage_clauses, members,
        })
    }

    fn parse_heritage_clauses(&mut self) -> Option<&'a [HeritageClause<'a>]> {
        if self.current_token() != SyntaxKind::ExtendsKeyword && self.current_token() != SyntaxKind::ImplementsKeyword {
            return None;
        }
        let mut clauses = Vec::new();
        while self.current_token() == SyntaxKind::ExtendsKeyword || self.current_token() == SyntaxKind::ImplementsKeyword {
            let hpos = self.token_pos();
            let token = self.current_token();
            self.next_token();
            let mut types = Vec::new();
            loop {
                let tpos = self.token_pos();
                let expr = self.parse_left_hand_side_expression();
                let expr_ref = self.arena.alloc(expr);
                let type_args = self.try_parse_type_arguments();
                let tend = self.token_end();
                types.push(ExpressionWithTypeArgumentsNode {
                    data: NodeData::new(SyntaxKind::ExpressionWithTypeArguments, tpos, tend),
                    expression: expr_ref,
                    type_arguments: type_args,
                });
                if self.optional_token(SyntaxKind::CommaToken).is_none() { break; }
            }
            let hend = self.token_end();
            clauses.push(HeritageClause {
                data: NodeData::new(SyntaxKind::HeritageClause, hpos, hend),
                token, types: alloc_vec_in(self.arena, types),
            });
        }
        Some(alloc_vec_in(self.arena, clauses))
    }

    fn parse_class_members(&mut self) -> &'a [ClassElement<'a>] {
        self.expect_token(SyntaxKind::OpenBraceToken);
        let mut members = Vec::new();
        while self.current_token() != SyntaxKind::CloseBraceToken && self.current_token() != SyntaxKind::EndOfFileToken {
            if self.current_token() == SyntaxKind::SemicolonToken {
                let spos = self.token_pos();
                let send = self.token_end();
                self.next_token();
                members.push(ClassElement::SemicolonClassElement(NodeData::new(SyntaxKind::SemicolonClassElement, spos, send)));
                continue;
            }
            members.push(self.parse_class_member());
        }
        self.expect_token(SyntaxKind::CloseBraceToken);
        alloc_vec_in(self.arena, members)
    }

    fn parse_class_member(&mut self) -> ClassElement<'a> {
        let pos = self.token_pos();

        // Skip decorators
        while self.current_token() == SyntaxKind::AtToken {
            self.next_token();
            self.parse_left_hand_side_expression();
        }

        // Parse modifiers (public, private, protected, static, abstract, readonly, override, accessor)
        let mut _is_static = false;
        let mut _is_abstract = false;
        let mut _is_readonly = false;
        loop {
            match self.current_token() {
                SyntaxKind::PublicKeyword | SyntaxKind::PrivateKeyword | SyntaxKind::ProtectedKeyword => { self.next_token(); }
                SyntaxKind::StaticKeyword => { _is_static = true; self.next_token(); }
                SyntaxKind::AbstractKeyword => { _is_abstract = true; self.next_token(); }
                SyntaxKind::ReadonlyKeyword => { _is_readonly = true; self.next_token(); }
                SyntaxKind::OverrideKeyword | SyntaxKind::DeclareKeyword => { self.next_token(); }
                SyntaxKind::AsyncKeyword => { self.next_token(); }
                _ if self.is_identifier_text("accessor") => { self.next_token(); }
                _ => break,
            }
        }

        // constructor
        if self.current_token() == SyntaxKind::ConstructorKeyword || self.is_identifier_text("constructor") {
            self.next_token();
            let type_params = self.try_parse_type_parameters();
            let (params, _ret) = self.parse_parameter_list_and_return_type();
            let body = if self.current_token() == SyntaxKind::OpenBraceToken { Some(self.parse_block()) } else { self.parse_expected_semicolon(); None };
            let end = self.token_end();
            return ClassElement::Constructor(ConstructorDeclaration {
                data: NodeData::new(SyntaxKind::Constructor, pos, end),
                type_parameters: type_params, parameters: params, body,
            });
        }

        // get/set accessor
        if self.is_identifier_text("get") || self.current_token() == SyntaxKind::GetKeyword {
            let next_is_paren = false; // simplified
            if !next_is_paren {
                self.next_token();
                let name = self.parse_property_name();
                let tp = self.try_parse_type_parameters();
                let (params, ret) = self.parse_parameter_list_and_return_type();
                let body = if self.current_token() == SyntaxKind::OpenBraceToken { Some(self.parse_block()) } else { self.parse_expected_semicolon(); None };
                let end = self.token_end();
                return ClassElement::GetAccessor(GetAccessorDeclaration {
                    data: NodeData::new(SyntaxKind::GetAccessor, pos, end),
                    name, type_parameters: tp, parameters: params, return_type: ret, body,
                });
            }
        }

        if self.is_identifier_text("set") || self.current_token() == SyntaxKind::SetKeyword {
            self.next_token();
            let name = self.parse_property_name();
            let tp = self.try_parse_type_parameters();
            let (params, _ret) = self.parse_parameter_list_and_return_type();
            let body = if self.current_token() == SyntaxKind::OpenBraceToken { Some(self.parse_block()) } else { self.parse_expected_semicolon(); None };
            let end = self.token_end();
            return ClassElement::SetAccessor(SetAccessorDeclaration {
                data: NodeData::new(SyntaxKind::SetAccessor, pos, end),
                name, type_parameters: tp, parameters: params, body,
            });
        }

        // Generator method
        let asterisk_token = self.optional_token(SyntaxKind::AsteriskToken);

        // Property name
        let name = self.parse_property_name();

        // Check for method vs property
        let question_token = self.optional_token(SyntaxKind::QuestionToken);
        let exclamation_token = self.optional_token(SyntaxKind::ExclamationToken);

        if self.current_token() == SyntaxKind::OpenParenToken || self.current_token() == SyntaxKind::LessThanToken {
            // Method
            let tp = self.try_parse_type_parameters();
            let (params, ret) = self.parse_parameter_list_and_return_type();
            let body = if self.current_token() == SyntaxKind::OpenBraceToken { Some(self.parse_block()) } else { self.parse_expected_semicolon(); None };
            let end = self.token_end();
            ClassElement::MethodDeclaration(MethodDeclaration {
                data: NodeData::new(SyntaxKind::MethodDeclaration, pos, end),
                name, question_token, asterisk_token, type_parameters: tp, parameters: params, return_type: ret, body,
            })
        } else {
            // Property
            let type_annotation = if self.optional_token(SyntaxKind::ColonToken).is_some() {
                Some(self.parse_type_and_alloc())
            } else { None };
            let initializer = if self.optional_token(SyntaxKind::EqualsToken).is_some() {
                Some(self.parse_assignment_expression_and_alloc())
            } else { None };
            let end = self.token_end();
            self.parse_expected_semicolon();
            ClassElement::PropertyDeclaration(PropertyDeclarationNode {
                data: NodeData::new(SyntaxKind::PropertyDeclaration, pos, end),
                name, question_token, exclamation_token, type_annotation, initializer,
            })
        }
    }

    fn parse_property_name(&mut self) -> PropertyName<'a> {
        match self.current_token() {
            SyntaxKind::StringLiteral => {
                let pos = self.token_pos();
                let end = self.token_end();
                self.next_token();
                PropertyName::StringLiteral(Token::new(SyntaxKind::StringLiteral, pos, end))
            }
            SyntaxKind::NumericLiteral => {
                let pos = self.token_pos();
                let end = self.token_end();
                self.next_token();
                PropertyName::NumericLiteral(Token::new(SyntaxKind::NumericLiteral, pos, end))
            }
            SyntaxKind::OpenBracketToken => {
                let pos = self.token_pos();
                self.next_token();
                let expr = self.parse_assignment_expression();
                let expr_ref = self.arena.alloc(expr);
                let end = self.token_end();
                self.expect_token(SyntaxKind::CloseBracketToken);
                PropertyName::ComputedPropertyName(self.arena.alloc(ComputedPropertyName {
                    data: NodeData::new(SyntaxKind::ComputedPropertyName, pos, end),
                    expression: expr_ref,
                }))
            }
            SyntaxKind::HashToken => {
                self.next_token();
                let id = self.parse_identifier();
                PropertyName::PrivateIdentifier(id)
            }
            _ => PropertyName::Identifier(self.parse_identifier()),
        }
    }

    // ========================================================================
    // Interface and TypeAlias
    // ========================================================================

    fn parse_interface_declaration(&mut self) -> Statement<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::InterfaceKeyword);
        let name = self.parse_identifier();
        let type_parameters = self.try_parse_type_parameters();
        let heritage_clauses = self.parse_heritage_clauses();

        self.expect_token(SyntaxKind::OpenBraceToken);
        let mut members = Vec::new();
        while self.current_token() != SyntaxKind::CloseBraceToken && self.current_token() != SyntaxKind::EndOfFileToken {
            members.push(self.parse_type_member());
            // Skip optional semicolon or comma
            if self.current_token() == SyntaxKind::SemicolonToken || self.current_token() == SyntaxKind::CommaToken {
                self.next_token();
            }
        }
        let end = self.token_end();
        self.expect_token(SyntaxKind::CloseBraceToken);

        Statement::InterfaceDeclaration(InterfaceDeclaration {
            data: NodeData::new(SyntaxKind::InterfaceDeclaration, pos, end),
            name, type_parameters, heritage_clauses,
            members: alloc_vec_in(self.arena, members),
        })
    }

    fn parse_type_member(&mut self) -> TypeElement<'a> {
        let pos = self.token_pos();

        // Skip readonly
        let _readonly = self.optional_token(SyntaxKind::ReadonlyKeyword).is_some();

        // Index signature: [key: type]: type
        if self.current_token() == SyntaxKind::OpenBracketToken {
            // Could be index signature or computed property
            self.next_token();
            let param_name = self.parse_identifier();
            if self.current_token() == SyntaxKind::ColonToken {
                // Index signature: [key: type]: type
                self.next_token();
                let param_type = self.parse_type_and_alloc();
                self.expect_token(SyntaxKind::CloseBracketToken);
                let type_annotation = if self.optional_token(SyntaxKind::ColonToken).is_some() {
                    Some(self.parse_type_and_alloc())
                } else { None };
                let end = self.token_end();
                let param = ParameterDeclaration {
                    data: NodeData::new(SyntaxKind::Parameter, pos, end),
                    dot_dot_dot_token: None,
                    name: BindingName::Identifier(param_name),
                    question_token: None,
                    type_annotation: Some(param_type),
                    initializer: None,
                };
                return TypeElement::IndexSignature(IndexSignatureNode {
                    data: NodeData::new(SyntaxKind::IndexSignature, pos, end),
                    parameters: alloc_vec_in(self.arena, vec![param]),
                    type_annotation,
                });
            }
            // If we see 'in' keyword, this is a mapped type that should have been caught earlier.
            // Skip until ] to recover.
            if self.current_token() == SyntaxKind::InKeyword {
                // Consume tokens until we hit ] to avoid infinite loop
                while self.current_token() != SyntaxKind::CloseBracketToken
                    && self.current_token() != SyntaxKind::CloseBraceToken
                    && self.current_token() != SyntaxKind::EndOfFileToken
                {
                    self.next_token();
                }
            }
            if self.current_token() == SyntaxKind::CloseBracketToken {
                self.next_token();
            }
            // Fall through to property signature
        }

        let name = self.parse_property_name();
        let question_token = self.optional_token(SyntaxKind::QuestionToken);

        if self.current_token() == SyntaxKind::OpenParenToken || self.current_token() == SyntaxKind::LessThanToken {
            // Method signature
            let tp = self.try_parse_type_parameters();
            let (params, ret) = self.parse_parameter_list_and_return_type();
            let end = self.token_end();
            TypeElement::MethodSignature(MethodSignatureNode {
                data: NodeData::new(SyntaxKind::MethodSignature, pos, end),
                name, question_token, type_parameters: tp, parameters: params, return_type: ret,
            })
        } else {
            // Property signature
            let type_annotation = if self.optional_token(SyntaxKind::ColonToken).is_some() {
                Some(self.parse_type_and_alloc())
            } else { None };
            let end = self.token_end();
            TypeElement::PropertySignature(PropertySignatureNode {
                data: NodeData::new(SyntaxKind::PropertySignature, pos, end),
                name, question_token, type_annotation,
            })
        }
    }

    fn parse_type_alias_declaration(&mut self) -> Statement<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::TypeKeyword);
        let name = self.parse_identifier();
        let type_parameters = self.try_parse_type_parameters();
        self.expect_token(SyntaxKind::EqualsToken);
        let type_node = self.parse_type_and_alloc();
        let end = self.token_end();
        self.parse_expected_semicolon();
        Statement::TypeAliasDeclaration(TypeAliasDeclaration {
            data: NodeData::new(SyntaxKind::TypeAliasDeclaration, pos, end),
            name, type_parameters, type_node,
        })
    }

    fn parse_enum_declaration(&mut self) -> Statement<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::EnumKeyword);
        let name = self.parse_identifier();
        self.expect_token(SyntaxKind::OpenBraceToken);
        let mut members = Vec::new();
        while self.current_token() != SyntaxKind::CloseBraceToken && self.current_token() != SyntaxKind::EndOfFileToken {
            let mpos = self.token_pos();
            let mname = self.parse_property_name();
            let initializer = if self.optional_token(SyntaxKind::EqualsToken).is_some() {
                Some(self.parse_assignment_expression_and_alloc())
            } else { None };
            let mend = self.token_end();
            members.push(EnumMemberNode {
                data: NodeData::new(SyntaxKind::EnumMember, mpos, mend),
                name: mname, initializer,
            });
            if self.optional_token(SyntaxKind::CommaToken).is_none() { break; }
        }
        let end = self.token_end();
        self.expect_token(SyntaxKind::CloseBraceToken);
        Statement::EnumDeclaration(EnumDeclaration {
            data: NodeData::new(SyntaxKind::EnumDeclaration, pos, end),
            name, members: alloc_vec_in(self.arena, members),
        })
    }

    fn parse_module_declaration(&mut self) -> Statement<'a> {
        let pos = self.token_pos();
        self.next_token(); // namespace/module keyword
        let name_id = self.parse_identifier();
        let body = if self.current_token() == SyntaxKind::OpenBraceToken {
            let block_pos = self.token_pos();
            self.expect_token(SyntaxKind::OpenBraceToken);
            let stmts = self.parse_statements();
            let block_end = self.token_end();
            self.expect_token(SyntaxKind::CloseBraceToken);
            Some(ModuleBody::ModuleBlock(ModuleBlock {
                data: NodeData::new(SyntaxKind::ModuleBlock, block_pos, block_end),
                statements: stmts,
            }))
        } else { None };
        let end = self.token_end();
        Statement::ModuleDeclaration(ModuleDeclaration {
            data: NodeData::new(SyntaxKind::ModuleDeclaration, pos, end),
            name: ModuleName::Identifier(name_id), body,
        })
    }

    // ========================================================================
    // Import/Export
    // ========================================================================

    fn parse_import_declaration(&mut self) -> Statement<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::ImportKeyword);

        // import type { ... } or import type X from ...
        let is_type_only_import = if self.current_token() == SyntaxKind::TypeKeyword {
            // Look ahead: if next token is identifier, { or *, it's `import type`
            let saved = self.scanner.save_state();
            let next = self.scanner.scan();
            let is_type_import = matches!(next,
                SyntaxKind::Identifier | SyntaxKind::OpenBraceToken | SyntaxKind::AsteriskToken
            );
            self.scanner.restore_state(saved);
            if is_type_import {
                self.next_token(); // consume 'type'
                true
            } else {
                false
            }
        } else {
            false
        };

        // Side-effect import: import 'module'
        if self.current_token() == SyntaxKind::StringLiteral {
            let module_specifier = self.parse_expression_and_alloc();
            let end = self.token_end();
            self.parse_expected_semicolon();
            return Statement::ImportDeclaration(ImportDeclaration {
                data: NodeData::new(SyntaxKind::ImportDeclaration, pos, end),
                import_clause: None, module_specifier, attributes: None,
            });
        }

        // Parse import clause
        let import_clause = if is_type_only_import {
            self.parse_import_clause_typed()
        } else {
            self.parse_import_clause()
        };

        self.expect_token(SyntaxKind::FromKeyword);
        let module_specifier = self.parse_expression_and_alloc();
        let end = self.token_end();
        self.parse_expected_semicolon();

        Statement::ImportDeclaration(ImportDeclaration {
            data: NodeData::new(SyntaxKind::ImportDeclaration, pos, end),
            import_clause: Some(import_clause), module_specifier, attributes: None,
        })
    }

    fn parse_import_clause(&mut self) -> ImportClause<'a> {
        self.parse_import_clause_inner(false)
    }

    fn parse_import_clause_typed(&mut self) -> ImportClause<'a> {
        self.parse_import_clause_inner(true)
    }

    fn parse_import_clause_inner(&mut self, is_type_only: bool) -> ImportClause<'a> {
        let pos = self.token_pos();

        // Default import or namespace or named
        let (name, named_bindings) = if self.current_token() == SyntaxKind::AsteriskToken {
            // import * as ns
            self.next_token();
            self.expect_token(SyntaxKind::AsKeyword);
            let ns_name = self.parse_identifier();
            let ns_pos = pos;
            let ns_end = self.token_end();
            (None, Some(NamedImportBindings::NamespaceImport(NamespaceImport {
                data: NodeData::new(SyntaxKind::NamespaceImport, ns_pos, ns_end),
                name: ns_name,
            })))
        } else if self.current_token() == SyntaxKind::OpenBraceToken {
            // import { a, b }
            (None, Some(NamedImportBindings::NamedImports(self.parse_named_imports())))
        } else {
            // import defaultExport
            let default_name = self.parse_identifier();
            if self.optional_token(SyntaxKind::CommaToken).is_some() {
                // import default, { named } or import default, * as ns
                if self.current_token() == SyntaxKind::AsteriskToken {
                    self.next_token();
                    self.expect_token(SyntaxKind::AsKeyword);
                    let ns_name = self.parse_identifier();
                    let ns_end = self.token_end();
                    (Some(default_name), Some(NamedImportBindings::NamespaceImport(NamespaceImport {
                        data: NodeData::new(SyntaxKind::NamespaceImport, pos, ns_end),
                        name: ns_name,
                    })))
                } else {
                    (Some(default_name), Some(NamedImportBindings::NamedImports(self.parse_named_imports())))
                }
            } else {
                (Some(default_name), None)
            }
        };

        let end = self.token_end();
        ImportClause {
            data: NodeData::new(SyntaxKind::ImportClause, pos, end),
            is_type_only, name, named_bindings,
        }
    }

    fn parse_named_imports(&mut self) -> NamedImports<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::OpenBraceToken);
        let mut elements = Vec::new();
        while self.current_token() != SyntaxKind::CloseBraceToken && self.current_token() != SyntaxKind::EndOfFileToken {
            let spos = self.token_pos();
            // Per-specifier type-only: `import { type Foo }` (TS 4.5+)
            // Check if current token is `type` followed by an identifier (not `as` or `}`)
            let is_type_only = if self.is_identifier_text("type") {
                let saved = self.scanner.save_state();
                let next = self.scanner.scan();
                let is_type_spec = matches!(next, SyntaxKind::Identifier)
                    || next.is_keyword(); // `type default` etc.
                self.scanner.restore_state(saved);
                if is_type_spec {
                    self.next_token(); // consume `type`
                    true
                } else {
                    false
                }
            } else {
                false
            };
            let first = self.parse_identifier();
            let (property_name, name) = if self.optional_token(SyntaxKind::AsKeyword).is_some() {
                (Some(first), self.parse_identifier())
            } else {
                (None, first)
            };
            let send = self.token_end();
            elements.push(ImportSpecifier {
                data: NodeData::new(SyntaxKind::ImportSpecifier, spos, send),
                is_type_only, property_name, name,
            });
            if self.optional_token(SyntaxKind::CommaToken).is_none() { break; }
        }
        let end = self.token_end();
        self.expect_token(SyntaxKind::CloseBraceToken);
        NamedImports {
            data: NodeData::new(SyntaxKind::NamedImports, pos, end),
            elements: alloc_vec_in(self.arena, elements),
        }
    }

    fn parse_export_declaration_or_assignment(&mut self) -> Statement<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::ExportKeyword);

        // export default
        if self.current_token() == SyntaxKind::DefaultKeyword {
            self.next_token();
            if self.current_token() == SyntaxKind::FunctionKeyword {
                return self.parse_function_declaration(false);
            }
            if self.current_token() == SyntaxKind::ClassKeyword {
                return self.parse_class_declaration(false);
            }
            let expr = self.parse_assignment_expression_and_alloc();
            let end = self.token_end();
            self.parse_expected_semicolon();
            return Statement::ExportAssignment(ExportAssignment {
                data: NodeData::new(SyntaxKind::ExportAssignment, pos, end),
                is_export_equals: false, expression: expr,
            });
        }

        // export =
        if self.optional_token(SyntaxKind::EqualsToken).is_some() {
            let expr = self.parse_assignment_expression_and_alloc();
            let end = self.token_end();
            self.parse_expected_semicolon();
            return Statement::ExportAssignment(ExportAssignment {
                data: NodeData::new(SyntaxKind::ExportAssignment, pos, end),
                is_export_equals: true, expression: expr,
            });
        }

        // export type { ... } or export type * ...
        let is_type_only_export = if self.current_token() == SyntaxKind::TypeKeyword {
            let saved = self.scanner.save_state();
            let next = self.scanner.scan();
            let is_type_export = matches!(next,
                SyntaxKind::OpenBraceToken | SyntaxKind::AsteriskToken
            );
            self.scanner.restore_state(saved);
            if is_type_export {
                self.next_token(); // consume 'type'
                true
            } else {
                false
            }
        } else {
            false
        };

        // export { ... } or export type { ... }
        if self.current_token() == SyntaxKind::OpenBraceToken {
            let named = self.parse_named_exports();
            let module_specifier = if self.current_token() == SyntaxKind::FromKeyword {
                self.next_token();
                Some(self.parse_expression_and_alloc())
            } else { None };
            let end = self.token_end();
            self.parse_expected_semicolon();
            return Statement::ExportDeclaration(ExportDeclaration {
                data: NodeData::new(SyntaxKind::ExportDeclaration, pos, end),
                is_type_only: is_type_only_export,
                export_clause: Some(NamedExportBindings::NamedExports(named)),
                module_specifier, attributes: None,
            });
        }

        // export * from '...' or export type * from '...'
        if self.current_token() == SyntaxKind::AsteriskToken {
            self.next_token();
            // export * as ns from '...'
            let export_clause = if self.optional_token(SyntaxKind::AsKeyword).is_some() {
                let ns_name = self.parse_identifier();
                let ns_end = self.token_end();
                Some(NamedExportBindings::NamespaceExport(NamespaceExport {
                    data: NodeData::new(SyntaxKind::NamespaceExport, pos, ns_end),
                    name: ns_name,
                }))
            } else { None };
            self.expect_token(SyntaxKind::FromKeyword);
            let module_specifier = self.parse_expression_and_alloc();
            let end = self.token_end();
            self.parse_expected_semicolon();
            return Statement::ExportDeclaration(ExportDeclaration {
                data: NodeData::new(SyntaxKind::ExportDeclaration, pos, end),
                is_type_only: false, export_clause,
                module_specifier: Some(module_specifier), attributes: None,
            });
        }

        // export declaration (function, class, var, etc.)
        self.parse_statement()
    }

    fn parse_named_exports(&mut self) -> NamedExports<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::OpenBraceToken);
        let mut elements = Vec::new();
        while self.current_token() != SyntaxKind::CloseBraceToken && self.current_token() != SyntaxKind::EndOfFileToken {
            let spos = self.token_pos();
            let first = self.parse_identifier();
            let (property_name, name) = if self.optional_token(SyntaxKind::AsKeyword).is_some() {
                (Some(first), self.parse_identifier())
            } else {
                (None, first)
            };
            let send = self.token_end();
            elements.push(ExportSpecifier {
                data: NodeData::new(SyntaxKind::ExportSpecifier, spos, send),
                is_type_only: false, property_name, name,
            });
            if self.optional_token(SyntaxKind::CommaToken).is_none() { break; }
        }
        let end = self.token_end();
        self.expect_token(SyntaxKind::CloseBraceToken);
        NamedExports {
            data: NodeData::new(SyntaxKind::NamedExports, pos, end),
            elements: alloc_vec_in(self.arena, elements),
        }
    }

    // ========================================================================
    // Signature/Parameter parsing
    // ========================================================================

    fn try_parse_type_parameters(&mut self) -> Option<&'a [TypeParameterDeclaration<'a>]> {
        if self.current_token() != SyntaxKind::LessThanToken { return None; }
        self.next_token();
        let mut params = Vec::new();
        loop {
            let tpos = self.token_pos();
            let name = self.parse_identifier();
            let constraint = if self.current_token() == SyntaxKind::ExtendsKeyword {
                self.next_token();
                Some(self.parse_type_and_alloc())
            } else { None };
            let default = if self.optional_token(SyntaxKind::EqualsToken).is_some() {
                Some(self.parse_type_and_alloc())
            } else { None };
            let tend = self.token_end();
            params.push(TypeParameterDeclaration {
                data: NodeData::new(SyntaxKind::TypeParameter, tpos, tend),
                name, constraint, default,
            });
            if self.optional_token(SyntaxKind::CommaToken).is_none() { break; }
        }
        self.expect_token(SyntaxKind::GreaterThanToken);
        Some(alloc_vec_in(self.arena, params))
    }

    fn try_parse_type_arguments(&mut self) -> Option<&'a [TypeNode<'a>]> {
        if self.current_token() != SyntaxKind::LessThanToken { return None; }
        self.next_token();
        let mut args = Vec::new();
        loop {
            args.push(self.parse_type());
            if self.optional_token(SyntaxKind::CommaToken).is_none() { break; }
        }
        self.expect_token(SyntaxKind::GreaterThanToken);
        Some(alloc_vec_in(self.arena, args))
    }

    fn parse_parameter_list_and_return_type(&mut self) -> (&'a [ParameterDeclaration<'a>], Option<&'a TypeNode<'a>>) {
        self.expect_token(SyntaxKind::OpenParenToken);
        let mut params = Vec::new();
        while self.current_token() != SyntaxKind::CloseParenToken && self.current_token() != SyntaxKind::EndOfFileToken {
            params.push(self.parse_parameter());
            if self.optional_token(SyntaxKind::CommaToken).is_none() { break; }
        }
        self.expect_token(SyntaxKind::CloseParenToken);
        let parameters = alloc_vec_in(self.arena, params);
        let return_type = if self.optional_token(SyntaxKind::ColonToken).is_some() {
            Some(self.parse_type_and_alloc())
        } else { None };
        (parameters, return_type)
    }

    fn parse_parameter(&mut self) -> ParameterDeclaration<'a> {
        let pos = self.token_pos();
        // Skip modifiers (public, private, protected, readonly, override)
        while matches!(
            self.current_token(),
            SyntaxKind::PublicKeyword
                | SyntaxKind::PrivateKeyword
                | SyntaxKind::ProtectedKeyword
                | SyntaxKind::ReadonlyKeyword
                | SyntaxKind::OverrideKeyword
        ) {
            self.next_token();
        }
        let dot_dot_dot_token = self.optional_token(SyntaxKind::DotDotDotToken);
        let name = self.parse_binding_name();
        let question_token = self.optional_token(SyntaxKind::QuestionToken);
        let type_annotation = if self.optional_token(SyntaxKind::ColonToken).is_some() {
            Some(self.parse_type_and_alloc())
        } else { None };
        let initializer = if self.optional_token(SyntaxKind::EqualsToken).is_some() {
            Some(self.parse_assignment_expression_and_alloc())
        } else { None };
        let end = self.token_end();
        ParameterDeclaration {
            data: NodeData::new(SyntaxKind::Parameter, pos, end),
            dot_dot_dot_token, name, question_token, type_annotation, initializer,
        }
    }

    // ========================================================================
    // Type parsing
    // ========================================================================

    fn parse_type_and_alloc(&mut self) -> &'a TypeNode<'a> {
        let ty = self.parse_type();
        self.arena.alloc(ty)
    }

    fn parse_type(&mut self) -> TypeNode<'a> {
        self.recursion_depth += 1;
        if self.recursion_depth > MAX_RECURSION_DEPTH {
            self.error(&rscript_diagnostics::messages::TYPE_EXPECTED, &[]);
            self.recursion_depth -= 1;
            let pos = self.token_pos();
            let end = self.token_end();
            return TypeNode::KeywordType(KeywordTypeNode {
                data: NodeData::new(SyntaxKind::AnyKeyword, pos, end),
            });
        }
        // Parse union type (which includes intersection, primary, and postfix)
        let result = self.parse_union_or_intersection_type();
        self.recursion_depth -= 1;
        result
    }

    fn parse_union_or_intersection_type(&mut self) -> TypeNode<'a> {
        // Leading | or &
        let _leading_bar = self.optional_token(SyntaxKind::BarToken);

        let first = self.parse_intersection_or_primary_type();

        // Union: T | U | V
        if self.current_token() == SyntaxKind::BarToken {
            let pos = first.data().range.pos;
            let mut types = vec![first];
            while self.optional_token(SyntaxKind::BarToken).is_some() {
                types.push(self.parse_intersection_or_primary_type());
            }
            let end = self.token_end();
            return TypeNode::UnionType(UnionTypeNode {
                data: NodeData::new(SyntaxKind::UnionType, pos, end),
                types: alloc_vec_in(self.arena, types),
            });
        }

        first
    }

    fn parse_intersection_or_primary_type(&mut self) -> TypeNode<'a> {
        let _leading_amp = self.optional_token(SyntaxKind::AmpersandToken);

        let first = self.parse_postfix_type();

        // Intersection: T & U & V
        if self.current_token() == SyntaxKind::AmpersandToken {
            let pos = first.data().range.pos;
            let mut types = vec![first];
            while self.optional_token(SyntaxKind::AmpersandToken).is_some() {
                types.push(self.parse_postfix_type());
            }
            let end = self.token_end();
            return TypeNode::IntersectionType(IntersectionTypeNode {
                data: NodeData::new(SyntaxKind::IntersectionType, pos, end),
                types: alloc_vec_in(self.arena, types),
            });
        }

        // Conditional type: T extends U ? X : Y
        if self.current_token() == SyntaxKind::ExtendsKeyword {
            let pos = first.data().range.pos;
            self.next_token();
            let extends_type = self.parse_type();
            let extends_ref = self.arena.alloc(extends_type);
            self.expect_token(SyntaxKind::QuestionToken);
            let true_type = self.parse_type();
            let true_ref = self.arena.alloc(true_type);
            self.expect_token(SyntaxKind::ColonToken);
            let false_type = self.parse_type();
            let false_ref = self.arena.alloc(false_type);
            let first_ref = self.arena.alloc(first);
            let end = self.token_end();
            return TypeNode::ConditionalType(ConditionalTypeNode {
                data: NodeData::new(SyntaxKind::ConditionalType, pos, end),
                check_type: first_ref, extends_type: extends_ref,
                true_type: true_ref, false_type: false_ref,
            });
        }

        first
    }

    fn parse_postfix_type(&mut self) -> TypeNode<'a> {
        let mut ty = self.parse_non_array_type();

        // Postfix: T[], T[K]
        loop {
            if self.current_token() == SyntaxKind::OpenBracketToken {
                let pos = ty.data().range.pos;
                self.next_token();
                if self.current_token() == SyntaxKind::CloseBracketToken {
                    // Array type: T[]
                    let end = self.token_end();
                    self.next_token();
                    let ty_ref = self.arena.alloc(ty);
                    ty = TypeNode::ArrayType(ArrayTypeNode {
                        data: NodeData::new(SyntaxKind::ArrayType, pos, end),
                        element_type: ty_ref,
                    });
                } else {
                    // Indexed access type: T[K]
                    let index_type = self.parse_type();
                    let index_ref = self.arena.alloc(index_type);
                    let end = self.token_end();
                    self.expect_token(SyntaxKind::CloseBracketToken);
                    let ty_ref = self.arena.alloc(ty);
                    ty = TypeNode::IndexedAccessType(IndexedAccessTypeNode {
                        data: NodeData::new(SyntaxKind::IndexedAccessType, pos, end),
                        object_type: ty_ref, index_type: index_ref,
                    });
                }
            } else {
                break;
            }
        }

        ty
    }

    fn parse_non_array_type(&mut self) -> TypeNode<'a> {
        match self.current_token() {
            // Keyword types
            SyntaxKind::StringKeyword | SyntaxKind::NumberKeyword | SyntaxKind::BooleanKeyword
            | SyntaxKind::AnyKeyword | SyntaxKind::VoidKeyword | SyntaxKind::NeverKeyword
            | SyntaxKind::UndefinedKeyword | SyntaxKind::NullKeyword | SyntaxKind::UnknownKeyword
            | SyntaxKind::ObjectKeyword | SyntaxKind::BigIntKeyword | SyntaxKind::SymbolKeyword
            | SyntaxKind::IntrinsicKeyword => {
                let pos = self.token_pos();
                let kind = self.current_token();
                let end = self.token_end();
                self.next_token();
                TypeNode::KeywordType(KeywordTypeNode {
                    data: NodeData::new(kind, pos, end),
                })
            }

            // this type
            SyntaxKind::ThisKeyword => {
                let pos = self.token_pos();
                let end = self.token_end();
                self.next_token();
                TypeNode::ThisType(ThisTypeNode { data: NodeData::new(SyntaxKind::ThisType, pos, end) })
            }

            // typeof type
            SyntaxKind::TypeOfKeyword => {
                let pos = self.token_pos();
                self.next_token();
                let name = self.parse_entity_name();
                let type_args = self.try_parse_type_arguments();
                let end = self.token_end();
                TypeNode::TypeQuery(TypeQueryNode {
                    data: NodeData::new(SyntaxKind::TypeQuery, pos, end),
                    expr_name: name, type_arguments: type_args,
                })
            }

            // keyof, unique, readonly type operators
            SyntaxKind::KeyOfKeyword | SyntaxKind::UniqueKeyword | SyntaxKind::ReadonlyKeyword => {
                let pos = self.token_pos();
                let operator = self.current_token();
                self.next_token();
                let operand = self.parse_postfix_type();
                let operand_ref = self.arena.alloc(operand);
                let end = self.token_end();
                TypeNode::TypeOperator(TypeOperatorNode {
                    data: NodeData::new(SyntaxKind::TypeOperator, pos, end),
                    operator, type_node: operand_ref,
                })
            }

            // infer T
            SyntaxKind::InferKeyword => {
                let pos = self.token_pos();
                self.next_token();
                let tp_pos = self.token_pos();
                let name = self.parse_identifier();
                let constraint = if self.current_token() == SyntaxKind::ExtendsKeyword {
                    self.next_token();
                    Some(self.parse_type_and_alloc())
                } else { None };
                let tp_end = self.token_end();
                let tp = TypeParameterDeclaration {
                    data: NodeData::new(SyntaxKind::TypeParameter, tp_pos, tp_end),
                    name, constraint, default: None,
                };
                let end = self.token_end();
                TypeNode::InferType(InferTypeNode {
                    data: NodeData::new(SyntaxKind::InferType, pos, end),
                    type_parameter: self.arena.alloc(tp),
                })
            }

            // Tuple type: [T, U, ...V]
            SyntaxKind::OpenBracketToken => self.parse_tuple_type(),

            // Parenthesized or function type
            SyntaxKind::OpenParenToken => self.parse_parenthesized_or_function_type(),

            // Object/mapped type literal: { ... }
            SyntaxKind::OpenBraceToken => self.parse_type_literal_or_mapped_type(),

            // Literal types: true, false (null is handled as a keyword type above)
            SyntaxKind::TrueKeyword | SyntaxKind::FalseKeyword => {
                let pos = self.token_pos();
                let kind = self.current_token();
                let end = self.token_end();
                self.next_token();
                let expr = if kind == SyntaxKind::TrueKeyword {
                    Expression::TrueKeyword(NodeData::new(kind, pos, end))
                } else {
                    Expression::FalseKeyword(NodeData::new(kind, pos, end))
                };
                let expr_ref = self.arena.alloc(expr);
                TypeNode::LiteralType(LiteralTypeNode {
                    data: NodeData::new(SyntaxKind::LiteralType, pos, end),
                    literal: expr_ref,
                })
            }

            SyntaxKind::StringLiteral | SyntaxKind::NumericLiteral => {
                let pos = self.token_pos();
                let end = self.token_end();
                let value = self.token_value().to_string();
                let expr = if self.current_token() == SyntaxKind::StringLiteral {
                    Expression::StringLiteral(StringLiteral {
                        data: NodeData::new(SyntaxKind::StringLiteral, pos, end),
                        text: InternedString::dummy(), text_name: value, is_single_quote: false,
                    })
                } else {
                    Expression::NumericLiteral(NumericLiteral {
                        data: NodeData::new(SyntaxKind::NumericLiteral, pos, end),
                        text: InternedString::dummy(), text_name: value, numeric_literal_flags: TokenFlags::NONE,
                    })
                };
                self.next_token();
                let expr_ref = self.arena.alloc(expr);
                TypeNode::LiteralType(LiteralTypeNode {
                    data: NodeData::new(SyntaxKind::LiteralType, pos, end),
                    literal: expr_ref,
                })
            }

            // Negative number literal type: -1
            SyntaxKind::MinusToken => {
                let pos = self.token_pos();
                self.next_token();
                let num_pos = self.token_pos();
                let num_end = self.token_end();
                self.next_token(); // consume number
                let inner = Expression::NumericLiteral(NumericLiteral {
                    data: NodeData::new(SyntaxKind::NumericLiteral, num_pos, num_end),
                    text: InternedString::dummy(), text_name: String::new(), numeric_literal_flags: TokenFlags::NONE,
                });
                let inner_ref = self.arena.alloc(inner);
                let prefix = Expression::PrefixUnary(PrefixUnaryExpression {
                    data: NodeData::new(SyntaxKind::PrefixUnaryExpression, pos, num_end),
                    operator: SyntaxKind::MinusToken, operand: inner_ref,
                });
                let prefix_ref = self.arena.alloc(prefix);
                TypeNode::LiteralType(LiteralTypeNode {
                    data: NodeData::new(SyntaxKind::LiteralType, pos, num_end),
                    literal: prefix_ref,
                })
            }

            // Template literal type
            SyntaxKind::NoSubstitutionTemplateLiteral | SyntaxKind::TemplateHead => {
                self.parse_template_literal_type()
            }

            // Type reference (identifier, possibly qualified, possibly with type args)
            _ if self.current_token() == SyntaxKind::Identifier || self.current_token().is_keyword() => {
                let pos = self.token_pos();
                let name = self.parse_entity_name();
                let type_arguments = self.try_parse_type_arguments();
                let end = self.token_end();
                TypeNode::TypeReference(TypeReferenceNode {
                    data: NodeData::new(SyntaxKind::TypeReference, pos, end),
                    type_name: name, type_arguments,
                })
            }

            _ => {
                let pos = self.token_pos();
                let end = self.token_end();
                self.error(&rscript_diagnostics::messages::TYPE_EXPECTED, &[]);
                self.next_token();
                TypeNode::KeywordType(KeywordTypeNode {
                    data: NodeData::new(SyntaxKind::AnyKeyword, pos, end),
                })
            }
        }
    }

    fn parse_entity_name(&mut self) -> EntityName<'a> {
        let mut name = EntityName::Identifier(self.parse_identifier());
        while self.optional_token(SyntaxKind::DotToken).is_some() {
            let pos = name.data().range.pos;
            let right = self.parse_identifier();
            let end = self.token_end();
            name = EntityName::QualifiedName(self.arena.alloc(QualifiedName {
                data: NodeData::new(SyntaxKind::QualifiedName, pos, end),
                left: name, right,
            }));
        }
        name
    }

    fn parse_tuple_type(&mut self) -> TypeNode<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::OpenBracketToken);
        let mut elements = Vec::new();
        while self.current_token() != SyntaxKind::CloseBracketToken && self.current_token() != SyntaxKind::EndOfFileToken {
            // Handle rest type: ...T
            if self.current_token() == SyntaxKind::DotDotDotToken {
                let rpos = self.token_pos();
                self.next_token();
                let inner = self.parse_type();
                let inner_ref = self.arena.alloc(inner);
                let rend = self.token_end();
                elements.push(TypeNode::RestType(RestTypeNode {
                    data: NodeData::new(SyntaxKind::RestType, rpos, rend),
                    type_node: inner_ref,
                }));
            } else {
                elements.push(self.parse_type());
            }
            // Check for optional: ?
            if self.current_token() == SyntaxKind::QuestionToken {
                // Convert last to optional type
                let last = elements.pop().unwrap();
                let opos = last.data().range.pos;
                let last_ref = self.arena.alloc(last);
                self.next_token();
                let oend = self.token_end();
                elements.push(TypeNode::OptionalType(OptionalTypeNode {
                    data: NodeData::new(SyntaxKind::OptionalType, opos, oend),
                    type_node: last_ref,
                }));
            }
            if self.optional_token(SyntaxKind::CommaToken).is_none() { break; }
        }
        let end = self.token_end();
        self.expect_token(SyntaxKind::CloseBracketToken);
        TypeNode::TupleType(TupleTypeNode {
            data: NodeData::new(SyntaxKind::TupleType, pos, end),
            elements: alloc_vec_in(self.arena, elements),
        })
    }

    /// Disambiguate `(T)` (parenthesized type) from `(a: T) => U` (function type).
    ///
    /// Uses a scanner look-ahead: skip tokens to the matching `)`, then check
    /// whether `=>` follows.  This matches TypeScript's own strategy and
    /// correctly handles arbitrarily complex parameter lists without
    /// speculative parsing.
    fn parse_parenthesized_or_function_type(&mut self) -> TypeNode<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::OpenParenToken);

        // Case 1: empty parameter list  →  `() => T` or standalone `()`
        if self.current_token() == SyntaxKind::CloseParenToken {
            self.next_token();
            if self.current_token() == SyntaxKind::EqualsGreaterThanToken {
                self.next_token();
                let return_type = self.parse_type_and_alloc();
                let end = self.token_end();
                return TypeNode::FunctionType(FunctionTypeNode {
                    data: NodeData::new(SyntaxKind::FunctionType, pos, end),
                    type_parameters: None,
                    parameters: &[],
                    return_type: Some(return_type),
                });
            }
            let end = self.token_end();
            return TypeNode::KeywordType(KeywordTypeNode {
                data: NodeData::new(SyntaxKind::VoidKeyword, pos, end),
            });
        }

        // Case 2: rest parameter `(...args) => T`  →  definitely a function type
        // Case 3: use look-ahead to check for `=>` after the matching `)`
        let is_function_type = self.current_token() == SyntaxKind::DotDotDotToken
            || self.is_start_of_function_type();

        if is_function_type {
            // Parse proper parameter list
            let mut params = Vec::new();
            while self.current_token() != SyntaxKind::CloseParenToken
                && self.current_token() != SyntaxKind::EndOfFileToken
            {
                params.push(self.parse_parameter());
                if self.optional_token(SyntaxKind::CommaToken).is_none() {
                    break;
                }
            }
            self.expect_token(SyntaxKind::CloseParenToken);
            self.expect_token(SyntaxKind::EqualsGreaterThanToken);
            let return_type = self.parse_type_and_alloc();
            let parameters = alloc_vec_in(self.arena, params);
            let end = self.token_end();
            return TypeNode::FunctionType(FunctionTypeNode {
                data: NodeData::new(SyntaxKind::FunctionType, pos, end),
                type_parameters: None,
                parameters,
                return_type: Some(return_type),
            });
        }

        // Case 4: parenthesized type  `(T)`
        let inner = self.parse_type();
        self.expect_token(SyntaxKind::CloseParenToken);

        // Edge case: `(T) => U` where T is a single unnamed param
        if self.current_token() == SyntaxKind::EqualsGreaterThanToken {
            self.next_token();
            let return_type = self.parse_type_and_alloc();
            let end = self.token_end();
            return TypeNode::FunctionType(FunctionTypeNode {
                data: NodeData::new(SyntaxKind::FunctionType, pos, end),
                type_parameters: None,
                parameters: &[],
                return_type: Some(return_type),
            });
        }

        let inner_ref = self.arena.alloc(inner);
        let end = self.token_end();
        TypeNode::ParenthesizedType(ParenthesizedTypeNode {
            data: NodeData::new(SyntaxKind::ParenthesizedType, pos, end),
            type_node: inner_ref,
        })
    }

    /// Look-ahead: scan forward (without consuming tokens) to the matching `)`
    /// and check whether `=>` follows.  Returns `true` if this is a function
    /// type signature.
    ///
    /// Called after the outer `(` has been consumed, so depth starts at 1.
    /// The *current* token (not yet consumed by scan()) must be accounted for
    /// because `scanner.scan()` returns the *next* token, skipping the current.
    fn is_start_of_function_type(&mut self) -> bool {
        let saved = self.scanner.save_state();
        let mut depth: u32 = 1;

        // Account for the current token before we start scanning forward.
        // Without this, nested parens like `((x: number) => boolean)` would
        // miscount and match the inner `)` instead of the outer one.
        match self.scanner.token() {
            SyntaxKind::OpenParenToken => depth += 1,
            SyntaxKind::CloseParenToken => {
                // Immediately closes — check for =>
                let next = self.scanner.scan();
                let is_arrow = next == SyntaxKind::EqualsGreaterThanToken;
                self.scanner.restore_state(saved);
                return is_arrow;
            }
            SyntaxKind::EndOfFileToken => {
                self.scanner.restore_state(saved);
                return false;
            }
            _ => {}
        }

        loop {
            let tok = self.scanner.scan();
            match tok {
                SyntaxKind::OpenParenToken => depth += 1,
                SyntaxKind::CloseParenToken => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                SyntaxKind::EndOfFileToken => {
                    self.scanner.restore_state(saved);
                    return false;
                }
                _ => {}
            }
        }
        let next = self.scanner.scan();
        let is_arrow = next == SyntaxKind::EqualsGreaterThanToken;
        self.scanner.restore_state(saved);
        is_arrow
    }

    fn parse_type_literal_or_mapped_type(&mut self) -> TypeNode<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::OpenBraceToken);

        // Detect mapped type: { [K in T]: V } or { readonly [K in T]?: V }
        // or { +readonly [K in T]-?: V } or { -readonly [K in T]+?: V }
        if self.is_mapped_type_start() {
            return self.parse_mapped_type(pos);
        }

        let mut members = Vec::new();
        while self.current_token() != SyntaxKind::CloseBraceToken && self.current_token() != SyntaxKind::EndOfFileToken {
            members.push(self.parse_type_member());
            if self.current_token() == SyntaxKind::SemicolonToken || self.current_token() == SyntaxKind::CommaToken {
                self.next_token();
            }
        }
        let end = self.token_end();
        self.expect_token(SyntaxKind::CloseBraceToken);
        TypeNode::TypeLiteral(TypeLiteralNode {
            data: NodeData::new(SyntaxKind::TypeLiteral, pos, end),
            members: alloc_vec_in(self.arena, members),
        })
    }

    /// Check if the current position starts a mapped type (after '{' has been consumed).
    fn is_mapped_type_start(&mut self) -> bool {
        // Save scanner state for lookahead
        let saved = self.scanner.save_state();

        let result = self.is_mapped_type_start_inner();

        // Restore scanner state
        self.scanner.restore_state(saved);
        result
    }

    fn is_mapped_type_start_inner(&mut self) -> bool {
        // Skip optional +/- readonly
        if self.current_token() == SyntaxKind::ReadonlyKeyword {
            self.next_token();
        } else if self.current_token() == SyntaxKind::PlusToken || self.current_token() == SyntaxKind::MinusToken {
            self.next_token();
            if self.current_token() == SyntaxKind::ReadonlyKeyword {
                self.next_token();
            } else {
                return false;
            }
        }

        // Must see [
        if self.current_token() != SyntaxKind::OpenBracketToken {
            return false;
        }
        self.next_token();

        // Must see identifier
        if self.current_token() != SyntaxKind::Identifier {
            return false;
        }
        self.next_token();

        // Must see 'in' keyword
        self.current_token() == SyntaxKind::InKeyword
    }

    fn parse_mapped_type(&mut self, pos: u32) -> TypeNode<'a> {
        // Skip optional readonly modifier
        let readonly_token = if self.current_token() == SyntaxKind::ReadonlyKeyword {
            let t = Some(Token::new(SyntaxKind::ReadonlyKeyword, self.token_pos(), self.token_end()));
            self.next_token();
            t
        } else if self.current_token() == SyntaxKind::PlusToken || self.current_token() == SyntaxKind::MinusToken {
            let kind = self.current_token();
            let tpos = self.token_pos();
            self.next_token();
            if self.current_token() == SyntaxKind::ReadonlyKeyword {
                let t = Some(Token::new(kind, tpos, self.token_end()));
                self.next_token();
                t
            } else {
                None
            }
        } else {
            None
        };

        // Expect [
        self.expect_token(SyntaxKind::OpenBracketToken);

        // Parse type parameter: K in T
        let tp_pos = self.token_pos();
        let name = self.parse_identifier();
        self.expect_token(SyntaxKind::InKeyword);
        let constraint = self.parse_type_and_alloc();

        // Optional 'as' clause: [K in T as U]
        let name_type = if self.current_token() == SyntaxKind::AsKeyword {
            self.next_token();
            Some(self.parse_type_and_alloc())
        } else {
            None
        };

        let tp_end = self.token_end();
        let type_param = self.arena.alloc(TypeParameterDeclaration {
            data: NodeData::new(SyntaxKind::TypeParameter, tp_pos, tp_end),
            name,
            constraint: Some(constraint),
            default: None,
        });

        self.expect_token(SyntaxKind::CloseBracketToken);

        // Optional +?/-? modifier
        let question_token = if self.current_token() == SyntaxKind::QuestionToken {
            let t = Some(Token::new(SyntaxKind::QuestionToken, self.token_pos(), self.token_end()));
            self.next_token();
            t
        } else if self.current_token() == SyntaxKind::PlusToken || self.current_token() == SyntaxKind::MinusToken {
            let kind = self.current_token();
            let tpos = self.token_pos();
            self.next_token();
            if self.current_token() == SyntaxKind::QuestionToken {
                let t = Some(Token::new(kind, tpos, self.token_end()));
                self.next_token();
                t
            } else {
                None
            }
        } else {
            None
        };

        // Parse type after ':'
        let type_node = if self.optional_token(SyntaxKind::ColonToken).is_some() {
            Some(self.parse_type_and_alloc())
        } else {
            None
        };

        // Expect optional ; then }
        if self.current_token() == SyntaxKind::SemicolonToken {
            self.next_token();
        }

        let end = self.token_end();
        self.expect_token(SyntaxKind::CloseBraceToken);

        TypeNode::MappedType(MappedTypeNode {
            data: NodeData::new(SyntaxKind::MappedType, pos, end),
            readonly_token,
            type_parameter: type_param,
            name_type,
            question_token,
            type_node,
            members: None,
        })
    }

    fn parse_template_literal_type(&mut self) -> TypeNode<'a> {
        let pos = self.token_pos();
        if self.current_token() == SyntaxKind::NoSubstitutionTemplateLiteral {
            let end = self.token_end();
            self.next_token();
            return TypeNode::TemplateLiteralType(TemplateLiteralTypeNode {
                data: NodeData::new(SyntaxKind::TemplateLiteralType, pos, end),
                head: Token::new(SyntaxKind::TemplateHead, pos, end),
                template_spans: &[],
            });
        }
        // TemplateHead
        let head_end = self.token_end();
        let head = Token::new(SyntaxKind::TemplateHead, pos, head_end);
        self.next_token();

        let mut spans = Vec::new();
        loop {
            let spos = self.token_pos();
            let type_node = self.parse_type_and_alloc();
            // Expect } and scan template middle/tail
            let lit_token = if self.current_token() == SyntaxKind::CloseBraceToken {
                let kind = self.scanner.rescan_template_token();
                let lpos = self.token_pos();
                let lend = self.token_end();
                self.next_token();
                Token::new(kind, lpos, lend)
            } else {
                let lpos = self.token_pos();
                let lend = self.token_end();
                self.next_token();
                Token::new(SyntaxKind::TemplateTail, lpos, lend)
            };
            let is_tail = lit_token.data.kind == SyntaxKind::TemplateTail;
            let send = self.token_end();
            spans.push(TemplateLiteralTypeSpan {
                data: NodeData::new(SyntaxKind::TemplateLiteralTypeSpan, spos, send),
                type_node, literal: lit_token,
            });
            if is_tail || self.current_token() == SyntaxKind::EndOfFileToken { break; }
        }

        let end = self.token_end();
        TypeNode::TemplateLiteralType(TemplateLiteralTypeNode {
            data: NodeData::new(SyntaxKind::TemplateLiteralType, pos, end),
            head, template_spans: alloc_vec_in(self.arena, spans),
        })
    }

    // ========================================================================
    // Expression parsing
    // ========================================================================

    fn parse_expression_and_alloc(&mut self) -> &'a Expression<'a> {
        let expr = self.parse_expression();
        self.arena.alloc(expr)
    }

    fn parse_assignment_expression_and_alloc(&mut self) -> &'a Expression<'a> {
        let expr = self.parse_assignment_expression();
        self.arena.alloc(expr)
    }

    fn parse_expression(&mut self) -> Expression<'a> {
        self.recursion_depth += 1;
        if self.recursion_depth > MAX_RECURSION_DEPTH {
            self.error(&rscript_diagnostics::messages::EXPRESSION_EXPECTED, &[]);
            self.recursion_depth -= 1;
            let pos = self.token_pos();
            let end = self.token_end();
            return Expression::Identifier(Identifier {
                data: NodeData::new(SyntaxKind::Identifier, pos, end),
                text: InternedString::dummy(),
                text_name: String::new(),
                original_keyword_kind: None,
            });
        }
        // Comma expression: a, b, c → represented as nested Binary(left, CommaToken, right)
        let mut expr = self.parse_assignment_expression();
        while self.current_token() == SyntaxKind::CommaToken {
            let pos = self.token_pos();
            self.next_token(); // consume comma
            let right = self.parse_assignment_expression();
            let end = self.token_end();
            let left_ref = self.arena.alloc(expr);
            let right_ref = self.arena.alloc(right);
            expr = Expression::Binary(BinaryExpression {
                data: NodeData::new(SyntaxKind::BinaryExpression, pos, end),
                left: left_ref,
                operator_token: Token::new(SyntaxKind::CommaToken, pos, pos + 1),
                right: right_ref,
            });
        }
        self.recursion_depth -= 1;
        expr
    }

    fn parse_assignment_expression(&mut self) -> Expression<'a> {
        // Yield expression
        if self.current_token() == SyntaxKind::YieldKeyword && self.context_flags.contains(NodeFlags::YIELD_CONTEXT) {
            return self.parse_yield_expression();
        }

        let expr = self.parse_conditional_expression();

        // Assignment operators
        if self.current_token().is_assignment_operator() {
            let pos = expr.data().range.pos;
            let op_token = Token::new(self.current_token(), self.token_pos(), self.token_end());
            self.next_token();
            let right = self.parse_assignment_expression();
            let right_ref = self.arena.alloc(right);
            let expr_ref = self.arena.alloc(expr);
            let end = self.token_end();
            return Expression::Binary(BinaryExpression {
                data: NodeData::new(SyntaxKind::BinaryExpression, pos, end),
                left: expr_ref, operator_token: op_token, right: right_ref,
            });
        }

        expr
    }

    fn parse_conditional_expression(&mut self) -> Expression<'a> {
        let expr = self.parse_binary_expression(OperatorPrecedence::Comma);

        // Ternary: cond ? true : false
        if self.current_token() == SyntaxKind::QuestionToken {
            let pos = expr.data().range.pos;
            let q_token = Token::new(SyntaxKind::QuestionToken, self.token_pos(), self.token_end());
            self.next_token();
            let when_true = self.parse_assignment_expression();
            let when_true_ref = self.arena.alloc(when_true);
            let c_token = Token::new(SyntaxKind::ColonToken, self.token_pos(), self.token_end());
            self.expect_token(SyntaxKind::ColonToken);
            let when_false = self.parse_assignment_expression();
            let when_false_ref = self.arena.alloc(when_false);
            let expr_ref = self.arena.alloc(expr);
            let end = self.token_end();
            return Expression::Conditional(ConditionalExpression {
                data: NodeData::new(SyntaxKind::ConditionalExpression, pos, end),
                condition: expr_ref, question_token: q_token,
                when_true: when_true_ref, colon_token: c_token, when_false: when_false_ref,
            });
        }

        // `as` type assertion
        if self.current_token() == SyntaxKind::AsKeyword {
            let pos = expr.data().range.pos;
            self.next_token();
            let type_node = self.parse_type_and_alloc();
            let expr_ref = self.arena.alloc(expr);
            let end = self.token_end();
            return Expression::As(AsExpression {
                data: NodeData::new(SyntaxKind::AsExpression, pos, end),
                expression: expr_ref, type_node,
            });
        }

        // `satisfies`
        if self.is_identifier_text("satisfies") {
            let pos = expr.data().range.pos;
            self.next_token();
            let type_node = self.parse_type_and_alloc();
            let expr_ref = self.arena.alloc(expr);
            let end = self.token_end();
            return Expression::Satisfies(SatisfiesExpression {
                data: NodeData::new(SyntaxKind::SatisfiesExpression, pos, end),
                expression: expr_ref, type_node,
            });
        }

        expr
    }

    fn parse_binary_expression(&mut self, min_precedence: OperatorPrecedence) -> Expression<'a> {
        let mut left = self.parse_unary_expression();

        loop {
            let precedence = get_binary_operator_precedence(self.current_token());
            if precedence == OperatorPrecedence::Invalid || precedence <= min_precedence {
                break;
            }

            let pos = left.data().range.pos;
            let op_token = Token::new(self.current_token(), self.token_pos(), self.token_end());
            self.next_token();
            let right = self.parse_binary_expression(precedence);
            let left_ref = self.arena.alloc(left);
            let right_ref = self.arena.alloc(right);
            let end = self.token_end();
            left = Expression::Binary(BinaryExpression {
                data: NodeData::new(SyntaxKind::BinaryExpression, pos, end),
                left: left_ref, operator_token: op_token, right: right_ref,
            });
        }

        left
    }

    fn parse_unary_expression(&mut self) -> Expression<'a> {
        match self.current_token() {
            SyntaxKind::PlusPlusToken | SyntaxKind::MinusMinusToken
            | SyntaxKind::PlusToken | SyntaxKind::MinusToken
            | SyntaxKind::TildeToken | SyntaxKind::ExclamationToken => {
                let pos = self.token_pos();
                let operator = self.current_token();
                self.next_token();
                let operand = self.parse_unary_expression();
                let operand_ref = self.arena.alloc(operand);
                let end = self.token_end();
                Expression::PrefixUnary(PrefixUnaryExpression {
                    data: NodeData::new(SyntaxKind::PrefixUnaryExpression, pos, end),
                    operator, operand: operand_ref,
                })
            }
            SyntaxKind::TypeOfKeyword => {
                let pos = self.token_pos();
                self.next_token();
                let expr = self.parse_unary_expression();
                let expr_ref = self.arena.alloc(expr);
                let end = self.token_end();
                Expression::TypeOf(TypeOfExpression {
                    data: NodeData::new(SyntaxKind::TypeOfExpression, pos, end),
                    expression: expr_ref,
                })
            }
            SyntaxKind::DeleteKeyword => {
                let pos = self.token_pos();
                self.next_token();
                let expr = self.parse_unary_expression();
                let expr_ref = self.arena.alloc(expr);
                let end = self.token_end();
                Expression::Delete(DeleteExpression {
                    data: NodeData::new(SyntaxKind::DeleteExpression, pos, end),
                    expression: expr_ref,
                })
            }
            SyntaxKind::VoidKeyword => {
                let pos = self.token_pos();
                self.next_token();
                let expr = self.parse_unary_expression();
                let expr_ref = self.arena.alloc(expr);
                let end = self.token_end();
                Expression::Void(VoidExpression {
                    data: NodeData::new(SyntaxKind::VoidExpression, pos, end),
                    expression: expr_ref,
                })
            }
            SyntaxKind::AwaitKeyword => {
                let pos = self.token_pos();
                self.next_token();
                let expr = self.parse_unary_expression();
                let expr_ref = self.arena.alloc(expr);
                let end = self.token_end();
                Expression::Await(AwaitExpression {
                    data: NodeData::new(SyntaxKind::AwaitExpression, pos, end),
                    expression: expr_ref,
                })
            }
            _ => self.parse_postfix_expression(),
        }
    }

    fn parse_postfix_expression(&mut self) -> Expression<'a> {
        let mut expr = self.parse_left_hand_side_expression();
        if !self.scanner.has_preceding_line_break() {
            match self.current_token() {
                SyntaxKind::PlusPlusToken | SyntaxKind::MinusMinusToken => {
                    let pos = expr.data().range.pos;
                    let operator = self.current_token();
                    let end = self.token_end();
                    self.next_token();
                    let expr_ref = self.arena.alloc(expr);
                    return Expression::PostfixUnary(PostfixUnaryExpression {
                        data: NodeData::new(SyntaxKind::PostfixUnaryExpression, pos, end),
                        operand: expr_ref, operator,
                    });
                }
                SyntaxKind::ExclamationToken if !self.scanner.has_preceding_line_break() => {
                    // Non-null assertion: expr!
                    let pos = expr.data().range.pos;
                    let end = self.token_end();
                    self.next_token();
                    let expr_ref = self.arena.alloc(expr);
                    expr = Expression::NonNull(NonNullExpression {
                        data: NodeData::new(SyntaxKind::NonNullExpression, pos, end),
                        expression: expr_ref,
                    });
                }
                _ => {}
            }
        }
        expr
    }

    fn parse_left_hand_side_expression(&mut self) -> Expression<'a> {
        let mut expr = if self.current_token() == SyntaxKind::NewKeyword {
            self.parse_new_expression()
        } else {
            self.parse_primary_expression()
        };

        loop {
            match self.current_token() {
                SyntaxKind::DotToken => {
                    let pos = expr.data().range.pos;
                    self.next_token();
                    let name = self.parse_identifier();
                    let end = self.token_end();
                    let expr_ref = self.arena.alloc(expr);
                    expr = Expression::PropertyAccess(PropertyAccessExpression {
                        data: NodeData::new(SyntaxKind::PropertyAccessExpression, pos, end),
                        expression: expr_ref, question_dot_token: None,
                        name: MemberName::Identifier(name),
                    });
                }
                SyntaxKind::QuestionDotToken => {
                    let pos = expr.data().range.pos;
                    let qd = Token::new(SyntaxKind::QuestionDotToken, self.token_pos(), self.token_end());
                    self.next_token();
                    if self.current_token() == SyntaxKind::OpenBracketToken {
                        // a?.[expr]
                        self.next_token();
                        let arg = self.parse_expression();
                        let arg_ref = self.arena.alloc(arg);
                        let end = self.token_end();
                        self.expect_token(SyntaxKind::CloseBracketToken);
                        let expr_ref = self.arena.alloc(expr);
                        expr = Expression::ElementAccess(ElementAccessExpression {
                            data: NodeData::new(SyntaxKind::ElementAccessExpression, pos, end),
                            expression: expr_ref, question_dot_token: Some(qd),
                            argument_expression: arg_ref,
                        });
                    } else if self.current_token() == SyntaxKind::OpenParenToken {
                        // a?.()
                        let args = self.parse_argument_list();
                        let end = self.token_end();
                        let expr_ref = self.arena.alloc(expr);
                        expr = Expression::Call(CallExpression {
                            data: NodeData::new(SyntaxKind::CallExpression, pos, end),
                            expression: expr_ref, question_dot_token: Some(qd),
                            type_arguments: None, arguments: args,
                        });
                    } else {
                        // a?.b
                        let name = self.parse_identifier();
                        let end = self.token_end();
                        let expr_ref = self.arena.alloc(expr);
                        expr = Expression::PropertyAccess(PropertyAccessExpression {
                            data: NodeData::new(SyntaxKind::PropertyAccessExpression, pos, end),
                            expression: expr_ref, question_dot_token: Some(qd),
                            name: MemberName::Identifier(name),
                        });
                    }
                }
                SyntaxKind::OpenBracketToken => {
                    let pos = expr.data().range.pos;
                    self.next_token();
                    let argument = self.parse_expression();
                    let arg_ref = self.arena.alloc(argument);
                    let end = self.token_end();
                    self.expect_token(SyntaxKind::CloseBracketToken);
                    let expr_ref = self.arena.alloc(expr);
                    expr = Expression::ElementAccess(ElementAccessExpression {
                        data: NodeData::new(SyntaxKind::ElementAccessExpression, pos, end),
                        expression: expr_ref, question_dot_token: None,
                        argument_expression: arg_ref,
                    });
                }
                SyntaxKind::OpenParenToken => {
                    let pos = expr.data().range.pos;
                    let arguments = self.parse_argument_list();
                    let end = self.token_end();
                    let expr_ref = self.arena.alloc(expr);
                    expr = Expression::Call(CallExpression {
                        data: NodeData::new(SyntaxKind::CallExpression, pos, end),
                        expression: expr_ref, question_dot_token: None,
                        type_arguments: None, arguments,
                    });
                }
                SyntaxKind::NoSubstitutionTemplateLiteral | SyntaxKind::TemplateHead => {
                    // Tagged template
                    let pos = expr.data().range.pos;
                    let template_expr = self.parse_template_expression();
                    let template_ref = self.arena.alloc(template_expr);
                    let expr_ref = self.arena.alloc(expr);
                    let end = self.token_end();
                    expr = Expression::TaggedTemplate(TaggedTemplateExpression {
                        data: NodeData::new(SyntaxKind::TaggedTemplateExpression, pos, end),
                        tag: expr_ref, type_arguments: None, template: template_ref,
                    });
                }
                _ => break,
            }
        }

        expr
    }

    fn parse_new_expression(&mut self) -> Expression<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::NewKeyword);
        let callee = self.parse_primary_expression();
        // Handle chained member access on new target
        let mut expr = callee;
        while self.current_token() == SyntaxKind::DotToken {
            let epos = expr.data().range.pos;
            self.next_token();
            let name = self.parse_identifier();
            let eend = self.token_end();
            let expr_ref = self.arena.alloc(expr);
            expr = Expression::PropertyAccess(PropertyAccessExpression {
                data: NodeData::new(SyntaxKind::PropertyAccessExpression, epos, eend),
                expression: expr_ref, question_dot_token: None,
                name: MemberName::Identifier(name),
            });
        }
        let callee_ref = self.arena.alloc(expr);
        let type_arguments = self.try_parse_type_arguments();
        let arguments = if self.current_token() == SyntaxKind::OpenParenToken {
            Some(self.parse_argument_list())
        } else { None };
        let end = self.token_end();
        Expression::New(NewExpression {
            data: NodeData::new(SyntaxKind::NewExpression, pos, end),
            expression: callee_ref, type_arguments, arguments,
        })
    }

    fn parse_argument_list(&mut self) -> &'a [Expression<'a>] {
        self.expect_token(SyntaxKind::OpenParenToken);
        let mut args = Vec::new();
        while self.current_token() != SyntaxKind::CloseParenToken && self.current_token() != SyntaxKind::EndOfFileToken {
            if self.current_token() == SyntaxKind::DotDotDotToken {
                let spos = self.token_pos();
                self.next_token();
                let inner = self.parse_assignment_expression();
                let inner_ref = self.arena.alloc(inner);
                let send = self.token_end();
                args.push(Expression::Spread(SpreadElement {
                    data: NodeData::new(SyntaxKind::SpreadElement, spos, send),
                    expression: inner_ref,
                }));
            } else {
                args.push(self.parse_assignment_expression());
            }
            if self.optional_token(SyntaxKind::CommaToken).is_none() { break; }
        }
        self.expect_token(SyntaxKind::CloseParenToken);
        alloc_vec_in(self.arena, args)
    }

    fn parse_primary_expression(&mut self) -> Expression<'a> {
        match self.current_token() {
            SyntaxKind::Identifier => {
                let id = self.parse_identifier();
                // Arrow function: x =>
                if self.current_token() == SyntaxKind::EqualsGreaterThanToken && !self.scanner.has_preceding_line_break() {
                    return self.parse_arrow_function_after_identifier(id);
                }
                Expression::Identifier(id)
            }
            SyntaxKind::NumericLiteral => {
                let pos = self.token_pos();
                let end = self.token_end();
                let value = self.token_value().to_string();
                self.next_token();
                Expression::NumericLiteral(NumericLiteral {
                    data: NodeData::new(SyntaxKind::NumericLiteral, pos, end),
                    text: InternedString::dummy(),
                    text_name: value,
                    numeric_literal_flags: TokenFlags::NONE,
                })
            }
            SyntaxKind::StringLiteral => {
                let pos = self.token_pos();
                let end = self.token_end();
                let value = self.token_value().to_string();
                self.next_token();
                Expression::StringLiteral(StringLiteral {
                    data: NodeData::new(SyntaxKind::StringLiteral, pos, end),
                    text: InternedString::dummy(), text_name: value, is_single_quote: false,
                })
            }
            SyntaxKind::BigIntLiteral => {
                let pos = self.token_pos();
                let end = self.token_end();
                self.next_token();
                Expression::BigIntLiteral(BigIntLiteral {
                    data: NodeData::new(SyntaxKind::BigIntLiteral, pos, end),
                    text: InternedString::dummy(),
                })
            }
            SyntaxKind::RegularExpressionLiteral => {
                let pos = self.token_pos();
                let end = self.token_end();
                self.next_token();
                Expression::RegularExpressionLiteral(RegularExpressionLiteral {
                    data: NodeData::new(SyntaxKind::RegularExpressionLiteral, pos, end),
                    text: InternedString::dummy(),
                })
            }
            SyntaxKind::NoSubstitutionTemplateLiteral | SyntaxKind::TemplateHead => {
                self.parse_template_expression()
            }
            SyntaxKind::TrueKeyword => { let pos = self.token_pos(); let end = self.token_end(); self.next_token(); Expression::TrueKeyword(NodeData::new(SyntaxKind::TrueKeyword, pos, end)) }
            SyntaxKind::FalseKeyword => { let pos = self.token_pos(); let end = self.token_end(); self.next_token(); Expression::FalseKeyword(NodeData::new(SyntaxKind::FalseKeyword, pos, end)) }
            SyntaxKind::NullKeyword => { let pos = self.token_pos(); let end = self.token_end(); self.next_token(); Expression::NullKeyword(NodeData::new(SyntaxKind::NullKeyword, pos, end)) }
            SyntaxKind::ThisKeyword => { let pos = self.token_pos(); let end = self.token_end(); self.next_token(); Expression::ThisKeyword(NodeData::new(SyntaxKind::ThisKeyword, pos, end)) }
            SyntaxKind::SuperKeyword => { let pos = self.token_pos(); let end = self.token_end(); self.next_token(); Expression::SuperKeyword(NodeData::new(SyntaxKind::SuperKeyword, pos, end)) }
            SyntaxKind::OpenParenToken => self.parse_parenthesized_expression(),
            SyntaxKind::OpenBracketToken => self.parse_array_literal(),
            SyntaxKind::OpenBraceToken => self.parse_object_literal(),
            SyntaxKind::FunctionKeyword => self.parse_function_expression(),
            SyntaxKind::ClassKeyword => self.parse_class_expression(),
            SyntaxKind::SlashToken | SyntaxKind::SlashEqualsToken => {
                // Could be regex
                let kind = self.scanner.rescan_slash_token();
                if kind == SyntaxKind::RegularExpressionLiteral {
                    let pos = self.token_pos();
                    let end = self.token_end();
                    self.next_token();
                    return Expression::RegularExpressionLiteral(RegularExpressionLiteral {
                        data: NodeData::new(SyntaxKind::RegularExpressionLiteral, pos, end),
                        text: InternedString::dummy(),
                    });
                }
                self.parse_missing_expression()
            }
            SyntaxKind::AsyncKeyword => {
                // async function or async arrow
                let pos = self.token_pos();
                self.next_token();
                if self.current_token() == SyntaxKind::FunctionKeyword && !self.scanner.has_preceding_line_break() {
                    return self.parse_function_expression();
                }
                // Async arrow: async (params) => body
                if self.current_token() == SyntaxKind::OpenParenToken && !self.scanner.has_preceding_line_break() {
                    if self.is_parenthesized_arrow_function() {
                        return self.parse_parenthesized_arrow_function(pos);
                    }
                }
                // Async arrow: async x => body
                if self.current_token() == SyntaxKind::Identifier && !self.scanner.has_preceding_line_break() {
                    let saved = self.scanner.save_state();
                    let id = self.parse_identifier();
                    if self.current_token() == SyntaxKind::EqualsGreaterThanToken && !self.scanner.has_preceding_line_break() {
                        return self.parse_arrow_function_after_identifier(id);
                    }
                    // Not an arrow — restore and return `async` as identifier
                    self.scanner.restore_state(saved);
                }
                Expression::Identifier(Identifier {
                    data: NodeData::new(SyntaxKind::Identifier, pos, self.token_end()),
                    text: InternedString::dummy(), text_name: "async".to_string(),
                    original_keyword_kind: Some(SyntaxKind::AsyncKeyword),
                })
            }
            _ => self.parse_missing_expression(),
        }
    }

    fn parse_missing_expression(&mut self) -> Expression<'a> {
        let pos = self.token_pos();
        let end = self.token_end();
        self.error(&rscript_diagnostics::messages::EXPRESSION_EXPECTED, &[]);
        self.next_token();
        Expression::Identifier(Identifier {
            data: NodeData::new(SyntaxKind::Identifier, pos, end),
            text: InternedString::dummy(), text_name: String::new(), original_keyword_kind: None,
        })
    }

    fn parse_parenthesized_expression(&mut self) -> Expression<'a> {
        let pos = self.token_pos();

        // Use lookahead to decide if this is an arrow function or parenthesized expression.
        // This resolves the classic ambiguity: `(x: number) => x` vs `(x + y)`.
        if self.is_parenthesized_arrow_function() {
            return self.parse_parenthesized_arrow_function(pos);
        }

        // Not an arrow function — parse as parenthesized expression.
        self.expect_token(SyntaxKind::OpenParenToken);
        let inner = self.parse_expression();
        let inner_ref = self.arena.alloc(inner);
        let end = self.token_end();
        self.expect_token(SyntaxKind::CloseParenToken);

        // Edge case: `(expr) =>` where expr is a simple identifier
        if self.current_token() == SyntaxKind::EqualsGreaterThanToken && !self.scanner.has_preceding_line_break() {
            // Convert the expression to a parameter
            let params = self.expression_to_parameters(inner_ref);
            return self.parse_arrow_function_body(pos, params, None);
        }

        Expression::Parenthesized(ParenthesizedExpression {
            data: NodeData::new(SyntaxKind::ParenthesizedExpression, pos, end),
            expression: inner_ref,
        })
    }

    /// Lookahead to determine if a `(` starts an arrow function parameter list.
    /// Saves and restores scanner state.
    fn is_parenthesized_arrow_function(&mut self) -> bool {
        let saved = self.scanner.save_state();
        let result = self.is_parenthesized_arrow_function_inner();
        self.scanner.restore_state(saved);
        result
    }

    fn is_parenthesized_arrow_function_inner(&mut self) -> bool {
        // We're at `(` — skip it
        debug_assert_eq!(self.scanner.token(), SyntaxKind::OpenParenToken);
        self.scanner.scan();

        // `() =>` — definitely arrow
        if self.scanner.token() == SyntaxKind::CloseParenToken {
            self.scanner.scan();
            return self.scanner.token() == SyntaxKind::EqualsGreaterThanToken
                || self.scanner.token() == SyntaxKind::ColonToken; // (): T =>
        }

        // `(...` — rest parameter, definitely arrow
        if self.scanner.token() == SyntaxKind::DotDotDotToken {
            return true;
        }

        // Scan through tokens inside the parens. If we see patterns that can only appear
        // in a parameter list (type annotations on identifiers, multiple comma-separated
        // identifiers with type annotations, `?:`, `=`), it's an arrow function.
        let mut depth: u32 = 1;
        let mut first_token_after_open = true;
        while depth > 0 {
            let tok = self.scanner.token();
            match tok {
                SyntaxKind::OpenParenToken => { depth += 1; }
                SyntaxKind::CloseParenToken => {
                    depth -= 1;
                    if depth == 0 {
                        // We've reached the matching `)`. Check what follows.
                        self.scanner.scan();
                        let next = self.scanner.token();
                        // `) =>` — definitely arrow
                        if next == SyntaxKind::EqualsGreaterThanToken {
                            return true;
                        }
                        // `): type =>` — return type annotation followed by arrow
                        if next == SyntaxKind::ColonToken {
                            // Skip past the return type to find `=>`
                            return self.skip_type_annotation_and_check_arrow();
                        }
                        return false;
                    }
                }
                SyntaxKind::EndOfFileToken => return false,
                SyntaxKind::ColonToken if depth == 1 && first_token_after_open => {
                    // `(x:` — type annotation on first param, definitely arrow
                    // But only if what preceded the colon was an identifier
                    // (already consumed, so we just trust the pattern)
                    return true;
                }
                _ => {}
            }
            first_token_after_open = false;
            // After seeing an identifier, check if next is `:`, `?`, `,` or `=`
            if tok == SyntaxKind::Identifier && depth == 1 {
                self.scanner.scan();
                let after_id = self.scanner.token();
                match after_id {
                    SyntaxKind::ColonToken => return true,     // `id:` — type annotation
                    SyntaxKind::QuestionToken => return true,   // `id?` — optional param
                    SyntaxKind::EqualsToken => return true,     // `id =` — default value
                    SyntaxKind::CommaToken => {
                        // `id,` — could be arrow or tuple destructuring. Keep scanning.
                        self.scanner.scan();
                        continue;
                    }
                    SyntaxKind::CloseParenToken => {
                        // `(id)` — might be arrow `(id) =>`, continue to check `=>`
                        continue;
                    }
                    _ => {
                        self.scanner.scan();
                        continue;
                    }
                }
            }
            self.scanner.scan();
        }
        false
    }

    /// After `)` and `:`, skip past a type annotation and check if `=>` follows.
    fn skip_type_annotation_and_check_arrow(&mut self) -> bool {
        // We're past the `:` — skip tokens until we find `=>` or `{` at depth 0.
        // Handle nested parens, brackets, braces, and angle brackets.
        self.scanner.scan(); // skip past `:`
        let mut depth: u32 = 0;
        loop {
            let tok = self.scanner.token();
            match tok {
                SyntaxKind::EqualsGreaterThanToken if depth == 0 => return true,
                SyntaxKind::OpenParenToken | SyntaxKind::LessThanToken | SyntaxKind::OpenBracketToken => {
                    depth += 1;
                }
                SyntaxKind::CloseParenToken | SyntaxKind::GreaterThanToken | SyntaxKind::CloseBracketToken => {
                    if depth > 0 { depth -= 1; }
                }
                SyntaxKind::OpenBraceToken if depth == 0 => return false,
                SyntaxKind::SemicolonToken | SyntaxKind::EndOfFileToken => return false,
                _ => {}
            }
            self.scanner.scan();
        }
    }

    /// Parse a parenthesized arrow function: `(params) => body` or `(params): returnType => body`
    fn parse_parenthesized_arrow_function(&mut self, pos: u32) -> Expression<'a> {
        let type_parameters = self.try_parse_type_parameters();
        self.expect_token(SyntaxKind::OpenParenToken);
        let mut params = Vec::new();
        while self.current_token() != SyntaxKind::CloseParenToken && self.current_token() != SyntaxKind::EndOfFileToken {
            params.push(self.parse_parameter());
            if self.optional_token(SyntaxKind::CommaToken).is_none() { break; }
        }
        self.expect_token(SyntaxKind::CloseParenToken);
        let return_type = if self.optional_token(SyntaxKind::ColonToken).is_some() {
            Some(self.parse_type_and_alloc())
        } else {
            None
        };
        let parameters = alloc_vec_in(self.arena, params);
        let eq_token = self.expect_token(SyntaxKind::EqualsGreaterThanToken);
        let body = if self.current_token() == SyntaxKind::OpenBraceToken {
            let block = self.parse_block();
            ArrowFunctionBody::Block(self.arena.alloc(block))
        } else {
            let expr = self.parse_assignment_expression();
            ArrowFunctionBody::Expression(self.arena.alloc(expr))
        };
        let end = self.token_end();
        Expression::ArrowFunction(ArrowFunction {
            data: NodeData::new(SyntaxKind::ArrowFunction, pos, end),
            type_parameters, parameters, return_type,
            equals_greater_than_token: eq_token, body,
        })
    }

    /// Convert a parsed expression back into arrow function parameters.
    /// Handles the edge case: `(x) =>` where we already parsed `x` as an expression.
    fn expression_to_parameters(&mut self, expr: &'a Expression<'a>) -> &'a [ParameterDeclaration<'a>] {
        match expr {
            Expression::Identifier(id) => {
                let param = ParameterDeclaration {
                    data: id.data.clone(),
                    dot_dot_dot_token: None,
                    name: BindingName::Identifier(id.clone()),
                    question_token: None,
                    type_annotation: None,
                    initializer: None,
                };
                alloc_vec_in(self.arena, vec![param])
            }
            _ => {
                // Fallback: no params (best effort)
                &[]
            }
        }
    }

    fn parse_arrow_function_after_identifier(&mut self, id: Identifier) -> Expression<'a> {
        let pos = id.data.range.pos;
        let param = ParameterDeclaration {
            data: id.data.clone(),
            dot_dot_dot_token: None,
            name: BindingName::Identifier(id),
            question_token: None,
            type_annotation: None,
            initializer: None,
        };
        let params = alloc_vec_in(self.arena, vec![param]);
        self.parse_arrow_function_body(pos, params, None)
    }

    fn parse_arrow_function_body(&mut self, pos: u32, parameters: &'a [ParameterDeclaration<'a>], return_type: Option<&'a TypeNode<'a>>) -> Expression<'a> {
        let eq_token = self.expect_token(SyntaxKind::EqualsGreaterThanToken);
        let body = if self.current_token() == SyntaxKind::OpenBraceToken {
            let block = self.parse_block();
            ArrowFunctionBody::Block(self.arena.alloc(block))
        } else {
            let expr = self.parse_assignment_expression();
            ArrowFunctionBody::Expression(self.arena.alloc(expr))
        };
        let end = self.token_end();
        Expression::ArrowFunction(ArrowFunction {
            data: NodeData::new(SyntaxKind::ArrowFunction, pos, end),
            type_parameters: None, parameters, return_type,
            equals_greater_than_token: eq_token, body,
        })
    }

    fn parse_array_literal(&mut self) -> Expression<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::OpenBracketToken);
        let mut elements = Vec::new();
        while self.current_token() != SyntaxKind::CloseBracketToken && self.current_token() != SyntaxKind::EndOfFileToken {
            if self.current_token() == SyntaxKind::CommaToken {
                let epos = self.token_pos();
                let eend = self.token_end();
                elements.push(Expression::OmittedExpression(NodeData::new(SyntaxKind::OmittedExpression, epos, eend)));
            } else if self.current_token() == SyntaxKind::DotDotDotToken {
                let spos = self.token_pos();
                self.next_token();
                let inner = self.parse_assignment_expression();
                let inner_ref = self.arena.alloc(inner);
                let send = self.token_end();
                elements.push(Expression::Spread(SpreadElement {
                    data: NodeData::new(SyntaxKind::SpreadElement, spos, send),
                    expression: inner_ref,
                }));
            } else {
                elements.push(self.parse_assignment_expression());
            }
            if self.optional_token(SyntaxKind::CommaToken).is_none() { break; }
        }
        let end = self.token_end();
        self.expect_token(SyntaxKind::CloseBracketToken);
        Expression::ArrayLiteral(ArrayLiteralExpression {
            data: NodeData::new(SyntaxKind::ArrayLiteralExpression, pos, end),
            elements: alloc_vec_in(self.arena, elements), multi_line: false,
        })
    }

    fn parse_object_literal(&mut self) -> Expression<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::OpenBraceToken);
        let mut properties = Vec::new();
        while self.current_token() != SyntaxKind::CloseBraceToken && self.current_token() != SyntaxKind::EndOfFileToken {
            if self.current_token() == SyntaxKind::DotDotDotToken {
                let spos = self.token_pos();
                self.next_token();
                let inner = self.parse_assignment_expression();
                let inner_ref = self.arena.alloc(inner);
                let send = self.token_end();
                properties.push(ObjectLiteralElement::SpreadAssignment(SpreadAssignment {
                    data: NodeData::new(SyntaxKind::SpreadAssignment, spos, send),
                    expression: inner_ref,
                }));
            } else {
                properties.push(self.parse_object_literal_element());
            }
            if self.optional_token(SyntaxKind::CommaToken).is_none() { break; }
        }
        let end = self.token_end();
        self.expect_token(SyntaxKind::CloseBraceToken);
        Expression::ObjectLiteral(ObjectLiteralExpression {
            data: NodeData::new(SyntaxKind::ObjectLiteralExpression, pos, end),
            properties: alloc_vec_in(self.arena, properties), multi_line: false,
        })
    }

    fn parse_object_literal_element(&mut self) -> ObjectLiteralElement<'a> {
        let pos = self.token_pos();

        // Method: get/set accessor or async/generator method
        if (self.is_identifier_text("get") || self.is_identifier_text("set"))
            && self.current_token() == SyntaxKind::Identifier
        {
            // Could be get/set accessor or shorthand property
            // Simplified: treat as property for now unless followed by identifier + (
        }

        let name = self.parse_property_name();

        if self.current_token() == SyntaxKind::OpenParenToken || self.current_token() == SyntaxKind::LessThanToken {
            // Method
            let tp = self.try_parse_type_parameters();
            let (params, ret) = self.parse_parameter_list_and_return_type();
            let body = if self.current_token() == SyntaxKind::OpenBraceToken { Some(self.parse_block()) } else { None };
            let end = self.token_end();
            return ObjectLiteralElement::MethodDeclaration(MethodDeclaration {
                data: NodeData::new(SyntaxKind::MethodDeclaration, pos, end),
                name, question_token: None, asterisk_token: None,
                type_parameters: tp, parameters: params, return_type: ret, body,
            });
        }

        if self.optional_token(SyntaxKind::ColonToken).is_some() {
            // PropertyAssignment: key: value
            let value = self.parse_assignment_expression();
            let value_ref = self.arena.alloc(value);
            let end = self.token_end();
            return ObjectLiteralElement::PropertyAssignment(PropertyAssignment {
                data: NodeData::new(SyntaxKind::PropertyAssignment, pos, end),
                name, initializer: value_ref,
            });
        }

        // ShorthandPropertyAssignment: { x } or { x = default }
        if let PropertyName::Identifier(id) = name {
            let obj_init = if self.optional_token(SyntaxKind::EqualsToken).is_some() {
                Some(self.parse_assignment_expression_and_alloc())
            } else { None };
            let end = self.token_end();
            return ObjectLiteralElement::ShorthandPropertyAssignment(ShorthandPropertyAssignment {
                data: NodeData::new(SyntaxKind::ShorthandPropertyAssignment, pos, end),
                name: id, object_assignment_initializer: obj_init,
            });
        }

        // Fallback
        let end = self.token_end();
        let dummy_expr = self.arena.alloc(Expression::Identifier(Identifier {
            data: NodeData::new(SyntaxKind::Identifier, pos, end),
            text: InternedString::dummy(), text_name: String::new(), original_keyword_kind: None,
        }));
        ObjectLiteralElement::PropertyAssignment(PropertyAssignment {
            data: NodeData::new(SyntaxKind::PropertyAssignment, pos, end),
            name, initializer: dummy_expr,
        })
    }

    fn parse_template_expression(&mut self) -> Expression<'a> {
        let pos = self.token_pos();
        if self.current_token() == SyntaxKind::NoSubstitutionTemplateLiteral {
            let end = self.token_end();
            self.next_token();
            return Expression::NoSubstitutionTemplateLiteral(NoSubstitutionTemplateLiteral {
                data: NodeData::new(SyntaxKind::NoSubstitutionTemplateLiteral, pos, end),
                text: InternedString::dummy(), raw_text: None,
            });
        }

        // TemplateHead `text${
        let head_end = self.token_end();
        let head = Token::new(SyntaxKind::TemplateHead, pos, head_end);
        self.next_token();

        let mut spans = Vec::new();
        loop {
            let spos = self.token_pos();
            let expr = self.parse_expression();
            let expr_ref = self.arena.alloc(expr);

            // After expression, expect } which becomes template middle/tail
            let lit_token = if self.current_token() == SyntaxKind::CloseBraceToken {
                let kind = self.scanner.rescan_template_token();
                let lpos = self.scanner.token_start() as u32;
                let lend = self.scanner.token_end() as u32;
                self.next_token();
                Token::new(kind, lpos, lend)
            } else {
                let lpos = self.token_pos();
                let lend = self.token_end();
                self.next_token();
                Token::new(SyntaxKind::TemplateTail, lpos, lend)
            };
            let is_tail = lit_token.data.kind == SyntaxKind::TemplateTail;
            let send = self.token_end();
            spans.push(TemplateSpan {
                data: NodeData::new(SyntaxKind::TemplateSpan, spos, send),
                expression: expr_ref, literal: lit_token,
            });
            if is_tail || self.current_token() == SyntaxKind::EndOfFileToken { break; }
        }
        let end = self.token_end();
        Expression::TemplateExpression(TemplateExpression {
            data: NodeData::new(SyntaxKind::TemplateExpression, pos, end),
            head, template_spans: alloc_vec_in(self.arena, spans),
        })
    }

    fn parse_function_expression(&mut self) -> Expression<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::FunctionKeyword);
        let asterisk_token = self.optional_token(SyntaxKind::AsteriskToken);
        let name = if self.current_token() == SyntaxKind::Identifier {
            Some(self.parse_identifier())
        } else { None };
        let type_parameters = self.try_parse_type_parameters();
        let (parameters, return_type) = self.parse_parameter_list_and_return_type();
        let body = self.parse_block();
        let body_ref = self.arena.alloc(body);
        let end = self.token_end();
        Expression::FunctionExpression(FunctionExpression {
            data: NodeData::new(SyntaxKind::FunctionExpression, pos, end),
            name, asterisk_token, type_parameters, parameters, return_type, body: body_ref,
        })
    }

    fn parse_class_expression(&mut self) -> Expression<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::ClassKeyword);
        let name = if self.current_token() == SyntaxKind::Identifier {
            Some(self.parse_identifier())
        } else { None };
        let type_parameters = self.try_parse_type_parameters();
        let heritage_clauses = self.parse_heritage_clauses();
        let members = self.parse_class_members();
        let end = self.token_end();
        Expression::ClassExpression(ClassExpression {
            data: NodeData::new(SyntaxKind::ClassExpression, pos, end),
            name, type_parameters, heritage_clauses, members,
        })
    }

    fn parse_yield_expression(&mut self) -> Expression<'a> {
        let pos = self.token_pos();
        self.expect_token(SyntaxKind::YieldKeyword);
        let asterisk_token = self.optional_token(SyntaxKind::AsteriskToken);
        let expression = if !self.scanner.has_preceding_line_break()
            && self.current_token() != SyntaxKind::SemicolonToken
            && self.current_token() != SyntaxKind::CloseBraceToken
            && self.current_token() != SyntaxKind::EndOfFileToken
        {
            Some(self.parse_assignment_expression_and_alloc())
        } else { None };
        let end = self.token_end();
        Expression::Yield(YieldExpression {
            data: NodeData::new(SyntaxKind::YieldExpression, pos, end),
            asterisk_token, expression,
        })
    }
}

// ========================================================================
// Helper trait for getting NodeData from enum variants
// ========================================================================

trait HasNodeData {
    fn data(&self) -> &NodeData;
}

impl HasNodeData for Expression<'_> {
    fn data(&self) -> &NodeData {
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
            Expression::OmittedExpression(n) => n,
            Expression::As(n) => &n.data,
            Expression::NonNull(n) => &n.data,
            Expression::MetaProperty(n) => &n.data,
            Expression::Satisfies(n) => &n.data,
            Expression::ThisKeyword(n) | Expression::SuperKeyword(n)
            | Expression::NullKeyword(n) | Expression::TrueKeyword(n)
            | Expression::FalseKeyword(n) => n,
        }
    }
}

impl HasNodeData for TypeNode<'_> {
    fn data(&self) -> &NodeData {
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

impl HasNodeData for EntityName<'_> {
    fn data(&self) -> &NodeData {
        match self {
            EntityName::Identifier(n) => &n.data,
            EntityName::QualifiedName(n) => &n.data,
        }
    }
}
