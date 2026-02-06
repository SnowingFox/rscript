//! rscript_emitter: JavaScript and declaration file output.
//!
//! Coordinates the transformation pipeline and output generation:
//! 1. Transform the AST (strip types, downlevel, etc.)
//! 2. Print to text
//! 3. Generate source maps
//! 4. Write output files

use rscript_ast::node::SourceFile;
use rscript_core::intern::StringInterner;
use rscript_printer::{Printer, PrinterOptions};
use std::path::{Path, PathBuf};

/// The emitter produces output files from the AST.
pub struct Emitter {
    /// Whether to emit declaration files.
    pub emit_declaration: bool,
    /// Whether to emit source maps.
    pub emit_source_map: bool,
    /// Whether to strip type annotations (emit JS).
    pub strip_types: bool,
    /// Output directory override.
    pub out_dir: Option<PathBuf>,
    /// Root directory for calculating relative paths.
    pub root_dir: Option<PathBuf>,
}

/// The result of emitting a source file.
pub struct EmitResult {
    /// The emitted JavaScript content.
    pub js_content: String,
    /// The emitted declaration content (if requested).
    pub dts_content: Option<String>,
    /// The source map content (if requested).
    pub source_map_content: Option<String>,
    /// Whether any errors occurred during emit.
    pub has_errors: bool,
    /// Output file paths.
    pub output_files: Vec<OutputFile>,
}

/// A file produced by the emitter.
#[derive(Debug, Clone)]
pub struct OutputFile {
    /// The output file path.
    pub path: PathBuf,
    /// The content of the file.
    pub text: String,
}

impl Emitter {
    pub fn new() -> Self {
        Self {
            emit_declaration: false,
            emit_source_map: false,
            strip_types: true,
            out_dir: None,
            root_dir: None,
        }
    }

    /// Emit a source file to JavaScript (and optionally .d.ts and source map).
    pub fn emit(&self, source_file: &SourceFile<'_>, interner: &StringInterner) -> EmitResult {
        let mut output_files = Vec::new();

        // Print JS output (with types stripped)
        let js_content = {
            let mut printer = Printer::with_options(interner, PrinterOptions {
                strip_types: self.strip_types,
                indent_str: "    ".to_string(),
                new_line: "\n".to_string(),
                trailing_newline: true,
            });
            printer.print_source_file(source_file)
        };

        // Calculate output path
        let source_path = Path::new(&source_file.file_name);
        let js_path = self.get_output_path(source_path, ".js");

        output_files.push(OutputFile {
            path: js_path,
            text: js_content.clone(),
        });

        // Generate declaration file if requested
        let dts_content = if self.emit_declaration {
            let mut printer = Printer::with_options(interner, PrinterOptions {
                strip_types: false,
                indent_str: "    ".to_string(),
                new_line: "\n".to_string(),
                trailing_newline: true,
            });
            let dts = printer.print_source_file(source_file);
            let dts_path = self.get_output_path(source_path, ".d.ts");
            output_files.push(OutputFile {
                path: dts_path,
                text: dts.clone(),
            });
            Some(dts)
        } else {
            None
        };

        // Generate source map if requested
        let source_map_content = if self.emit_source_map {
            let source_map = self.generate_source_map(source_path, &js_content);
            let map_path = self.get_output_path(source_path, ".js.map");
            output_files.push(OutputFile {
                path: map_path,
                text: source_map.clone(),
            });
            Some(source_map)
        } else {
            None
        };

        EmitResult {
            js_content,
            dts_content,
            source_map_content,
            has_errors: false,
            output_files,
        }
    }

    /// Write output files to disk.
    pub fn write_output_files(&self, result: &EmitResult) -> std::io::Result<()> {
        for file in &result.output_files {
            if let Some(parent) = file.path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&file.path, &file.text)?;
        }
        Ok(())
    }

    fn get_output_path(&self, source: &Path, ext: &str) -> PathBuf {
        let stem = source.file_stem().unwrap_or_default();
        let base_dir = if let Some(ref out_dir) = self.out_dir {
            out_dir.clone()
        } else {
            source.parent().unwrap_or_else(|| Path::new(".")).to_path_buf()
        };
        base_dir.join(format!("{}{}", stem.to_string_lossy(), ext))
    }

    fn generate_source_map(&self, source_path: &Path, _js_content: &str) -> String {
        let source_name = source_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        format!(
            r#"{{"version":3,"file":"{}","sourceRoot":"","sources":["{}"],"names":[],"mappings":""}}"#,
            source_name.replace(".ts", ".js").replace(".tsx", ".js"),
            source_name,
        )
    }
}

impl Default for Emitter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_emitter_creation() {
        let emitter = Emitter::new();
        assert!(emitter.strip_types);
        assert!(!emitter.emit_declaration);
        assert!(!emitter.emit_source_map);
    }

    #[test]
    fn test_output_path() {
        let emitter = Emitter::new();
        let path = emitter.get_output_path(Path::new("src/foo.ts"), ".js");
        assert_eq!(path, PathBuf::from("src/foo.js"));
    }

    #[test]
    fn test_output_path_with_outdir() {
        let mut emitter = Emitter::new();
        emitter.out_dir = Some(PathBuf::from("dist"));
        let path = emitter.get_output_path(Path::new("src/foo.ts"), ".js");
        assert_eq!(path, PathBuf::from("dist/foo.js"));
    }

    #[test]
    fn test_source_map_generation() {
        let emitter = Emitter::new();
        let map = emitter.generate_source_map(Path::new("foo.ts"), "var x = 1;");
        assert!(map.contains("\"version\":3"));
        assert!(map.contains("foo.ts"));
    }
}
