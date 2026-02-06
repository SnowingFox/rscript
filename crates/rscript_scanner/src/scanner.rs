//! The TypeScript scanner/lexer.
//!
//! This is a faithful port of TypeScript's scanner.ts. It converts source text
//! into a stream of tokens that the parser consumes.

use crate::char_codes::*;
use crate::token::TokenInfo;
use rscript_ast::syntax_kind::SyntaxKind;
use rscript_ast::types::TokenFlags;
use rscript_diagnostics::{Diagnostic, DiagnosticCollection};

/// Saved scanner state for lookahead.
pub struct ScannerState {
    pub pos: usize,
    pub token_start: usize,
    pub token: SyntaxKind,
    pub token_value: String,
    pub token_flags: TokenFlags,
}

/// The scanner converts TypeScript source text into tokens.
pub struct Scanner {
    /// The source text being scanned.
    text: Vec<char>,
    /// Current position in the text.
    pos: usize,
    /// Start of the current token (after leading trivia).
    token_start: usize,
    /// The current token kind.
    token: SyntaxKind,
    /// The text of the current token.
    token_value: String,
    /// Token flags for the current token.
    token_flags: TokenFlags,
    /// Whether we are scanning JSX.
    in_jsx: bool,
    /// Accumulated diagnostics.
    diagnostics: DiagnosticCollection,
}

impl Scanner {
    /// Create a new scanner for the given source text.
    pub fn new(text: &str) -> Self {
        Self {
            text: text.chars().collect(),
            pos: 0,
            token_start: 0,
            token: SyntaxKind::Unknown,
            token_value: String::new(),
            token_flags: TokenFlags::NONE,
            in_jsx: false,
            diagnostics: DiagnosticCollection::new(),
        }
    }

    /// Set whether the scanner is in a JSX context.
    pub fn set_in_jsx(&mut self, in_jsx: bool) {
        self.in_jsx = in_jsx;
    }

    /// Skip a shebang line at the very beginning of the file (e.g., `#!/usr/bin/env node`).
    /// Call this before the first `scan()` call.
    pub fn skip_shebang(&mut self) {
        if self.pos == 0 && self.text.len() >= 2 && self.text[0] == '#' && self.text[1] == '!' {
            self.pos = 2;
            while !self.is_eof() && !is_line_break(self.text[self.pos]) {
                self.pos += 1;
            }
        }
    }

    /// Get the full source text length.
    pub fn text_len(&self) -> usize {
        self.text.len()
    }

