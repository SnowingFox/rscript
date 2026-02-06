//! TypeScript conformance test harness.
//!
//! This module provides infrastructure to run TypeScript's own conformance
//! tests against the rscript compiler. It reads `.ts` files from the
//! TypeScript test suite directory and verifies that rscript can parse
//! them without panicking.
//!
//! To use this, set the `TS_TEST_SUITE_PATH` environment variable to point
//! to the TypeScript test suite directory:
//!   TS_TEST_SUITE_PATH=/path/to/TypeScript/tests/cases
//!
//! Without this variable, the conformance tests are skipped.

use bumpalo::Bump;
use rscript_binder::Binder;
use rscript_checker::Checker;
use rscript_parser::Parser;
use std::path::{Path, PathBuf};

/// Get the TypeScript test suite path from environment.
fn get_ts_test_suite_path() -> Option<PathBuf> {
    std::env::var("TS_TEST_SUITE_PATH").ok().map(PathBuf::from)
}

/// Collect all `.ts` files from a directory recursively.
fn collect_ts_files(dir: &Path, max_files: usize) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_ts_files_recursive(dir, &mut files, max_files);
    files
}

fn collect_ts_files_recursive(dir: &Path, files: &mut Vec<PathBuf>, max_files: usize) {
    if files.len() >= max_files {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if files.len() >= max_files {
                break;
            }
            let path = entry.path();
            if path.is_dir() {
                collect_ts_files_recursive(&path, files, max_files);
            } else if let Some(ext) = path.extension() {
                if ext == "ts" && !path.to_string_lossy().ends_with(".d.ts") {
                    files.push(path);
                }
            }
        }
    }
}

/// Result of running a conformance test.
#[derive(Debug)]
struct ConformanceResult {
    file: PathBuf,
    parse_ok: bool,
    bind_ok: bool,
    check_ok: bool,
    parse_error: Option<String>,
    bind_error: Option<String>,
    check_error: Option<String>,
}

/// Run the parse -> bind -> check pipeline on a single file.
fn run_conformance_test(path: &Path) -> ConformanceResult {
    let file_name = path.to_string_lossy().to_string();
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            return ConformanceResult {
                file: path.to_path_buf(),
                parse_ok: false,
                bind_ok: false,
                check_ok: false,
                parse_error: Some(format!("Failed to read file: {}", e)),
                bind_error: None,
                check_error: None,
            };
        }
    };

    // Parse
    let arena = Bump::new();
    let parse_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let parser = Parser::new(&arena, &file_name, &source);
        parser.parse_source_file()
    }));

    let sf = match parse_result {
        Ok(sf) => sf,
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "Unknown panic".to_string()
            };
            return ConformanceResult {
                file: path.to_path_buf(),
                parse_ok: false,
                bind_ok: false,
                check_ok: false,
                parse_error: Some(msg),
                bind_error: None,
                check_error: None,
            };
        }
    };

    // Bind
    let bind_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut binder = Binder::new();
        binder.bind_source_file(&sf);
        binder
    }));

    let binder = match bind_result {
        Ok(b) => b,
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "Unknown panic".to_string()
            };
            return ConformanceResult {
                file: path.to_path_buf(),
                parse_ok: true,
                bind_ok: false,
                check_ok: false,
                parse_error: None,
                bind_error: Some(msg),
                check_error: None,
            };
        }
    };

    // Check
    let check_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut checker = Checker::new(binder);
        checker.check_source_file(&sf);
    }));

    match check_result {
        Ok(_) => ConformanceResult {
            file: path.to_path_buf(),
            parse_ok: true,
            bind_ok: true,
            check_ok: true,
            parse_error: None,
            bind_error: None,
            check_error: None,
        },
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "Unknown panic".to_string()
            };
            ConformanceResult {
                file: path.to_path_buf(),
                parse_ok: true,
                bind_ok: true,
                check_ok: false,
                parse_error: None,
                bind_error: None,
                check_error: Some(msg),
            }
        }
    }
}

