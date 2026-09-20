use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;
use std::time::Instant;
use wfl::Interpreter;
use wfl::analyzer::{Analyzer, StaticAnalyzer};
use wfl::config;
use wfl::debug_report;
use wfl::diagnostics::{DiagnosticReporter, Severity};
use wfl::fixer::{CodeFixer, validate_source, write_fixed_file};
use wfl::lexer::lex_wfl_with_positions_checked;
use wfl::linter::Linter;
use wfl::parser::Parser;
use wfl::repl;
use wfl::typechecker::{TypeCheckError, TypeChecker};
use wfl::wfl_config;
use wfl::{error, exec_trace, info};

mod project_init;

/// Describe supported operations and the configuration controls used by the CLI.
fn print_help() {
    println!("WebFirst Language (WFL) Compiler and Interpreter");
    println!();
    println!("USAGE:");
    println!("    wfl [FLAGS] [OPTIONS] [file]");
    println!("    wfl config");
    println!("    wfl init");
    println!();
    println!("FLAGS:");
    println!("    --help, -h         Prints this help information");
    println!("    --version, -v, -V  Prints the version information");
    println!("    --lint             Run the linter on the specified file");
    println!("    --lint --fix       Apply auto-fixes after linting");
    println!("        --in-place     Overwrite the file in place");
    println!("        --diff         Show a diff instead of rewriting");
    println!("    --analyze          Run the static analyzer on the specified file");
    println!("    --step             Run in single-step execution mode");
    println!("    --edit             Open the specified file in the default editor");
    println!("    --lex              Dump lexer output to a text file and exit");
    println!("    --ast, --parse      Dump abstract syntax tree to a text file and exit");
    println!("    --dump-env         Dump the current environment details for troubleshooting");
    println!("        --output <file>    Specify an output file for the environment dump");
    println!("    --time             Measure and display execution time");
    println!("    --test             Run file in test mode");
    println!("    --execution-timeout <seconds>");
    println!("                       Set this invocation's finite execution budget (1–31536000)");
    println!("                       Place before the source filename; other limits are unchanged");
    println!();
    println!("Configuration Maintenance:");
    println!("    --configCheck      Check configuration files for issues");
    println!("    --configFix        Check and fix configuration files");
    println!("    config            Set up global WFL configuration interactively");
    println!("    init              Create project configuration and agent guidance");
    println!();
    println!("ENVIRONMENT VARIABLES:");
    println!("    WFL_GLOBAL_CONFIG_PATH  Override the global configuration path");
    println!();
    println!("NOTES:");
    println!("    All runs are now type‑checked and semantically analyzed by default.");
    println!("    This ensures that scripts are validated for semantic correctness");
    println!("    and type safety before execution, preventing many common runtime errors.");
    println!();
    println!("If no file is specified, the REPL will be started.");
}

/// Initialize the current project without prompts or replacement of existing files.
/// `args` contains only the arguments after `init`; a sole help flag returns
/// without writing, while invalid arguments or initialization failures exit 2.
fn run_init_command(args: &[String]) -> io::Result<()> {
    if args.len() == 1 && matches!(args[0].as_str(), "--help" | "-h") {
        println!("USAGE: wfl init");
        println!("Create .wflcfg, AGENTS.md, and CLAUDE.md in the current directory.");
        println!("Existing regular files are preserved; missing files are created.");
        println!("No prompts, downloads, or changes to global configuration.");
        return Ok(());
    }

    if !args.is_empty() {
        eprintln!("Error: wfl init does not accept arguments.");
        eprintln!("Usage: wfl init");
        process::exit(2);
    }

    match env::current_dir().and_then(|directory| project_init::initialize(&directory)) {
        Ok(report) => {
            for name in report.created {
                println!("Created {name}");
            }
            for name in report.skipped {
                println!("Skipped {name} (existing file preserved)");
            }
            println!("Read CLAUDE.md for WFL syntax, tooling, testing, and documentation.");
            Ok(())
        }
        Err(error) => {
            eprintln!("Error initializing project: {error}");
            eprintln!("Resolve the error and rerun wfl init to create any missing files.");
            process::exit(2);
        }
    }
}

/// Validate global setup arguments and require confirmation before replacement.
fn run_config_command(args: &[String]) -> io::Result<()> {
    if args.len() == 1 && matches!(args[0].as_str(), "--help" | "-h") {
        println!("USAGE: wfl config");
        println!("Set up global WFL configuration interactively.");
        println!(
            "Configuration file: {}",
            wfl_config::ConfigChecker::get_global_config_path().display()
        );
        println!("WFL_GLOBAL_CONFIG_PATH overrides the global configuration path.");
        println!("Press Enter to accept defaults or skip optional settings without defaults.");
        return Ok(());
    }

    if !args.is_empty() {
        eprintln!("Error: wfl config does not accept arguments.");
        eprintln!("Usage: wfl config");
        process::exit(2);
    }

    let config_path = wfl_config::ConfigChecker::get_global_config_path();
    println!("Configuration file: {}", config_path.display());
    if config_path.exists() {
        eprint!(
            "File {} already exists. Overwrite? (y/n): ",
            config_path.display()
        );
        io::stderr().flush()?;
        let mut response = String::new();
        io::stdin().read_line(&mut response)?;
        if !response.trim().to_lowercase().starts_with('y') {
            println!("Aborted.");
            return Ok(());
        }
    }

    match wfl_config::run_wizard(&config_path) {
        Ok(()) => {
            println!("\n✅ Configuration created: {}", config_path.display());
            println!("You can edit this file directly or run 'wfl config' again.");
            Ok(())
        }
        Err(error) => {
            eprintln!("Error: {error}");
            process::exit(2);
        }
    }
}

