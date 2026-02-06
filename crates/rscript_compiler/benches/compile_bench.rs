//! Benchmark harness for rscript compiler.
//!
//! Uses criterion for reliable benchmarking.
//! Run with: cargo bench -p rscript_compiler

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use bumpalo::Bump;
use rscript_binder::Binder;
use rscript_checker::Checker;
use rscript_compiler::Program;
use rscript_parser::Parser;
use rscript_tsoptions::CompilerOptions;

/// Small TypeScript source for micro-benchmarks.
const SMALL_SOURCE: &str = r#"
const x: number = 42;
let greeting: string = "hello";
function add(a: number, b: number): number {
    return a + b;
}
const result = add(1, 2);
"#;

/// Medium TypeScript source for realistic benchmarks.
const MEDIUM_SOURCE: &str = r#"
interface Shape {
    area(): number;
    perimeter(): number;
}

class Circle implements Shape {
    constructor(private radius: number) {}

    area(): number {
        return Math.PI * this.radius ** 2;
    }

    perimeter(): number {
        return 2 * Math.PI * this.radius;
    }
}

class Rectangle implements Shape {
    constructor(
        private width: number,
        private height: number,
    ) {}

    area(): number {
        return this.width * this.height;
    }

    perimeter(): number {
        return 2 * (this.width + this.height);
    }
}

function totalArea(shapes: Shape[]): number {
    let total = 0;
    for (const shape of shapes) {
        total += shape.area();
    }
    return total;
}

type Result<T> = { ok: true; value: T } | { ok: false; error: string };

function parseNumber(input: string): Result<number> {
    const n = Number(input);
    if (isNaN(n)) {
        return { ok: false, error: `"${input}" is not a valid number` };
    }
    return { ok: true, value: n };
}

enum Direction {
    Up = "UP",
    Down = "DOWN",
    Left = "LEFT",
    Right = "RIGHT",
}

async function fetchData<T>(url: string): Promise<T> {
    const response = await fetch(url);
    return response.json();
}

const shapes: Shape[] = [
    new Circle(5),
    new Rectangle(3, 4),
    new Circle(10),
];

export { totalArea, parseNumber, Direction };
export type { Shape, Result };
"#;

/// Generate a large TypeScript source.
fn generate_large_source(num_classes: usize, num_functions: usize) -> String {
    let mut source = String::new();

    for i in 0..num_classes {
        source.push_str(&format!(
            "class Class{i} {{
    private field{i}: number;
    constructor(value: number) {{
        this.field{i} = value;
    }}
    method{i}(): number {{
        return this.field{i} * 2;
    }}
    static create(value: number): Class{i} {{
        return new Class{i}(value);
    }}
}}\n\n"
        ));
    }

    for i in 0..num_functions {
        source.push_str(&format!(
            "function func{i}(x: number, y: string): {{ num: number; str: string }} {{
    return {{ num: x + {i}, str: y + '{i}' }};
}}\n\n"
        ));
    }

    source
}

// ============================================================================
// Scanner Benchmarks
// ============================================================================

fn bench_scanner(c: &mut Criterion) {
    let mut group = c.benchmark_group("scanner");

    group.bench_function("small", |b| {
        b.iter(|| {
            let mut scanner = rscript_scanner::Scanner::new(black_box(SMALL_SOURCE));
            while scanner.scan() != rscript_ast::syntax_kind::SyntaxKind::EndOfFileToken {}
        });
    });

    group.bench_function("medium", |b| {
        b.iter(|| {
            let mut scanner = rscript_scanner::Scanner::new(black_box(MEDIUM_SOURCE));
            while scanner.scan() != rscript_ast::syntax_kind::SyntaxKind::EndOfFileToken {}
        });
    });

    let large = generate_large_source(50, 50);
    group.bench_function("large", |b| {
        b.iter(|| {
            let mut scanner = rscript_scanner::Scanner::new(black_box(&large));
            while scanner.scan() != rscript_ast::syntax_kind::SyntaxKind::EndOfFileToken {}
        });
    });

    group.finish();
}

// ============================================================================
// Parser Benchmarks
// ============================================================================