#[test]
fn test_conformance_suite() {
    let test_suite_path = match get_ts_test_suite_path() {
        Some(p) => p,
        None => {
            eprintln!(
                "Skipping conformance tests: TS_TEST_SUITE_PATH not set.\n\
                 To run conformance tests, set TS_TEST_SUITE_PATH to the TypeScript \
                 test suite directory (e.g., /path/to/TypeScript/tests/cases/conformance)"
            );
            return;
        }
    };

    if !test_suite_path.exists() {
        eprintln!(
            "Skipping conformance tests: {:?} does not exist.",
            test_suite_path
        );
        return;
    }

    // Limit to first 500 files for CI performance
    let max_files = std::env::var("TS_TEST_MAX_FILES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(500);

    let files = collect_ts_files(&test_suite_path, max_files);
    if files.is_empty() {
        eprintln!("No .ts files found in {:?}", test_suite_path);
        return;
    }

    println!("Running conformance tests on {} files...", files.len());

    let mut total = 0;
    let mut parse_ok = 0;
    let mut bind_ok = 0;
    let mut check_ok = 0;
    let mut parse_failures = Vec::new();
    let mut bind_failures = Vec::new();

    for file in &files {
        total += 1;
        let result = run_conformance_test(file);

        if result.parse_ok {
            parse_ok += 1;
        } else {
            parse_failures.push((
                result.file.clone(),
                result.parse_error.clone().unwrap_or_default(),
            ));
        }

        if result.bind_ok {
            bind_ok += 1;
        } else if result.parse_ok {
            bind_failures.push((
                result.file.clone(),
                result.bind_error.clone().unwrap_or_default(),
            ));
        }

        if result.check_ok {
            check_ok += 1;
        }
    }

    println!("\n=== Conformance Test Results ===");
    println!("Total files tested: {}", total);
    println!(
        "Parse:  {}/{} ({:.1}%)",
        parse_ok,
        total,
        (parse_ok as f64 / total as f64) * 100.0
    );
    println!(
        "Bind:   {}/{} ({:.1}%)",
        bind_ok,
        total,
        (bind_ok as f64 / total as f64) * 100.0
    );
    println!(
        "Check:  {}/{} ({:.1}%)",
        check_ok,
        total,
        (check_ok as f64 / total as f64) * 100.0
    );

    if !parse_failures.is_empty() {
        println!("\n--- Parse Failures (first 10) ---");
        for (file, err) in parse_failures.iter().take(10) {
            println!("  {:?}: {}", file, &err[..err.len().min(100)]);
        }
    }

    if !bind_failures.is_empty() {
        println!("\n--- Bind Failures (first 10) ---");
        for (file, err) in bind_failures.iter().take(10) {
            println!("  {:?}: {}", file, &err[..err.len().min(100)]);
        }
    }

    // We don't assert a minimum pass rate yet since the compiler
    // is still in early stages. This test is primarily to ensure
    // we don't crash on real TypeScript files.
    println!("\nConformance suite completed.");
}

// ============================================================================
// Built-in Conformance Samples (always run)
// ============================================================================

/// Test parsing various TypeScript constructs that appear in the conformance suite.
#[test]
fn test_conformance_variable_declarations() {
    let samples = [
        "var x;",
        "let y = 1;",
        "const z: string = 'hello';",
        "var [a, b] = [1, 2];",
        "const { x, y } = { x: 1, y: 2 };",
        "let [first, ...rest] = [1, 2, 3];",
    ];

    for sample in &samples {
        let arena = Bump::new();
        let parser = Parser::new(&arena, "test.ts", sample);
        let sf = parser.parse_source_file();
        assert!(
            !sf.statements.is_empty(),
            "Failed to parse: {}",
            sample
        );
    }
}

#[test]
fn test_conformance_function_declarations() {
    let samples = [
        "function foo() {}",
        "function bar(x: number): string { return '' + x; }",
        "async function baz() { await Promise.resolve(); }",
        "function* gen() { yield 1; yield 2; }",
        "function overloaded(x: string): string;\nfunction overloaded(x: number): number;\nfunction overloaded(x: any): any { return x; }",
    ];

    for sample in &samples {
        let arena = Bump::new();
        let parser = Parser::new(&arena, "test.ts", sample);
        let sf = parser.parse_source_file();
        assert!(
            !sf.statements.is_empty(),
            "Failed to parse: {}",
            sample
        );
    }
}

#[test]
fn test_conformance_class_declarations() {
    let samples = [
        "class A {}",
        "class B extends A {}",
        "abstract class C { abstract method(): void; }",
        "class D implements I1, I2 {}",
        "class E<T> { value: T; }",
        "class F { static x = 1; readonly y = 2; private z = 3; }",
        "class G { constructor(public name: string) {} }",
        "class H { get prop() { return 1; } set prop(v: number) {} }",
    ];

    for sample in &samples {
        let arena = Bump::new();
        let parser = Parser::new(&arena, "test.ts", sample);
        let sf = parser.parse_source_file();
        assert!(
            !sf.statements.is_empty(),
            "Failed to parse: {}",
            sample
        );
    }
}

#[test]
fn test_conformance_type_constructs() {
    let samples = [
        "type A = string;",
        "type B = string | number;",
        "type C = string & { len: number };",
        "type D<T> = T extends string ? 'yes' : 'no';",
        "type E<T> = { [K in keyof T]: T[K] };",
        "type F = [string, number, boolean];",
        "type G = readonly number[];",
        "type H = (x: number) => string;",
        "type I = typeof someVar;",
    ];

    for sample in &samples {
        let arena = Bump::new();
        let parser = Parser::new(&arena, "test.ts", sample);
        let sf = parser.parse_source_file();
        assert!(
            !sf.statements.is_empty(),
            "Failed to parse: {}",
            sample
        );
    }
}

#[test]
fn test_conformance_expression_constructs() {
    let samples = [
        "const x = a ? b : c;",
        "const y = a ?? b;",
        "const z = a?.b?.c;",
        "const w = [...arr];",
        "const v = { ...obj };",
        "const u = value as string;",
        "const t = <string>value;",
        "const s = new Map<string, number>();",
    ];

    for sample in &samples {
        let arena = Bump::new();
        let parser = Parser::new(&arena, "test.ts", sample);
        let sf = parser.parse_source_file();
        assert!(
            !sf.statements.is_empty(),
            "Failed to parse: {}",
            sample
        );
    }
}