/// Build the asynchronous runtime used by CLI operations and the interpreter.
fn build_runtime() -> io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
}

/// Keep informational commands lightweight and reserve interpreter stack otherwise.
fn main() -> io::Result<()> {
    // Trivial, non-interpreting invocations (`--help`, `--version`) never
    // recurse, so run them on the ordinary stack — don't make printing help
    // depend on reserving a large stack (which can fail under a tight
    // address-space limit or on a 32-bit target).
    let arg1 = std::env::args().nth(1);
    let trivial = matches!(
        arg1.as_deref(),
        Some("--help" | "-h" | "--version" | "-v" | "-V")
    );
    if trivial {
        return build_runtime()?.block_on(run());
    }

    // Multiple rustls crypto providers are linked in (aws-lc-rs via reqwest,
    // ring via sqlx); install one process-level default before any interpreter
    // path that may build a TLS config, so an ambient-default lookup can't
    // panic. Placed after the trivial check so `--help`/`--version` (which never
    // touch TLS) stay lightweight. Shared helper so every binary/embedder matches.
    wfl::init_rustls_crypto_provider();

    // Otherwise run on a dedicated large-stack thread (the shared
    // `wfl::run_with_interpreter_stack` helper, also intended for library
    // embedders) so the shared budget's `max_call_depth` turns runaway recursion
    // into a clean, catchable error instead of a native stack overflow. If that
    // reservation fails (tight RLIMIT_AS / 32-bit), fall back to the default
    // stack rather than refusing to start — shallow programs and non-interpreting
    // commands still work.
    match wfl::run_with_interpreter_stack(|| build_runtime()?.block_on(run())) {
        Ok(result) => result,
        Err(e) => {
            eprintln!(
                "warning: could not reserve a large interpreter stack ({e}); \
                 using the default stack (deep recursion may hit the OS limit \
                 before max_call_depth)"
            );
            build_runtime()?.block_on(run())
        }
    }
}

/// Read a WFL source file under the shared source-size ceiling. Reads at most
/// `max_source_size + 1` bytes so an oversized file is refused (exit code 2)
/// without ever allocating the whole thing — even when the file's metadata is
/// unavailable, stale, or reports `0` (special files).
fn read_source_bounded(
    path: &str,
    budget: &wfl::exec::budget::ExecutionBudget,
) -> io::Result<String> {
    use std::io::Read;
    let max = budget.max_source_bytes();
    let read_cap = (max as u64).saturating_add(1);
    let file = fs::File::open(path)?;
    let mut buf = Vec::new();
    file.take(read_cap).read_to_end(&mut buf)?;
    if let Err(exceeded) = budget.check_source_bytes(buf.len()) {
        eprintln!("Error: {}", exceeded.message());
        process::exit(2);
    }
    String::from_utf8(buf).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("source file '{path}' is not valid UTF-8"),
        )
    })
}

