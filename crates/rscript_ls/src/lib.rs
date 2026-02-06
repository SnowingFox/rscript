//! rscript_ls: Language service.
//!
//! Provides IDE features like completions, hover, go-to-definition,
//! diagnostics, references, etc. The language service is consumed by the LSP server.

use bumpalo::Bump;
use rscript_binder::Binder;
use rscript_checker::Checker;
use rscript_parser::Parser;
use std::collections::HashMap;

/// A document tracked by the language service.
#[derive(Debug)]
struct Document {
    /// The file name.
    file_name: String,
    /// The current source text.
    text: String,
    /// Version number for incremental updates.
    version: i32,
}

/// Language service providing IDE features.
pub struct LanguageService {
    /// Open documents tracked by URI.
    documents: HashMap<String, Document>,
}

impl LanguageService {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    /// Open or update a document.
    pub fn open_document(&mut self, uri: String, text: String, version: i32) {
        self.documents.insert(uri.clone(), Document {
            file_name: uri,
            text,
            version,
        });
    }

    /// Update document content.
    pub fn update_document(&mut self, uri: &str, text: String, version: i32) {
        if let Some(doc) = self.documents.get_mut(uri) {
            doc.text = text;
            doc.version = version;
        }
    }

    /// Close a document.
    pub fn close_document(&mut self, uri: &str) {
        self.documents.remove(uri);
    }

    /// Get the current text of a document.
    pub fn get_document_text(&self, uri: &str) -> Option<&str> {
        self.documents.get(uri).map(|d| d.text.as_str())
    }

    /// Get diagnostics for a file.
    pub fn get_diagnostics(&self, file_name: &str) -> Vec<rscript_diagnostics::Diagnostic> {
        let text = match self.documents.get(file_name) {
            Some(doc) => &doc.text,
            None => return Vec::new(),
        };

        let arena = Bump::new();
        let parser = Parser::new(&arena, file_name, text);
        let source_file = parser.parse_source_file();

        let mut binder = Binder::new();
        binder.bind_source_file(&source_file);

        let mut checker = Checker::new(binder);
        checker.check_source_file(&source_file);

        let diags = checker.take_diagnostics();
        diags.into_diagnostics()
    }

    /// Get completions at a position.
    pub fn get_completions(&self, file_name: &str, position: u32) -> Vec<CompletionItem> {
        let text = match self.documents.get(file_name) {
            Some(doc) => &doc.text,
            None => return Vec::new(),
        };

        let mut completions = Vec::new();
        let prefix = get_word_at_position(text, position);

        // Add keyword completions
        let keywords = [
            "abstract", "any", "as", "async", "await", "boolean", "break",
            "case", "catch", "class", "const", "continue", "debugger",
            "declare", "default", "delete", "do", "else", "enum", "export",
            "extends", "false", "finally", "for", "from", "function",
            "get", "if", "implements", "import", "in", "infer",
            "instanceof", "interface", "is", "keyof", "let", "module",
            "namespace", "never", "new", "null", "number", "object",
            "of", "override", "private", "protected", "public",
            "readonly", "require", "return", "satisfies", "set",
            "static", "string", "super", "switch", "symbol", "this",
            "throw", "true", "try", "type", "typeof", "undefined",
            "unique", "unknown", "var", "void", "while", "with", "yield",
        ];

        for kw in &keywords {
            if prefix.is_empty() || kw.starts_with(&prefix) {
                completions.push(CompletionItem {
                    label: kw.to_string(),
                    kind: CompletionItemKind::Keyword,
                    detail: Some("keyword".to_string()),
                    insert_text: None,
                    sort_text: Some(format!("1_{}", kw)),
                });
            }
        }

        completions
    }

    /// Get hover information at a position.
    pub fn get_hover(&self, file_name: &str, position: u32) -> Option<HoverInfo> {
        let text = self.documents.get(file_name)?.text.as_str();
        let word = get_word_at_position(text, position);
        if word.is_empty() { return None; }

        if is_keyword(&word) {
            return Some(HoverInfo {
                contents: format!("(keyword) {}", word),
                range: None,
            });
        }

        Some(HoverInfo {
            contents: format!("(identifier) {}", word),
            range: None,
        })
    }

    /// Get the definition location of a symbol at a position.
    pub fn get_definition(&self, _file_name: &str, _position: u32) -> Vec<DefinitionInfo> {
        // Requires source map from AST nodes to positions
        Vec::new()
    }

    /// Find all references to a symbol at a position.
    pub fn get_references(&self, file_name: &str, position: u32) -> Vec<ReferenceInfo> {
        let text = match self.documents.get(file_name) {
            Some(doc) => &doc.text,
            None => return Vec::new(),
        };

        let word = get_word_at_position(text, position);
        if word.is_empty() { return Vec::new(); }

        // Simple text-based search for references
        let mut references = Vec::new();
        let bytes = text.as_bytes();
        let word_bytes = word.as_bytes();
        let word_len = word_bytes.len();

        let mut i = 0;
        while i + word_len <= bytes.len() {
            if &bytes[i..i + word_len] == word_bytes {
                let before_ok = i == 0 || !is_identifier_char(bytes[i - 1]);
                let after_ok = i + word_len >= bytes.len() || !is_identifier_char(bytes[i + word_len]);
                if before_ok && after_ok {
                    references.push(ReferenceInfo {
                        file_name: file_name.to_string(),
                        span: rscript_core::text::TextSpan::new(i as u32, word_len as u32),
                        is_definition: false,
                    });
                }
            }
            i += 1;
        }

        references
    }

