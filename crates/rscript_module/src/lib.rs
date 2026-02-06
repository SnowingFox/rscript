//! rscript_module: Module resolution.
//!
//! Implements TypeScript's module resolution algorithms:
//! - Node10 (classic node_modules)
//! - Node16/NodeNext (ESM-aware)
//! - Bundler (for bundler-like resolution)
//! - Classic (TypeScript's original resolution)

use rscript_tspath::Extension;
use std::path::{Path, PathBuf};

/// Module resolution strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleResolutionKind {
    Classic,
    Node10,
    Node16,
    NodeNext,
    Bundler,
}

/// The result of resolving a module.
#[derive(Debug, Clone)]
pub struct ResolvedModule {
    /// The resolved file path.
    pub resolved_file_name: String,
    /// The extension of the resolved file.
    pub extension: Extension,
    /// Whether resolution went through a package.json.
    pub is_external_library_import: bool,
    /// The package.json path (if applicable).
    pub package_json_path: Option<String>,
}

/// Module resolution options.
#[derive(Debug, Clone)]
pub struct ModuleResolutionOptions {
    pub kind: ModuleResolutionKind,
    pub root_dir: String,
    pub base_url: Option<String>,
    pub paths: Vec<(String, Vec<String>)>,
    pub root_dirs: Vec<String>,
    pub type_roots: Option<Vec<String>>,
    pub node_modules_search_dirs: Vec<String>,
}

/// Parsed package.json relevant fields.
#[derive(Debug, Clone, Default)]
pub struct PackageJson {
    pub name: Option<String>,
    pub version: Option<String>,
    pub main: Option<String>,
    pub module: Option<String>,
    pub types: Option<String>,
    pub typings: Option<String>,
    pub exports: Option<PackageExports>,
    pub type_field: Option<String>, // "module" or "commonjs"
}

/// Package.json exports field (simplified).
#[derive(Debug, Clone)]
pub enum PackageExports {
    /// Simple string export
    String(String),
    /// Conditional exports map
    Map(Vec<(String, PackageExports)>),
    /// Array of fallback exports
    Array(Vec<PackageExports>),
}

/// Parse a package.json from a string.
pub fn parse_package_json(content: &str) -> Option<PackageJson> {
    let v: serde_json::Value = serde_json::from_str(content).ok()?;
    let obj = v.as_object()?;

    Some(PackageJson {
        name: obj.get("name").and_then(|v| v.as_str()).map(String::from),
        version: obj.get("version").and_then(|v| v.as_str()).map(String::from),
        main: obj.get("main").and_then(|v| v.as_str()).map(String::from),
        module: obj.get("module").and_then(|v| v.as_str()).map(String::from),
        types: obj.get("types").and_then(|v| v.as_str()).map(String::from),
        typings: obj.get("typings").and_then(|v| v.as_str()).map(String::from),
        exports: parse_exports_field(obj.get("exports")),
        type_field: obj.get("type").and_then(|v| v.as_str()).map(String::from),
    })
}

fn parse_exports_field(value: Option<&serde_json::Value>) -> Option<PackageExports> {
    let v = value?;
    match v {
        serde_json::Value::String(s) => Some(PackageExports::String(s.clone())),
        serde_json::Value::Object(obj) => {
            let entries: Vec<(String, PackageExports)> = obj.iter()
                .filter_map(|(k, v)| {
                    parse_exports_field(Some(v)).map(|e| (k.clone(), e))
                })
                .collect();
            Some(PackageExports::Map(entries))
        }
        serde_json::Value::Array(arr) => {
            let entries: Vec<PackageExports> = arr.iter()
                .filter_map(|v| parse_exports_field(Some(v)))
                .collect();
            Some(PackageExports::Array(entries))
        }
        _ => None,
    }
}

