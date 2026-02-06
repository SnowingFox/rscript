//! rscript_tsoptions: tsconfig.json parsing and compiler options.
//!
//! Parses tsconfig.json files and provides the CompilerOptions structure
//! matching TypeScript's compiler options.

use serde::{Deserialize, Serialize};

/// TypeScript compiler options, matching the tsconfig.json schema.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilerOptions {
    // -- Basic Options --
    pub target: Option<ScriptTarget>,
    pub module: Option<ModuleKind>,
    pub lib: Option<Vec<String>>,
    pub jsx: Option<JsxEmit>,
    pub declaration: Option<bool>,
    pub declaration_map: Option<bool>,
    pub source_map: Option<bool>,
    pub out_file: Option<String>,
    pub out_dir: Option<String>,
    pub root_dir: Option<String>,
    pub composite: Option<bool>,
    pub incremental: Option<bool>,
    pub ts_build_info_file: Option<String>,
    pub remove_comments: Option<bool>,
    pub no_emit: Option<bool>,

    // -- Strict Type-Checking Options --
    pub strict: Option<bool>,
    pub no_implicit_any: Option<bool>,
    pub strict_null_checks: Option<bool>,
    pub strict_function_types: Option<bool>,
    pub strict_bind_call_apply: Option<bool>,
    pub strict_property_initialization: Option<bool>,
    pub no_implicit_this: Option<bool>,
    pub always_strict: Option<bool>,
    pub use_unknown_in_catch_variables: Option<bool>,

    // -- Module Resolution Options --
    pub module_resolution: Option<String>,
    pub base_url: Option<String>,
    pub paths: Option<std::collections::HashMap<String, Vec<String>>>,
    pub root_dirs: Option<Vec<String>>,
    pub type_roots: Option<Vec<String>>,
    pub types: Option<Vec<String>>,
    pub allow_synthetic_default_imports: Option<bool>,
    pub es_module_interop: Option<bool>,
    pub resolve_json_module: Option<bool>,

    // -- Source Map Options --
    pub source_root: Option<String>,
    pub map_root: Option<String>,
    pub inline_source_map: Option<bool>,
    pub inline_sources: Option<bool>,

    // -- Additional Checks --
    pub no_unused_locals: Option<bool>,
    pub no_unused_parameters: Option<bool>,
    pub no_implicit_returns: Option<bool>,
    pub no_fallthrough_cases_in_switch: Option<bool>,

    // -- Experimental Options --
    pub experimental_decorators: Option<bool>,
    pub emit_decorator_metadata: Option<bool>,

    // -- Advanced Options --
    pub skip_lib_check: Option<bool>,
    pub force_consistent_casing_in_file_names: Option<bool>,
    pub allow_js: Option<bool>,
    pub check_js: Option<bool>,
    pub no_resolve: Option<bool>,
    pub isolate_modules: Option<bool>,
    #[serde(rename = "isolatedModules")]
    pub isolated_modules: Option<bool>,
    pub verbatim_module_syntax: Option<bool>,
}

/// Script target version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScriptTarget {
    ES3,
    ES5,
    ES2015,
    ES2016,
    ES2017,
    ES2018,
    ES2019,
    ES2020,
    ES2021,
    ES2022,
    ES2023,
    ES2024,
    ESNext,
    Latest,
}

/// Module kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleKind {
    None,
    CommonJS,
    AMD,
    UMD,
    System,
    ES2015,
    ES2020,
    ES2022,
    ESNext,
    Node16,
    NodeNext,
    Preserve,
}

/// JSX emit mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JsxEmit {
    None,
    Preserve,
    React,
    ReactNative,
    ReactJSX,
    ReactJSXDev,
}

/// The tsconfig.json file structure.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TsConfig {
    pub compiler_options: Option<CompilerOptions>,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    pub files: Option<Vec<String>>,
    pub extends: Option<String>,
    pub references: Option<Vec<ProjectReference>>,
}

/// A project reference in tsconfig.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectReference {
    pub path: String,
    pub prepend: Option<bool>,
}

/// Parse a tsconfig.json file from a string.
pub fn parse_tsconfig(content: &str) -> Result<TsConfig, serde_json::Error> {
    serde_json::from_str(content)
}

/// Parse a tsconfig.json file from a path.
pub fn parse_tsconfig_file(path: &str) -> Result<TsConfig, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let config = parse_tsconfig(&content)?;
    Ok(config)
}
