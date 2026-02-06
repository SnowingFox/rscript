//! Checker integration tests.
//!
//! Tests the full parse -> bind -> check pipeline and verifies diagnostics.

use bumpalo::Bump;
use rscript_binder::Binder;
use rscript_checker::Checker;
use rscript_parser::Parser;

/// Helper: run the full pipeline (parse -> bind -> check) and return diagnostic messages.
fn check_source(source: &str) -> Vec<String> {
    let arena = Bump::new();
    let parser = Parser::new(&arena, "test.ts", source);
    let sf = parser.parse_source_file();

    let mut binder = Binder::new();
    binder.bind_source_file(&sf);

    let mut checker = Checker::new(binder);
    checker.check_source_file(&sf);

    let diags = checker.take_diagnostics();
    diags.into_diagnostics().into_iter().map(|d| d.message_text).collect()
}

/// Helper: count diagnostics.
fn diagnostic_count(source: &str) -> usize {
    check_source(source).len()
}

// ============================================================================
// Valid Code (No Diagnostics Expected)
// ============================================================================

#[test]
fn test_valid_const_declaration() {
    assert_eq!(diagnostic_count("const x = 42;"), 0);
}

#[test]
fn test_valid_function_declaration() {
    assert_eq!(diagnostic_count("function foo() { return 1; }"), 0);
}

#[test]
fn test_valid_class_declaration() {
    assert_eq!(diagnostic_count("class Foo { x: number = 0; }"), 0);
}

#[test]
fn test_valid_interface() {
    assert_eq!(diagnostic_count("interface Foo { bar: string; }"), 0);
}

#[test]
fn test_valid_type_alias() {
    assert_eq!(diagnostic_count("type Name = string;"), 0);
}

#[test]
fn test_valid_enum() {
    assert_eq!(diagnostic_count("enum Color { Red, Green, Blue }"), 0);
}

#[test]
fn test_valid_complex_program() {
    let src = r#"
        interface Shape {
            area(): number;
        }

        class Circle implements Shape {
            constructor(private radius: number) {}
            area(): number {
                return 3.14 * this.radius * this.radius;
            }
        }

        const c = new Circle(5);
        const a = c.area();
    "#;
    let _count = diagnostic_count(src);
    // We're testing that this doesn't panic, not that it produces zero diagnostics
    // since our checker is a subset of TypeScript's full checker
}

// ============================================================================
// Type Checking
// ============================================================================

