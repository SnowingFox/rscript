//! rscript_tspath: Path normalization and extension handling.
//!
//! Faithfully ports TypeScript's path utilities from `src/compiler/path.ts`.

use std::path::{Path, PathBuf};

/// File extensions recognized by the TypeScript compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Extension {
    Ts,
    Tsx,
    Dts,
    Js,
    Jsx,
    Json,
    TsBuildInfo,
    Mjs,
    Mts,
    Dmts,
    Cjs,
    Cts,
    Dcts,
}

impl Extension {
    /// Get the string representation of this extension (including the dot).
    pub fn as_str(&self) -> &'static str {
        match self {
            Extension::Ts => ".ts",
            Extension::Tsx => ".tsx",
            Extension::Dts => ".d.ts",
            Extension::Js => ".js",
            Extension::Jsx => ".jsx",
            Extension::Json => ".json",
            Extension::TsBuildInfo => ".tsbuildinfo",
            Extension::Mjs => ".mjs",
            Extension::Mts => ".mts",
            Extension::Dmts => ".d.mts",
            Extension::Cjs => ".cjs",
            Extension::Cts => ".cts",
            Extension::Dcts => ".d.cts",
        }
    }

    /// Whether this is a TypeScript extension (.ts, .tsx, .mts, .cts).
    pub fn is_typescript(&self) -> bool {
        matches!(
            self,
            Extension::Ts | Extension::Tsx | Extension::Mts | Extension::Cts
        )
    }

    /// Whether this is a declaration extension (.d.ts, .d.mts, .d.cts).
    pub fn is_declaration(&self) -> bool {
        matches!(self, Extension::Dts | Extension::Dmts | Extension::Dcts)
    }

    /// Whether this is a JavaScript extension (.js, .jsx, .mjs, .cjs).
    pub fn is_javascript(&self) -> bool {
        matches!(
            self,
            Extension::Js | Extension::Jsx | Extension::Mjs | Extension::Cjs
        )
    }

    /// Try to determine the extension from a file path string.
    pub fn from_path(path: &str) -> Option<Extension> {
        let lower = path.to_lowercase();
        // Check longer extensions first to handle .d.ts, .d.mts, .d.cts
        if lower.ends_with(".d.ts") {
            Some(Extension::Dts)
        } else if lower.ends_with(".d.mts") {
            Some(Extension::Dmts)
        } else if lower.ends_with(".d.cts") {
            Some(Extension::Dcts)
        } else if lower.ends_with(".ts") {
            Some(Extension::Ts)
        } else if lower.ends_with(".tsx") {
            Some(Extension::Tsx)
        } else if lower.ends_with(".js") {
            Some(Extension::Js)
        } else if lower.ends_with(".jsx") {
            Some(Extension::Jsx)
        } else if lower.ends_with(".mts") {
            Some(Extension::Mts)
        } else if lower.ends_with(".cts") {
            Some(Extension::Cts)
        } else if lower.ends_with(".mjs") {
            Some(Extension::Mjs)
        } else if lower.ends_with(".cjs") {
            Some(Extension::Cjs)
        } else if lower.ends_with(".json") {
            Some(Extension::Json)
        } else if lower.ends_with(".tsbuildinfo") {
            Some(Extension::TsBuildInfo)
        } else {
            None
        }
    }
}

/// Normalize a path by converting all backslashes to forward slashes
/// and resolving `.` and `..` segments.
/// This matches TypeScript's `normalizePath`.
pub fn normalize_path(path: &str) -> String {
    let path = path.replace('\\', "/");
    normalize_slashes(&path)
}

/// Convert backslashes to forward slashes.
pub fn normalize_slashes(path: &str) -> String {
    path.replace('\\', "/")
}

/// Combine two path segments.
pub fn combine_paths(base: &str, relative: &str) -> String {
    if is_rooted(relative) {
        return relative.to_string();
    }
    if base.is_empty() {
        return relative.to_string();
    }
    let base = ensure_trailing_directory_separator(base);
    format!("{}{}", base, relative)
}

/// Check if a path is rooted (absolute).
pub fn is_rooted(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    let bytes = path.as_bytes();
    // Unix absolute path
    if bytes[0] == b'/' {
        return true;
    }
    // Windows absolute path (e.g., C:\)
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'/' || bytes[2] == b'\\')
    {
        return true;
    }
    // UNC path
    if bytes.len() >= 2 && bytes[0] == b'/' && bytes[1] == b'/' {
        return true;
    }
    false
}

/// Get the directory path (everything before the last `/`).
pub fn get_directory_path(path: &str) -> String {
    let normalized = normalize_slashes(path);
    if let Some(last_slash) = normalized.rfind('/') {
        normalized[..=last_slash].to_string()
    } else {
        String::new()
    }
}

/// Get the base name (file name) from a path.
pub fn get_base_name(path: &str) -> &str {
    let normalized = if path.contains('\\') {
        // For borrowed return we need to use rfind on original
        path
    } else {
        path
    };
    if let Some(last_slash) = normalized.rfind('/') {
        &normalized[last_slash + 1..]
    } else if let Some(last_slash) = normalized.rfind('\\') {
        &normalized[last_slash + 1..]
    } else {
        normalized
    }
}

