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
use wfl::fixer::{CodeFixer, FixerOutputMode};
use wfl::lexer::lex_wfl_with_positions;
use wfl::linter::Linter;
use wfl::parser::Parser;
use wfl::repl;
use wfl::transpiler::{TranspilerConfig, TranspilerTarget};
use wfl::typechecker::TypeChecker;
use wfl::wfl_config;
use wfl::{error, exec_trace, info};

fn print_help() {
    println!("WebFirst Language (WFL) Compiler and Interpreter");
    println!();
    println!("USAGE:");
    println!("    wfl [FLAGS] [OPTIONS] [file]");
    println!();
    println!("FLAGS:");
    println!("    --help             Prints this help information");
    println!("    --version, -v      Prints the version information");
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
    println!();
    println!("TRANSPILATION:");
    println!("    --transpile        Transpile WFL code to JavaScript");
    println!("        --target <env> Target environment: node (default), browser, universal");
    println!("        --output <file> Output file (default: <input>.js)");
    println!("        --no-runtime   Don't include WFL runtime in output");
    println!("        --es-modules   Generate ES modules (export/import)");
    println!();
    println!("Configuration Maintenance:");
    println!("    --configCheck      Check configuration files for issues");
    println!("    --configFix        Check and fix configuration files");
    println!("    --init [dir]       Create .wflcfg interactively (default: current directory)");
    println!();
    println!("ENVIRONMENT VARIABLES:");
    println!("    WFL_GLOBAL_CONFIG_PATH  Override the global configuration path");
    println!();
    println!("NOTES:");
    println!("    All runs are now type‑checked and semantically analyzed by default.");
    println!("    This ensures that scripts are validated for semantic correctness");
    println!("    and type safety before execution, preventing many common runtime errors.");
    println!();
    println!("PACKAGE MANAGEMENT:");
    println!("    create [project] [called <name>]  Create a new WFL project");
    println!("    add <package> [constraint]         Add a dependency");
    println!("    remove <package>                   Remove a dependency");
    println!("    update [package]                   Update dependencies");
    println!("    build                              Build the project");
    println!("    run                                Run the project entry point");
    println!("    share                              Publish to the registry");
    println!("    search <query>                     Search for packages");
    println!("    info <package>                     Show package details");
    println!("    login / logout                     Registry authentication");
    println!("    check security                     Audit for vulnerabilities");
    println!("    check compatibility                Check API compatibility");
    println!();
    println!("If no file is specified, the REPL will be started.");
}