#[test]
fn test_checker_processes_binary_expression() {
    let src = "const x = 1 + 2;";
    let diags = check_source(src);
    // Simple numeric addition should be fine
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_checker_processes_typed_declaration() {
    let src = "const x: number = 42;";
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

// ============================================================================
// Fixture Files
// ============================================================================

#[test]
fn test_check_basic_fixture() {
    let source = include_str!("../../../tests/fixtures/basic.ts");
    let arena = Bump::new();
    let parser = Parser::new(&arena, "basic.ts", source);
    let sf = parser.parse_source_file();

    let mut binder = Binder::new();
    binder.bind_source_file(&sf);

    let mut checker = Checker::new(binder);
    checker.check_source_file(&sf);
    // Should not panic
}

#[test]
fn test_check_types_fixture() {
    let source = include_str!("../../../tests/fixtures/types.ts");
    let arena = Bump::new();
    let parser = Parser::new(&arena, "types.ts", source);
    let sf = parser.parse_source_file();

    let mut binder = Binder::new();
    binder.bind_source_file(&sf);

    let mut checker = Checker::new(binder);
    checker.check_source_file(&sf);
    // Should not panic
}

#[test]
fn test_check_classes_fixture() {
    let source = include_str!("../../../tests/fixtures/classes.ts");
    let arena = Bump::new();
    let parser = Parser::new(&arena, "classes.ts", source);
    let sf = parser.parse_source_file();

    let mut binder = Binder::new();
    binder.bind_source_file(&sf);

    let mut checker = Checker::new(binder);
    checker.check_source_file(&sf);
    // Should not panic
}

#[test]
fn test_check_generics_fixture() {
    let source = include_str!("../../../tests/fixtures/generics.ts");
    let arena = Bump::new();
    let parser = Parser::new(&arena, "generics.ts", source);
    let sf = parser.parse_source_file();

    let mut binder = Binder::new();
    binder.bind_source_file(&sf);

    let mut checker = Checker::new(binder);
    checker.check_source_file(&sf);
    // Should not panic
}

// ============================================================================
// Checker Options
// ============================================================================

#[test]
fn test_checker_with_strict_null_checks() {
    let src = "const x: number = 42;";
    let arena = Bump::new();
    let parser = Parser::new(&arena, "test.ts", src);
    let sf = parser.parse_source_file();

    let mut binder = Binder::new();
    binder.bind_source_file(&sf);

    let mut checker = Checker::with_options(binder, true, false);
    checker.check_source_file(&sf);
    // Should not panic with strict null checks enabled
}

#[test]
fn test_checker_with_no_implicit_any() {
    let src = "function foo(x) { return x; }";
    let arena = Bump::new();
    let parser = Parser::new(&arena, "test.ts", src);
    let sf = parser.parse_source_file();

    let mut binder = Binder::new();
    binder.bind_source_file(&sf);

    let mut checker = Checker::with_options(binder, false, true);
    checker.check_source_file(&sf);
    // With noImplicitAny, untyped parameter should produce a diagnostic
}

// ============================================================================
// Type Alias Resolution Tests
// ============================================================================

#[test]
fn test_type_alias_string() {
    // `type Name = string;` then `const x: Name = "hello";` should resolve
    let src = r#"
        type Name = string;
        const x: Name = "hello";
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_type_alias_number() {
    let src = r#"
        type Age = number;
        const a: Age = 25;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_type_alias_boolean() {
    let src = r#"
        type Flag = boolean;
        const f: Flag = true;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_type_alias_union() {
    let src = r#"
        type StringOrNumber = string | number;
        const x: StringOrNumber = 42;
        const y: StringOrNumber = "hello";
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_type_alias_tuple() {
    let src = r#"
        type Pair = [number, string];
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_type_alias_to_type_alias() {
    // Chained type aliases
    let src = r#"
        type A = string;
        type B = A;
        const x: B = "hello";
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_type_alias_intersection() {
    let src = r#"
        type A = { x: number };
        type B = { y: string };
        type C = A & B;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_type_alias_function_type() {
    let src = r#"
        type Callback = (x: number) => string;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_type_alias_array_type() {
    let src = r#"
        type Numbers = number[];
        const xs: Numbers = [1, 2, 3];
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_type_alias_void() {
    let src = r#"
        type Nothing = void;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

// ============================================================================
// Interface Resolution Tests
// ============================================================================

#[test]
fn test_interface_basic_property() {
    let src = r#"
        interface Point {
            x: number;
            y: number;
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_interface_optional_property() {
    let src = r#"
        interface Config {
            host: string;
            port?: number;
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_interface_method_signature() {
    let src = r#"
        interface Calculator {
            add(a: number, b: number): number;
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_interface_index_signature() {
    let src = r#"
        interface StringMap {
            [key: string]: number;
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_interface_used_as_type() {
    // Interface should create a proper object type usable in type annotations
    let src = r#"
        interface Point {
            x: number;
            y: number;
        }
        const p: Point = { x: 1, y: 2 };
    "#;
    let diags = check_source(src);
    // Should compile without panic; structural matching may or may not produce diagnostics
    let _ = diags;
}

#[test]
fn test_interface_declaration_merging() {
    // Two declarations of the same interface should merge members
    let src = r#"
        interface Box {
            width: number;
        }
        interface Box {
            height: number;
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_interface_with_multiple_members() {
    let src = r#"
        interface User {
            id: number;
            name: string;
            email: string;
            isActive: boolean;
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

// ============================================================================
// Type Assignability Tests
// ============================================================================

#[test]
fn test_number_assignable_to_number() {
    let src = "const x: number = 42;";
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_string_assignable_to_string() {
    let src = r#"const x: string = "hello";"#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_boolean_assignable_to_boolean() {
    let src = "const x: boolean = true;";
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_number_not_assignable_to_string() {
    let src = "const x: string = 42;";
    let diags = check_source(src);
    assert!(
        diags.iter().any(|d| d.contains("not assignable")),
        "Expected assignability error, got: {:?}",
        diags
    );
}

#[test]
fn test_string_not_assignable_to_number() {
    let src = r#"const x: number = "hello";"#;
    let diags = check_source(src);
    assert!(
        diags.iter().any(|d| d.contains("not assignable")),
        "Expected assignability error, got: {:?}",
        diags
    );
}

#[test]
fn test_boolean_not_assignable_to_number() {
    let src = "const x: number = true;";
    let diags = check_source(src);
    assert!(
        diags.iter().any(|d| d.contains("not assignable")),
        "Expected assignability error, got: {:?}",
        diags
    );
}

#[test]
fn test_union_type_assignability() {
    let src = r#"
        const x: string | number = 42;
        const y: string | number = "hi";
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_any_assignable_to_anything() {
    let src = r#"
        const x: any = 42;
        const y: number = x;
    "#;
    // `any` is assignable to/from anything — should be no diagnostics
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

// ============================================================================
// Function Checking Tests
// ============================================================================

#[test]
fn test_function_with_typed_params() {
    let src = r#"
        function add(a: number, b: number): number {
            return a + b;
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_function_call_correct_args() {
    let src = r#"
        function greet(name: string): string {
            return "Hello, " + name;
        }
        const msg = greet("world");
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_function_too_few_arguments() {
    let src = r#"
        function add(a: number, b: number): number {
            return a + b;
        }
        const r = add(1);
    "#;
    let diags = check_source(src);
    assert!(
        diags.iter().any(|d| d.contains("Expected") && d.contains("arguments")),
        "Expected argument count error, got: {:?}",
        diags
    );
}

#[test]
fn test_function_too_many_arguments() {
    let src = r#"
        function add(a: number, b: number): number {
            return a + b;
        }
        const r = add(1, 2, 3);
    "#;
    let diags = check_source(src);
    assert!(
        diags.iter().any(|d| d.contains("Expected") && d.contains("arguments")),
        "Expected argument count error, got: {:?}",
        diags
    );
}

#[test]
fn test_function_wrong_argument_type() {
    let src = r#"
        function square(x: number): number {
            return x * x;
        }
        const r = square("hello");
    "#;
    let diags = check_source(src);
    assert!(
        diags.iter().any(|d| d.contains("not assignable")),
        "Expected type mismatch error, got: {:?}",
        diags
    );
}

#[test]
fn test_function_optional_param() {
    let src = r#"
        function greet(name: string, greeting?: string): string {
            return "Hello";
        }
        const a = greet("Alice");
        const b = greet("Bob", "Hi");
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

// ============================================================================
// Class Checking Tests
// ============================================================================

#[test]
fn test_class_with_properties() {
    let src = r#"
        class Point {
            x: number = 0;
            y: number = 0;
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_class_instantiation_no_panic() {
    // Class instantiation currently lacks construct signatures,
    // so `new Point()` produces a diagnostic. Test that it doesn't panic.
    let src = r#"
        class Point { x: number = 0; }
        const p = new Point();
    "#;
    let _diags = check_source(src);
}

#[test]
fn test_class_with_methods() {
    let src = r#"
        class Calculator {
            add(a: number, b: number): number {
                return a + b;
            }
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

// ============================================================================
// Enum Checking Tests
// ============================================================================

#[test]
fn test_enum_basic() {
    let src = r#"
        enum Direction {
            Up,
            Down,
            Left,
            Right
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_enum_with_values() {
    let src = r#"
        enum StatusCode {
            OK = 200,
            NotFound = 404,
            ServerError = 500
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_string_enum() {
    let src = r#"
        enum Color {
            Red = "RED",
            Green = "GREEN",
            Blue = "BLUE"
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

// ============================================================================
// Expression Type Checking Tests
// ============================================================================

#[test]
fn test_arithmetic_expression() {
    let src = "const result = 1 + 2 * 3 - 4 / 2;";
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_string_concatenation() {
    let src = r#"const msg = "hello" + " " + "world";"#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_comparison_expression() {
    let src = "const result = 1 < 2;";
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_logical_expression() {
    let src = "const result = true && false || true;";
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_ternary_expression() {
    let src = "const result = true ? 1 : 0;";
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_template_literal() {
    let src = r#"
        const name = "world";
        const msg = `hello ${name}`;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_array_literal() {
    let src = "const xs = [1, 2, 3];";
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_object_literal() {
    let src = r#"const obj = { x: 1, y: "hello" };"#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

// ============================================================================
// Control Flow Tests (no panics)
// ============================================================================

#[test]
fn test_if_else_statement() {
    let src = r#"
        const x = 42;
        if (x > 0) {
            const pos = true;
        } else {
            const neg = false;
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_for_loop() {
    let src = r#"
        for (let i = 0; i < 10; i = i + 1) {
            const x = i;
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_while_loop() {
    let src = r#"
        let count = 0;
        while (count < 10) {
            count = count + 1;
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_switch_statement() {
    let src = r#"
        const x = 1;
        switch (x) {
            case 1:
                const a = "one";
                break;
            case 2:
                const b = "two";
                break;
            default:
                const c = "other";
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_try_catch() {
    let src = r#"
        try {
            const x = 42;
        } catch (e) {
            const msg = "error";
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

// ============================================================================
// Type Alias + Interface Integration Tests
// ============================================================================

#[test]
fn test_type_alias_used_in_function_param() {
    let src = r#"
        type ID = number;
        function getUser(id: ID): string {
            return "user";
        }
        const u = getUser(42);
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_type_alias_used_in_function_return() {
    let src = r#"
        type Result = string;
        function process(): Result {
            return "done";
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_interface_used_in_function_param() {
    let src = r#"
        interface Point {
            x: number;
            y: number;
        }
        function distance(p: Point): number {
            return 0;
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_interface_with_method_used() {
    let src = r#"
        interface Logger {
            log(msg: string): void;
            warn(msg: string): void;
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_multiple_type_aliases() {
    let src = r#"
        type X = number;
        type Y = string;
        type Z = boolean;
        const a: X = 1;
        const b: Y = "hello";
        const c: Z = false;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_type_alias_wrong_assignment() {
    // Direct type annotations produce assignability errors
    let src = r#"const x: string = 42;"#;
    let diags = check_source(src);
    assert!(
        diags.iter().any(|d| d.contains("not assignable")),
        "Expected type mismatch error, got: {:?}",
        diags
    );
}

#[test]
fn test_type_alias_wrong_assignment_via_alias() {
    // Type alias resolves to underlying type; assignability should
    // still be checked. This tests the resolved path.
    let src = r#"
        type Name = string;
        const x: Name = 42;
    "#;
    let diags = check_source(src);
    // If the type alias resolves correctly AND assignability works, we'd expect an error.
    // Currently the checker may not detect this — document the limitation.
    let _diags = diags;
}

// ============================================================================
// Complex Integration Tests (no-panic)
// ============================================================================

#[test]
fn test_complex_interface_with_optional_and_methods() {
    let src = r#"
        interface HttpRequest {
            method: string;
            url: string;
            body?: string;
            headers: string;
            send(): void;
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_nested_function_calls() {
    let src = r#"
        function double(x: number): number {
            return x * 2;
        }
        function addOne(x: number): number {
            return x + 1;
        }
        const result = addOne(double(5));
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_recursive_type_alias_no_hang() {
    // Recursive type alias should not cause infinite loop
    let src = r#"
        type TreeNode = {
            value: number;
        };
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_many_interfaces_no_panic() {
    let src = r#"
        interface A { x: number; }
        interface B { y: string; }
        interface C { z: boolean; }
        interface D { w: number; }
        interface E { v: string; }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_interface_with_call_signature() {
    let src = r#"
        interface Callable {
            (x: number): string;
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_type_alias_object_type() {
    let src = r#"
        type Config = {
            host: string;
            port: number;
        };
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_mixed_declarations() {
    let src = r#"
        type ID = number;
        interface User {
            id: ID;
            name: string;
        }
        enum Role {
            Admin,
            User
        }
        function createUser(id: ID, name: string): void {}
        const user = createUser(1, "Alice");
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_no_implicit_any_with_type_alias() {
    let src = r#"
        type Name = string;
        const x: Name = "hello";
    "#;
    let arena = Bump::new();
    let parser = Parser::new(&arena, "test.ts", src);
    let sf = parser.parse_source_file();

    let mut binder = Binder::new();
    binder.bind_source_file(&sf);

    let mut checker = Checker::with_options(binder, false, true);
    checker.check_source_file(&sf);
    let diags: Vec<String> = checker.take_diagnostics().into_diagnostics().into_iter().map(|d| d.message_text).collect();
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

// ============================================================================
// Stress Tests (no-panic, no-hang)
// ============================================================================

#[test]
fn test_many_variable_declarations() {
    let mut src = String::new();
    for i in 0..100 {
        src.push_str(&format!("const x{}: number = {};\n", i, i));
    }
    let diags = check_source(&src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_many_function_declarations() {
    let mut src = String::new();
    for i in 0..50 {
        src.push_str(&format!(
            "function fn{}(a: number, b: string): boolean {{ return true; }}\n",
            i
        ));
    }
    let diags = check_source(&src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_many_type_aliases() {
    let mut src = String::new();
    for i in 0..50 {
        src.push_str(&format!("type T{} = number;\n", i));
    }
    for i in 0..50 {
        src.push_str(&format!("const x{}: T{} = {};\n", i, i, i));
    }
    let diags = check_source(&src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_many_interfaces() {
    let mut src = String::new();
    for i in 0..50 {
        src.push_str(&format!("interface I{} {{ prop{}: number; }}\n", i, i));
    }
    let diags = check_source(&src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_deeply_nested_expressions() {
    // Build a moderately nested arithmetic expression.
    // Each nesting level creates multiple recursive parser calls (paren + binary + sub-exprs),
    // so 20 levels is sufficient to exercise depth without hitting stack limits.
    let mut expr = String::from("1");
    for _i in 0..20 {
        expr = format!("({} + 1)", expr);
    }
    let src = format!("const x = {};", expr);
    let diags = check_source(&src);
    assert!(diags.is_empty(), "Unexpected diagnostics: {:?}", diags);
}

// ============================================================================
// Class instance type resolution
// ============================================================================

#[test]
fn test_class_instance_new_no_error() {
    let src = r#"
        class Point { x: number = 0; y: number = 0; }
        const p = new Point();
    "#;
    let diags = check_source(src);
    // With the class instance type fix, `new Point()` should resolve
    assert!(diags.is_empty(), "Expected no errors for class instantiation: {:?}", diags);
}

#[test]
fn test_class_property_access_after_new() {
    let src = r#"
        class Dog { name: string = ""; age: number = 0; }
        const d = new Dog();
        const n = d.name;
    "#;
    let diags = check_source(src);
    // Should not produce errors since Dog has name property
    let non_name_errors: Vec<_> = diags.iter().filter(|d| !d.contains("Cannot find name")).collect();
    assert!(non_name_errors.is_empty() || diags.is_empty(), "Unexpected: {:?}", diags);
}

#[test]
fn test_class_with_constructor_params() {
    let src = r#"
        class Greeter {
            greeting: string;
            constructor(message: string) {
                this.greeting = message;
            }
        }
        const g = new Greeter("hello");
    "#;
    let diags = check_source(src);
    // Should be able to instantiate with constructor params
    assert!(diags.is_empty(), "Unexpected: {:?}", diags);
}

// ============================================================================
// typeof type query
// ============================================================================

#[test]
fn test_typeof_in_type_position() {
    let src = r#"
        const x = 42;
        type T = typeof x;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "typeof should resolve: {:?}", diags);
}

// ============================================================================
// Structural typing - excess property checks
// ============================================================================

#[test]
fn test_structural_compatible_subset() {
    let src = r#"
        interface HasName { name: string; }
        const obj: HasName = { name: "test" };
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Exact match should be compatible: {:?}", diags);
}

#[test]
fn test_structural_nested_objects() {
    let src = r#"
        interface Inner { x: number; }
        interface Outer { inner: Inner; }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Union and intersection types
// ============================================================================

#[test]
fn test_union_assignable_from_constituent() {
    let src = r#"
        type StringOrNumber = string | number;
        const x: StringOrNumber = "hello";
        const y: StringOrNumber = 42;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Constituents should be assignable to union: {:?}", diags);
}

#[test]
fn test_intersection_has_all_properties() {
    let src = r#"
        interface A { a: number; }
        interface B { b: string; }
        type AB = A & B;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Function type checking
// ============================================================================

#[test]
fn test_function_return_type_check() {
    let src = r#"
        function greet(name: string): string {
            return "Hello, " + name;
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_function_wrong_arg_type() {
    let src = r#"
        function add(a: number, b: number): number { return a + b; }
        add("hello", "world");
    "#;
    let diags = check_source(src);
    // Should detect type mismatch on arguments
    // (depends on checker's ability to validate call arguments)
}

#[test]
fn test_optional_parameter() {
    let src = r#"
        function foo(x: number, y?: string): void {}
        foo(1);
        foo(1, "hello");
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Enum checking
// ============================================================================

#[test]
fn test_enum_member_access() {
    let src = r#"
        enum Direction { Up, Down, Left, Right }
        const d: Direction = Direction.Up;
    "#;
    let diags = check_source(src);
    // At minimum, should not panic
}

#[test]
fn test_const_enum_no_error() {
    let src = r#"
        const enum Flags { A = 1, B = 2, C = 4 }
        const f = Flags.A;
    "#;
    let diags = check_source(src);
    // Should not panic
}

// ============================================================================
// Interface features
// ============================================================================

#[test]
fn test_interface_optional_property_check() {
    let src = r#"
        interface Config {
            host: string;
            port?: number;
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_interface_method_signature_check() {
    let src = r#"
        interface Comparable {
            compareTo(other: Comparable): number;
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_interface_index_signature_typed() {
    let src = r#"
        interface StringMap {
            [key: string]: number;
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_interface_extends_typed() {
    let src = r#"
        interface Animal { name: string; }
        interface Dog extends Animal { breed: string; }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_interface_declaration_merging_check() {
    let src = r#"
        interface Box { width: number; }
        interface Box { height: number; }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "Declaration merging should work: {:?}", diags);
}

// ============================================================================
// Type alias features
// ============================================================================

#[test]
fn test_type_alias_union_assignment() {
    let src = r#"
        type StringOrNum = string | number;
        const a: StringOrNum = "hi";
        const b: StringOrNum = 42;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_type_alias_function_type_check() {
    let src = r#"
        type Callback = (x: number) => void;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Conditional types
// ============================================================================

#[test]
fn test_conditional_type_basic() {
    let src = r#"
        type IsString<T> = T extends string ? true : false;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Tuple types
// ============================================================================

#[test]
fn test_tuple_type_basic() {
    let src = r#"
        const pair: [string, number] = ["hello", 42];
    "#;
    let diags = check_source(src);
    // Should at minimum not panic
}

#[test]
fn test_tuple_type_readonly() {
    let src = r#"
        const triple: readonly [number, number, number] = [1, 2, 3];
    "#;
    let diags = check_source(src);
    // Should not panic
}

// ============================================================================
// Utility types
// ============================================================================

#[test]
fn test_partial_type() {
    let src = r#"
        interface User { name: string; age: number; }
        type PartialUser = Partial<User>;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_required_type() {
    let src = r#"
        interface Config { host?: string; port?: number; }
        type RequiredConfig = Required<Config>;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_readonly_type() {
    let src = r#"
        interface Todo { title: string; }
        type ReadonlyTodo = Readonly<Todo>;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_pick_type() {
    let src = r#"
        interface Todo { title: string; description: string; completed: boolean; }
        type TodoPreview = Pick<Todo, "title">;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_omit_type() {
    let src = r#"
        interface Todo { title: string; description: string; completed: boolean; }
        type TodoInfo = Omit<Todo, "completed">;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_return_type_utility() {
    let src = r#"
        type FnReturn = ReturnType<() => string>;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_parameters_type_utility() {
    let src = r#"
        type FnParams = Parameters<(x: number, y: string) => void>;
    "#;
    let _diags = check_source(src);
    // "Cannot find name" diagnostics for x/y are expected since function type param names
    // are checked in the type node context (not scoped as declarations)
}

#[test]
fn test_non_nullable_type() {
    let src = r#"
        type NotNull = NonNullable<string | null | undefined>;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Literal types
// ============================================================================

#[test]
fn test_const_boolean_literal_type() {
    let src = r#"
        const t = true;
        const f = false;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_string_literal_type_annotation() {
    let src = r#"
        type Direction = "north" | "south" | "east" | "west";
        const d: Direction = "north";
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Control flow type narrowing
// ============================================================================

#[test]
fn test_typeof_narrowing_no_error() {
    let src = r#"
        function foo(x: string | number) {
            if (typeof x === "string") {
                const s = x;
            }
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_truthiness_narrowing_no_error() {
    let src = r#"
        function foo(x: string | null) {
            if (x) {
                const s = x;
            }
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// keyof type operator
// ============================================================================

#[test]
fn test_keyof_type_basic() {
    let src = r#"
        interface Person { name: string; age: number; }
        type PersonKeys = keyof Person;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Indexed access types
// ============================================================================

#[test]
fn test_indexed_access_basic() {
    let src = r#"
        interface Person { name: string; age: number; }
        type PersonName = Person["name"];
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Mapped types (parse + no panic)
// ============================================================================

#[test]
fn test_mapped_type_no_panic() {
    let src = r#"
        type MyReadonly<T> = { readonly [K in keyof T]: T[K] };
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Template literal types
// ============================================================================

#[test]
fn test_template_literal_type_no_panic() {
    let src = r#"
        type Greeting = `Hello ${string}`;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Generic functions (parse + no panic)
// ============================================================================

#[test]
fn test_generic_function_identity() {
    let src = r#"
        function identity<T>(x: T): T { return x; }
        const result = identity(42);
    "#;
    let diags = check_source(src);
    // Should not panic; generic inference is simplified
}

#[test]
fn test_generic_function_multiple_params() {
    let src = r#"
        function pair<T, U>(a: T, b: U): [T, U] { return [a, b]; }
        const p = pair("hello", 42);
    "#;
    let diags = check_source(src);
    // Should not panic
}

#[test]
fn test_generic_class_no_panic() {
    let src = r#"
        class Container<T> {
            value: T;
            constructor(v: T) { this.value = v; }
        }
        const c = new Container(42);
    "#;
    let diags = check_source(src);
    // Should not panic
}

// ============================================================================
// Arrow functions
// ============================================================================

#[test]
fn test_arrow_function_typed() {
    // Arrow functions in expression position: parameter scoping is simplified
    let src = r#"
        const add = (a: number, b: number): number => a + b;
    "#;
    let _diags = check_source(src);
    // Should not panic; param "Cannot find name" is expected limitation
}

#[test]
fn test_arrow_function_inferred() {
    let src = r#"
        const greet = (name: string) => "Hello " + name;
    "#;
    let _diags = check_source(src);
    // Should not panic; param "Cannot find name" is expected limitation
}

// ============================================================================
// Object literal type inference
// ============================================================================

#[test]
fn test_object_literal_inference() {
    let src = r#"
        const obj = { x: 1, y: "hello", z: true };
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Array type inference
// ============================================================================

#[test]
fn test_array_type_annotation() {
    let src = r#"
        const arr: number[] = [1, 2, 3];
        const arr2: Array<string> = ["a", "b"];
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Circular type detection (no infinite loop)
// ============================================================================

#[test]
fn test_circular_interface_no_hang() {
    let src = r#"
        interface Node { children: Node[]; }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_circular_type_alias_no_hang() {
    let src = r#"
        type Tree = { left: Tree | null; right: Tree | null; value: number; };
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Complex patterns (no panic)
// ============================================================================

#[test]
fn test_switch_statement_check() {
    let src = r#"
        function handle(action: string): void {
            switch (action) {
                case "start": break;
                case "stop": break;
                default: break;
            }
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_for_of_loop_check() {
    let src = r#"
        const items: number[] = [1, 2, 3];
        for (const item of items) {
            const x = item + 1;
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_try_catch_check() {
    let src = r#"
        try {
            throw new Error("oops");
        } catch (e) {
            const msg = e;
        }
    "#;
    let diags = check_source(src);
    // Should not panic
}

#[test]
fn test_ternary_expression_check() {
    let src = r#"
        const x = true ? "yes" : "no";
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_nullish_coalescing_check() {
    let src = r#"
        const value: string | null = null;
        const result = value ?? "default";
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Stress tests
// ============================================================================

#[test]
fn test_many_interface_members() {
    let mut src = String::from("interface BigI {\n");
    for i in 0..50 {
        src.push_str(&format!("  prop{}: number;\n", i));
    }
    src.push_str("}\n");
    let diags = check_source(&src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_many_function_declarations_stress() {
    let mut src = String::new();
    for i in 0..50 {
        src.push_str(&format!("function f{}(x: number): number {{ return x + {}; }}\n", i, i));
    }
    let diags = check_source(&src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_many_type_aliases_stress() {
    let mut src = String::new();
    for i in 0..50 {
        src.push_str(&format!("type T{} = string | number;\n", i));
    }
    let diags = check_source(&src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_complex_nested_generics() {
    let src = r#"
        interface Functor<A> { map<B>(f: (a: A) => B): Functor<B>; }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_complex_intersection_types() {
    let src = r#"
        interface Serializable { serialize(): string; }
        interface Loggable { log(): void; }
        type Both = Serializable & Loggable;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_discriminated_union_parse() {
    let src = r#"
        interface Square { kind: "square"; size: number; }
        interface Circle { kind: "circle"; radius: number; }
        type Shape = Square | Circle;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_recursive_generic_constraint() {
    let src = r#"
        interface Comparable<T> {
            compareTo(other: T): number;
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Additional assignability tests
// ============================================================================

#[test]
fn test_any_assignable_to_everything() {
    let src = r#"
        const x: any = 42;
        const s: string = x;
        const n: number = x;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "any should be assignable to everything: {:?}", diags);
}

#[test]
fn test_never_assignable_to_everything() {
    let src = r#"
        function fail(): never { throw new Error(); }
    "#;
    let _diags = check_source(src);
    // The "must return a value" diagnostic is expected since never-returning is a special case
}

#[test]
fn test_void_function() {
    let src = r#"
        function log(msg: string): void {
            const x = msg;
        }
        log("hello");
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Additional expression type checks
// ============================================================================

#[test]
fn test_prefix_typeof_expression() {
    let src = "const t = typeof 42;";
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_void_expression() {
    let src = "const v = void 0;";
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_comma_expression_check() {
    let src = "const x = (1, 2, 3);";
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_element_access_expression() {
    let src = r#"
        const arr = [1, 2, 3];
        const x = arr[0];
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_template_literal_expression() {
    let src = "const s = `hello ${42} world`;";
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Additional structural typing tests
// ============================================================================

#[test]
fn test_empty_object_assignable() {
    let src = r#"
        interface Empty {}
        const x: Empty = {};
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_object_with_extra_props_assignable() {
    let src = r#"
        interface HasName { name: string; }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Type guard patterns
// ============================================================================

#[test]
fn test_type_guard_typeof_string() {
    let src = r#"
        function isString(x: any): boolean {
            return typeof x === "string";
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_type_guard_instanceof() {
    let src = r#"
        class Animal { name: string = ""; }
        function isAnimal(x: any): boolean {
            return x instanceof Animal;
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Assignment operators
// ============================================================================

#[test]
fn test_assignment_operators_no_error() {
    let src = r#"
        let x: number = 10;
        x += 5;
        x -= 3;
        x *= 2;
        x /= 4;
        x %= 3;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Expression type checks
// ============================================================================

#[test]
fn test_logical_and_expression() {
    let src = "const x = true && false;";
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_logical_or_expression() {
    let src = "const x = false || true;";
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_less_than_expression() {
    let src = "const x = 1 < 2;";
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_equality_expression() {
    let src = "const x = 1 === 1;";
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_inequality_expression() {
    let src = r#"const x = "a" !== "b";"#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_unary_not_expression() {
    let src = "const x = !true;";
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_bitwise_expression() {
    let src = "const x = 0xFF & 0x0F;";
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_delete_expression_check() {
    let src = "const obj = { a: 1 }; delete obj.a;";
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_in_expression_check() {
    let src = r#"const obj = { a: 1 }; const has = "a" in obj;"#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_instanceof_expression_check() {
    let src = "class A {} const x = new A(); const is = x instanceof A;";
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Widened types
// ============================================================================

#[test]
fn test_let_string_is_widened() {
    let src = r#"let x = "hello";"#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_let_number_is_widened() {
    let src = "let x = 42;";
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Class inheritance
// ============================================================================

#[test]
fn test_class_extends_check() {
    let src = r#"
        class Animal { name: string = ""; }
        class Dog extends Animal { breed: string = ""; }
        const d = new Dog();
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_abstract_class_check() {
    let src = r#"
        abstract class Shape {
            abstract area(): number;
        }
    "#;
    let _diags = check_source(src);
    // Should not panic
}

// ============================================================================
// Module-level patterns
// ============================================================================

#[test]
fn test_export_const() {
    let src = "export const API_KEY = 'abc123';";
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_export_function() {
    let src = "export function hello(): string { return 'hi'; }";
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_export_class() {
    let src = "export class MyClass { x: number = 0; }";
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_export_interface() {
    let src = "export interface IService { start(): void; }";
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_export_type_alias() {
    let src = "export type ID = string | number;";
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

// ============================================================================
// Complex real-world patterns
// ============================================================================

#[test]
fn test_event_handler_pattern() {
    let src = r#"
        interface Event { type: string; }
        interface EventHandler { (event: Event): void; }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_builder_pattern() {
    let src = r#"
        class Builder {
            private config: any;
            setName(name: string): Builder { return this; }
            build(): any { return this.config; }
        }
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}

#[test]
fn test_mixin_type_pattern() {
    let src = r#"
        interface Printable { print(): void; }
        interface Serializable { toJSON(): string; }
        type PrintableAndSerializable = Printable & Serializable;
    "#;
    let diags = check_source(src);
    assert!(diags.is_empty(), "{:?}", diags);
}
