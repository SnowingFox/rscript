//! Token information produced by the scanner.

use rscript_ast::syntax_kind::SyntaxKind;
use rscript_ast::types::TokenFlags;

/// Information about a scanned token.
#[derive(Debug, Clone)]
pub struct TokenInfo {
    /// The kind of token.
    pub kind: SyntaxKind,
    /// Start position in the source text.
    pub pos: u32,
    /// End position in the source text (exclusive).
    pub end: u32,
    /// The text of the token (for identifiers, literals, etc.).
    pub text: String,
    /// Token flags (preceding line break, numeric format, etc.).
    pub flags: TokenFlags,
}

impl TokenInfo {
    pub fn new(kind: SyntaxKind, pos: u32, end: u32) -> Self {
        Self {
            kind,
            pos,
            end,
            text: String::new(),
            flags: TokenFlags::NONE,
        }
    }

    pub fn with_text(mut self, text: String) -> Self {
        self.text = text;
        self
    }

    pub fn with_flags(mut self, flags: TokenFlags) -> Self {
        self.flags = flags;
        self
    }

    /// The length of this token in bytes.
    pub fn len(&self) -> u32 {
        self.end - self.pos
    }

    /// Whether this token has zero length.
    pub fn is_empty(&self) -> bool {
        self.pos == self.end
    }

    /// Whether there was a line break before this token.
    pub fn has_preceding_line_break(&self) -> bool {
        self.flags.contains(TokenFlags::PRECEDING_LINE_BREAK)
    }
}
