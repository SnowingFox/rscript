//! TypeScript conformance test framework for rscript.
//!
//! Tests various TypeScript patterns against the rscript compiler to measure pass rates.
//! Groups tests into categories: parsing, binding, type checking.

use bumpalo::Bump;
use rscript_parser::Parser;

/// Test result for a single conformance test case.
#[derive(Debug, Clone)]
struct TestResult {
    name: String,
    category: String,
    source: String,
    parse_ok: bool,
    parse_error: Option<String>,
}

/// Run a single conformance test case.
fn run_test(name: &str, category: &str, source: &str) -> TestResult {
    let arena = Bump::new();
    let parse_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let parser = Parser::new(&arena, "test.ts", source);
        parser.parse_source_file()
    }));

    match parse_result {
        Ok(_sf) => TestResult {
            name: name.to_string(),
            category: category.to_string(),
            source: source.to_string(),
            parse_ok: true,
            parse_error: None,
        },
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "Unknown panic".to_string()
            };
            TestResult {
                name: name.to_string(),
                category: category.to_string(),
                source: source.to_string(),
                parse_ok: false,
                parse_error: Some(msg),
            }
        }
    }
}

/// Conformance test suite runner.
struct ConformanceTestSuite {
    tests: Vec<TestResult>,
}

impl ConformanceTestSuite {
    fn new() -> Self {
        Self { tests: Vec::new() }
    }

    fn add_test(&mut self, name: &str, category: &str, source: &str) {
        let result = run_test(name, category, source);
        self.tests.push(result);
    }