    /// Get document symbols (outline).
    pub fn get_document_symbols(&self, file_name: &str) -> Vec<DocumentSymbol> {
        let text = match self.documents.get(file_name) {
            Some(doc) => &doc.text,
            None => return Vec::new(),
        };

        let arena = Bump::new();
        let parser = Parser::new(&arena, file_name, text);
        let source_file = parser.parse_source_file();

        let mut binder = Binder::new();
        binder.bind_source_file(&source_file);

        let mut symbols = Vec::new();
        for symbol in binder.get_symbols() {
            let kind = symbol_to_document_symbol_kind(&symbol.flags);
            symbols.push(DocumentSymbol {
                name: format!("symbol_{}", symbol.id.index()),
                kind,
                range: rscript_core::text::TextSpan::new(0, 0),
                selection_range: rscript_core::text::TextSpan::new(0, 0),
                children: vec![],
            });
        }

        symbols
    }
}

impl Default for LanguageService {
    fn default() -> Self {
        Self::new()
    }
}

/// A completion item.
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionItemKind,
    pub detail: Option<String>,
    pub insert_text: Option<String>,
    pub sort_text: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum CompletionItemKind {
    Variable,
    Function,
    Class,
    Interface,
    Module,
    Property,
    Method,
    Keyword,
    Type,
    Enum,
    EnumMember,
    Constant,
}

/// Hover information.
#[derive(Debug, Clone)]
pub struct HoverInfo {
    pub contents: String,
    pub range: Option<rscript_core::text::TextSpan>,
}

/// Definition location.
#[derive(Debug, Clone)]
pub struct DefinitionInfo {
    pub file_name: String,
    pub span: rscript_core::text::TextSpan,
}

/// Reference location.
#[derive(Debug, Clone)]
pub struct ReferenceInfo {
    pub file_name: String,
    pub span: rscript_core::text::TextSpan,
    pub is_definition: bool,
}

/// Document symbol (for outline/breadcrumbs).
#[derive(Debug, Clone)]
pub struct DocumentSymbol {
    pub name: String,
    pub kind: DocumentSymbolKind,
    pub range: rscript_core::text::TextSpan,
    pub selection_range: rscript_core::text::TextSpan,
    pub children: Vec<DocumentSymbol>,
}

#[derive(Debug, Clone, Copy)]
pub enum DocumentSymbolKind {
    File,
    Module,
    Namespace,
    Class,
    Method,
    Property,
    Function,
    Variable,
    Constant,
    Enum,
    Interface,
    TypeParameter,
    EnumMember,
}

// Helper functions

fn get_word_at_position(text: &str, position: u32) -> String {
    let pos = position as usize;
    if pos >= text.len() { return String::new(); }

    let bytes = text.as_bytes();
    let mut start = pos;
    let mut end = pos;

    while start > 0 && is_identifier_char(bytes[start - 1]) {
        start -= 1;
    }
    while end < bytes.len() && is_identifier_char(bytes[end]) {
        end += 1;
    }

    text[start..end].to_string()
}

fn is_identifier_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'$'
}

fn is_keyword(word: &str) -> bool {
    matches!(word,
        "abstract" | "any" | "as" | "async" | "await" | "boolean" | "break" |
        "case" | "catch" | "class" | "const" | "continue" | "debugger" |
        "declare" | "default" | "delete" | "do" | "else" | "enum" | "export" |
        "extends" | "false" | "finally" | "for" | "from" | "function" |
        "get" | "if" | "implements" | "import" | "in" | "infer" |
        "instanceof" | "interface" | "is" | "keyof" | "let" | "module" |
        "namespace" | "never" | "new" | "null" | "number" | "object" |
        "of" | "override" | "private" | "protected" | "public" |
        "readonly" | "require" | "return" | "satisfies" | "set" |
        "static" | "string" | "super" | "switch" | "symbol" | "this" |
        "throw" | "true" | "try" | "type" | "typeof" | "undefined" |
        "unique" | "unknown" | "var" | "void" | "while" | "with" | "yield"
    )
}

fn symbol_to_document_symbol_kind(flags: &rscript_ast::types::SymbolFlags) -> DocumentSymbolKind {
    if flags.contains(rscript_ast::types::SymbolFlags::FUNCTION) { DocumentSymbolKind::Function }
    else if flags.contains(rscript_ast::types::SymbolFlags::CLASS) { DocumentSymbolKind::Class }
    else if flags.contains(rscript_ast::types::SymbolFlags::INTERFACE) { DocumentSymbolKind::Interface }
    else if flags.contains(rscript_ast::types::SymbolFlags::ENUM) { DocumentSymbolKind::Enum }
    else if flags.contains(rscript_ast::types::SymbolFlags::MODULE) { DocumentSymbolKind::Module }
    else { DocumentSymbolKind::Variable }
}
