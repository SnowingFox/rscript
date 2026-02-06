//! rsc: The rscript TypeScript compiler CLI.
//!
//! Usage:
//!   rsc [options] [file...]
//!
//! This provides a tsc-compatible command-line interface.

use clap::Parser as ClapParser;
use std::path::{Path, PathBuf};
use std::process;
use std::time::Instant;

#[derive(ClapParser, Debug)]
#[command(name = "rsc", about = "rscript - A fast TypeScript compiler written in Rust", disable_version_flag = true)]
struct Cli {
    /// TypeScript files to compile.
    #[arg(value_name = "FILE")]
    files: Vec<String>,

    /// Path to tsconfig.json.
    #[arg(short = 'p', long = "project")]
    project: Option<String>,

    /// Do not emit outputs.
    #[arg(long = "noEmit")]
    no_emit: bool,

    /// Enable all strict type-checking options.
    #[arg(long)]
    strict: bool,

    /// Specify ECMAScript target version.
    #[arg(long)]
    target: Option<String>,

    /// Specify module code generation.
    #[arg(long)]
    module: Option<String>,

    /// Redirect output structure to the directory.
    #[arg(long = "outDir")]
    out_dir: Option<String>,

    /// Generate .d.ts declaration files.
    #[arg(short = 'd', long)]
    declaration: bool,

    /// Generate source map files.
    #[arg(long = "sourceMap")]
    source_map: bool,

    /// Watch input files.
    #[arg(short = 'w', long)]
    watch: bool,

    /// Build project references.
    #[arg(short = 'b', long)]
    build: bool,

    /// Initialize a tsconfig.json file.
    #[arg(long)]
    init: bool,

    /// Print the compiler version.
    #[arg(short = 'v', long)]
    version: bool,

    /// List all files that are part of the compilation.
    #[arg(long = "listFiles")]
    list_files: bool,

    /// Enable pretty printing for diagnostics.
    #[arg(long, default_value_t = true)]
    pretty: bool,

    /// Start the language server.
    #[arg(long)]
    lsp: bool,
}

// ANSI color codes
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const GRAY: &str = "\x1b[90m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

fn main() {
    let cli = Cli::parse();

    if cli.version {
        println!("rsc Version 0.1.0");
        return;
    }

    if cli.init {
        run_init();
        return;
    }

    if cli.lsp {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(rscript_lsp::start_lsp_server());
        return;
    }

    if cli.build {
        run_build(&cli);
        return;
    }

    if cli.watch {
        run_watch(&cli);
        return;
    }

    // Normal compilation
    let exit_code = run_compile(&cli);
    process::exit(exit_code);
}

fn run_compile(cli: &Cli) -> i32 {
    let start = Instant::now();

    // Determine source files
    let (files, config) = resolve_input_files(cli);

    if files.is_empty() {
        print_error("No input files found.");
        return 1;
    }

    if cli.list_files {
        for f in &files {
            println!("{}", f);
        }
    }

    // Build compiler options
    let mut options = if let Some(ref cfg) = config {
        cfg.compiler_options.clone().unwrap_or_default()
    } else {
        rscript_tsoptions::CompilerOptions::default()
    };

    // CLI options override tsconfig
    if cli.strict { options.strict = Some(true); }
    if cli.declaration { options.declaration = Some(true); }
    if cli.source_map { options.source_map = Some(true); }
    if cli.out_dir.is_some() { options.out_dir = cli.out_dir.clone(); }
    if cli.no_emit { options.no_emit = Some(true); }

    // Create and run the program
    let arena = bumpalo::Bump::new();
    let mut program = rscript_compiler::Program::new(&arena, files, options);

    if let Err(e) = program.load_root_files() {
        print_error(&format!("Failed to load source files: {}", e));
        return 1;
    }

    let diagnostics = program.compile();

    // Print diagnostics with color
    let use_color = cli.pretty && atty_is_terminal();
    for diag in diagnostics.diagnostics() {
        print_diagnostic(diag, use_color);
    }

    let elapsed = start.elapsed();

    if diagnostics.has_errors() {
        let count = diagnostics.error_count();
        if use_color {
            eprintln!(
                "\n{}Found {} error{}.{}",
                RED,
                count,
                if count == 1 { "" } else { "s" },
                RESET
            );
        } else {
            eprintln!(
                "\nFound {} error{}.",
                count,
                if count == 1 { "" } else { "s" }
            );
        }
        return 2;
    }

    // Emit if not --noEmit
    if !cli.no_emit {
        let _results = program.emit();
    }

    if use_color {
        eprintln!(
            "{}Compilation completed in {:.2}s.{}",
            GRAY,
            elapsed.as_secs_f64(),
            RESET
        );
    }

    0
}

fn run_init() {
    let tsconfig_path = Path::new("tsconfig.json");
    if tsconfig_path.exists() {
        print_error("A tsconfig.json file already exists in the current directory.");
        process::exit(1);
    }

    let default_config = r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "outDir": "./dist",
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist"]
}
"#;

    match std::fs::write(tsconfig_path, default_config) {
        Ok(()) => {
            println!(
                "Successfully created a tsconfig.json file."
            );
        }
        Err(e) => {
            print_error(&format!("Failed to create tsconfig.json: {}", e));
            process::exit(1);
        }
    }
}

