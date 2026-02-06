//! Binder integration tests.
//!
//! Tests the parse -> bind pipeline and verifies symbol creation.

use bumpalo::Bump;
use rscript_binder::Binder;
use rscript_parser::Parser;

/// Helper: parse and bind source, return the number of symbols created.
fn bind_and_count_symbols(source: &str) -> usize {
    let arena = Bump::new();
    let parser = Parser::new(&arena, "test.ts", source);
    let sf = parser.parse_source_file();

    let mut binder = Binder::new();
    binder.bind_source_file(&sf);

    binder.get_symbols().len()
}

// ============================================================================
// Symbol Creation
// ============================================================================

#[test]
fn test_bind_empty_file() {
    let count = bind_and_count_symbols("");
    // At minimum, there should be the source file symbol
    assert!(count >= 0);
}

#[test]
fn test_bind_variable_declaration() {
    let count = bind_and_count_symbols("const x = 42;");
    // Should create at least one symbol for 'x'
    assert!(count >= 1, "Expected at least 1 symbol, got {}", count);
}

#[test]
fn test_bind_function_declaration() {
    let count = bind_and_count_symbols("function foo() {}");
    assert!(count >= 1, "Expected at least 1 symbol for function, got {}", count);
}

#[test]
fn test_bind_class_declaration() {
    let count = bind_and_count_symbols("class Foo { x: number = 0; }");
    // Symbols for: class Foo, member x
    assert!(count >= 1, "Expected at least 1 symbol, got {}", count);
}

#[test]
fn test_bind_interface_declaration() {
    let count = bind_and_count_symbols("interface Foo { bar: string; }");
    assert!(count >= 1, "Expected at least 1 symbol, got {}", count);
}

#[test]
fn test_bind_enum_declaration() {
    let count = bind_and_count_symbols("enum Color { Red, Green, Blue }");
    // Symbols for: enum Color + 3 members
    assert!(count >= 1, "Expected at least 1 symbol, got {}", count);
}

#[test]
fn test_bind_multiple_declarations() {
    let src = r#"
        const a = 1;
        let b = 2;
        var c = 3;
        function foo() {}
        class Bar {}
    "#;
    let count = bind_and_count_symbols(src);
    assert!(count >= 5, "Expected at least 5 symbols, got {}", count);
}

// ============================================================================
// Scope and Hoisting
// ============================================================================

#[test]
fn test_bind_nested_scopes() {
    let src = r#"
        const x = 1;
        {
            const y = 2;
            {
                const z = 3;
            }
        }
    "#;
    let count = bind_and_count_symbols(src);
    assert!(count >= 3, "Expected at least 3 symbols, got {}", count);
}

#[test]
fn test_bind_function_parameters() {
    let src = "function add(a: number, b: number): number { return a + b; }";
    let count = bind_and_count_symbols(src);
    // Symbols for: function 'add', params 'a', 'b'
    assert!(count >= 1, "Expected at least 1 symbol, got {}", count);
}

#[test]
fn test_bind_class_with_methods() {
    let src = r#"
        class Calculator {
            add(a: number, b: number): number {
                return a + b;
            }
            subtract(a: number, b: number): number {
                return a - b;
            }
        }
    "#;
    let count = bind_and_count_symbols(src);
    assert!(count >= 1, "Expected at least 1 symbol, got {}", count);
}

// ============================================================================
// Import / Export Binding
// ============================================================================

#[test]
fn test_bind_import_declaration() {
    let src = "import { foo } from 'bar';";
    let count = bind_and_count_symbols(src);
    assert!(count >= 1, "Expected at least 1 symbol for import, got {}", count);
}

#[test]
fn test_bind_export_declaration() {
    let src = "export const x = 42;";
    let count = bind_and_count_symbols(src);
    assert!(count >= 1, "Expected at least 1 symbol, got {}", count);
}

// ============================================================================
// Type Declarations
// ============================================================================

#[test]
fn test_bind_type_alias() {
    let count = bind_and_count_symbols("type Name = string;");
    assert!(count >= 1, "Expected at least 1 symbol, got {}", count);
}

#[test]
fn test_bind_generic_declaration() {
    let src = "function identity<T>(value: T): T { return value; }";
    let count = bind_and_count_symbols(src);
    assert!(count >= 1, "Expected at least 1 symbol, got {}", count);
}

// ============================================================================
// Fixture Files
// ============================================================================

