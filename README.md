# rscript

A production-grade TypeScript compiler written in Rust, inspired by [tsgo](https://github.com/microsoft/typescript-go) (TypeScript 7).

## Goals

- **1:1 TypeScript compatibility**: Faithful port of TypeScript's compiler behavior, verified against the official 70K+ test suite
- **Performance**: Leveraging Rust's zero-cost abstractions, arena allocation, string interning, and parallelism to exceed tsgo's 10x speedup over tsc
- **Production-ready**: Standalone CLI tool (`rsc`) that can replace `tsc`, plus an LSP server for editor integration

## Architecture

```
rscript/
  crates/
    rscript_core/         Core utilities, arena allocator, string interning
    rscript_diagnostics/  Diagnostic messages (ported from diagnosticMessages.json)
    rscript_ast/          AST nodes, SyntaxKind (300+ variants), flag types
    rscript_scanner/      Lexer/tokenizer (port of scanner.ts)
    rscript_parser/       Recursive descent parser (port of parser.ts)
    rscript_binder/       Symbol table, scope chains, declaration merging
    rscript_checker/      Type checker (port of checker.ts)
    rscript_emitter/      JS/DTS output coordinator
    rscript_transformers/ AST transforms (TypeScript strip, JSX, decorators, downlevel)
    rscript_printer/      AST to text output
    rscript_nodebuilder/  Synthetic node construction for type display
    rscript_module/       Module resolution (Node10, Node16, Bundler, Classic)
    rscript_tsoptions/    tsconfig.json parsing, CompilerOptions
    rscript_tspath/       Path normalization, extension handling
    rscript_sourcemap/    Source map generation
    rscript_evaluator/    Constant expression evaluation
    rscript_compiler/     Program creation, compilation orchestration
    rscript_ls/           Language service (completions, hover, go-to-def)
    rscript_lsp/          Language Server Protocol implementation
    rscript_cli/          CLI binary (`rsc`)
```

## Key Design Decisions

1. **Arena Allocation (bumpalo)**: All AST nodes allocated in a bump arena. O(1) deallocation, cache-friendly linear layout.
2. **String Interning (lasso)**: All identifiers interned for O(1) comparison via integer IDs.
3. **ID-based Type References**: Types stored in a `TypeTable` arena, referenced by `TypeId`. Avoids lifetime complexity in the checker.
4. **Faithful 1:1 Port**: Following tsgo's proven strategy of faithful porting rather than reimagining (which caused stc to fail).

## Building

```bash
# Build everything
cargo build --release

# Run the compiler
./target/release/rsc --version
./target/release/rsc file.ts --noEmit

# Run tests
cargo test
```

## Usage

```bash
# Type-check files
rsc file.ts --noEmit

# Compile with options
rsc src/**/*.ts --outDir dist --declaration --sourceMap

# Use tsconfig.json
rsc -p tsconfig.json

# Start language server
rsc --lsp

# Watch mode
rsc -w file.ts
```

## Performance Targets

| Metric       | tsc (JS) | tsgo (Go) | rscript (Rust) Target |
| ------------ | -------- | --------- | --------------------- |
| Cold compile | 10s      | 1s        | < 0.8s                |
| Memory usage | 1x       | 0.5x      | < 0.3x                |
| Incremental  | 5s       | 0.5s      | < 0.4s                |
| LSP startup  | 10s      | 1.2s      | < 1s                  |

## License

MIT OR Apache-2.0