/// Resolve a module name to a file path.
pub fn resolve_module_name(
    module_name: &str,
    containing_file: &str,
    options: &ModuleResolutionOptions,
) -> Option<ResolvedModule> {
    // Try path mappings first
    if let Some(resolved) = try_path_mappings(module_name, options) {
        return Some(resolved);
    }

    match options.kind {
        ModuleResolutionKind::Node10 => resolve_node10(module_name, containing_file, options),
        ModuleResolutionKind::Node16 | ModuleResolutionKind::NodeNext => {
            resolve_node16(module_name, containing_file, options)
        }
        ModuleResolutionKind::Bundler => resolve_bundler(module_name, containing_file, options),
        ModuleResolutionKind::Classic => resolve_classic(module_name, containing_file, options),
    }
}

/// Try to resolve using tsconfig paths mappings.
fn try_path_mappings(
    module_name: &str,
    options: &ModuleResolutionOptions,
) -> Option<ResolvedModule> {
    let base_url = options.base_url.as_deref()?;

    for (pattern, substitutions) in &options.paths {
        if pattern == module_name {
            // Exact match
            for sub in substitutions {
                let candidate = format!("{}/{}", base_url, sub);
                if let Some(resolved) = try_file_extensions(&candidate) {
                    return Some(resolved);
                }
            }
        } else if pattern.ends_with('*') {
            // Wildcard match
            let prefix = &pattern[..pattern.len() - 1];
            if let Some(rest) = module_name.strip_prefix(prefix) {
                for sub in substitutions {
                    let actual = sub.replace('*', rest);
                    let candidate = format!("{}/{}", base_url, actual);
                    if let Some(resolved) = try_file_extensions(&candidate) {
                        return Some(resolved);
                    }
                }
            }
        }
    }
    None
}

const TS_EXTENSIONS: &[Extension] = &[Extension::Ts, Extension::Tsx, Extension::Dts];
#[allow(dead_code)]
const JS_EXTENSIONS: &[Extension] = &[Extension::Js, Extension::Jsx];
const ALL_EXTENSIONS: &[Extension] = &[Extension::Ts, Extension::Tsx, Extension::Dts, Extension::Js, Extension::Jsx];

fn try_file_extensions(candidate: &str) -> Option<ResolvedModule> {
    // Try the path as-is first (if it already has an extension)
    if Path::new(candidate).exists() {
        let ext = detect_extension(candidate);
        return Some(ResolvedModule {
            resolved_file_name: candidate.to_string(),
            extension: ext,
            is_external_library_import: false,
            package_json_path: None,
        });
    }

    // Try with extensions
    for ext in ALL_EXTENSIONS {
        let path = format!("{}{}", candidate, ext.as_str());
        if Path::new(&path).exists() {
            return Some(ResolvedModule {
                resolved_file_name: path,
                extension: *ext,
                is_external_library_import: false,
                package_json_path: None,
            });
        }
    }

    // Try /index
    for ext in ALL_EXTENSIONS {
        let path = format!("{}/index{}", candidate, ext.as_str());
        if Path::new(&path).exists() {
            return Some(ResolvedModule {
                resolved_file_name: path,
                extension: *ext,
                is_external_library_import: false,
                package_json_path: None,
            });
        }
    }

    None
}

fn detect_extension(path: &str) -> Extension {
    if path.ends_with(".d.ts") { Extension::Dts }
    else if path.ends_with(".ts") { Extension::Ts }
    else if path.ends_with(".tsx") { Extension::Tsx }
    else if path.ends_with(".jsx") { Extension::Jsx }
    else if path.ends_with(".js") { Extension::Js }
    else if path.ends_with(".json") { Extension::Json }
    else { Extension::Js }
}

fn resolve_node10(
    module_name: &str,
    containing_file: &str,
    options: &ModuleResolutionOptions,
) -> Option<ResolvedModule> {
    let containing_dir = rscript_tspath::get_directory_path(containing_file);

    // Relative import
    if module_name.starts_with('.') {
        let candidate = rscript_tspath::combine_paths(&containing_dir, module_name);
        return try_file_extensions(&candidate);
    }

    // Non-relative (bare specifier): search node_modules
    resolve_node_modules(module_name, &containing_dir, options)
}

