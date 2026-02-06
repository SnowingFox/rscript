//! rscript_compiler: Compiler orchestration.
//!
//! Creates the program, coordinates parsing, binding, type checking,
//! and emit across all source files.

use bumpalo::Bump;
use rscript_binder::Binder;
use rscript_checker::Checker;
use rscript_core::intern::StringInterner;
use rscript_diagnostics::DiagnosticCollection;
use rscript_emitter::{Emitter, EmitResult};
use rscript_parser::Parser;
use rscript_tsoptions::CompilerOptions;

/// The program represents the entire compilation unit.
pub struct Program<'a> {
    /// Compiler options.
    pub options: CompilerOptions,
    /// The root file names.
    pub root_files: Vec<String>,
    /// All source files in the program.
    arena: &'a Bump,
    /// String interner for identifier resolution.
    interner: StringInterner,
    /// Parsed source files (stored as raw text + file name for now).
    source_files: Vec<(String, String)>,
}

impl<'a> Program<'a> {
    /// Create a new program from root files and options.
    pub fn new(arena: &'a Bump, root_files: Vec<String>, options: CompilerOptions) -> Self {
        Self {
            options,
            root_files,
            arena,
            interner: StringInterner::new(),
            source_files: Vec::new(),
        }
    }

    /// Add a source file to the program.
    pub fn add_source(&mut self, file_name: String, source_text: String) {
        self.source_files.push((file_name, source_text));
    }

    /// Load all root files from disk.
    pub fn load_root_files(&mut self) -> Result<(), std::io::Error> {
        for file in &self.root_files.clone() {
            let content = std::fs::read_to_string(file)?;
            self.source_files.push((file.clone(), content));
        }
        Ok(())
    }

    /// Run the full compilation pipeline: parse -> bind -> check.
    /// Returns all diagnostics.
    pub fn compile(&self) -> DiagnosticCollection {
        let mut all_diagnostics = DiagnosticCollection::new();

        for (file_name, source_text) in &self.source_files {
            // Parse
            let parser = Parser::new(self.arena, file_name, source_text);
            let source_file = parser.parse_source_file();

            // Bind
            let mut binder = Binder::new();
            binder.bind_source_file(&source_file);

            // Check
            let mut checker = Checker::new(binder);
            checker.check_source_file(&source_file);

            let diags = checker.take_diagnostics();
            all_diagnostics.extend(diags);
        }

        all_diagnostics.sort();
        all_diagnostics
    }

    /// Emit output files for all source files.
    pub fn emit(&self) -> Vec<EmitResult> {
        let emitter = Emitter::new();
        let mut results = Vec::new();

        for (file_name, source_text) in &self.source_files {
            let parser = Parser::new(self.arena, file_name, source_text);
            let source_file = parser.parse_source_file();
            let result = emitter.emit(&source_file, &self.interner);
            results.push(result);
        }

        results
    }
}
