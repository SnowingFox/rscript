//! Symbol and symbol table definitions.

use rscript_ast::types::{SymbolFlags, SymbolId, NodeId};
use rscript_core::intern::InternedString;
use rustc_hash::FxHashMap;

/// A symbol represents a named entity in the program (variable, function,
/// class, interface, type, etc.).
#[derive(Debug, Clone)]
pub struct Symbol {
    /// Unique identifier for this symbol.
    pub id: SymbolId,
    /// The name of this symbol (interned).
    pub name: InternedString,
    /// The actual text name of this symbol.
    pub name_text: String,
    /// Symbol flags describing what kind of entity this is.
    pub flags: SymbolFlags,
    /// The declarations that contribute to this symbol.
    pub declarations: Vec<NodeId>,
    /// The value declaration (if any).
    pub value_declaration: Option<NodeId>,
    /// Members of this symbol (for classes, interfaces, etc.).
    pub members: Option<SymbolTable>,
    /// Exports of this symbol (for modules).
    pub exports: Option<SymbolTable>,
    /// The parent symbol (for nested symbols).
    pub parent: Option<SymbolId>,
}

impl Symbol {
    pub fn new(id: SymbolId, name: InternedString, flags: SymbolFlags) -> Self {
        Self {
            id,
            name,
            name_text: String::new(),
            flags,
            declarations: Vec::new(),
            value_declaration: None,
            members: None,
            exports: None,
            parent: None,
        }
    }

    pub fn with_name_text(id: SymbolId, name: InternedString, name_text: String, flags: SymbolFlags) -> Self {
        Self {
            id,
            name,
            name_text,
            flags,
            declarations: Vec::new(),
            value_declaration: None,
            members: None,
            exports: None,
            parent: None,
        }
    }
}

/// A symbol table maps names to symbols.
#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    table: FxHashMap<InternedString, SymbolId>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            table: FxHashMap::default(),
        }
    }

    pub fn get(&self, name: &InternedString) -> Option<SymbolId> {
        self.table.get(name).copied()
    }

    pub fn set(&mut self, name: InternedString, symbol: SymbolId) {
        self.table.insert(name, symbol);
    }

    pub fn has(&self, name: &InternedString) -> bool {
        self.table.contains_key(name)
    }

    pub fn len(&self) -> usize {
        self.table.len()
    }

    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&InternedString, &SymbolId)> {
        self.table.iter()
    }
}
