//! rscript_nodebuilder: Synthetic AST node construction.
//!
//! Creates AST nodes for type display in error messages and
//! declaration emit (.d.ts generation).

pub struct NodeBuilder;

impl NodeBuilder {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NodeBuilder {
    fn default() -> Self {
        Self::new()
    }
}
