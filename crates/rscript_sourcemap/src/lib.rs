//! rscript_sourcemap: Source map generation.
//!
//! Generates V3 source maps for mapping output JS/DTS back to
//! original TypeScript source.

/// A source map builder that accumulates mappings.
pub struct SourceMapBuilder {
    mappings: Vec<Mapping>,
    sources: Vec<String>,
    names: Vec<String>,
}

/// A single mapping entry.
#[derive(Debug, Clone)]
pub struct Mapping {
    pub generated_line: u32,
    pub generated_column: u32,
    pub source_index: Option<u32>,
    pub original_line: Option<u32>,
    pub original_column: Option<u32>,
    pub name_index: Option<u32>,
}

impl SourceMapBuilder {
    pub fn new() -> Self {
        Self {
            mappings: Vec::new(),
            sources: Vec::new(),
            names: Vec::new(),
        }
    }

    pub fn add_source(&mut self, source: String) -> u32 {
        let idx = self.sources.len() as u32;
        self.sources.push(source);
        idx
    }

    pub fn add_mapping(&mut self, mapping: Mapping) {
        self.mappings.push(mapping);
    }

    /// Encode the source map as a JSON string.
    pub fn to_json(&self) -> String {
        // TODO: Implement V3 source map encoding with VLQ
        String::from("{\"version\":3}")
    }
}

impl Default for SourceMapBuilder {
    fn default() -> Self {
        Self::new()
    }
}