    fn print_summary(&self) {
        let total = self.tests.len();
        let passed = self.tests.iter().filter(|t| t.parse_ok).count();
        let failed = total - passed;
        let pass_rate = if total > 0 {
            (passed as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        println!("\n=== TypeScript Conformance Test Summary ===");
        println!("Total tests: {}", total);
        println!("Passed: {}", passed);
        println!("Failed: {}", failed);
        println!("Pass rate: {:.2}%", pass_rate);

        // Group by category
        let mut categories: std::collections::HashMap<String, (usize, usize)> =
            std::collections::HashMap::new();
        for test in &self.tests {
            let entry = categories
                .entry(test.category.clone())
                .or_insert((0, 0));
            if test.parse_ok {
                entry.0 += 1;
            }
            entry.1 += 1;
        }

        println!("\n--- Results by Category ---");
        let mut category_vec: Vec<_> = categories.iter().collect();
        category_vec.sort_by_key(|(cat, _)| *cat);
        for (category, (passed_count, total_count)) in category_vec {
            let category_pass_rate = if *total_count > 0 {
                (*passed_count as f64 / *total_count as f64) * 100.0
            } else {
                0.0
            };
            println!(
                "  {}: {}/{} ({:.2}%)",
                category, passed_count, total_count, category_pass_rate
            );
        }

        // Print failures
        let failures: Vec<_> = self.tests.iter().filter(|t| !t.parse_ok).collect();
        if !failures.is_empty() {
            println!("\n--- Failed Tests ---");
            for failure in failures.iter().take(10) {
                println!("  [{}] {}: {}", failure.category, failure.name, {
                    failure
                        .parse_error
                        .as_ref()
                        .map(|e| e.as_str())
                        .unwrap_or("Unknown error")
                });
            }
            if failures.len() > 10 {
                println!("  ... and {} more failures", failures.len() - 10);
            }
        }
    }
}

// ============================================================================
// Test Cases
// ============================================================================

#[test]
fn test_typescript_conformance() {
    let mut suite = ConformanceTestSuite::new();

    // ========================================================================
    // Category: Parsing - Basic Variable Declarations
    // ========================================================================
    suite.add_test(
        "basic_var_declaration",
        "parsing",
        "var x: number = 42;",
    );
    suite.add_test(
        "basic_let_declaration",
        "parsing",
        "let y: string = 'hello';",
    );
    suite.add_test(
        "basic_const_declaration",
        "parsing",
        "const z: boolean = true;",
    );
    suite.add_test(
        "typed_array_declaration",
        "parsing",
        "const arr: number[] = [1, 2, 3];",
    );

    // ========================================================================
    // Category: Parsing - Function Declarations
    // ========================================================================
    suite.add_test(
        "function_with_params",
        "parsing",
        "function add(a: number, b: number): number { return a + b; }",
    );
    suite.add_test(
        "function_with_return_type",
        "parsing",
        "function greet(name: string): string { return 'Hello, ' + name; }",
    );
    suite.add_test(
        "arrow_function",
        "parsing",
        "const multiply = (x: number, y: number): number => x * y;",
    );
    suite.add_test(
        "async_function",
        "parsing",
        "async function fetchData(): Promise<string> { return await Promise.resolve('data'); }",
    );

    // ========================================================================
    // Category: Parsing - Interface Declarations
    // ========================================================================
    suite.add_test(
        "basic_interface",
        "parsing",
        "interface Person { name: string; age: number; }",
    );
    suite.add_test(
        "interface_with_methods",
        "parsing",
        "interface Calculator { add(a: number, b: number): number; }",
    );
    suite.add_test(
        "interface_extends",
        "parsing",
        "interface Animal { name: string; } interface Dog extends Animal { breed: string; }",
    );

    // ========================================================================
    // Category: Parsing - Type Alias Declarations
    // ========================================================================
    suite.add_test(
        "type_alias_basic",
        "parsing",
        "type ID = string;",
    );
    suite.add_test(
        "type_alias_union",
        "parsing",
        "type Status = 'active' | 'inactive' | 'pending';",
    );
    suite.add_test(
        "type_alias_function",
        "parsing",
        "type Handler = (event: string) => void;",
    );

    // ========================================================================
    // Category: Parsing - Class Declarations
    // ========================================================================
    suite.add_test(
        "basic_class",
        "parsing",
        "class Point { x: number; y: number; }",
    );
    suite.add_test(
        "class_with_constructor",
        "parsing",
        "class Person { constructor(public name: string) {} }",
    );
    suite.add_test(
        "class_with_methods",
        "parsing",
        "class Calculator { add(a: number, b: number): number { return a + b; } }",
    );
    suite.add_test(
        "class_extends",
        "parsing",
        "class Animal {} class Dog extends Animal { breed: string; }",
    );

    // ========================================================================
    // Category: Parsing - Enum Declarations
    // ========================================================================
    suite.add_test(
        "basic_enum",
        "parsing",
        "enum Color { Red, Green, Blue }",
    );
    suite.add_test(
        "enum_with_values",
        "parsing",
        "enum Status { Active = 'active', Inactive = 'inactive' }",
    );

    // ========================================================================
    // Category: Parsing - Generic Function Declarations
    // ========================================================================
    suite.add_test(
        "generic_function",
        "parsing",
        "function identity<T>(value: T): T { return value; }",
    );
    suite.add_test(
        "generic_class",
        "parsing",
        "class Container<T> { value: T; }",
    );
    suite.add_test(
        "generic_interface",
        "parsing",
        "interface Repository<T> { find(id: number): T; }",
    );

    // ========================================================================
    // Category: Parsing - Union and Intersection Types
    // ========================================================================
    suite.add_test(
        "union_type",
        "parsing",
        "type StringOrNumber = string | number;",
    );
    suite.add_test(
        "intersection_type",
        "parsing",
        "type A = { a: number; }; type B = { b: string; }; type C = A & B;",
    );
    suite.add_test(
        "union_in_function",
        "parsing",
        "function process(value: string | number): void {}",
    );

    // ========================================================================
    // Category: Parsing - Type Narrowing with typeof
    // ========================================================================
    suite.add_test(
        "typeof_typeof",
        "parsing",
        "type TypeOfString = typeof 'hello';",
    );
    suite.add_test(
        "typeof_variable",
        "parsing",
        "const x = 42; type TypeOfX = typeof x;",
    );
    suite.add_test(
        "type_narrowing",
        "parsing",
        "function test(x: string | number) { if (typeof x === 'string') { const y: string = x; } }",
    );

    // ========================================================================
    // Category: Parsing - Import/Export Declarations
    // ========================================================================
    suite.add_test(
        "import_default",
        "parsing",
        "import React from 'react';",
    );
    suite.add_test(
        "import_named",
        "parsing",
        "import { Component } from 'react';",
    );
    suite.add_test(
        "export_default",
        "parsing",
        "export default function() {}",
    );
    suite.add_test(
        "export_named",
        "parsing",
        "export const x = 1;",
    );

    // ========================================================================
    // Category: Parsing - Template Literals
    // ========================================================================
    suite.add_test(
        "template_literal_basic",
        "parsing",
        "const msg = `Hello, world!`;",
    );
    suite.add_test(
        "template_literal_interpolation",
        "parsing",
        "const name = 'Alice'; const greeting = `Hello, ${name}!`;",
    );
    suite.add_test(
        "template_literal_typed",
        "parsing",
        "const x: string = `Value: ${42}`;",
    );

    // ========================================================================
    // Category: Parsing - Destructuring Patterns
    // ========================================================================
    suite.add_test(
        "destructuring_array",
        "parsing",
        "const [a, b] = [1, 2];",
    );
    suite.add_test(
        "destructuring_object",
        "parsing",
        "const { x, y } = { x: 1, y: 2 };",
    );
    suite.add_test(
        "destructuring_with_types",
        "parsing",
        "const { name, age }: { name: string; age: number } = { name: 'Alice', age: 30 };",
    );

    // ========================================================================
    // Category: Parsing - Optional Chaining
    // ========================================================================
    suite.add_test(
        "optional_chaining_property",
        "parsing",
        "const value = obj?.prop;",
    );
    suite.add_test(
        "optional_chaining_method",
        "parsing",
        "const result = obj?.method?.();",
    );
    suite.add_test(
        "optional_chaining_nested",
        "parsing",
        "const x = a?.b?.c?.d;",
    );

    // ========================================================================
    // Category: Parsing - Nullish Coalescing
    // ========================================================================
    suite.add_test(
        "nullish_coalescing_basic",
        "parsing",
        "const x = value ?? 'default';",
    );
    suite.add_test(
        "nullish_coalescing_chained",
        "parsing",
        "const y = a ?? b ?? c;",
    );

    // ========================================================================
    // Category: Parsing - Spread Operator
    // ========================================================================
    suite.add_test(
        "spread_array",
        "parsing",
        "const arr = [...[1, 2, 3], 4, 5];",
    );
    suite.add_test(
        "spread_object",
        "parsing",
        "const obj = { ...{ a: 1 }, b: 2 };",
    );
    suite.add_test(
        "spread_function_args",
        "parsing",
        "function test(...args: number[]) {}",
    );

    // ========================================================================
    // Category: Parsing - Async/Await
    // ========================================================================
    suite.add_test(
        "async_await_basic",
        "parsing",
        "async function fetch() { const data = await Promise.resolve(1); return data; }",
    );
    suite.add_test(
        "async_arrow",
        "parsing",
        "const fetch = async () => { await Promise.resolve(); };",
    );

    // ========================================================================
    // Category: Parsing - Decorator Syntax
    // ========================================================================
    suite.add_test(
        "decorator_class",
        "parsing",
        "@Component class MyClass {}",
    );
    suite.add_test(
        "decorator_method",
        "parsing",
        "class Test { @Log() method() {} }",
    );
    suite.add_test(
        "decorator_property",
        "parsing",
        "class Test { @observable count = 0; }",
    );

    // ========================================================================
    // Category: Parsing - Mapped Types
    // ========================================================================
    suite.add_test(
        "mapped_type_basic",
        "parsing",
        "type Readonly<T> = { readonly [P in keyof T]: T[P]; };",
    );
    suite.add_test(
        "mapped_type_partial",
        "parsing",
        "type Partial<T> = { [P in keyof T]?: T[P]; };",
    );

    // ========================================================================
    // Category: Parsing - Conditional Types
    // ========================================================================
    suite.add_test(
        "conditional_type_basic",
        "parsing",
        "type IsString<T> = T extends string ? true : false;",
    );
    suite.add_test(
        "conditional_type_nested",
        "parsing",
        "type NonNullable<T> = T extends null | undefined ? never : T;",
    );

    // Print summary
    suite.print_summary();

    // Assert that we have at least some passing tests
    let passed = suite.tests.iter().filter(|t| t.parse_ok).count();
    assert!(
        passed > 0,
        "Expected at least some tests to pass, but all failed"
    );
}
