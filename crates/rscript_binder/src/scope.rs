//! Scope management for the binder.

use crate::symbol::SymbolTable;
use rscript_ast::types::SymbolId;
use std::collections::HashMap;

/// A scope in the binding phase. Scopes form a chain from inner to outer.
#[derive(Debug)]
pub struct Scope {
    /// The symbols declared in this scope.
    pub locals: SymbolTable,
    /// String-based name lookup for actual identifier resolution.
    pub names: HashMap<String, SymbolId>,
    /// The parent scope (None for the global scope).
    pub parent: Option<Box<Scope>>,
    /// The block-scoped container symbol.
    #[allow(dead_code)]
    pub container: Option<SymbolId>,
}

impl Scope {
    pub fn new(parent: Option<Box<Scope>>) -> Self {
        Self {
            locals: SymbolTable::new(),
            names: HashMap::new(),
            parent,
            container: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_container(parent: Option<Box<Scope>>, container: SymbolId) -> Self {
        Self {
            locals: SymbolTable::new(),
            names: HashMap::new(),
            parent,
            container: Some(container),
        }
    }
}