/// Parse operation flags, validate their combination, and dispatch the requested work.
async fn run() -> io::Result<()> {
    // Initialize dhat profiler if enabled
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    #[cfg(feature = "dhat-ad-hoc")]
    let _profiler = dhat::Profiler::new_ad_hoc();

    let args: Vec<String> = env::args().collect();

    if args.len() == 1 {
        if let Err(e) = repl::run_repl().await {
            eprintln!("REPL error: {e}");
        }
        return Ok(());
    }

    // Accept the same spellings `main()` treats as trivial invocations, so `-h`
    // is never mistaken for an input file path.
    if args.len() >= 2 && matches!(args[1].as_str(), "--help" | "-h") {
        print_help();
        return Ok(());
    }

    // System setup works from any directory, including one containing a file
    // named config. Explicit program paths such as ./config remain runnable.
    if args[1] == "config" {
        return run_config_command(&args[2..]);
    }
    if args[1] == "init" {
        return run_init_command(&args[2..]);
    }

    // Check for version flag only in WFL flags (before script filename)
    // This check is moved into the main argument parsing loop below

    let mut lint_mode = false;
    let mut analyze_mode = false;
    let mut fix_mode = false;
    let mut fix_in_place = false;
    let mut fix_diff = false;
    let mut config_check_mode = false;
    let mut config_fix_mode = false;
    let mut step_mode = false;
    let mut edit_mode = false;
    let mut lex_dump = false;
    let mut ast_dump = false;
    let mut dump_env_mode = false;
    let mut output_path = None;
    let mut time_mode = false;
    let mut test_mode = false;
    let mut execution_timeout = None;
    let mut file_path = String::new();

    let mut i = 1;
    while i < args.len() {
        // Single-dash names were accepted after --lint/--fix before those flags
        // became order-independent. Every lint/fix option establishes a source
        // position where -v/-V remain filenames; standalone aliases and
        // --version still print the version.
        let lint_options = lint_mode || fix_mode || fix_diff || fix_in_place;
        let lint_source_position = lint_options
            && (file_path.is_empty()
                || matches!(
                    args[i - 1].as_str(),
                    "--lint" | "--fix" | "--diff" | "--in-place"
                ));
        match args[i].as_str() {
            "--execution-timeout" => {
                if !file_path.is_empty() {
                    eprintln!("Error: --execution-timeout must appear before the source filename");
                    process::exit(2);
                }
                if execution_timeout.is_some() {
                    eprintln!("Error: --execution-timeout may be specified only once");
                    process::exit(2);
                }
                let seconds = args
                    .get(i + 1)
                    .filter(|value| !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()))
                    .and_then(|value| value.parse::<u64>().ok())
                    .filter(|seconds| (1..=31_536_000).contains(seconds));
                let Some(seconds) = seconds else {
                    eprintln!(
                        "Error: --execution-timeout requires a whole number of seconds from 1 to 31536000"
                    );
                    process::exit(2);
                };
                execution_timeout = Some(std::time::Duration::from_secs(seconds));
                i += 2;
            }
            "--init" => {
                eprintln!("Error: initialization is a command. Use: wfl init");
                process::exit(2);
            }
            "--dump-env" => {
                dump_env_mode = true;
                i += 1;
                if i < args.len() && args[i] == "--output" {
                    // This is handled in the next iteration or inner logic if we want to support ordered args
                    // But current loop handles it fine if we just continue
                }
            }
            "--output" => {
                if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                    output_path = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --output requires a file path");
                    process::exit(2);
                }
            }
            "--lex" => {
                lex_dump = true;
                i += 1;
            }
            "--ast" | "--parse" => {
                ast_dump = true;
                i += 1;
            }
            "--configCheck" => {
                if lint_mode || analyze_mode || fix_mode || config_fix_mode {
                    eprintln!(
                        "Error: --configCheck cannot be combined with --lint, --analyze, --fix, or --configFix"
                    );
                    process::exit(2);
                }
                config_check_mode = true;
                i += 1;
                if i < args.len() && !args[i].starts_with("--") {
                    file_path = args[i].clone();
                    i += 1;
                }
            }
            "--configFix" => {
                if lint_mode || analyze_mode || fix_mode || config_check_mode {
                    eprintln!(
                        "Error: --configFix cannot be combined with --lint, --analyze, --fix, or --configCheck"
                    );
                    process::exit(2);
                }
                config_fix_mode = true;
                i += 1;
                if i < args.len() && !args[i].starts_with("--") {
                    file_path = args[i].clone();
                    i += 1;
                }
            }
            "--lint" => {
                if analyze_mode || config_check_mode || config_fix_mode {
                    eprintln!(
                        "Error: --lint cannot be combined with --analyze, --configCheck, or --configFix"
                    );
                    process::exit(2);
                }
                lint_mode = true;
                i += 1;
            }
            "--analyze" => {
                if lint_mode || analyze_mode || fix_mode || config_check_mode || config_fix_mode {
                    eprintln!(
                        "Error: --analyze cannot be combined with --lint, --fix, --configCheck, or --configFix"
                    );
                    process::exit(2);
                }
                analyze_mode = true;
                i += 1;
                if i < args.len() && !args[i].starts_with("--") {
                    file_path = args[i].clone();
                    i += 1;
                } else {
                    eprintln!("Error: --analyze requires a file path");
                    process::exit(2);
                }
            }
            "--edit" => {
                if lint_mode || analyze_mode || fix_mode || config_check_mode || config_fix_mode {
                    eprintln!("Error: --edit cannot be combined with other operation flags");
                    process::exit(2);
                }
                edit_mode = true;
                i += 1;
                if i < args.len() && !args[i].starts_with("--") {
                    file_path = args[i].clone();
                    i += 1;
                } else {
                    eprintln!("Error: --edit requires a file path");
                    process::exit(2);
                }
            }
            "--fix" => {
                if analyze_mode {
                    eprintln!("Error: --fix and --analyze flags are mutually exclusive");
                    process::exit(2);
                }
                fix_mode = true;
                i += 1;
            }
            "--in-place" => {
                fix_in_place = true;
                i += 1;
            }
            "--diff" => {
                fix_diff = true;
                i += 1;
            }
            "--step" => {
                if lint_mode || analyze_mode || fix_mode || config_check_mode || config_fix_mode {
                    eprintln!(
                        "Error: --step cannot be combined with --lint, --analyze, --fix, --configCheck, or --configFix"
                    );
                    process::exit(2);
                }
                step_mode = true;
                i += 1;
            }
            "--time" => {
                time_mode = true;
                i += 1;
            }
            // The WFL to JavaScript transpiler has been sunset. These flags are
            // kept only so their former users get a clear explanation instead of
            // the flag being mistaken for an input file path.
            "--transpile" | "--target" | "--no-runtime" | "--es-modules" => {
                eprintln!("Error: the WFL to JavaScript transpiler has been removed.");
                eprintln!(
                    "  '{}' was a transpiler-only option, so it is no longer supported.",
                    args[i]
                );
                eprintln!("  Removed together: --transpile, --target, --no-runtime, --es-modules.");
                eprintln!(
                    "  ('--output' still exists, but only for --dump-env; it no longer emits JavaScript.)"
                );
                eprintln!("  Run WFL programs directly with the interpreter: wfl <file.wfl>");
                process::exit(2);
            }
            "--test" => {
                if lint_mode || analyze_mode || fix_mode || config_check_mode || config_fix_mode {
                    eprintln!(
                        "Error: --test cannot be combined with --lint, --analyze, --fix, --configCheck, or --configFix"
                    );
                    process::exit(2);
                }
                test_mode = true;
                i += 1;
            }
            "--version" | "-v" | "-V" if args[i] == "--version" || !lint_source_position => {
                println!("WebFirst Language (WFL) version {}", wfl::version::VERSION);
                return Ok(());
            }
            _ => {
                if lint_options && args[i].starts_with("--") {
                    eprintln!("Error: Unknown option '{}'", args[i]);
                    process::exit(2);
                }
                if file_path.is_empty() {
                    file_path = args[i].clone();
                    i += 1;
                    // Lint options may occur on either side of the source path.
                    // Executable scripts still receive every subsequent argument
                    // verbatim, including strings that look like WFL options.
                    if !lint_options {
                        break;
                    }
                } else if lint_options
                    && args[i] == file_path
                    && matches!(args[i - 1].as_str(), "--lint" | "--fix")
                {
                    // Older releases required `--lint file --fix file`.
                    // Preserve that spelling when both paths are identical.
                    i += 1;
                } else if lint_options {
                    eprintln!("Error: --lint accepts only one file path");
                    process::exit(2);
                } else {
                    i += 1;
                }
            }
        }
    }

    // Validate the completed option set before running any operation or writing
    // output. Checking here makes conflicts independent of argument order.
    if execution_timeout.is_some()
        && (config_check_mode || config_fix_mode || edit_mode || dump_env_mode)
    {
        eprintln!(
            "Error: --execution-timeout requires source execution or analysis; it cannot be combined with --configCheck, --configFix, --edit, or --dump-env"
        );
        process::exit(2);
    }
    if fix_diff && fix_in_place {
        eprintln!("Error: --in-place and --diff flags are mutually exclusive");
        process::exit(2);
    }
    if (fix_diff || fix_in_place) && !fix_mode {
        eprintln!("Error: --diff or --in-place requires --fix");
        process::exit(2);
    }
    if fix_mode && !lint_mode {
        eprintln!("Error: --fix must be combined with --lint");
        process::exit(2);
    }
    if lint_mode
        && (analyze_mode
            || config_check_mode
            || config_fix_mode
            || step_mode
            || edit_mode
            || lex_dump
            || ast_dump
            || dump_env_mode
            || test_mode
            || output_path.is_some())
    {
        eprintln!("Error: --lint cannot be combined with other operation flags");
        process::exit(2);
    }

    // Handle environment dump
    if dump_env_mode {
        if let Err(e) = wfl::env_dump::dump_env(output_path.as_deref()) {
            eprintln!("Error dumping environment: {e}");
            process::exit(1);
        }
        return Ok(());
    }

    // Collect remaining arguments as script arguments
    let script_args: Vec<String> = if i < args.len() {
        args[i..].to_vec()
    } else {
        Vec::new()
    };

    if config_check_mode || config_fix_mode {
        let dir = if !file_path.is_empty() {
            if Path::new(&file_path).is_file() {
                Path::new(&file_path)
                    .parent()
                    .unwrap_or(Path::new("."))
                    .to_path_buf()
            } else {
                Path::new(&file_path).to_path_buf()
            }
        } else {
            std::env::current_dir()?
        };

        if config_check_mode {
            match wfl_config::check_config(&dir) {
                Ok((_, success)) => {
                    if success {
                        println!("\n✅ Configuration check passed!");
                        process::exit(0);
                    } else {
                        process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Error checking configuration: {e}");
                    process::exit(2);
                }
            }
        } else if config_fix_mode {
            match wfl_config::fix_config(&dir) {
                Ok((_, success)) => {
                    if success {
                        println!("\n✅ Configuration fixed successfully!");
                        process::exit(0);
                    } else {
                        println!("\n⚠️ Some issues could not be fixed automatically.");
                        process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Error fixing configuration: {e}");
                    process::exit(2);
                }
            }
        }
    }

    if file_path.is_empty() && !config_check_mode && !config_fix_mode {
        eprintln!("Error: No file path provided");
        process::exit(2);
    }

    // Handle edit mode - launch the default editor for the file
    if edit_mode {
        let path = Path::new(&file_path);

        // Ensure the file exists
        if !path.exists() {
            // Create an empty file if it doesn't exist
            println!("File doesn't exist. Creating empty file: {file_path}");
            fs::write(&file_path, "")?;
        }

        // Use the system's default program to open the file
        println!("Opening file in default editor: {file_path}");

        #[cfg(target_os = "windows")]
        {
            use std::process::Command;
            Command::new("cmd")
                .args(["/C", "start", "", &file_path])
                .spawn()?;
        }

        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            Command::new("open").arg(&file_path).spawn()?;
        }

        #[cfg(target_os = "linux")]
        {
            use std::process::Command;
            Command::new("xdg-open").arg(&file_path).spawn()?;
        }

        println!("Editor launched. Exiting WFL.");
        return Ok(());
    }

    let script_dir = Path::new(&file_path).parent().unwrap_or(Path::new("."));
    let config = config::load_config(script_dir);

    // Preserve the historic config timeout cap and its per-operation uses.
    // An explicit invocation override changes ONLY the shared budget duration,
    // not request/stream timeouts, main-loop subprocess waits, or other limits.
    // The one deadline starts before reading/lexing/parsing/analysis and is
    // reused by interpretation and nested executed files.
    let mut run_config = config.clone();
    run_config.timeout_seconds = run_config.timeout_seconds.min(300);
    let run_config = std::sync::Arc::new(run_config);
    let mut budget_limits = wfl::exec::budget::BudgetLimits::from_config(&run_config);
    if let Some(duration) = execution_timeout {
        budget_limits.max_duration = Some(duration);
    }
    let budget = std::sync::Arc::new(wfl::exec::budget::ExecutionBudget::new(budget_limits));

    // Install the run budget as the current-thread budget for the ENTIRE run, so
    // every front-end phase — lexing, parsing, analysis, type checking — and the
    // dump/`--analyze` modes consult *one* budget and honor its deadline and
    // cooperative cancellation, not just the interpreter. The parser and the
    // analyzer/type-checker read it via `ExecutionBudget::current()` at their
    // top-level checkpoints; the interpreter re-enters the same budget when it
    // runs. Held for the whole function (restored on drop).
    let _run_budget_guard =
        wfl::exec::budget::ExecutionBudget::enter(std::sync::Arc::clone(&budget));

    // Read the source under the shared source-size ceiling: read at most
    // `max_source_size + 1` bytes so an oversized file is refused without ever
    // allocating the whole thing (this holds even if metadata is unavailable).
    let input = match read_source_bounded(&file_path, &budget) {
        Ok(input) => input,
        Err(error) if lint_mode => {
            eprintln!("Error reading '{file_path}': {error}");
            process::exit(2);
        }
        Err(error) => return Err(error),
    };

    // Lex under the shared run budget (installed above): a deadline /
    // cancellation / operation-ceiling breach during tokenization surfaces as a
    // fatal error and exits, instead of returning a silently truncated token
    // stream that a later phase could parse, analyze, or execute as if it were
    // the whole program. Used by every source-lexing mode below.
    let lex_checked = |source: &str| match lex_wfl_with_positions_checked(source) {
        Ok(tokens) => tokens,
        Err(exceeded) => {
            eprintln!("Error: {}", exceeded.message());
            process::exit(2);
        }
    };

    // Handle lexer and AST dump flags
    if lex_dump || ast_dump {
        let tokens_with_pos = lex_checked(&input);

        // Function to write data to a file with appropriate error handling
        fn write_to_file(path: &str, content: &str) -> io::Result<()> {
            let mut file = fs::File::create(path)?;
            file.write_all(content.as_bytes())?;
            Ok(())
        }

        // Dumps are ephemeral reports, so they go under target/reports/dumps/
        // rather than beside the source file (REPOSITORY_HYGIENE.md §5 —
        // writing next to the source litters the checkout, and dumps embed
        // absolute paths that must never end up tracked).
        fn dump_output_path(file_path: &str, suffix: &str) -> String {
            let dump_dir = std::path::Path::new("target/reports/dumps");
            let _ = fs::create_dir_all(dump_dir);
            let flat_name = file_path
                .trim_start_matches("./")
                .replace(['/', '\\', ':'], "_");
            dump_dir
                .join(format!("{flat_name}{suffix}"))
                .to_string_lossy()
                .into_owned()
        }

        // Handle lexer dump
        if lex_dump {
            let lex_output_path = dump_output_path(&file_path, ".lex.txt");

            // Format lexer output
            let mut lex_output = String::new();
            lex_output.push_str(&format!("Lexer output for: {file_path}\n"));
            lex_output.push_str("==============================================\n\n");

            for (i, token) in tokens_with_pos.iter().enumerate() {
                lex_output.push_str(&format!(
                    "{:4}: {:?} at line {}, column {} (length: {})\n",
                    i, token.token, token.line, token.column, token.length
                ));
            }

            // Write to file
            if let Err(e) = write_to_file(&lex_output_path, &lex_output) {
                eprintln!("Error writing lexer output to {lex_output_path}: {e}");
                process::exit(1);
            }

            println!("Lexer output written to: {lex_output_path}");
        }

        // Handle AST dump
        if ast_dump {
            let ast_output_path = dump_output_path(&file_path, ".ast.txt");

            // Parse tokens into AST
            match Parser::new(&tokens_with_pos).parse() {
                Ok(program) => {
                    // Format AST output
                    let mut ast_output = String::new();
                    ast_output.push_str(&format!("AST output for: {file_path}\n"));
                    ast_output.push_str("==============================================\n\n");
                    ast_output.push_str(&format!(
                        "Program with {} statements:\n\n",
                        program.statements.len()
                    ));

                    // Format each statement
                    for (i, stmt) in program.statements.iter().enumerate() {
                        ast_output.push_str(&format!("Statement #{}: {:#?}\n\n", i + 1, stmt));
                    }

                    // Write to file
                    if let Err(e) = write_to_file(&ast_output_path, &ast_output) {
                        eprintln!("Error writing AST output to {ast_output_path}: {e}");
                        process::exit(1);
                    }

                    println!("AST output written to: {ast_output_path}");
                }
                Err(errors) => {
                    eprintln!("Cannot generate AST dump due to parse errors:");

                    let mut reporter = DiagnosticReporter::new();
                    let file_id = reporter.add_file(&file_path, &input);

                    for error in errors {
                        let diagnostic = reporter.convert_parse_error(file_id, &error);
                        if let Err(e) = reporter.report_diagnostic(file_id, &diagnostic) {
                            eprintln!("Error displaying diagnostic: {e}");
                            eprintln!("Error: {error}");
                        }
                    }

                    process::exit(2);
                }
            }
        }

        // Exit after dump operations are complete
        process::exit(0);
    }

    if step_mode {
        println!("Boot phase: Configuration loaded");

        print!("continue (y/n)? ");
        if let Err(e) = io::stdout().flush() {
            eprintln!("Error flushing stdout: {e}");
        }

        let mut input_line = String::new();
        match io::stdin().read_line(&mut input_line) {
            Ok(_) => {
                let input_line = input_line.trim().to_lowercase();
                if input_line != "y" {
                    process::exit(0);
                }
            }
            Err(e) => {
                eprintln!("Error reading input: {e}");
                process::exit(1);
            }
        }
    }

    if lint_mode {
        if let Err(error) = validate_source(&input) {
            eprintln!("Error: {error}");
            process::exit(2);
        }
        let tokens_with_pos = lex_checked(&input);
        match Parser::new(&tokens_with_pos).parse() {
            Ok(program) => {
                let mut linter = Linter::new();
                linter.load_config(script_dir);

                let (diagnostics, _success) = linter.lint(&program, &input, &file_path);

                if fix_mode {
                    let mut fixer = CodeFixer::new();
                    fixer.set_indent_size(config.indent_size);
                    fixer.load_config(script_dir);

                    let (fixed_code, summary) = match fixer.fix_checked(&program, &input) {
                        Ok(result) => result,
                        Err(error) => {
                            eprintln!("Error fixing code: {error}");
                            process::exit(2);
                        }
                    };

                    if fix_in_place {
                        if let Err(error) =
                            write_fixed_file(Path::new(&file_path), &input, &fixed_code)
                        {
                            eprintln!("Error writing '{file_path}': {error}");
                            process::exit(2);
                        }
                        println!("✔ Auto-fixed {} issues in place.", summary.total());
                    } else if fix_diff {
                        io::stdout().write_all(
                            fixer
                                .diff_for_path(Path::new(&file_path), &input, &fixed_code)
                                .as_bytes(),
                        )?;
                    } else {
                        io::stdout().write_all(fixed_code.as_bytes())?;
                    }
                    process::exit(0);
                } else if !diagnostics.is_empty() {
                    eprintln!("Lint warnings:");

                    let mut reporter = DiagnosticReporter::new();
                    let file_id = reporter.add_file(&file_path, &input);

                    for diagnostic in diagnostics {
                        if let Err(e) = reporter.report_diagnostic(file_id, &diagnostic) {
                            eprintln!("Error displaying diagnostic: {e}");
                            eprintln!("{}", diagnostic.message);
                        }
                    }

                    process::exit(1);
                } else {
                    println!("No lint warnings found.");
                    process::exit(0);
                }
            }
            Err(errors) => {
                eprintln!("Parse errors:");

                let mut reporter = DiagnosticReporter::new();
                let file_id = reporter.add_file(&file_path, &input);

                for error in errors {
                    let diagnostic = reporter.convert_parse_error(file_id, &error);
                    if let Err(e) = reporter.report_diagnostic(file_id, &diagnostic) {
                        eprintln!("Error displaying diagnostic: {e}");
                        eprintln!("Error: {error}");
                    }
                }

                process::exit(2);
            }
        }
    } else if analyze_mode {
        let tokens_with_pos = lex_checked(&input);
        match Parser::new(&tokens_with_pos).parse() {
            Ok(program) => {
                let mut analyzer = Analyzer::new();

                let mut reporter = DiagnosticReporter::new();
                let file_id = reporter.add_file(&file_path, &input);
                let diagnostics = analyzer.analyze_static(&program, file_id);

                if !diagnostics.is_empty() {
                    eprintln!("Static analysis warnings:");

                    let mut reporter = DiagnosticReporter::new();
                    let file_id = reporter.add_file(&file_path, &input);

                    for diagnostic in diagnostics {
                        if let Err(e) = reporter.report_diagnostic(file_id, &diagnostic) {
                            eprintln!("Error displaying diagnostic: {e}");
                            eprintln!("{}", diagnostic.message);
                        }
                    }

                    process::exit(1);
                } else {
                    println!("No static analysis warnings found.");
                    process::exit(0);
                }
            }
            Err(errors) => {
                eprintln!("Parse errors:");

                let mut reporter = DiagnosticReporter::new();
                let file_id = reporter.add_file(&file_path, &input);

                for error in errors {
                    let diagnostic = reporter.convert_parse_error(file_id, &error);
                    if let Err(e) = reporter.report_diagnostic(file_id, &diagnostic) {
                        eprintln!("Error displaying diagnostic: {e}");
                        eprintln!("Error: {error}");
                    }
                }

                process::exit(2);
            }
        }
    } else {
        let tokens_with_pos = lex_checked(&input);

        // Initialize both regular and execution logging first so debug output goes to log
        let log_path = script_dir.join("wfl.log");
        wfl::init_loggers(&log_path, script_dir);

        if config.logging_enabled {
            info!("WebFirst Language started with script: {}", &file_path);
        }

        // Use exec_trace for compilation debug output
        exec_trace!("Parsing and executing script...");
        let mut parser = Parser::new(&tokens_with_pos);
        match parser.parse() {
            Ok(program) => {
                exec_trace!("AST: [large output suppressed]");
                exec_trace!("Program has {} statements", program.statements.len());

                let mut analyzer = Analyzer::new();
                let mut reporter = DiagnosticReporter::new();
                let file_id = reporter.add_file(&file_path, &input);
                let sema_diags = analyzer.analyze_static(&program, file_id);
                let mut has_fatal_errors = false;
                if !sema_diags.is_empty() {
                    for d in &sema_diags {
                        reporter.report_diagnostic(file_id, d)?;
                        // Check if this is a fatal error that should prevent execution
                        if d.severity == Severity::Error {
                            has_fatal_errors = true;
                        }
                    }
                }

                // Exit if we found fatal semantic errors
                if has_fatal_errors {
                    exec_trace!("Semantic analysis found fatal errors. Execution aborted.");
                    process::exit(3);
                }

                exec_trace!("Semantic analysis passed.");

                // Create TypeChecker with the same analyzer to share action parameters
                let mut tc = TypeChecker::with_analyzer(analyzer);
                if let Err(failure) = tc.check_types(&program) {
                    match failure {
                        // A shared-budget breach during type checking is FATAL —
                        // the type diagnostics below are otherwise treated as
                        // non-fatal warnings, which would let an expired deadline
                        // / cancellation / resource breach slip into execution.
                        TypeCheckError::Budget(exceeded) => {
                            eprintln!("Error: {}", exceeded.message());
                            process::exit(2);
                        }
                        TypeCheckError::Types(errors) => {
                            // Filter out errors for action parameters
                            let action_params = tc.get_action_parameters();
                            let filtered_errors: Vec<_> = errors
                                .into_iter()
                                .filter(|e| {
                                    // Check if this is an undefined variable error for an action parameter
                                    if e.message.starts_with("Variable '")
                                        && e.message.ends_with("' is not defined")
                                    {
                                        let var_name = e
                                            .message
                                            .trim_start_matches("Variable '")
                                            .trim_end_matches("' is not defined");

                                        // Skip this error if the variable is an action parameter
                                        if action_params.contains(var_name) {
                                            return false;
                                        }
                                    }

                                    // Filter out "Symbol already defined" errors at line 0, column 0
                                    // These are likely from imported files or standard library definitions
                                    if e.message.starts_with("Symbol '")
                                        && e.message.contains("' is already defined in this scope")
                                        && e.line == 0
                                        && e.column == 0
                                    {
                                        return false;
                                    }

                                    true
                                })
                                .collect();

                            if !filtered_errors.is_empty() {
                                eprintln!("Type checking warnings:");

                                let mut reporter = DiagnosticReporter::new();
                                let file_id = reporter.add_file(&file_path, &input);

                                for error in &filtered_errors {
                                    let diagnostic = reporter.convert_type_error(file_id, error);
                                    if let Err(e) = reporter.report_diagnostic(file_id, &diagnostic)
                                    {
                                        eprintln!("Error displaying diagnostic: {e}");
                                        eprintln!("{error}"); // Fallback to simple error display
                                    }
                                }
                            }
                        }
                    }
                }
                exec_trace!("Type checking completed.");

                exec_trace!("Script directory: {:?}", script_dir);
                exec_trace!("Timeout seconds: {}", config.timeout_seconds);

                // Log execution start if execution logging is enabled
                exec_trace!("Starting execution of script: {}", &file_path);

                // Reuse the single budget (and the timeout-capped `run_config`)
                // built at the top of the run, so the source check and the
                // interpreter share one deadline/operation/cancellation budget.
                // `run_config` preserves config policy, while `budget` may carry
                // an explicit CLI execution duration. Main loops keep their
                // existing lifetime exemption and finite operation timeouts.
                let mut interpreter = Interpreter::with_config_and_budget(
                    std::sync::Arc::clone(&run_config),
                    std::sync::Arc::clone(&budget),
                );
                interpreter.set_step_mode(step_mode); // Set step mode from CLI flag
                interpreter.set_test_mode(test_mode); // Set test mode from CLI flag
                interpreter.set_script_args(script_args); // Pass script arguments
                interpreter.set_source_file(std::path::PathBuf::from(&file_path)); // Set source file for module resolution

                if step_mode {
                    println!("Boot phase: Configuration loaded");

                    println!("Program has 4 statements");

                    if !interpreter.prompt_continue() {
                        process::exit(0);
                    }
                }

                // Log program details if execution logging is enabled
                exec_trace!("Program contains {} statements", program.statements.len());

                // Start timing if requested
                let start_time = if time_mode {
                    Some(Instant::now())
                } else {
                    None
                };

                let interpret_result = interpreter.interpret(&program).await;

                // Calculate and display execution time if timing was requested
                if let Some(start) = start_time {
                    let elapsed = start.elapsed();

                    // Format the time appropriately
                    if elapsed.as_secs() > 0 {
                        println!("\nExecution time: {:.3}s", elapsed.as_secs_f64());
                    } else {
                        let millis = elapsed.as_millis();
                        if millis > 0 {
                            println!("\nExecution time: {millis}ms");
                        } else {
                            let micros = elapsed.as_micros();
                            println!("\nExecution time: {micros}µs");
                        }
                    }
                }

                match interpret_result {
                    Ok(_result) => {
                        if config.logging_enabled {
                            info!("Program executed successfully");
                        }
                        exec_trace!("Execution completed successfully. Result: {:?}", _result);

                        // Handle test mode results
                        if test_mode {
                            let results = interpreter.get_test_results();

                            println!("\n{}", "=".repeat(60));
                            println!("Test Results");
                            println!("{}", "=".repeat(60));
                            println!("Total:  {}", results.total_tests);
                            println!("Passed: {} ✓", results.passed_tests);
                            println!("Failed: {} ✗", results.failed_tests);

                            if !results.failures.is_empty() {
                                println!("\n{}", "─".repeat(60));
                                println!("Failures:");
                                println!("{}", "─".repeat(60));

                                for (i, failure) in results.failures.iter().enumerate() {
                                    println!("\n{}. {}", i + 1, failure.test_name);
                                    if !failure.describe_context.is_empty() {
                                        println!(
                                            "   Context: {}",
                                            failure.describe_context.join(" > ")
                                        );
                                    }
                                    println!("   {}", failure.assertion_message);
                                    println!("   at line {}", failure.line);
                                }
                            }

                            println!("\n{}", "=".repeat(60));

                            // Exit with error code if tests failed
                            if results.failed_tests > 0 {
                                drop(interpreter);
                                process::exit(1);
                            }
                        }
                        let program_exit_code = interpreter.program_exit_code();
                        if program_exit_code != 0 {
                            drop(interpreter);
                            process::exit(program_exit_code);
                        }
                    }
                    Err(errors) => {
                        if config.logging_enabled {
                            error!("Runtime errors occurred");
                        }

                        eprintln!("Runtime errors:");

                        let mut reporter = DiagnosticReporter::new();
                        let file_id = reporter.add_file(&file_path, &input);

                        if config.debug_report_enabled && !errors.is_empty() {
                            let error = &errors[0]; // Take the first error
                            let call_stack = interpreter.get_call_stack();
                            match debug_report::create_report(
                                error,
                                &call_stack,
                                &input,
                                &file_path,
                            ) {
                                Ok(report_path) => {
                                    let report_msg =
                                        format!("Debug report created: {}", report_path.display());
                                    eprintln!("{report_msg}");

                                    if config.logging_enabled {
                                        info!("{}", report_msg);
                                    }
                                }
                                Err(_) => {
                                    eprintln!("Could not create debug report");

                                    if config.logging_enabled {
                                        error!("Could not create debug report");
                                    }
                                }
                            }
                        }

                        for error in errors {
                            let diagnostic = reporter.convert_runtime_error(file_id, &error);
                            if let Err(e) = reporter.report_diagnostic(file_id, &diagnostic) {
                                eprintln!("Error displaying diagnostic: {e}");
                                eprintln!("{error}"); // Fallback to simple error display
                            }
                        }

                        // A program that died with a runtime error must not
                        // report success to the shell.
                        drop(interpreter);
                        process::exit(1);
                    }
                }
            }
            Err(errors) => {
                eprintln!("Parse errors:");

                let mut reporter = DiagnosticReporter::new();
                let file_id = reporter.add_file(&file_path, &input);

                for error in errors {
                    let diagnostic = reporter.convert_parse_error(file_id, &error);
                    if let Err(e) = reporter.report_diagnostic(file_id, &diagnostic) {
                        eprintln!("Error displaying diagnostic: {e}");
                        eprintln!("Error: {error}"); // Fallback to simple error display
                    }
                }

                process::exit(2);
            }
        }
    }

    Ok(())
}
