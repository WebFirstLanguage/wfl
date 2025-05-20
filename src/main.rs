use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;
use wfl::Interpreter;
use wfl::analyzer::{Analyzer, StaticAnalyzer};
use wfl::config;
use wfl::debug_report;
use wfl::diagnostics::DiagnosticReporter;
use wfl::fixer::{CodeFixer, FixerOutputMode};
use wfl::lexer::lex_wfl_with_positions;
use wfl::linter::Linter;
use wfl::parser::Parser;
use wfl::repl;
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
    println!("    --version          Prints the version information");
    println!("    --lint             Run the linter on the specified file");
    println!("    --lint --fix       Apply auto-fixes after linting");
    println!("        --in-place     Overwrite the file in place");
    println!("        --diff         Show a diff instead of rewriting");
    println!("    --analyze          Run the static analyzer on the specified file");
    println!("    --step             Run in single-step execution mode");
    println!("    --edit             Open the specified file in the default editor");
    println!();
    println!("Configuration Maintenance:");
    println!("    --configCheck      Check configuration files for issues");
    println!("    --configFix        Check and fix configuration files");
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

#[tokio::main]
async fn main() -> io::Result<()> {
    // Initialize dhat profiler if enabled
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    #[cfg(feature = "dhat-ad-hoc")]
    let _profiler = dhat::Profiler::new_ad_hoc();

    let args: Vec<String> = env::args().collect();

    if args.len() == 1 {
        if let Err(e) = repl::run_repl().await {
            eprintln!("REPL error: {}", e);
        }
        return Ok(());
    }

    if args.len() >= 2 && args[1] == "--help" {
        print_help();
        return Ok(());
    }

    if args.len() >= 2 && args[1] == "--version" {
        println!("WebFirst Language (WFL) version {}", wfl::version::VERSION);
        return Ok(());
    }

    let mut lint_mode = false;
    let mut analyze_mode = false;
    let mut fix_mode = false;
    let mut fix_in_place = false;
    let mut fix_diff = false;
    let mut config_check_mode = false;
    let mut config_fix_mode = false;
    let mut step_mode = false;
    let mut edit_mode = false;
    let mut file_path = String::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
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
            _ => {
                if file_path.is_empty() {
                    file_path = args[i].clone();
                }
                i += 1;
            }
        }
    }

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
                    eprintln!("Error checking configuration: {}", e);
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
                    eprintln!("Error fixing configuration: {}", e);
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
            println!("File doesn't exist. Creating empty file: {}", file_path);
            fs::write(&file_path, "")?;
        }

        // Use the system's default program to open the file
        println!("Opening file in default editor: {}", file_path);

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

    if step_mode {
        println!("Boot phase: Configuration loaded");

        print!("continue (y/n)? ");
        if let Err(e) = io::stdout().flush() {
            eprintln!("Error flushing stdout: {}", e);
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
                eprintln!("Error reading input: {}", e);
                process::exit(1);
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
                        println!("Fixed code:\n{}", fixed_code);
                    }
                    process::exit(0);
                } else if !diagnostics.is_empty() {
                    eprintln!("Lint warnings:");

                    let mut reporter = DiagnosticReporter::new();
                    let file_id = reporter.add_file(&file_path, &input);

                    for diagnostic in diagnostics {
                        if let Err(e) = reporter.report_diagnostic(file_id, &diagnostic) {
                            eprintln!("Error displaying diagnostic: {}", e);
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
                        eprintln!("Error displaying diagnostic: {}", e);
                        eprintln!("Error: {}", error);
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
                            eprintln!("Error displaying diagnostic: {}", e);
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
                        eprintln!("Error displaying diagnostic: {}", e);
                        eprintln!("Error: {}", error);
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
                        eprintln!("Error fixing code: {}", e);
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
                        eprintln!("Error displaying diagnostic: {}", e);
                        eprintln!("Error: {}", error);
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
                if !sema_diags.is_empty() {
                    for d in &sema_diags {
                        reporter.report_diagnostic(file_id, d)?;
                    }
                    process::exit(2);
                }
                exec_trace!("Semantic analysis passed.");

                let mut tc = TypeChecker::new();
                if let Err(errors) = tc.check_types(&program) {
                    for e in &errors {
                        eprintln!("{e}");
                    }
                    process::exit(2);
                }
                exec_trace!("Type checking passed.");

                exec_trace!("Script directory: {:?}", script_dir);
                exec_trace!("Timeout seconds: {}", config.timeout_seconds);

                // Log execution start if execution logging is enabled
                exec_trace!("Starting execution of script: {}", &file_path);

                let mut interpreter = Interpreter::with_timeout(config.timeout_seconds);
                interpreter.set_step_mode(step_mode); // Set step mode from CLI flag

                if step_mode {
                    println!("Boot phase: Interpreter initialized");
                    if !interpreter.prompt_continue() {
                        process::exit(0);
                    }
                }

                // Log program details if execution logging is enabled
                exec_trace!("Program contains {} statements", program.statements.len());

                let interpret_result = interpreter.interpret(&program).await;
                match interpret_result {
                    Ok(_result) => {
                        if config.logging_enabled {
                            info!("Program executed successfully");
                        }
                        exec_trace!("Execution completed successfully. Result: {:?}", _result);
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
                                    eprintln!("{}", report_msg);

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
                                eprintln!("Error displaying diagnostic: {}", e);
                                eprintln!("{}", error); // Fallback to simple error display
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
                        eprintln!("Error displaying diagnostic: {}", e);
                        eprintln!("Error: {}", error); // Fallback to simple error display
                    }
                }
            }
        }
    }

    Ok(())
}