fn run_watch(cli: &Cli) {
    println!("Starting compilation in watch mode...");
    println!();

    // Initial compile
    let exit_code = run_compile(cli);
    let _ = exit_code;

    println!();
    println!("Watching for file changes...");

    // Use notify crate for file watching
    // For now, poll-based watching
    let (files, _config) = resolve_input_files(cli);
    let mut last_modified = get_latest_mtime(&files);

    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));
        let current_mtime = get_latest_mtime(&files);
        if current_mtime > last_modified {
            last_modified = current_mtime;
            println!();
            println!("File change detected. Starting compilation...");
            println!();
            let _code = run_compile(cli);
        }
    }
}

fn run_build(cli: &Cli) {
    // Build mode: compile project references in order
    let project_path = cli.project.as_deref().unwrap_or("tsconfig.json");
    let config = match rscript_tsoptions::parse_tsconfig_file(project_path) {
        Ok(c) => c,
        Err(e) => {
            print_error(&format!("Failed to read project '{}': {}", project_path, e));
            process::exit(1);
        }
    };

    if let Some(refs) = &config.references {
        for reference in refs {
            println!("Building project reference: {}", reference.path);
            let ref_config_path = PathBuf::from(&reference.path).join("tsconfig.json");
            let ref_cli = Cli {
                files: vec![],
                project: Some(ref_config_path.to_string_lossy().to_string()),
                no_emit: cli.no_emit,
                strict: cli.strict,
                target: cli.target.clone(),
                module: cli.module.clone(),
                out_dir: cli.out_dir.clone(),
                declaration: true, // Always generate declarations for references
                source_map: cli.source_map,
                watch: false,
                build: false,
                init: false,
                version: false,
                list_files: cli.list_files,
                pretty: cli.pretty,
                lsp: false,
            };
            let code = run_compile(&ref_cli);
            if code != 0 {
                process::exit(code);
            }
        }
    }

    // Build the main project
    let code = run_compile(cli);
    process::exit(code);
}

fn resolve_input_files(cli: &Cli) -> (Vec<String>, Option<rscript_tsoptions::TsConfig>) {
    if let Some(ref project) = cli.project {
        match load_files_from_tsconfig(project) {
            Ok((files, config)) => (files, Some(config)),
            Err(e) => {
                print_error(&format!("Failed to read project '{}': {}", project, e));
                process::exit(1);
            }
        }
    } else if !cli.files.is_empty() {
        (cli.files.clone(), None)
    } else if Path::new("tsconfig.json").exists() {
        match load_files_from_tsconfig("tsconfig.json") {
            Ok((files, config)) => (files, Some(config)),
            Err(e) => {
                print_error(&format!("Failed to read tsconfig.json: {}", e));
                process::exit(1);
            }
        }
    } else {
        (vec![], None)
    }
}

fn load_files_from_tsconfig(path: &str) -> Result<(Vec<String>, rscript_tsoptions::TsConfig), Box<dyn std::error::Error>> {
    let config = rscript_tsoptions::parse_tsconfig_file(path)?;
    let root_dir = Path::new(path).parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());

    // If "files" is specified, use it directly
    if let Some(ref files) = config.files {
        let resolved: Vec<String> = files.iter()
            .map(|f| {
                let p = PathBuf::from(&root_dir).join(f);
                p.to_string_lossy().to_string()
            })
            .collect();
        return Ok((resolved, config));
    }

    // Use include/exclude patterns
    let default_include = vec!["src/**/*".to_string()];
    let default_exclude = vec!["node_modules".to_string()];
    let include = config.include.as_deref().unwrap_or(&default_include);
    let exclude = config.exclude.as_deref().unwrap_or(&default_exclude);

    let files = rscript_module::discover_source_files(
        &root_dir,
        include,
        exclude,
        None,
    );

    Ok((files, config))
}

fn print_diagnostic(diag: &rscript_diagnostics::Diagnostic, use_color: bool) {
    if use_color {
        let color = if diag.is_error() { RED } else { YELLOW };
        let category = if diag.is_error() { "error" } else { "warning" };
        if let Some(ref file) = diag.file {
            eprint!("{}{}{}", CYAN, file, RESET);
            if let Some(span) = diag.span {
                eprint!("({})", span.start);
            }
            eprint!(": ");
        }
        eprintln!(
            "{}{}{}{} {}{}{}: {}",
            BOLD, color, category, RESET,
            CYAN, format!("TS{}", diag.code), RESET,
            diag.message_text
        );
    } else {
        eprintln!("{}", diag);
    }
}

fn print_error(msg: &str) {
    if atty_is_terminal() {
        eprintln!("{}{}error{}: {}", BOLD, RED, RESET, msg);
    } else {
        eprintln!("error: {}", msg);
    }
}

fn atty_is_terminal() -> bool {
    // Simple check - on Unix, check if stderr is a terminal
    #[cfg(unix)]
    {
        unsafe { libc::isatty(2) != 0 }
    }
    #[cfg(not(unix))]
    {
        true // Assume terminal on other platforms
    }
}

fn get_latest_mtime(files: &[String]) -> std::time::SystemTime {
    let mut latest = std::time::SystemTime::UNIX_EPOCH;
    for f in files {
        if let Ok(metadata) = std::fs::metadata(f) {
            if let Ok(mtime) = metadata.modified() {
                if mtime > latest {
                    latest = mtime;
                }
            }
        }
    }
    latest
}