/// Parse "create project called <name>" args for the package manager.
fn parse_create_project_args(args: &[String]) -> Option<String> {
    // Skip "project" keyword if present
    let args = if !args.is_empty() && args[0] == "project" {
        &args[1..]
    } else {
        args
    };

    // Look for "called <name>"
    if args.len() >= 2 && args[0] == "called" {
        Some(args[1].clone())
    } else if args.len() == 1 && args[0] != "called" {
        Some(args[0].clone())
    } else {
        None
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // Initialize dhat profiler if enabled
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    #[cfg(feature = "dhat-ad-hoc")]
    let _profiler = dhat::Profiler::new_ad_hoc();

    let mut args: Vec<String> = env::args().collect();

    if args.len() == 1 {
        if let Err(e) = repl::run_repl().await {
            eprintln!("REPL error: {e}");
        }
        return Ok(());
    }

    if args.len() >= 2 && args[1] == "--help" {
        print_help();
        return Ok(());
    }

    // Package manager subcommands (positional, not flags)
    if args.len() >= 2 && !args[1].starts_with('-') {
        let subcommand = args[1].as_str();
        let sub_args: Vec<String> = args[2..].to_vec();
        let cwd = std::env::current_dir()?;

        match subcommand {
            "create" => {
                // Parse "create project called <name>" or "create project"
                let name = parse_create_project_args(&sub_args);
                match wflpkg::commands::create::create_project(name.as_deref(), &cwd) {
                    Ok(_) => return Ok(()),
                    Err(e) => {
                        eprintln!("{}", e);
                        process::exit(1);
                    }
                }
            }
            "add" => match wflpkg::commands::add::add_dependency(&sub_args, &cwd) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    eprintln!("{}", e);
                    process::exit(1);
                }
            },
            "remove" => {
                if sub_args.is_empty() {
                    eprintln!("Usage: wfl remove <package-name>");
                    process::exit(2);
                }
                match wflpkg::commands::remove::remove_dependency(&sub_args[0], &cwd) {
                    Ok(()) => return Ok(()),
                    Err(e) => {
                        eprintln!("{}", e);
                        process::exit(1);
                    }
                }
            }
            "update" => {
                let pkg = sub_args.first().map(|s| s.as_str());
                match wflpkg::commands::update::update_dependencies(pkg, &cwd) {
                    Ok(()) => return Ok(()),
                    Err(e) => {
                        eprintln!("{}", e);
                        process::exit(1);
                    }
                }
            }
            "build" => match wflpkg::commands::build::build_project(&cwd).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    eprintln!("{}", e);
                    process::exit(1);
                }
            },
            "run" => {
                if sub_args.iter().any(|a| a.ends_with(".wfl")) {
                    // "wfl run file.wfl" — strip "run" so normal file execution handles it
                    args.remove(1);
                } else {
                    // "wfl run" (no file arg) — package run command
                    match wflpkg::commands::run::run_project(&cwd).await {
                        Ok(()) => return Ok(()),
                        Err(e) => {
                            eprintln!("{}", e);
                            process::exit(1);
                        }
                    }
                }
            }
            "share" => match wflpkg::commands::share::share_package(&cwd).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    eprintln!("{}", e);
                    process::exit(1);
                }
            },
            "search" => {
                if sub_args.is_empty() {
                    eprintln!("Usage: wfl search <query>");
                    process::exit(2);
                }
                match wflpkg::commands::search::search_packages(&sub_args[0], "wflhub.org").await {
                    Ok(()) => return Ok(()),
                    Err(e) => {
                        eprintln!("{}", e);
                        process::exit(1);
                    }
                }
            }
            "info" => {
                if sub_args.is_empty() {
                    eprintln!("Usage: wfl info <package-name>");
                    process::exit(2);
                }
                match wflpkg::commands::info::show_package_info(&sub_args[0], "wflhub.org").await {
                    Ok(()) => return Ok(()),
                    Err(e) => {
                        eprintln!("{}", e);
                        process::exit(1);
                    }
                }
            }
            "login" => match wflpkg::commands::login::login("wflhub.org") {
                Ok(()) => return Ok(()),
                Err(e) => {
                    eprintln!("{}", e);
                    process::exit(1);
                }
            },
            "logout" => match wflpkg::commands::login::logout() {
                Ok(()) => return Ok(()),
                Err(e) => {
                    eprintln!("{}", e);
                    process::exit(1);
                }
            },
            "check" => {
                if sub_args.is_empty() {
                    eprintln!("Usage: wfl check <security|compatibility>");
                    process::exit(2);
                }
                match sub_args[0].as_str() {
                    "security" => match wflpkg::commands::check::check_security(&cwd).await {
                        Ok(()) => return Ok(()),
                        Err(e) => {
                            eprintln!("{}", e);
                            process::exit(1);
                        }
                    },
                    "compatibility" => {
                        match wflpkg::commands::check::check_compatibility(&cwd).await {
                            Ok(()) => return Ok(()),
                            Err(e) => {
                                eprintln!("{}", e);
                                process::exit(1);
                            }
                        }
                    }
                    other => {
                        eprintln!(
                            "Unknown check type: \"{}\"\n\nValid options:\n  wfl check security\n  wfl check compatibility",
                            other
                        );
                        process::exit(2);
                    }
                }
            }
            "test" => {
                if sub_args.iter().any(|a| a.ends_with(".wfl")) {
                    // "wfl test file.wfl" — strip "test" and inject "--test" for normal handling
                    args.remove(1);
                    args.insert(1, "--test".to_string());
                } else {
                    // "wfl test" without a .wfl file — package test command
                    // TODO: Run test files from project
                    println!("Package test mode not yet implemented.");
                    println!("Use 'wfl --test <file.wfl>' to run tests on a specific file.");
                    return Ok(());
                }
            }
            _ => {
                // Fall through to existing flag/file parsing
            }
        }
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
    let mut init_mode = false;
    let mut step_mode = false;
    let mut edit_mode = false;
    let mut lex_dump = false;
    let mut ast_dump = false;
    let mut dump_env_mode = false;
    let mut output_path = None;
    let mut time_mode = false;
    let mut transpile_mode = false;
    let mut transpile_target = TranspilerTarget::Node;
    let mut transpile_no_runtime = false;
    let mut transpile_es_modules = false;
    let mut test_mode = false;
    let mut file_path = String::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
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
            "--init" => {
                if lint_mode || analyze_mode || fix_mode || config_check_mode || config_fix_mode {
                    eprintln!("Error: --init cannot be combined with other operation flags");
                    process::exit(2);
                }
                init_mode = true;
                i += 1;
                // Optional: accept directory path
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
                if i < args.len() && !args[i].starts_with("--") {
                    file_path = args[i].clone();
                    i += 1;
                } else {
                    eprintln!("Error: --lint requires a file path");
                    process::exit(2);
                }
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
                if i < args.len() && !args[i].starts_with("--") {
                    file_path = args[i].clone();
                    i += 1;
                } else {
                    eprintln!("Error: --fix requires a file path");
                    process::exit(2);
                }

                while i < args.len() && args[i].starts_with("--") {
                    match args[i].as_str() {
                        "--in-place" => {
                            if fix_diff {
                                eprintln!(
                                    "Error: --in-place and --diff flags are mutually exclusive"
                                );
                                process::exit(2);
                            }
                            fix_in_place = true;
                            i += 1;
                        }
                        "--diff" => {
                            if fix_in_place {
                                eprintln!(
                                    "Error: --in-place and --diff flags are mutually exclusive"
                                );
                                process::exit(2);
                            }
                            fix_diff = true;
                            i += 1;
                        }
                        _ => {
                            break;
                        }
                    }
                }
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
            "--transpile" => {
                if lint_mode || analyze_mode || fix_mode || config_check_mode || config_fix_mode {
                    eprintln!(
                        "Error: --transpile cannot be combined with --lint, --analyze, --fix, --configCheck, or --configFix"
                    );
                    process::exit(2);
                }
                transpile_mode = true;
                i += 1;
                // Parse transpile options
                while i < args.len() && args[i].starts_with("--") {
                    match args[i].as_str() {
                        "--target" => {
                            if i + 1 < args.len() {
                                transpile_target = match args[i + 1].as_str() {
                                    "node" => TranspilerTarget::Node,
                                    "browser" => TranspilerTarget::Browser,
                                    "universal" => TranspilerTarget::Universal,
                                    _ => {
                                        eprintln!(
                                            "Error: Unknown transpile target '{}'. Use: node, browser, or universal",
                                            args[i + 1]
                                        );
                                        process::exit(2);
                                    }
                                };
                                i += 2;
                            } else {
                                eprintln!("Error: --target requires an argument");
                                process::exit(2);
                            }
                        }
                        "--no-runtime" => {
                            transpile_no_runtime = true;
                            i += 1;
                        }
                        "--es-modules" => {
                            transpile_es_modules = true;
                            i += 1;
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
                        _ => break,
                    }
                }
                // Get input file if not already set
                if i < args.len() && !args[i].starts_with("--") && file_path.is_empty() {
                    file_path = args[i].clone();
                    i += 1;
                }
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
            "--version" | "-v" => {
                println!("WebFirst Language (WFL) version {}", wfl::version::VERSION);
                return Ok(());
            }
            _ => {
                if file_path.is_empty() {
                    file_path = args[i].clone();
                    i += 1;
                    // All remaining arguments after the file path are script arguments
                    break;
                } else {
                    i += 1;
                }
            }
        }
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

    if fix_mode && !lint_mode {
        eprintln!("Error: --fix must be combined with --lint");
        process::exit(2);
    }

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

    // Handle --init mode
    if init_mode {
        use std::io::Write;

        let target_dir = if !file_path.is_empty() {
            std::path::Path::new(&file_path)
        } else {
            std::path::Path::new(".")
        };

        if !target_dir.is_dir() {
            eprintln!("Error: --init requires a valid directory");
            process::exit(2);
        }

        let config_path = target_dir.join(".wflcfg");

        // Check if file exists and prompt for overwrite
        if config_path.exists() {
            eprint!(
                "File {} already exists. Overwrite? (y/n): ",
                config_path.display()
            );
            std::io::stdout().flush()?;
            let mut response = String::new();
            std::io::stdin().read_line(&mut response)?;
            if !response.trim().to_lowercase().starts_with('y') {
                println!("Aborted.");
                process::exit(0);
            }
        }

        match wfl_config::run_wizard(&config_path) {
            Ok(()) => {
                println!("\n✅ Configuration created: {}", config_path.display());
                println!("You can edit this file directly or run 'wfl --init' again.");
                process::exit(0);
            }
            Err(e) => {
                eprintln!("Error: {e}");
                process::exit(2);
            }
        }
    }

    if file_path.is_empty() && !config_check_mode && !config_fix_mode && !init_mode {
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

    let input = fs::read_to_string(&file_path)?;
    let script_dir = Path::new(&file_path).parent().unwrap_or(Path::new("."));
    let config = config::load_config(script_dir);

    // Handle lexer and AST dump flags
    if lex_dump || ast_dump {
        let tokens_with_pos = lex_wfl_with_positions(&input);

        // Function to write data to a file with appropriate error handling
        fn write_to_file(path: &str, content: &str) -> io::Result<()> {
            let mut file = fs::File::create(path)?;
            file.write_all(content.as_bytes())?;
            Ok(())
        }

        // Handle lexer dump
        if lex_dump {
            let lex_output_path = format!("{file_path}.lex.txt");

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
            let ast_output_path = format!("{file_path}.ast.txt");

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

    // Handle transpile mode
    if transpile_mode {
        let tokens_with_pos = lex_wfl_with_positions(&input);
        match Parser::new(&tokens_with_pos).parse() {
            Ok(program) => {
                // Configure the transpiler
                let transpiler_config = TranspilerConfig {
                    include_runtime: !transpile_no_runtime,
                    source_maps: false,
                    target: transpile_target,
                    minify: false,
                    indent: "  ".to_string(),
                    es_modules: transpile_es_modules,
                };

                // Run the transpiler
                match wfl::transpiler::transpile(&program, &transpiler_config) {
                    Ok(result) => {
                        // Show warnings if any
                        for warning in &result.warnings {
                            eprintln!(
                                "Warning at line {}, column {}: {}",
                                warning.line, warning.column, warning.message
                            );
                        }

                        // Determine output path
                        let output_file = output_path.unwrap_or_else(|| {
                            let base = Path::new(&file_path);
                            let stem = base.file_stem().unwrap_or_default().to_string_lossy();
                            format!("{}.js", stem)
                        });

                        // Write output
                        if let Err(e) = fs::write(&output_file, &result.code) {
                            eprintln!("Error writing output file: {e}");
                            process::exit(1);
                        }

                        println!("Transpiled to: {output_file}");
                        if !result.warnings.is_empty() {
                            println!("  ({} warnings)", result.warnings.len());
                        }
                        process::exit(0);
                    }
                    Err(e) => {
                        eprintln!(
                            "Transpilation error at line {}, column {}: {}",
                            e.line, e.column, e.message
                        );
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
                        eprintln!("Error: {error}");
                    }
                }

                process::exit(2);
            }
        }
    }

    if lint_mode {
        let tokens_with_pos = lex_wfl_with_positions(&input);
        match Parser::new(&tokens_with_pos).parse() {
            Ok(program) => {
                let mut linter = Linter::new();
                linter.load_config(script_dir);

                let (diagnostics, _success) = linter.lint(&program, &input, &file_path);

                if fix_mode {
                    let mut fixer = CodeFixer::new();
                    fixer.set_indent_size(config.indent_size);
                    fixer.load_config(script_dir);

                    let (fixed_code, summary) = fixer.fix(&program, &input);

                    if fix_in_place {
                        fs::write(&file_path, &fixed_code)?;
                        println!("✔ Auto-fixed {} issues in place.", summary.total());
                    } else if fix_diff {
                        println!("{}", fixer.diff(&input, &fixed_code));
                    } else {
                        println!("Fixed code:\n{fixed_code}");
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
        let tokens_with_pos = lex_wfl_with_positions(&input);
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
    } else if fix_mode {
        let tokens_with_pos = lex_wfl_with_positions(&input);
        match Parser::new(&tokens_with_pos).parse() {
            Ok(_program) => {
                let mut fixer = CodeFixer::new();
                fixer.set_indent_size(config.indent_size);
                fixer.load_config(script_dir);

                let output_mode = if fix_in_place {
                    FixerOutputMode::InPlace
                } else if fix_diff {
                    FixerOutputMode::Diff
                } else {
                    FixerOutputMode::Stdout
                };

                match fixer.fix_file(Path::new(&file_path), output_mode) {
                    Ok(summary) => {
                        println!("Code fixing summary:");
                        println!("  Lines reformatted: {}", summary.lines_reformatted);
                        println!("  Variables renamed: {}", summary.vars_renamed);
                        println!("  Dead code removed: {}", summary.dead_code_removed);
                        process::exit(0);
                    }
                    Err(e) => {
                        eprintln!("Error fixing code: {e}");
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
                        eprintln!("Error: {error}");
                    }
                }

                process::exit(2);
            }
        }
    } else {
        let tokens_with_pos = lex_wfl_with_positions(&input);

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
                if let Err(errors) = tc.check_types(&program) {
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
                            if let Err(e) = reporter.report_diagnostic(file_id, &diagnostic) {
                                eprintln!("Error displaying diagnostic: {e}");
                                eprintln!("{error}"); // Fallback to simple error display
                            }
                        }
                    }
                }
                exec_trace!("Type checking completed.");

                exec_trace!("Script directory: {:?}", script_dir);
                exec_trace!("Timeout seconds: {}", config.timeout_seconds);

                // Log execution start if execution logging is enabled
                exec_trace!("Starting execution of script: {}", &file_path);

                let mut interpreter = Interpreter::with_timeout(config.timeout_seconds);
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
                                process::exit(1);
                            }
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
            }
        }
    }

    Ok(())
}