    /// Look ahead: save position, call f, restore position and return result.
    pub fn look_ahead<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        let save_pos = self.pos;
        let save_start = self.token_start;
        let save_token = self.token;
        let save_value = self.token_value.clone();
        let save_flags = self.token_flags;
        let result = f(self);
        self.pos = save_pos;
        self.token_start = save_start;
        self.token = save_token;
        self.token_value = save_value;
        self.token_flags = save_flags;
        result
    }

    /// Try scanning: save state, call f, if result is None restore state.
    pub fn try_scan<T>(&mut self, f: impl FnOnce(&mut Self) -> Option<T>) -> Option<T> {
        let save_pos = self.pos;
        let save_start = self.token_start;
        let save_token = self.token;
        let save_value = self.token_value.clone();
        let save_flags = self.token_flags;
        let result = f(self);
        if result.is_none() {
            self.pos = save_pos;
            self.token_start = save_start;
            self.token = save_token;
            self.token_value = save_value;
            self.token_flags = save_flags;
        }
        result
    }

    /// Get the current token kind.
    #[inline]
    pub fn token(&self) -> SyntaxKind {
        self.token
    }

    /// Get the current token's text value.
    #[inline]
    pub fn token_value(&self) -> &str {
        &self.token_value
    }

    /// Get the start position of the current token (after trivia).
    #[inline]
    pub fn token_start(&self) -> usize {
        self.token_start
    }

    /// Get the current position (end of current token).
    #[inline]
    pub fn token_end(&self) -> usize {
        self.pos
    }

    /// Get the current token flags.
    #[inline]
    pub fn token_flags(&self) -> TokenFlags {
        self.token_flags
    }

    /// Whether the current token was preceded by a line break.
    #[inline]
    pub fn has_preceding_line_break(&self) -> bool {
        self.token_flags
            .contains(TokenFlags::PRECEDING_LINE_BREAK)
    }

    /// Get the accumulated diagnostics.
    pub fn diagnostics(&self) -> &DiagnosticCollection {
        &self.diagnostics
    }

    /// Take the accumulated diagnostics, leaving an empty collection.
    pub fn take_diagnostics(&mut self) -> DiagnosticCollection {
        std::mem::take(&mut self.diagnostics)
    }

    /// Get a TokenInfo for the current token.
    pub fn token_info(&self) -> TokenInfo {
        TokenInfo {
            kind: self.token,
            pos: self.token_start as u32,
            end: self.pos as u32,
            text: self.token_value.clone(),
            flags: self.token_flags,
        }
    }

    /// Save the full scanner state for lookahead.
    pub fn save_state(&self) -> ScannerState {
        ScannerState {
            pos: self.pos,
            token_start: self.token_start,
            token: self.token,
            token_value: self.token_value.clone(),
            token_flags: self.token_flags,
        }
    }

    /// Restore the full scanner state from a saved state.
    pub fn restore_state(&mut self, state: ScannerState) {
        self.pos = state.pos;
        self.token_start = state.token_start;
        self.token = state.token;
        self.token_value = state.token_value;
        self.token_flags = state.token_flags;
    }

    /// Reset the scanner to a specific position.
    pub fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
        self.token_start = pos;
        self.token = SyntaxKind::Unknown;
        self.token_value.clear();
        self.token_flags = TokenFlags::NONE;
    }

    // ========================================================================
    // Core scanning
    // ========================================================================

    /// Look at the character at the current position without advancing.
    #[inline]
    fn current_char(&self) -> Option<char> {
        self.text.get(self.pos).copied()
    }

    /// Look at the character at position pos + offset.
    #[inline]
    fn char_at(&self, offset: usize) -> Option<char> {
        self.text.get(self.pos + offset).copied()
    }

    /// Advance position by one character and return the character.
    #[inline]
    fn next_char(&mut self) -> Option<char> {
        self.pos += 1;
        self.current_char()
    }

    /// Whether we've reached the end of the text.
    #[inline]
    fn is_eof(&self) -> bool {
        self.pos >= self.text.len()
    }

    /// Skip whitespace and comments (trivia), setting token_flags for line breaks.
    fn skip_trivia(&mut self) {
        loop {
            if self.is_eof() {
                return;
            }
            let ch = self.text[self.pos];
            match ch {
                '\r' => {
                    self.token_flags |= TokenFlags::PRECEDING_LINE_BREAK;
                    self.pos += 1;
                    if self.current_char() == Some('\n') {
                        self.pos += 1;
                    }
                }
                '\n' | '\u{2028}' | '\u{2029}' => {
                    self.token_flags |= TokenFlags::PRECEDING_LINE_BREAK;
                    self.pos += 1;
                }
                '\t' | '\u{000B}' | '\u{000C}' | ' ' | '\u{00A0}' | '\u{FEFF}' => {
                    self.pos += 1;
                }
                '/' => {
                    if self.char_at(1) == Some('/') {
                        // Single-line comment
                        self.pos += 2;
                        while !self.is_eof() {
                            if is_line_break(self.text[self.pos]) {
                                break;
                            }
                            self.pos += 1;
                        }
                    } else if self.char_at(1) == Some('*') {
                        // Multi-line comment
                        self.pos += 2;
                        while !self.is_eof() {
                            if self.text[self.pos] == '*' && self.char_at(1) == Some('/') {
                                self.pos += 2;
                                break;
                            }
                            if is_line_break(self.text[self.pos]) {
                                self.token_flags |= TokenFlags::PRECEDING_LINE_BREAK;
                            }
                            self.pos += 1;
                        }
                    } else {
                        return;
                    }
                }
                c if is_white_space_single_line(c) => {
                    self.pos += 1;
                }
                '<' | '>' | '=' | '|' if self.is_conflict_marker_trivia() => {
                    self.try_skip_conflict_marker();
                }
                _ => return,
            }
        }
    }

    /// Scan the next token and return its kind.
    pub fn scan(&mut self) -> SyntaxKind {
        self.token_flags = TokenFlags::NONE;
        self.token_value.clear();

        // Skip trivia (whitespace, comments)
        self.skip_trivia();
        self.token_start = self.pos;

        if self.is_eof() {
            self.token = SyntaxKind::EndOfFileToken;
            return self.token;
        }

        let ch = self.text[self.pos];
        self.token = match ch {
            '(' => { self.pos += 1; SyntaxKind::OpenParenToken }
            ')' => { self.pos += 1; SyntaxKind::CloseParenToken }
            '{' => { self.pos += 1; SyntaxKind::OpenBraceToken }
            '}' => { self.pos += 1; SyntaxKind::CloseBraceToken }
            '[' => { self.pos += 1; SyntaxKind::OpenBracketToken }
            ']' => { self.pos += 1; SyntaxKind::CloseBracketToken }
            ';' => { self.pos += 1; SyntaxKind::SemicolonToken }
            ',' => { self.pos += 1; SyntaxKind::CommaToken }
            '~' => { self.pos += 1; SyntaxKind::TildeToken }
            '@' => { self.pos += 1; SyntaxKind::AtToken }
            '#' => { self.pos += 1; SyntaxKind::HashToken }

            '.' => self.scan_dot(),
            ':' => { self.pos += 1; SyntaxKind::ColonToken }
            '?' => self.scan_question(),
            '<' => self.scan_less_than(),
            '>' => self.scan_greater_than(),
            '=' => self.scan_equals(),
            '!' => self.scan_exclamation(),
            '+' => self.scan_plus(),
            '-' => self.scan_minus(),
            '*' => self.scan_asterisk(),
            '/' => self.scan_slash(),
            '%' => self.scan_percent(),
            '&' => self.scan_ampersand(),
            '|' => self.scan_bar(),
            '^' => self.scan_caret(),

            '\'' | '"' => self.scan_string_literal(ch),
            '`' => self.scan_template_literal(),

            '0'..='9' => self.scan_number(),

            _ if is_identifier_start(ch) => self.scan_identifier(),

            _ => {
                self.pos += 1;
                self.diagnostics.add(Diagnostic::new(
                    &rscript_diagnostics::messages::INVALID_CHARACTER,
                    &[],
                ));
                SyntaxKind::Unknown
            }
        };

        self.token
    }

    // ========================================================================
    // Token-specific scanning methods
    // ========================================================================

    fn scan_dot(&mut self) -> SyntaxKind {
        if self.char_at(1) == Some('.') && self.char_at(2) == Some('.') {
            self.pos += 3;
            SyntaxKind::DotDotDotToken
        } else if self.char_at(1).map_or(false, is_digit) {
            self.scan_number()
        } else {
            self.pos += 1;
            SyntaxKind::DotToken
        }
    }

    fn scan_question(&mut self) -> SyntaxKind {
        if self.char_at(1) == Some('?') {
            if self.char_at(2) == Some('=') {
                self.pos += 3;
                SyntaxKind::QuestionQuestionEqualsToken
            } else {
                self.pos += 2;
                SyntaxKind::QuestionQuestionToken
            }
        } else if self.char_at(1) == Some('.') && !self.char_at(2).map_or(false, is_digit) {
            self.pos += 2;
            SyntaxKind::QuestionDotToken
        } else {
            self.pos += 1;
            SyntaxKind::QuestionToken
        }
    }

    fn scan_less_than(&mut self) -> SyntaxKind {
        if self.char_at(1) == Some('<') {
            if self.char_at(2) == Some('=') {
                self.pos += 3;
                SyntaxKind::LessThanLessThanEqualsToken
            } else {
                self.pos += 2;
                SyntaxKind::LessThanLessThanToken
            }
        } else if self.char_at(1) == Some('=') {
            self.pos += 2;
            SyntaxKind::LessThanEqualsToken
        } else if self.char_at(1) == Some('/') {
            self.pos += 2;
            SyntaxKind::LessThanSlashToken
        } else {
            self.pos += 1;
            SyntaxKind::LessThanToken
        }
    }

    fn scan_greater_than(&mut self) -> SyntaxKind {
        // Note: nested >> and >>> are handled during parsing for type arguments
        self.pos += 1;
        SyntaxKind::GreaterThanToken
    }

    fn scan_equals(&mut self) -> SyntaxKind {
        if self.char_at(1) == Some('=') {
            if self.char_at(2) == Some('=') {
                self.pos += 3;
                SyntaxKind::EqualsEqualsEqualsToken
            } else {
                self.pos += 2;
                SyntaxKind::EqualsEqualsToken
            }
        } else if self.char_at(1) == Some('>') {
            self.pos += 2;
            SyntaxKind::EqualsGreaterThanToken
        } else {
            self.pos += 1;
            SyntaxKind::EqualsToken
        }
    }

    fn scan_exclamation(&mut self) -> SyntaxKind {
        if self.char_at(1) == Some('=') {
            if self.char_at(2) == Some('=') {
                self.pos += 3;
                SyntaxKind::ExclamationEqualsEqualsToken
            } else {
                self.pos += 2;
                SyntaxKind::ExclamationEqualsToken
            }
        } else {
            self.pos += 1;
            SyntaxKind::ExclamationToken
        }
    }

    fn scan_plus(&mut self) -> SyntaxKind {
        if self.char_at(1) == Some('+') {
            self.pos += 2;
            SyntaxKind::PlusPlusToken
        } else if self.char_at(1) == Some('=') {
            self.pos += 2;
            SyntaxKind::PlusEqualsToken
        } else {
            self.pos += 1;
            SyntaxKind::PlusToken
        }
    }

    fn scan_minus(&mut self) -> SyntaxKind {
        if self.char_at(1) == Some('-') {
            self.pos += 2;
            SyntaxKind::MinusMinusToken
        } else if self.char_at(1) == Some('=') {
            self.pos += 2;
            SyntaxKind::MinusEqualsToken
        } else {
            self.pos += 1;
            SyntaxKind::MinusToken
        }
    }

    fn scan_asterisk(&mut self) -> SyntaxKind {
        if self.char_at(1) == Some('*') {
            if self.char_at(2) == Some('=') {
                self.pos += 3;
                SyntaxKind::AsteriskAsteriskEqualsToken
            } else {
                self.pos += 2;
                SyntaxKind::AsteriskAsteriskToken
            }
        } else if self.char_at(1) == Some('=') {
            self.pos += 2;
            SyntaxKind::AsteriskEqualsToken
        } else {
            self.pos += 1;
            SyntaxKind::AsteriskToken
        }
    }

    fn scan_slash(&mut self) -> SyntaxKind {
        // Comments are handled in skip_trivia, so if we get here it's division or regex
        if self.char_at(1) == Some('=') {
            self.pos += 2;
            SyntaxKind::SlashEqualsToken
        } else {
            self.pos += 1;
            SyntaxKind::SlashToken
        }
    }

    fn scan_percent(&mut self) -> SyntaxKind {
        if self.char_at(1) == Some('=') {
            self.pos += 2;
            SyntaxKind::PercentEqualsToken
        } else {
            self.pos += 1;
            SyntaxKind::PercentToken
        }
    }

    fn scan_ampersand(&mut self) -> SyntaxKind {
        if self.char_at(1) == Some('&') {
            if self.char_at(2) == Some('=') {
                self.pos += 3;
                SyntaxKind::AmpersandAmpersandEqualsToken
            } else {
                self.pos += 2;
                SyntaxKind::AmpersandAmpersandToken
            }
        } else if self.char_at(1) == Some('=') {
            self.pos += 2;
            SyntaxKind::AmpersandEqualsToken
        } else {
            self.pos += 1;
            SyntaxKind::AmpersandToken
        }
    }

    fn scan_bar(&mut self) -> SyntaxKind {
        if self.char_at(1) == Some('|') {
            if self.char_at(2) == Some('=') {
                self.pos += 3;
                SyntaxKind::BarBarEqualsToken
            } else {
                self.pos += 2;
                SyntaxKind::BarBarToken
            }
        } else if self.char_at(1) == Some('=') {
            self.pos += 2;
            SyntaxKind::BarEqualsToken
        } else {
            self.pos += 1;
            SyntaxKind::BarToken
        }
    }

    fn scan_caret(&mut self) -> SyntaxKind {
        if self.char_at(1) == Some('=') {
            self.pos += 2;
            SyntaxKind::CaretEqualsToken
        } else {
            self.pos += 1;
            SyntaxKind::CaretToken
        }
    }

    fn scan_string_literal(&mut self, quote: char) -> SyntaxKind {
        self.pos += 1; // skip opening quote
        let mut result = String::new();
        loop {
            if self.is_eof() {
                self.diagnostics.add(Diagnostic::new(
                    &rscript_diagnostics::messages::UNTERMINATED_STRING_LITERAL,
                    &[],
                ));
                self.token_flags |= TokenFlags::UNTERMINATED;
                break;
            }
            let ch = self.text[self.pos];
            if ch == quote {
                self.pos += 1;
                break;
            }
            if ch == '\\' {
                result.push(ch);
                self.pos += 1;
                if !self.is_eof() {
                    result.push(self.text[self.pos]);
                    self.pos += 1;
                }
                continue;
            }
            if is_line_break(ch) {
                self.diagnostics.add(Diagnostic::new(
                    &rscript_diagnostics::messages::UNTERMINATED_STRING_LITERAL,
                    &[],
                ));
                self.token_flags |= TokenFlags::UNTERMINATED;
                break;
            }
            result.push(ch);
            self.pos += 1;
        }
        self.token_value = result;
        SyntaxKind::StringLiteral
    }

    fn scan_template_literal(&mut self) -> SyntaxKind {
        self.pos += 1; // skip backtick
        let mut result = String::new();
        loop {
            if self.is_eof() {
                self.diagnostics.add(Diagnostic::new(
                    &rscript_diagnostics::messages::UNTERMINATED_TEMPLATE_LITERAL,
                    &[],
                ));
                self.token_flags |= TokenFlags::UNTERMINATED;
                self.token_value = result;
                return SyntaxKind::NoSubstitutionTemplateLiteral;
            }
            let ch = self.text[self.pos];
            if ch == '`' {
                self.pos += 1;
                self.token_value = result;
                return SyntaxKind::NoSubstitutionTemplateLiteral;
            }
            if ch == '$' && self.char_at(1) == Some('{') {
                self.pos += 2;
                self.token_value = result;
                return SyntaxKind::TemplateHead;
            }
            if ch == '\\' {
                result.push(ch);
                self.pos += 1;
                if !self.is_eof() {
                    result.push(self.text[self.pos]);
                    self.pos += 1;
                }
                continue;
            }
            if is_line_break(ch) {
                result.push(ch);
                self.pos += 1;
                if ch == '\r' && self.current_char() == Some('\n') {
                    result.push('\n');
                    self.pos += 1;
                }
                continue;
            }
            result.push(ch);
            self.pos += 1;
        }
    }

    /// Scan a template middle or tail (called after `}` in template expression).
    pub fn rescan_template_token(&mut self) -> SyntaxKind {
        self.token_start = self.pos;
        self.token_value.clear();
        let mut result = String::new();

        loop {
            if self.is_eof() {
                self.diagnostics.add(Diagnostic::new(
                    &rscript_diagnostics::messages::UNTERMINATED_TEMPLATE_LITERAL,
                    &[],
                ));
                self.token_flags |= TokenFlags::UNTERMINATED;
                self.token_value = result;
                self.token = SyntaxKind::TemplateTail;
                return self.token;
            }
            let ch = self.text[self.pos];
            if ch == '`' {
                self.pos += 1;
                self.token_value = result;
                self.token = SyntaxKind::TemplateTail;
                return self.token;
            }
            if ch == '$' && self.char_at(1) == Some('{') {
                self.pos += 2;
                self.token_value = result;
                self.token = SyntaxKind::TemplateMiddle;
                return self.token;
            }
            if ch == '\\' {
                result.push(ch);
                self.pos += 1;
                if !self.is_eof() {
                    result.push(self.text[self.pos]);
                    self.pos += 1;
                }
                continue;
            }
            result.push(ch);
            self.pos += 1;
        }
    }

    /// Rescan the current token as a regex literal (called by the parser).
    pub fn rescan_slash_token(&mut self) -> SyntaxKind {
        self.pos = self.token_start + 1; // after the /
        let mut result = String::from("/");
        let mut in_character_class = false;

        loop {
            if self.is_eof() {
                self.diagnostics.add(Diagnostic::new(
                    &rscript_diagnostics::messages::UNTERMINATED_REGULAR_EXPRESSION_LITERAL,
                    &[],
                ));
                self.token_flags |= TokenFlags::UNTERMINATED;
                break;
            }
            let ch = self.text[self.pos];
            if is_line_break(ch) {
                self.diagnostics.add(Diagnostic::new(
                    &rscript_diagnostics::messages::UNTERMINATED_REGULAR_EXPRESSION_LITERAL,
                    &[],
                ));
                self.token_flags |= TokenFlags::UNTERMINATED;
                break;
            }
            if ch == '\\' {
                result.push(ch);
                self.pos += 1;
                if !self.is_eof() && !is_line_break(self.text[self.pos]) {
                    result.push(self.text[self.pos]);
                    self.pos += 1;
                }
                continue;
            }
            if ch == '[' {
                in_character_class = true;
            } else if ch == ']' {
                in_character_class = false;
            } else if ch == '/' && !in_character_class {
                result.push(ch);
                self.pos += 1;
                // Scan flags
                while !self.is_eof() && is_identifier_part(self.text[self.pos]) {
                    result.push(self.text[self.pos]);
                    self.pos += 1;
                }
                break;
            }
            result.push(ch);
            self.pos += 1;
        }

        self.token_value = result;
        self.token = SyntaxKind::RegularExpressionLiteral;
        self.token
    }

    /// Rescan `>` as `>=`, `>>`, `>>=`, `>>>`, or `>>>=`.
    pub fn rescan_greater_than_token(&mut self) -> SyntaxKind {
        if self.token == SyntaxKind::GreaterThanToken {
            if self.current_char() == Some('>') {
                if self.char_at(1) == Some('>') {
                    if self.char_at(2) == Some('=') {
                        self.pos += 3;
                        self.token = SyntaxKind::GreaterThanGreaterThanGreaterThanEqualsToken;
                    } else {
                        self.pos += 2;
                        self.token = SyntaxKind::GreaterThanGreaterThanGreaterThanToken;
                    }
                } else if self.char_at(1) == Some('=') {
                    self.pos += 2;
                    self.token = SyntaxKind::GreaterThanGreaterThanEqualsToken;
                } else {
                    self.pos += 1;
                    self.token = SyntaxKind::GreaterThanGreaterThanToken;
                }
            } else if self.current_char() == Some('=') {
                self.pos += 1;
                self.token = SyntaxKind::GreaterThanEqualsToken;
            }
        }
        self.token
    }

    fn scan_number(&mut self) -> SyntaxKind {
        let start = self.pos;
        let first_char = self.text[self.pos];

        if first_char == '0' {
            match self.char_at(1) {
                Some('x') | Some('X') => return self.scan_hex_number(start),
                Some('b') | Some('B') => return self.scan_binary_number(start),
                Some('o') | Some('O') => return self.scan_octal_number(start),
                _ => {}
            }
        }

        // Decimal number
        self.scan_digits();

        if self.current_char() == Some('.') {
            self.pos += 1;
            self.scan_digits();
        }

        // Exponent
        if let Some('e') | Some('E') = self.current_char() {
            self.pos += 1;
            self.token_flags |= TokenFlags::SCIENTIFIC;
            if let Some('+') | Some('-') = self.current_char() {
                self.pos += 1;
            }
            self.scan_digits();
        }

        // BigInt suffix
        if self.current_char() == Some('n') {
            self.pos += 1;
            self.token_value = self.chars_to_string(start, self.pos);
            return SyntaxKind::BigIntLiteral;
        }

        self.token_value = self.chars_to_string(start, self.pos);
        SyntaxKind::NumericLiteral
    }

    fn scan_hex_number(&mut self, start: usize) -> SyntaxKind {
        self.pos += 2; // skip 0x
        self.token_flags |= TokenFlags::HEX_SPECIFIER;
        self.scan_hex_digits();
        if self.current_char() == Some('n') {
            self.pos += 1;
            self.token_value = self.chars_to_string(start, self.pos);
            return SyntaxKind::BigIntLiteral;
        }
        self.token_value = self.chars_to_string(start, self.pos);
        SyntaxKind::NumericLiteral
    }

    fn scan_binary_number(&mut self, start: usize) -> SyntaxKind {
        self.pos += 2; // skip 0b
        self.token_flags |= TokenFlags::BINARY_SPECIFIER;
        while !self.is_eof() {
            let ch = self.text[self.pos];
            if ch == '_' {
                self.token_flags |= TokenFlags::CONTAINS_SEPARATOR;
                self.pos += 1;
            } else if ch == '0' || ch == '1' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.current_char() == Some('n') {
            self.pos += 1;
            self.token_value = self.chars_to_string(start, self.pos);
            return SyntaxKind::BigIntLiteral;
        }
        self.token_value = self.chars_to_string(start, self.pos);
        SyntaxKind::NumericLiteral
    }

    fn scan_octal_number(&mut self, start: usize) -> SyntaxKind {
        self.pos += 2; // skip 0o
        self.token_flags |= TokenFlags::OCTAL_SPECIFIER;
        while !self.is_eof() {
            let ch = self.text[self.pos];
            if ch == '_' {
                self.token_flags |= TokenFlags::CONTAINS_SEPARATOR;
                self.pos += 1;
            } else if ch >= '0' && ch <= '7' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.current_char() == Some('n') {
            self.pos += 1;
            self.token_value = self.chars_to_string(start, self.pos);
            return SyntaxKind::BigIntLiteral;
        }
        self.token_value = self.chars_to_string(start, self.pos);
        SyntaxKind::NumericLiteral
    }

    fn scan_digits(&mut self) {
        while !self.is_eof() {
            let ch = self.text[self.pos];
            if ch == '_' {
                self.token_flags |= TokenFlags::CONTAINS_SEPARATOR;
                self.pos += 1;
            } else if is_digit(ch) {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn scan_hex_digits(&mut self) {
        while !self.is_eof() {
            let ch = self.text[self.pos];
            if ch == '_' {
                self.token_flags |= TokenFlags::CONTAINS_SEPARATOR;
                self.pos += 1;
            } else if is_hex_digit(ch) {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn scan_identifier(&mut self) -> SyntaxKind {
        let start = self.pos;
        self.pos += 1;
        while !self.is_eof() && is_identifier_part(self.text[self.pos]) {
            self.pos += 1;
        }
        let text = self.chars_to_string(start, self.pos);

        // Check if it's a keyword
        if let Some(keyword) = SyntaxKind::from_keyword(&text) {
            self.token_value = text;
            return keyword;
        }

        self.token_value = text;
        SyntaxKind::Identifier
    }

    /// Scan JSX text content (everything between tags that isn't `{` or `<`).
    pub fn scan_jsx_text(&mut self) -> SyntaxKind {
        self.token_start = self.pos;
        self.token_flags = TokenFlags::NONE;
        let mut result = String::new();

        while !self.is_eof() {
            let ch = self.text[self.pos];
            if ch == '{' || ch == '<' {
                break;
            }
            if ch == '>' {
                // Treat `>` as jsx text if encountered (shouldn't happen in valid JSX)
                // but TypeScript includes it
            }
            result.push(ch);
            self.pos += 1;
        }

        self.token_value = result;
        if self.token_value.is_empty() {
            // At a `{` or `<`, don't return jsx text
            return self.scan();
        }
        self.token = SyntaxKind::JsxText;
        self.token
    }

    /// Scan a JSX token - like scan() but in a JSX context.
    /// Returns JsxText for text content, or normal tokens for `{`, `<`, `</>`, etc.
    pub fn scan_jsx_token(&mut self) -> SyntaxKind {
        self.token_flags = TokenFlags::NONE;
        self.token_value.clear();
        self.token_start = self.pos;

        if self.is_eof() {
            self.token = SyntaxKind::EndOfFileToken;
            return self.token;
        }

        let ch = self.text[self.pos];
        match ch {
            '{' => {
                self.pos += 1;
                self.token = SyntaxKind::OpenBraceToken;
            }
            '<' => {
                if self.char_at(1) == Some('/') {
                    self.pos += 2;
                    self.token = SyntaxKind::LessThanSlashToken;
                } else {
                    self.pos += 1;
                    self.token = SyntaxKind::LessThanToken;
                }
            }
            _ => {
                // JSX text
                return self.scan_jsx_text();
            }
        }
        self.token
    }

    /// Scan JSX attribute value (string in quotes).
    pub fn scan_jsx_attribute_value(&mut self) -> SyntaxKind {
        self.token_start = self.pos;
        match self.current_char() {
            Some('"') | Some('\'') => {
                let quote = self.text[self.pos];
                self.token = self.scan_string_literal(quote);
                self.token
            }
            _ => {
                // Not a string - scan normally
                self.scan()
            }
        }
    }

    /// Check for conflict markers (<<<<<<<, =======, >>>>>>>).
    fn is_conflict_marker_trivia(&self) -> bool {
        if self.pos + 6 >= self.text.len() {
            return false;
        }
        let ch = self.text[self.pos];
        if ch == '<' || ch == '>' || ch == '|' || ch == '=' {
            // Check for 7 repeats of the same char
            for i in 1..7 {
                if self.text.get(self.pos + i) != Some(&ch) {
                    return false;
                }
            }
            // Must be at start of line
            if self.pos == 0 {
                return true;
            }
            let prev = self.text[self.pos - 1];
            is_line_break(prev)
        } else {
            false
        }
    }

    /// Skip a conflict marker and return true if one was found.
    fn try_skip_conflict_marker(&mut self) -> bool {
        if !self.is_conflict_marker_trivia() {
            return false;
        }
        let ch = self.text[self.pos];
        // Skip to end of line
        while !self.is_eof() && !is_line_break(self.text[self.pos]) {
            self.pos += 1;
        }
        if ch == '=' {
            // For =======, we also skip the line break
            if !self.is_eof() {
                if self.text[self.pos] == '\r' {
                    self.pos += 1;
                    if self.current_char() == Some('\n') {
                        self.pos += 1;
                    }
                } else {
                    self.pos += 1;
                }
            }
        }
        self.token_flags |= TokenFlags::PRECEDING_LINE_BREAK;
        true
    }

    /// Convert a range of chars to a String.
    fn chars_to_string(&self, start: usize, end: usize) -> String {
        self.text[start..end].iter().collect()
    }

    /// Get a substring of the source text.
    pub fn get_text_slice(&self, start: usize, end: usize) -> String {
        let s = start.min(self.text.len());
        let e = end.min(self.text.len());
        self.text[s..e].iter().collect()
    }

    /// Get the full source text.
    pub fn get_text(&self) -> String {
        self.text.iter().collect()
    }
}

/// Check if a character can start an identifier.
fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_ascii_alphabetic() || (ch as u32 > 0x7F && unicode_xid::UnicodeXID::is_xid_start(ch))
}

/// Check if a character can be part of an identifier.
fn is_identifier_part(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_ascii_alphanumeric() || (ch as u32 > 0x7F && unicode_xid::UnicodeXID::is_xid_continue(ch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_simple_tokens() {
        let mut scanner = Scanner::new("( ) { } [ ] ; , :");
        assert_eq!(scanner.scan(), SyntaxKind::OpenParenToken);
        assert_eq!(scanner.scan(), SyntaxKind::CloseParenToken);
        assert_eq!(scanner.scan(), SyntaxKind::OpenBraceToken);
        assert_eq!(scanner.scan(), SyntaxKind::CloseBraceToken);
        assert_eq!(scanner.scan(), SyntaxKind::OpenBracketToken);
        assert_eq!(scanner.scan(), SyntaxKind::CloseBracketToken);
        assert_eq!(scanner.scan(), SyntaxKind::SemicolonToken);
        assert_eq!(scanner.scan(), SyntaxKind::CommaToken);
        assert_eq!(scanner.scan(), SyntaxKind::ColonToken);
        assert_eq!(scanner.scan(), SyntaxKind::EndOfFileToken);
    }

    #[test]
    fn test_scan_operators() {
        let mut scanner = Scanner::new("+ ++ += - -- -= * ** *= / /= % %= === !== == !=");
        assert_eq!(scanner.scan(), SyntaxKind::PlusToken);
        assert_eq!(scanner.scan(), SyntaxKind::PlusPlusToken);
        assert_eq!(scanner.scan(), SyntaxKind::PlusEqualsToken);
        assert_eq!(scanner.scan(), SyntaxKind::MinusToken);
        assert_eq!(scanner.scan(), SyntaxKind::MinusMinusToken);
        assert_eq!(scanner.scan(), SyntaxKind::MinusEqualsToken);
        assert_eq!(scanner.scan(), SyntaxKind::AsteriskToken);
        assert_eq!(scanner.scan(), SyntaxKind::AsteriskAsteriskToken);
        assert_eq!(scanner.scan(), SyntaxKind::AsteriskEqualsToken);
        assert_eq!(scanner.scan(), SyntaxKind::SlashToken);
        assert_eq!(scanner.scan(), SyntaxKind::SlashEqualsToken);
        assert_eq!(scanner.scan(), SyntaxKind::PercentToken);
        assert_eq!(scanner.scan(), SyntaxKind::PercentEqualsToken);
        assert_eq!(scanner.scan(), SyntaxKind::EqualsEqualsEqualsToken);
        assert_eq!(scanner.scan(), SyntaxKind::ExclamationEqualsEqualsToken);
        assert_eq!(scanner.scan(), SyntaxKind::EqualsEqualsToken);
        assert_eq!(scanner.scan(), SyntaxKind::ExclamationEqualsToken);
        assert_eq!(scanner.scan(), SyntaxKind::EndOfFileToken);
    }

    #[test]
    fn test_scan_identifier_and_keyword() {
        let mut scanner = Scanner::new("let x = 42;");
        assert_eq!(scanner.scan(), SyntaxKind::LetKeyword);
        assert_eq!(scanner.scan(), SyntaxKind::Identifier);
        assert_eq!(scanner.token_value(), "x");
        assert_eq!(scanner.scan(), SyntaxKind::EqualsToken);
        assert_eq!(scanner.scan(), SyntaxKind::NumericLiteral);
        assert_eq!(scanner.token_value(), "42");
        assert_eq!(scanner.scan(), SyntaxKind::SemicolonToken);
        assert_eq!(scanner.scan(), SyntaxKind::EndOfFileToken);
    }

    #[test]
    fn test_scan_string_literal() {
        let mut scanner = Scanner::new(r#""hello" 'world'"#);
        assert_eq!(scanner.scan(), SyntaxKind::StringLiteral);
        assert_eq!(scanner.token_value(), "hello");
        assert_eq!(scanner.scan(), SyntaxKind::StringLiteral);
        assert_eq!(scanner.token_value(), "world");
    }

    #[test]
    fn test_scan_template_literal() {
        let mut scanner = Scanner::new("`hello`");
        assert_eq!(scanner.scan(), SyntaxKind::NoSubstitutionTemplateLiteral);
        assert_eq!(scanner.token_value(), "hello");
    }

    #[test]
    fn test_scan_template_head() {
        let mut scanner = Scanner::new("`hello ${");
        assert_eq!(scanner.scan(), SyntaxKind::TemplateHead);
        assert_eq!(scanner.token_value(), "hello ");
    }

    #[test]
    fn test_scan_number_formats() {
        let mut scanner = Scanner::new("42 3.14 0xff 0b1010 0o777 1_000");
        assert_eq!(scanner.scan(), SyntaxKind::NumericLiteral);
        assert_eq!(scanner.token_value(), "42");
        assert_eq!(scanner.scan(), SyntaxKind::NumericLiteral);
        assert_eq!(scanner.token_value(), "3.14");
        assert_eq!(scanner.scan(), SyntaxKind::NumericLiteral);
        assert_eq!(scanner.token_value(), "0xff");
        assert_eq!(scanner.scan(), SyntaxKind::NumericLiteral);
        assert_eq!(scanner.token_value(), "0b1010");
        assert_eq!(scanner.scan(), SyntaxKind::NumericLiteral);
        assert_eq!(scanner.token_value(), "0o777");
        assert_eq!(scanner.scan(), SyntaxKind::NumericLiteral);
        assert_eq!(scanner.token_value(), "1_000");
    }

    #[test]
    fn test_scan_bigint() {
        let mut scanner = Scanner::new("42n 0xFFn");
        assert_eq!(scanner.scan(), SyntaxKind::BigIntLiteral);
        assert_eq!(scanner.token_value(), "42n");
        assert_eq!(scanner.scan(), SyntaxKind::BigIntLiteral);
        assert_eq!(scanner.token_value(), "0xFFn");
    }

    #[test]
    fn test_scan_comments() {
        let mut scanner = Scanner::new("// comment\nlet /* block */ x");
        assert_eq!(scanner.scan(), SyntaxKind::LetKeyword);
        assert!(scanner.has_preceding_line_break());
        assert_eq!(scanner.scan(), SyntaxKind::Identifier);
        assert_eq!(scanner.token_value(), "x");
    }

    #[test]
    fn test_scan_arrow_function() {
        let mut scanner = Scanner::new("=> ??= &&=");
        assert_eq!(scanner.scan(), SyntaxKind::EqualsGreaterThanToken);
        assert_eq!(scanner.scan(), SyntaxKind::QuestionQuestionEqualsToken);
        assert_eq!(scanner.scan(), SyntaxKind::AmpersandAmpersandEqualsToken);
    }

    #[test]
    fn test_scan_dot_variations() {
        let mut scanner = Scanner::new(". ... ?.");
        assert_eq!(scanner.scan(), SyntaxKind::DotToken);
        assert_eq!(scanner.scan(), SyntaxKind::DotDotDotToken);
        assert_eq!(scanner.scan(), SyntaxKind::QuestionDotToken);
    }

    #[test]
    fn test_scan_at_and_hash() {
        let mut scanner = Scanner::new("@ #");
        assert_eq!(scanner.scan(), SyntaxKind::AtToken);
        assert_eq!(scanner.scan(), SyntaxKind::HashToken);
    }

    #[test]
    fn test_scan_logical_assignment() {
        let mut scanner = Scanner::new("??= ||= &&=");
        assert_eq!(scanner.scan(), SyntaxKind::QuestionQuestionEqualsToken);
        assert_eq!(scanner.scan(), SyntaxKind::BarBarEqualsToken);
        assert_eq!(scanner.scan(), SyntaxKind::AmpersandAmpersandEqualsToken);
    }

    #[test]
    fn test_scan_exponentiation() {
        let mut scanner = Scanner::new("** **=");
        assert_eq!(scanner.scan(), SyntaxKind::AsteriskAsteriskToken);
        assert_eq!(scanner.scan(), SyntaxKind::AsteriskAsteriskEqualsToken);
    }

    #[test]
    fn test_scan_optional_chaining() {
        let mut scanner = Scanner::new("?. ?? ?.()");
        assert_eq!(scanner.scan(), SyntaxKind::QuestionDotToken);
        assert_eq!(scanner.scan(), SyntaxKind::QuestionQuestionToken);
        assert_eq!(scanner.scan(), SyntaxKind::QuestionDotToken);
        assert_eq!(scanner.scan(), SyntaxKind::OpenParenToken);
        assert_eq!(scanner.scan(), SyntaxKind::CloseParenToken);
    }

    #[test]
    fn test_shebang() {
        let mut scanner = Scanner::new("#!/usr/bin/env node\nlet x = 1;");
        scanner.skip_shebang();
        assert_eq!(scanner.scan(), SyntaxKind::LetKeyword);
    }

    #[test]
    fn test_scan_jsx_text() {
        let mut scanner = Scanner::new("Hello World{");
        scanner.set_in_jsx(true);
        let kind = scanner.scan_jsx_text();
        assert_eq!(kind, SyntaxKind::JsxText);
        assert_eq!(scanner.token_value(), "Hello World");
    }

    #[test]
    fn test_look_ahead() {
        let mut scanner = Scanner::new("let x = 1;");
        scanner.scan(); // let
        let next = scanner.look_ahead(|s| {
            s.scan()
        });
        assert_eq!(next, SyntaxKind::Identifier);
        // Position should be restored
        assert_eq!(scanner.token(), SyntaxKind::LetKeyword);
    }
}