/// Resolve a bare module specifier by walking up the directory tree
/// looking in node_modules directories.
fn resolve_node_modules(
    module_name: &str,
    starting_dir: &str,
    _options: &ModuleResolutionOptions,
) -> Option<ResolvedModule> {
    let mut dir = PathBuf::from(starting_dir);
    loop {
        let node_modules = dir.join("node_modules");
        if node_modules.exists() {
            // Split module name for scoped packages (@scope/name)
            let (package_name, subpath) = split_module_name(module_name);
            let package_dir = node_modules.join(package_name);

            if package_dir.exists() {
                // Try package.json resolution
                let pkg_json_path = package_dir.join("package.json");
                if pkg_json_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&pkg_json_path) {
                        if let Some(pkg) = parse_package_json(&content) {
                            // If subpath is specified, resolve within the package
                            if !subpath.is_empty() {
                                let sub_candidate = package_dir.join(subpath).to_string_lossy().to_string();
                                if let Some(mut resolved) = try_file_extensions(&sub_candidate) {
                                    resolved.is_external_library_import = true;
                                    resolved.package_json_path = Some(pkg_json_path.to_string_lossy().to_string());
                                    return Some(resolved);
                                }
                            }

                            // Try "types" or "typings" field
                            if let Some(ref types_entry) = pkg.types.as_ref().or(pkg.typings.as_ref()) {
                                let types_path = package_dir.join(types_entry);
                                if types_path.exists() {
                                    return Some(ResolvedModule {
                                        resolved_file_name: types_path.to_string_lossy().to_string(),
                                        extension: detect_extension(&types_path.to_string_lossy()),
                                        is_external_library_import: true,
                                        package_json_path: Some(pkg_json_path.to_string_lossy().to_string()),
                                    });
                                }
                            }

                            // Try "main" field
                            if let Some(ref main_entry) = pkg.main {
                                let main_path = package_dir.join(main_entry);
                                if let Some(mut resolved) = try_file_extensions(&main_path.to_string_lossy()) {
                                    resolved.is_external_library_import = true;
                                    resolved.package_json_path = Some(pkg_json_path.to_string_lossy().to_string());
                                    return Some(resolved);
                                }
                            }
                        }
                    }
                }

                // Try index files
                let pkg_str = package_dir.to_string_lossy().to_string();
                if let Some(mut resolved) = try_file_extensions(&pkg_str) {
                    resolved.is_external_library_import = true;
                    return Some(resolved);
                }
            }

            // Try @types package
            let at_types_dir = node_modules.join("@types").join(package_name.trim_start_matches('@').replace('/', "__"));
            if at_types_dir.exists() {
                let idx_path = at_types_dir.join("index.d.ts");
                if idx_path.exists() {
                    return Some(ResolvedModule {
                        resolved_file_name: idx_path.to_string_lossy().to_string(),
                        extension: Extension::Dts,
                        is_external_library_import: true,
                        package_json_path: None,
                    });
                }
            }
        }

        if !dir.pop() {
            break;
        }
    }

    None
}

/// Split a module name into package name and subpath.
/// e.g. "@scope/pkg/sub/path" -> ("@scope/pkg", "sub/path")
/// e.g. "lodash/fp" -> ("lodash", "fp")
fn split_module_name(module_name: &str) -> (&str, &str) {
    if let Some(rest) = module_name.strip_prefix('@') {
        // Scoped package: @scope/pkg/sub/path
        if let Some(slash_pos) = rest.find('/') {
            let first_slash = slash_pos + 1; // position in `rest`
            if let Some(second_slash) = rest[first_slash + 1..].find('/') {
                let split_pos = first_slash + 1 + second_slash + 1; // +1 for the '@'
                return (&module_name[..split_pos], &module_name[split_pos + 1..]);
            }
            return (module_name, "");
        }
        return (module_name, "");
    }

    if let Some(pos) = module_name.find('/') {
        (&module_name[..pos], &module_name[pos + 1..])
    } else {
        (module_name, "")
    }
}