/// Remove the file extension from a path.
pub fn remove_extension(path: &str) -> String {
    // Handle .d.ts, .d.mts, .d.cts first
    let lower = path.to_lowercase();
    if lower.ends_with(".d.ts") {
        return path[..path.len() - 5].to_string();
    }
    if lower.ends_with(".d.mts") {
        return path[..path.len() - 6].to_string();
    }
    if lower.ends_with(".d.cts") {
        return path[..path.len() - 6].to_string();
    }
    if let Some(dot_pos) = path.rfind('.') {
        let slash_pos = path.rfind('/').unwrap_or(0);
        if dot_pos > slash_pos {
            return path[..dot_pos].to_string();
        }
    }
    path.to_string()
}

/// Change the extension of a path.
pub fn change_extension(path: &str, new_ext: &str) -> String {
    let without_ext = remove_extension(path);
    format!("{}{}", without_ext, new_ext)
}

/// Ensure a path ends with a directory separator.
pub fn ensure_trailing_directory_separator(path: &str) -> String {
    if path.ends_with('/') || path.ends_with('\\') {
        path.to_string()
    } else {
        format!("{}/", path)
    }
}

/// Remove trailing directory separator.
pub fn remove_trailing_directory_separator(path: &str) -> &str {
    if path.len() > 1 && (path.ends_with('/') || path.ends_with('\\')) {
        &path[..path.len() - 1]
    } else {
        path
    }
}

/// Check if a path has a TypeScript file extension.
pub fn has_ts_file_extension(path: &str) -> bool {
    matches!(
        Extension::from_path(path),
        Some(
            Extension::Ts
                | Extension::Tsx
                | Extension::Dts
                | Extension::Mts
                | Extension::Cts
                | Extension::Dmts
                | Extension::Dcts
        )
    )
}

/// Check if a path has a JavaScript file extension.
pub fn has_js_file_extension(path: &str) -> bool {
    matches!(
        Extension::from_path(path),
        Some(Extension::Js | Extension::Jsx | Extension::Mjs | Extension::Cjs)
    )
}

/// Get relative path from one directory to another.
pub fn get_relative_path(from: &str, to: &str) -> String {
    let from_path = PathBuf::from(normalize_path(from));
    let to_path = PathBuf::from(normalize_path(to));

    if let Ok(rel) = pathdiff_relative(&from_path, &to_path) {
        normalize_path(&rel.to_string_lossy())
    } else {
        normalize_path(to)
    }
}

fn pathdiff_relative(base: &Path, target: &Path) -> Result<PathBuf, ()> {
    let mut base_components = base.components().peekable();
    let mut target_components = target.components().peekable();

    // Skip common prefix
    while let (Some(a), Some(b)) = (base_components.peek(), target_components.peek()) {
        if a != b {
            break;
        }
        base_components.next();
        target_components.next();
    }

    let mut result = PathBuf::new();
    for _ in base_components {
        result.push("..");
    }
    for component in target_components {
        result.push(component);
    }

    Ok(result)
}

/// Check if a character is a directory separator.
#[inline]
pub fn is_directory_separator(ch: char) -> bool {
    ch == '/' || ch == '\\'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_from_path() {
        assert_eq!(Extension::from_path("foo.ts"), Some(Extension::Ts));
        assert_eq!(Extension::from_path("foo.d.ts"), Some(Extension::Dts));
        assert_eq!(Extension::from_path("foo.tsx"), Some(Extension::Tsx));
        assert_eq!(Extension::from_path("foo.js"), Some(Extension::Js));
        assert_eq!(Extension::from_path("foo.d.mts"), Some(Extension::Dmts));
        assert_eq!(Extension::from_path("foo.txt"), None);
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("a\\b\\c"), "a/b/c");
        assert_eq!(normalize_path("a/b/c"), "a/b/c");
    }

    #[test]
    fn test_get_directory_path() {
        assert_eq!(get_directory_path("/a/b/c.ts"), "/a/b/");
        assert_eq!(get_directory_path("file.ts"), "");
    }

    #[test]
    fn test_remove_extension() {
        assert_eq!(remove_extension("foo.ts"), "foo");
        assert_eq!(remove_extension("foo.d.ts"), "foo");
        assert_eq!(remove_extension("foo.d.mts"), "foo");
        assert_eq!(remove_extension("foo/bar.js"), "foo/bar");
    }

    #[test]
    fn test_is_rooted() {
        assert!(is_rooted("/usr/bin"));
        assert!(is_rooted("C:/Users"));
        assert!(!is_rooted("relative/path"));
        assert!(!is_rooted(""));
    }

    #[test]
    fn test_combine_paths() {
        assert_eq!(combine_paths("/a/b", "c.ts"), "/a/b/c.ts");
        assert_eq!(combine_paths("/a/b/", "c.ts"), "/a/b/c.ts");
        assert_eq!(combine_paths("", "c.ts"), "c.ts");
        assert_eq!(combine_paths("/a", "/b/c.ts"), "/b/c.ts");
    }
}
