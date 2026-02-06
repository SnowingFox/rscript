//! Compiler integration tests.
//!
//! End-to-end tests for the compilation pipeline: parse -> bind -> check.
//! Note: emit tests are limited because the printer requires a shared
//! StringInterner with the parser, which is an architectural improvement TODO.

use bumpalo::Bump;
use rscript_compiler::Program;
use rscript_tsoptions::CompilerOptions;

/// Helper: create a program, add a source, compile, and return diagnostic count.
fn compile_source(source: &str) -> usize {
    let arena = Bump::new();
    let options = CompilerOptions::default();
    let mut program = Program::new(&arena, vec![], options);
    program.add_source("test.ts".to_string(), source.to_string());

    let diags = program.compile();
    diags.len()
}

// ============================================================================
// Basic Compilation (parse -> bind -> check)
// ============================================================================

#[test]
fn test_compile_empty_file() {
    assert_eq!(compile_source(""), 0);
}

#[test]
fn test_compile_simple_variable() {
    let count = compile_source("const x = 42;");
    assert_eq!(count, 0);
}

#[test]
fn test_compile_function() {
    let count = compile_source(
        "function add(a: number, b: number): number { return a + b; }"
    );
    assert_eq!(count, 0);
}

#[test]
fn test_compile_class() {
    let src = r#"
        class Greeter {
            greeting: string;
            constructor(message: string) {
                this.greeting = message;
            }
            greet() {
                return "Hello, " + this.greeting;
            }
        }
    "#;
    let _count = compile_source(src);
    // Just ensure it doesn't panic; exact diagnostic count depends on checker depth
}

// ============================================================================
// Multiple Files
// ============================================================================

#[test]
fn test_compile_multiple_files() {
    let arena = Bump::new();
    let options = CompilerOptions::default();
    let mut program = Program::new(&arena, vec![], options);

    program.add_source("a.ts".to_string(), "export const x = 1;".to_string());
    program.add_source("b.ts".to_string(), "export const y = 2;".to_string());

    let diags = program.compile();
    assert_eq!(diags.len(), 0);
}

// ============================================================================
// Fixture File Compilation
// ============================================================================

#[test]
fn test_compile_basic_fixture() {
    let source = include_str!("../../../tests/fixtures/basic.ts");
    let _count = compile_source(source);
    // Should compile without panicking
}

#[test]
fn test_compile_types_fixture() {
    let source = include_str!("../../../tests/fixtures/types.ts");
    let _count = compile_source(source);
}

#[test]
fn test_compile_classes_fixture() {
    let source = include_str!("../../../tests/fixtures/classes.ts");
    let _count = compile_source(source);
}

#[test]
fn test_compile_generics_fixture() {
    let source = include_str!("../../../tests/fixtures/generics.ts");
    let _count = compile_source(source);
}

#[test]
fn test_compile_modules_fixture() {
    let source = include_str!("../../../tests/fixtures/modules.ts");
    let _count = compile_source(source);
}

#[test]
fn test_compile_enums_fixture() {
    let source = include_str!("../../../tests/fixtures/enums.ts");
    let _count = compile_source(source);
}

// ============================================================================
// Stress Tests
// ============================================================================

#[test]
fn test_compile_large_file() {
    // Generate a large source file with many declarations
    let mut source = String::new();
    for i in 0..100 {
        source.push_str(&format!("const var_{}: number = {};\n", i, i));
    }
    for i in 0..50 {
        source.push_str(&format!(
            "function func_{}(x: number): number {{ return x + {}; }}\n",
            i, i
        ));
    }
    for i in 0..20 {
        source.push_str(&format!(
            "interface Iface_{} {{ prop_{}: string; }}\n",
            i, i
        ));
    }

    let _count = compile_source(&source);
    // Just ensure no panic
}

#[test]
fn test_compile_deeply_nested() {
    // Deeply nested function calls
    let mut source = String::from("const x = ");
    for _ in 0..20 {
        source.push_str("f(");
    }
    source.push('1');
    for _ in 0..20 {
        source.push(')');
    }
    source.push(';');

    // Should not stack overflow or panic
    let _count = compile_source(&source);
}

#[test]
fn test_compile_many_variables() {
    let mut source = String::new();
    for i in 0..500 {
        source.push_str(&format!("let v{} = {};\n", i, i));
    }
    let _count = compile_source(&source);
}