fn resolve_node16(
    module_name: &str,
    containing_file: &str,
    options: &ModuleResolutionOptions,
) -> Option<ResolvedModule> {
    // Node16/NodeNext is similar to Node10 with ESM awareness
    // In ESM mode, relative imports must include extensions
    // For now, fall back to Node10 resolution
    resolve_node10(module_name, containing_file, options)
}

fn resolve_bundler(
    module_name: &str,
    containing_file: &str,
    options: &ModuleResolutionOptions,
) -> Option<ResolvedModule> {
    // Bundler resolution is similar to Node but without strict ESM rules
    resolve_node10(module_name, containing_file, options)
}

fn resolve_classic(
    module_name: &str,
    containing_file: &str,
    _options: &ModuleResolutionOptions,
) -> Option<ResolvedModule> {
    let containing_dir = rscript_tspath::get_directory_path(containing_file);

    if module_name.starts_with('.') {
        let candidate = rscript_tspath::combine_paths(&containing_dir, module_name);
        for ext in TS_EXTENSIONS {
            let path = format!("{}{}", candidate, ext.as_str());
            if Path::new(&path).exists() {
                return Some(ResolvedModule {
                    resolved_file_name: path,
                    extension: *ext,
                    is_external_library_import: false,
                    package_json_path: None,
                });
            }
        }
    } else {
        // Non-relative: walk up directory tree looking for .ts files
        let mut dir = PathBuf::from(&containing_dir);
        loop {
            for ext in TS_EXTENSIONS {
                let path = dir.join(format!("{}{}", module_name, ext.as_str()));
                if path.exists() {
                    return Some(ResolvedModule {
                        resolved_file_name: path.to_string_lossy().to_string(),
                        extension: *ext,
                        is_external_library_import: false,
                        package_json_path: None,
                    });
                }
            }
            if !dir.pop() { break; }
        }
    }

    None
}

/// Discover source files matching include/exclude patterns.
pub fn discover_source_files(
    root_dir: &str,
    include: &[String],
    exclude: &[String],
    files: Option<&[String]>,
) -> Vec<String> {
    let mut result = Vec::new();

    // If explicit "files" array is provided, use that
    if let Some(file_list) = files {
        for f in file_list {
            let path = if Path::new(f).is_absolute() {
                PathBuf::from(f)
            } else {
                PathBuf::from(root_dir).join(f)
            };
            if path.exists() {
                result.push(path.to_string_lossy().to_string());
            }
        }
        return result;
    }

    // Otherwise, use include patterns (simplified glob matching)
    for pattern in include {
        collect_matching_files(root_dir, pattern, exclude, &mut result);
    }

    result.sort();
    result.dedup();
    result
}

fn collect_matching_files(
    root_dir: &str,
    pattern: &str,
    exclude: &[String],
    result: &mut Vec<String>,
) {
    let root = PathBuf::from(root_dir);

    // Simple pattern handling
    if pattern.contains("**") {
        // Recursive glob - walk directory tree
        walk_directory(&root, &root, pattern, exclude, result);
    } else if pattern.contains('*') {
        // Single-level glob
        if let Ok(entries) = std::fs::read_dir(&root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && matches_simple_glob(&path, pattern) {
                    let path_str = path.to_string_lossy().to_string();
                    if !is_excluded(&path_str, exclude) {
                        result.push(path_str);
                    }
                }
            }
        }
    } else {
        // Literal path
        let path = root.join(pattern);
        if path.exists() && path.is_file() {
            let path_str = path.to_string_lossy().to_string();
            if !is_excluded(&path_str, exclude) {
                result.push(path_str);
            }
        }
    }
}