fn bench_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser");

    group.bench_function("small", |b| {
        b.iter(|| {
            let arena = Bump::new();
            let parser = Parser::new(&arena, "bench.ts", black_box(SMALL_SOURCE));
            let _ = black_box(parser.parse_source_file());
        });
    });

    group.bench_function("medium", |b| {
        b.iter(|| {
            let arena = Bump::new();
            let parser = Parser::new(&arena, "bench.ts", black_box(MEDIUM_SOURCE));
            let _ = black_box(parser.parse_source_file());
        });
    });

    let large = generate_large_source(50, 50);
    group.bench_function("large", |b| {
        b.iter(|| {
            let arena = Bump::new();
            let parser = Parser::new(&arena, "bench.ts", black_box(&large));
            let _ = black_box(parser.parse_source_file());
        });
    });

    group.finish();
}

// ============================================================================
// Binder Benchmarks
// ============================================================================

fn bench_binder(c: &mut Criterion) {
    let mut group = c.benchmark_group("binder");

    group.bench_function("medium", |b| {
        b.iter(|| {
            let arena = Bump::new();
            let parser = Parser::new(&arena, "bench.ts", black_box(MEDIUM_SOURCE));
            let sf = parser.parse_source_file();
            let mut binder = Binder::new();
            binder.bind_source_file(black_box(&sf));
        });
    });

    let large = generate_large_source(50, 50);
    group.bench_function("large", |b| {
        b.iter(|| {
            let arena = Bump::new();
            let parser = Parser::new(&arena, "bench.ts", black_box(&large));
            let sf = parser.parse_source_file();
            let mut binder = Binder::new();
            binder.bind_source_file(black_box(&sf));
        });
    });

    group.finish();
}

// ============================================================================
// Full Pipeline Benchmarks
// ============================================================================

fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline");

    group.bench_function("small", |b| {
        b.iter(|| {
            let arena = Bump::new();
            let parser = Parser::new(&arena, "bench.ts", black_box(SMALL_SOURCE));
            let sf = parser.parse_source_file();
            let mut binder = Binder::new();
            binder.bind_source_file(&sf);
            let mut checker = Checker::new(binder);
            checker.check_source_file(&sf);
        });
    });

    group.bench_function("medium", |b| {
        b.iter(|| {
            let arena = Bump::new();
            let parser = Parser::new(&arena, "bench.ts", black_box(MEDIUM_SOURCE));
            let sf = parser.parse_source_file();
            let mut binder = Binder::new();
            binder.bind_source_file(&sf);
            let mut checker = Checker::new(binder);
            checker.check_source_file(&sf);
        });
    });

    let large = generate_large_source(50, 50);
    group.bench_function("large", |b| {
        b.iter(|| {
            let arena = Bump::new();
            let parser = Parser::new(&arena, "bench.ts", black_box(&large));
            let sf = parser.parse_source_file();
            let mut binder = Binder::new();
            binder.bind_source_file(&sf);
            let mut checker = Checker::new(binder);
            checker.check_source_file(&sf);
        });
    });

    group.finish();
}

// ============================================================================
// Compile (Program) Benchmarks
// ============================================================================

fn bench_program_compile(c: &mut Criterion) {
    let mut group = c.benchmark_group("program_compile");

    group.bench_function("single_file", |b| {
        b.iter(|| {
            let arena = Bump::new();
            let options = CompilerOptions::default();
            let mut program = Program::new(&arena, vec![], options);
            program.add_source("bench.ts".to_string(), MEDIUM_SOURCE.to_string());
            let _ = black_box(program.compile());
        });
    });

    group.bench_function("multiple_files", |b| {
        b.iter(|| {
            let arena = Bump::new();
            let options = CompilerOptions::default();
            let mut program = Program::new(&arena, vec![], options);
            for i in 0..10 {
                program.add_source(
                    format!("file_{}.ts", i),
                    MEDIUM_SOURCE.to_string(),
                );
            }
            let _ = black_box(program.compile());
        });
    });

    group.finish();
}

// ============================================================================
// Scaling Benchmarks
// ============================================================================

fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling");

    for size in [10, 50, 100, 200] {
        let source = generate_large_source(size / 2, size / 2);
        group.bench_with_input(
            BenchmarkId::new("classes_and_functions", size),
            &source,
            |b, source| {
                b.iter(|| {
                    let arena = Bump::new();
                    let parser = Parser::new(&arena, "bench.ts", black_box(source));
                    let sf = parser.parse_source_file();
                    let mut binder = Binder::new();
                    binder.bind_source_file(&sf);
                    let mut checker = Checker::new(binder);
                    checker.check_source_file(&sf);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_scanner,
    bench_parser,
    bench_binder,
    bench_full_pipeline,
    bench_program_compile,
    bench_scaling,
);
criterion_main!(benches);