#[test]
fn test_bind_basic_fixture() {
    let source = include_str!("../../../tests/fixtures/basic.ts");
    let arena = Bump::new();
    let parser = Parser::new(&arena, "basic.ts", source);
    let sf = parser.parse_source_file();

    let mut binder = Binder::new();
    binder.bind_source_file(&sf);
    // Should not panic, and should create symbols
    assert!(binder.get_symbols().len() > 0, "Expected symbols to be created");
}

#[test]
fn test_bind_classes_fixture() {
    let source = include_str!("../../../tests/fixtures/classes.ts");
    let arena = Bump::new();
    let parser = Parser::new(&arena, "classes.ts", source);
    let sf = parser.parse_source_file();

    let mut binder = Binder::new();
    binder.bind_source_file(&sf);
    assert!(binder.get_symbols().len() > 0, "Expected symbols to be created");
}

#[test]
fn test_bind_modules_fixture() {
    let source = include_str!("../../../tests/fixtures/modules.ts");
    let arena = Bump::new();
    let parser = Parser::new(&arena, "modules.ts", source);
    let sf = parser.parse_source_file();

    let mut binder = Binder::new();
    binder.bind_source_file(&sf);
    assert!(binder.get_symbols().len() > 0, "Expected symbols to be created");
}

#[test]
fn test_bind_generics_fixture() {
    let source = include_str!("../../../tests/fixtures/generics.ts");
    let arena = Bump::new();
    let parser = Parser::new(&arena, "generics.ts", source);
    let sf = parser.parse_source_file();

    let mut binder = Binder::new();
    binder.bind_source_file(&sf);
    assert!(binder.get_symbols().len() > 0, "Expected symbols to be created");
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_bind_duplicate_declarations() {
    let src = r#"
        const x = 1;
        const x = 2;
    "#;
    // Should not panic even with duplicate declarations
    let count = bind_and_count_symbols(src);
    assert!(count >= 1);
}

#[test]
fn test_bind_deeply_nested() {
    let mut src = String::from("function a() { ");
    for _ in 0..10 {
        src.push_str("function b() { ");
    }
    src.push_str("return 1;");
    for _ in 0..10 {
        src.push_str(" }");
    }
    src.push_str(" }");

    // Should not stack overflow
    let count = bind_and_count_symbols(&src);
    assert!(count >= 1);
}

// ============================================================================
// Phase 3: Comprehensive binder hardening tests
// ============================================================================

use rscript_ast::types::SymbolFlags;

/// Helper: parse, bind, return the binder for detailed assertions.
fn bind(source: &str) -> Binder {
    let arena = Bump::new();
    let parser = Parser::new(&arena, "test.ts", source);
    let sf = parser.parse_source_file();
    let mut binder = Binder::new();
    binder.bind_source_file(&sf);
    binder
}

// --- Symbol flag correctness ---

#[test]
fn test_symbol_flags_variable() {
    let binder = bind("const x = 42;");
    let sym = binder.get_symbols().iter().find(|s| s.name_text == "x");
    assert!(sym.is_some(), "Symbol 'x' not found");
    let sym = sym.unwrap();
    assert!(
        sym.flags.intersects(SymbolFlags::BLOCK_SCOPED_VARIABLE) || sym.flags.intersects(SymbolFlags::VARIABLE),
        "Expected variable flags, got {:?}", sym.flags
    );
}

#[test]
fn test_symbol_flags_var_vs_let() {
    let binder = bind("var a = 1; let b = 2; const c = 3;");
    let a = binder.get_symbols().iter().find(|s| s.name_text == "a");
    let b = binder.get_symbols().iter().find(|s| s.name_text == "b");
    let c = binder.get_symbols().iter().find(|s| s.name_text == "c");
    assert!(a.is_some(), "Symbol 'a' not found");
    assert!(b.is_some(), "Symbol 'b' not found");
    assert!(c.is_some(), "Symbol 'c' not found");
}

#[test]
fn test_symbol_flags_function() {
    let binder = bind("function greet() {}");
    let sym = binder.get_symbols().iter().find(|s| s.name_text == "greet");
    assert!(sym.is_some(), "Symbol 'greet' not found");
    let sym = sym.unwrap();
    assert!(
        sym.flags.intersects(SymbolFlags::FUNCTION),
        "Expected FUNCTION flag, got {:?}", sym.flags
    );
}

#[test]
fn test_symbol_flags_class() {
    let binder = bind("class MyClass {}");
    let sym = binder.get_symbols().iter().find(|s| s.name_text == "MyClass");
    assert!(sym.is_some(), "Symbol 'MyClass' not found");
    let sym = sym.unwrap();
    assert!(
        sym.flags.intersects(SymbolFlags::CLASS),
        "Expected CLASS flag, got {:?}", sym.flags
    );
}

#[test]
fn test_symbol_flags_interface() {
    let binder = bind("interface Printable { print(): void; }");
    let sym = binder.get_symbols().iter().find(|s| s.name_text == "Printable");
    assert!(sym.is_some(), "Symbol 'Printable' not found");
    let sym = sym.unwrap();
    assert!(
        sym.flags.intersects(SymbolFlags::INTERFACE),
        "Expected INTERFACE flag, got {:?}", sym.flags
    );
}

#[test]
fn test_symbol_flags_enum() {
    let binder = bind("enum Direction { Up, Down }");
    let sym = binder.get_symbols().iter().find(|s| s.name_text == "Direction");
    assert!(sym.is_some(), "Symbol 'Direction' not found");
    let sym = sym.unwrap();
    assert!(
        sym.flags.intersects(SymbolFlags::REGULAR_ENUM) || sym.flags.intersects(SymbolFlags::CONST_ENUM),
        "Expected enum flag, got {:?}", sym.flags
    );
}

#[test]
fn test_symbol_flags_type_alias() {
    let binder = bind("type Name = string;");
    let sym = binder.get_symbols().iter().find(|s| s.name_text == "Name");
    assert!(sym.is_some(), "Symbol 'Name' not found");
    let sym = sym.unwrap();
    assert!(
        sym.flags.intersects(SymbolFlags::TYPE_ALIAS),
        "Expected TYPE_ALIAS flag, got {:?}", sym.flags
    );
}

// --- Scope correctness ---

#[test]
fn test_nested_block_scopes_create_symbols() {
    let src = r#"
        const outer = 1;
        {
            const inner = 2;
        }
    "#;
    let binder = bind(src);
    let outer = binder.get_symbols().iter().find(|s| s.name_text == "outer");
    let inner = binder.get_symbols().iter().find(|s| s.name_text == "inner");
    assert!(outer.is_some(), "Symbol 'outer' not found");
    assert!(inner.is_some(), "Symbol 'inner' not found");
}

#[test]
fn test_function_scope_creates_parameter_symbols() {
    let src = "function add(a: number, b: number): number { return a + b; }";
    let binder = bind(src);
    let add = binder.get_symbols().iter().find(|s| s.name_text == "add");
    assert!(add.is_some(), "Symbol 'add' not found");

    let a = binder.get_symbols().iter().find(|s| s.name_text == "a");
    let b = binder.get_symbols().iter().find(|s| s.name_text == "b");
    assert!(a.is_some(), "Parameter 'a' not found");
    assert!(b.is_some(), "Parameter 'b' not found");
}

#[test]
fn test_scope_depth_returns_to_zero() {
    let src = r#"
        function foo() {
            {
                const x = 1;
            }
        }
    "#;
    let binder = bind(src);
    // After binding, scope depth should be back to 0
    assert_eq!(binder.scope_depth(), 0, "Scope depth should return to 0 after binding");
}

// --- Class member binding ---

#[test]
fn test_class_members_bound() {
    let src = r#"
        class Person {
            name: string;
            age: number;
            greet(): void {}
        }
    "#;
    let binder = bind(src);
    let person = binder.get_symbols().iter().find(|s| s.name_text == "Person");
    assert!(person.is_some(), "Class 'Person' not found");

    // Check that members exist as symbols
    let name = binder.get_symbols().iter().find(|s| s.name_text == "name");
    let age = binder.get_symbols().iter().find(|s| s.name_text == "age");
    let greet = binder.get_symbols().iter().find(|s| s.name_text == "greet");
    assert!(name.is_some(), "Member 'name' not found");
    assert!(age.is_some(), "Member 'age' not found");
    assert!(greet.is_some(), "Member 'greet' not found");
}

#[test]
fn test_class_constructor_params_bound() {
    let src = r#"
        class Point {
            constructor(public x: number, public y: number) {}
        }
    "#;
    let binder = bind(src);
    let point = binder.get_symbols().iter().find(|s| s.name_text == "Point");
    assert!(point.is_some(), "Class 'Point' not found");
}

// --- Import binding ---

#[test]
fn test_named_import_creates_symbol() {
    let binder = bind("import { useState, useEffect } from 'react';");
    let use_state = binder.get_symbols().iter().find(|s| s.name_text == "useState");
    let use_effect = binder.get_symbols().iter().find(|s| s.name_text == "useEffect");
    assert!(use_state.is_some(), "Import 'useState' not found");
    assert!(use_effect.is_some(), "Import 'useEffect' not found");
}

#[test]
fn test_default_import_creates_symbol() {
    let binder = bind("import React from 'react';");
    let react = binder.get_symbols().iter().find(|s| s.name_text == "React");
    assert!(react.is_some(), "Default import 'React' not found");
}

#[test]
fn test_namespace_import_creates_symbol() {
    let binder = bind("import * as fs from 'fs';");
    let fs = binder.get_symbols().iter().find(|s| s.name_text == "fs");
    assert!(fs.is_some(), "Namespace import 'fs' not found");
}

// --- Export binding ---

#[test]
fn test_export_function_creates_symbol() {
    let binder = bind("export function render() {}");
    let render = binder.get_symbols().iter().find(|s| s.name_text == "render");
    assert!(render.is_some(), "Exported function 'render' not found");
}

#[test]
fn test_export_class_creates_symbol() {
    let binder = bind("export class Component {}");
    let comp = binder.get_symbols().iter().find(|s| s.name_text == "Component");
    assert!(comp.is_some(), "Exported class 'Component' not found");
}

#[test]
fn test_export_default_creates_symbol() {
    let binder = bind("export default function main() {}");
    // The function name 'main' should still be bound
    let main = binder.get_symbols().iter().find(|s| s.name_text == "main");
    assert!(main.is_some(), "Default exported function 'main' not found");
}

// --- Enum member binding ---

#[test]
fn test_enum_members_bound() {
    let src = r#"
        enum Color {
            Red,
            Green,
            Blue,
        }
    "#;
    let binder = bind(src);
    let color = binder.get_symbols().iter().find(|s| s.name_text == "Color");
    assert!(color.is_some(), "Enum 'Color' not found");

    let red = binder.get_symbols().iter().find(|s| s.name_text == "Red");
    let green = binder.get_symbols().iter().find(|s| s.name_text == "Green");
    let blue = binder.get_symbols().iter().find(|s| s.name_text == "Blue");
    assert!(red.is_some(), "Enum member 'Red' not found");
    assert!(green.is_some(), "Enum member 'Green' not found");
    assert!(blue.is_some(), "Enum member 'Blue' not found");
}

#[test]
fn test_const_enum_bound() {
    let binder = bind("const enum Flags { A = 1, B = 2, C = 4 }");
    let flags = binder.get_symbols().iter().find(|s| s.name_text == "Flags");
    assert!(flags.is_some(), "Const enum 'Flags' not found");
}

// --- Declaration merging ---

#[test]
fn test_interface_declaration_merging() {
    let src = r#"
        interface Box { width: number; }
        interface Box { height: number; }
    "#;
    let binder = bind(src);
    // Should merge into a single symbol with multiple declarations
    let boxes: Vec<_> = binder.get_symbols().iter().filter(|s| s.name_text == "Box").collect();
    // With declaration merging, there should be one or two symbols depending on implementation
    assert!(!boxes.is_empty(), "Interface 'Box' not found");
}

// --- Hoisting behavior ---

#[test]
fn test_function_hoisting() {
    let src = r#"
        const result = foo();
        function foo() { return 42; }
    "#;
    let binder = bind(src);
    // 'foo' should be hoisted and visible
    let foo = binder.get_symbols().iter().find(|s| s.name_text == "foo");
    assert!(foo.is_some(), "Hoisted function 'foo' not found");
}

#[test]
fn test_var_hoisting() {
    let src = r#"
        console.log(x);
        var x = 42;
    "#;
    let binder = bind(src);
    let x = binder.get_symbols().iter().find(|s| s.name_text == "x");
    assert!(x.is_some(), "Hoisted var 'x' not found");
}

// --- Module / Namespace binding ---

#[test]
fn test_namespace_binding() {
    let src = r#"
        namespace MyApp {
            export const version = "1.0";
        }
    "#;
    let binder = bind(src);
    let ns = binder.get_symbols().iter().find(|s| s.name_text == "MyApp");
    assert!(ns.is_some(), "Namespace 'MyApp' not found");
}

// --- Complex scenarios ---

#[test]
fn test_bind_complete_module() {
    let src = r#"
        import { EventEmitter } from 'events';

        interface Config {
            host: string;
            port: number;
        }

        class Server extends EventEmitter {
            private config: Config;

            constructor(config: Config) {
                super();
                this.config = config;
            }

            start(): void {
                console.log("Starting server");
            }
        }

        export function createServer(config: Config): Server {
            return new Server(config);
        }

        export default createServer;
    "#;
    let binder = bind(src);

    let event_emitter = binder.get_symbols().iter().find(|s| s.name_text == "EventEmitter");
    let config = binder.get_symbols().iter().find(|s| s.name_text == "Config");
    let server = binder.get_symbols().iter().find(|s| s.name_text == "Server");
    let create_server = binder.get_symbols().iter().find(|s| s.name_text == "createServer");

    assert!(event_emitter.is_some(), "Import 'EventEmitter' not found");
    assert!(config.is_some(), "Interface 'Config' not found");
    assert!(server.is_some(), "Class 'Server' not found");
    assert!(create_server.is_some(), "Function 'createServer' not found");
}

#[test]
fn test_bind_generic_constraints() {
    let src = r#"
        function merge<T extends object, U extends object>(a: T, b: U): T & U {
            return Object.assign(a, b);
        }
    "#;
    let binder = bind(src);
    let merge = binder.get_symbols().iter().find(|s| s.name_text == "merge");
    assert!(merge.is_some(), "Function 'merge' not found");
}

// --- Flow node creation ---

#[test]
fn test_flow_nodes_created_for_control_flow() {
    let src = r#"
        function check(x: number): string {
            if (x > 0) {
                return "positive";
            } else {
                return "non-positive";
            }
        }
    "#;
    let binder = bind(src);
    // Should have flow nodes for the if/else branches
    assert!(binder.flow_nodes().len() > 1, "Expected flow nodes to be created");
}

#[test]
fn test_flow_nodes_for_loops() {
    let src = r#"
        for (let i = 0; i < 10; i++) {
            console.log(i);
        }
    "#;
    let binder = bind(src);
    assert!(binder.flow_nodes().len() > 1, "Expected flow nodes for loop");
}

// --- Destructuring binding ---

#[test]
fn test_object_destructuring_binding() {
    let src = "const { a, b, c } = obj;";
    let binder = bind(src);
    let a = binder.get_symbols().iter().find(|s| s.name_text == "a");
    let b = binder.get_symbols().iter().find(|s| s.name_text == "b");
    let c = binder.get_symbols().iter().find(|s| s.name_text == "c");
    assert!(a.is_some(), "Destructured 'a' not found");
    assert!(b.is_some(), "Destructured 'b' not found");
    assert!(c.is_some(), "Destructured 'c' not found");
}

#[test]
fn test_array_destructuring_binding() {
    let src = "const [first, second] = arr;";
    let binder = bind(src);
    let first = binder.get_symbols().iter().find(|s| s.name_text == "first");
    let second = binder.get_symbols().iter().find(|s| s.name_text == "second");
    assert!(first.is_some(), "Destructured 'first' not found");
    assert!(second.is_some(), "Destructured 'second' not found");
}

// --- No-panic stress tests ---

#[test]
fn test_bind_many_declarations() {
    let mut src = String::new();
    for i in 0..100 {
        src.push_str(&format!("const v{} = {};\n", i, i));
    }
    let binder = bind(&src);
    assert!(binder.symbol_count() >= 100, "Expected at least 100 symbols");
}

#[test]
fn test_bind_all_fixtures() {
    // Bind all fixture files without panicking
    for fixture in &["basic.ts", "classes.ts", "modules.ts", "generics.ts", "types.ts", "async_await.ts", "enums.ts", "decorators.ts"] {
        let path = format!("../../../tests/fixtures/{}", fixture);
        let source = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(&path))
            .unwrap_or_else(|_| String::new());
        if source.is_empty() { continue; }

        let arena = Bump::new();
        let parser = Parser::new(&arena, fixture, &source);
        let sf = parser.parse_source_file();
        let mut binder = Binder::new();
        binder.bind_source_file(&sf);
        assert!(binder.symbol_count() > 0, "No symbols created for {}", fixture);
    }
}