#[allow(clippy::only_used_in_recursion)]
fn walk_directory(
    base: &Path,
    dir: &Path,
    pattern: &str,
    exclude: &[String],
    result: &mut Vec<String>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let path_str = path.to_string_lossy().to_string();

        if is_excluded(&path_str, exclude) {
            continue;
        }

        if path.is_dir() {
            // Skip common non-source directories
            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if dir_name == "node_modules" || dir_name == ".git" || dir_name == "dist" || dir_name == "build" {
                continue;
            }
            walk_directory(base, &path, pattern, exclude, result);
        } else if path.is_file() {
            // Check if file matches the pattern's extension part
            let ext_part = extract_extension_pattern(pattern);
            if matches_extension(&path, ext_part) {
                result.push(path_str);
            }
        }
    }
}

fn extract_extension_pattern(pattern: &str) -> &str {
    // Extract the file extension pattern from a glob like "**/*.ts"
    if let Some(dot_pos) = pattern.rfind('.') {
        &pattern[dot_pos..]
    } else {
        ""
    }
}

fn matches_extension(path: &Path, ext_pattern: &str) -> bool {
    if ext_pattern.is_empty() {
        // Match default TS extensions
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        matches!(ext, "ts" | "tsx" | "js" | "jsx" | "mts" | "mjs" | "cts" | "cjs")
    } else {
        let path_str = path.to_string_lossy();
        path_str.ends_with(ext_pattern)
    }
}

fn matches_simple_glob(path: &Path, pattern: &str) -> bool {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if let Some(star_pos) = pattern.find('*') {
        let prefix = &pattern[..star_pos];
        let suffix = &pattern[star_pos + 1..];
        file_name.starts_with(prefix) && file_name.ends_with(suffix)
    } else {
        file_name == pattern
    }
}

fn is_excluded(path: &str, exclude: &[String]) -> bool {
    for pattern in exclude {
        if pattern.contains("node_modules") && path.contains("node_modules") {
            return true;
        }
        if path.contains(pattern.trim_start_matches("./").trim_start_matches("**/")) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_module_name_simple() {
        let (pkg, sub) = split_module_name("lodash");
        assert_eq!(pkg, "lodash");
        assert_eq!(sub, "");
    }

    #[test]
    fn test_split_module_name_with_subpath() {
        let (pkg, sub) = split_module_name("lodash/fp");
        assert_eq!(pkg, "lodash");
        assert_eq!(sub, "fp");
    }

    #[test]
    fn test_split_module_name_scoped() {
        let (pkg, sub) = split_module_name("@types/node");
        assert_eq!(pkg, "@types/node");
        assert_eq!(sub, "");
    }

    #[test]
    fn test_split_module_name_scoped_with_subpath() {
        let (pkg, sub) = split_module_name("@angular/core/testing");
        assert_eq!(pkg, "@angular/core");
        assert_eq!(sub, "testing");
    }

    #[test]
    fn test_parse_package_json() {
        let content = r#"{"name": "test", "version": "1.0.0", "main": "index.js", "types": "index.d.ts"}"#;
        let pkg = parse_package_json(content).unwrap();
        assert_eq!(pkg.name, Some("test".to_string()));
        assert_eq!(pkg.main, Some("index.js".to_string()));
        assert_eq!(pkg.types, Some("index.d.ts".to_string()));
    }

    #[test]
    fn test_parse_package_json_with_exports() {
        let content = r#"{"name": "test", "exports": {"./foo": "./dist/foo.js"}}"#;
        let pkg = parse_package_json(content).unwrap();
        assert!(pkg.exports.is_some());
    }

    #[test]
    fn test_detect_extension() {
        assert_eq!(detect_extension("foo.ts"), Extension::Ts);
        assert_eq!(detect_extension("foo.tsx"), Extension::Tsx);
        assert_eq!(detect_extension("foo.d.ts"), Extension::Dts);
        assert_eq!(detect_extension("foo.js"), Extension::Js);
        assert_eq!(detect_extension("foo.jsx"), Extension::Jsx);
    }

    #[test]
    fn test_is_excluded() {
        assert!(is_excluded("/project/node_modules/foo", &["**/node_modules/**".to_string()]));
        assert!(!is_excluded("/project/src/foo.ts", &["**/node_modules/**".to_string()]));
    }
}
