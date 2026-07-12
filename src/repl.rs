use crate::analyzer::Analyzer;
use crate::analyzer::static_analyzer::StaticAnalyzer;
use crate::diagnostics::DiagnosticReporter;
use crate::exec::budget::ExecutionBudget;
use crate::interpreter::Interpreter;
use crate::lexer::{lex_wfl_with_positions, token::TokenWithPosition};
use crate::parser::{
    Parser,
    ast::{Program, Statement},
};
use crate::typechecker::{TypeCheckError, TypeChecker};
use codespan_reporting::term;
use codespan_reporting::term::termcolor::Buffer;
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result as RustylineResult};
use std::io::{self, Write};

#[derive(Debug, PartialEq)]
pub enum CommandResult {
    Help(String),
    History(String),
    ClearedScreen,
    Unknown(String),
}

pub struct ReplState {
    interpreter: Interpreter,
    input_buffer: String,
    in_multiline: bool,
    history: Vec<String>,
}

impl Default for ReplState {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplState {
    pub fn new() -> Self {
        // Honor the user's `.wflcfg` (e.g. a smaller `max_source_size` or
        // `timeout_seconds`) rather than hard-coding defaults. Fall back to the
        // defaults if the current directory can't be determined.
        let config = std::sync::Arc::new(
            std::env::current_dir()
                .map(|dir| crate::config::load_config_with_global(&dir))
                .unwrap_or_default(),
        );
        // One interpreter serves the whole session, but each command gets its
        // own fresh budget (see `reset_command_budget`), so the wall-clock
        // deadline is *per command* — a runaway command still times out, while a
        // long-idle session is never penalized on its next command. The initial
        // budget is replaced before the first command runs.
        let budget = std::sync::Arc::new(ExecutionBudget::from_config(&config));
        let interpreter =
            Interpreter::with_config_and_budget(std::sync::Arc::clone(&config), budget);

        ReplState {
            interpreter,
            input_buffer: String::new(),
            in_multiline: false,
            history: Vec::new(),
        }
    }

    /// Give the next command a fresh budget (new wall-clock deadline, cleared
    /// operation/cancellation counters) while preserving the session's
    /// environment. Returns a handle to the new budget so the caller can request
    /// cooperative cancellation (Ctrl-C) during execution.
    pub fn reset_command_budget(&mut self) -> std::sync::Arc<ExecutionBudget> {
        let budget = std::sync::Arc::new(ExecutionBudget::from_config(self.interpreter.config()));
        self.interpreter.set_budget(std::sync::Arc::clone(&budget));
        budget
    }

    pub async fn process_line(&mut self, line: &str) -> Result<Option<String>, String> {
        if line.trim().starts_with('.') {
            match self.handle_repl_command(line.trim())? {
                CommandResult::Help(text) => return Ok(Some(text)),
                CommandResult::History(text) => return Ok(Some(text)),
                CommandResult::ClearedScreen => return Ok(None),
                CommandResult::Unknown(text) => return Ok(Some(text)),
            }
        }

        // Enforce the source-size ceiling on the *prospective* buffer length —
        // computed with `checked_add` — BEFORE appending, so a single pasted line
        // above the cap is never copied into the buffer at all (nor re-cloned and
        // re-tokenized on every following line). Only mutate the buffer once the
        // new size is known to fit.
        let newline = usize::from(!self.input_buffer.is_empty());
        let prospective = self
            .input_buffer
            .len()
            .checked_add(newline)
            .and_then(|n| n.checked_add(line.len()));
        let within_cap = matches!(
            prospective.map(|len| self.interpreter.budget().check_source_bytes(len)),
            Some(Ok(())),
        );
        if !within_cap {
            self.input_buffer.clear();
            self.in_multiline = false;
            let max = self.interpreter.budget().max_source_bytes();
            return Err(format!(
                "Source too large: exceeds the configured limit ({max} bytes)"
            ));
        }

        if newline == 1 {
            self.input_buffer.push('\n');
        }
        self.input_buffer.push_str(line);

        let input = self.input_buffer.clone();
        let tokens = lex_wfl_with_positions(&input);

        if self.is_input_incomplete(&tokens) {
            self.in_multiline = true;
            return Ok(None); // Need more input
        }

        self.in_multiline = false;

        if !input.trim().is_empty() {
            self.history.push(input.clone());
        }

        let result = self.process_complete_input(&input).await;

        self.input_buffer.clear();

        result
    }

    fn handle_repl_command(&mut self, command: &str) -> Result<CommandResult, String> {
        match command {
            ".exit" => std::process::exit(0),
            ".help" => Ok(CommandResult::Help(
                "WFL REPL Commands:\n\
                 .exit    - Exit the REPL\n\
                 .help    - Show this help message\n\
                 .history - Show command history\n\
                 .clear   - Clear the screen\n"
                    .to_string(),
            )),
            ".history" => {
                let mut result = String::new();
                for (i, cmd) in self.history.iter().enumerate() {
                    result.push_str(&format!("{}: {}\n", i + 1, cmd));
                }
                Ok(CommandResult::History(result))
            }
            ".clear" => {
                print!("\x1B[2J\x1B[1;1H");
                if let Err(e) = io::stdout().flush() {
                    eprintln!("Flush failed: {e}");
                }
                Ok(CommandResult::ClearedScreen)
            }
            _ => Ok(CommandResult::Unknown(format!(
                "Unknown command: {command}"
            ))),
        }
    }

    fn is_input_incomplete(&self, tokens: &[TokenWithPosition]) -> bool {
        if tokens.is_empty() {
            return false;
        }

        let mut parser = Parser::new(tokens);
        match parser.parse() {
            Err(errors) => errors.iter().any(|e| {
                e.message.contains("Unexpected end of input")
                    || (e.message.contains("expected") && e.message.contains("end"))
            }),
            Ok(_) => false, // Successfully parsed, input is complete
        }
    }

    async fn process_complete_input(&mut self, input: &str) -> Result<Option<String>, String> {
        // Apply the same source-size ceiling the CLI uses, so pasting an
        // oversized blob into the REPL is refused before it is lexed/parsed.
        if let Err(exceeded) = self.interpreter.budget().check_source_bytes(input.len()) {
            return Err(exceeded.message());
        }

        let tokens = lex_wfl_with_positions(input);

        let mut parser = Parser::new(&tokens);
        let program = match parser.parse() {
            Ok(prog) => prog,
            Err(errors) => {
                let mut error_messages = Vec::new();
                let mut reporter = DiagnosticReporter::new();
                let file_id = reporter.add_file("repl", input);

                for error in &errors {
                    let diagnostic = reporter.convert_parse_error(file_id, error);

                    let mut buffer = Buffer::ansi();
                    let config = term::Config::default();
                    if let Err(_e) = term::emit(
                        &mut buffer,
                        &config,
                        &reporter.files,
                        &diagnostic.to_codespan_diagnostic(file_id),
                    ) {
                        error_messages.push(format!(
                            "Parse error at line {}, column {}: {}",
                            error.line, error.column, error.message
                        ));
                        continue;
                    }

                    let output = String::from_utf8_lossy(buffer.as_slice()).to_string();
                    error_messages.push(output);
                }

                return Ok(Some(error_messages.join("\n")));
            }
        };

        if program.statements.is_empty() {
            return Ok(None);
        }

        let mut analyzer = Analyzer::new();
        let mut reporter = DiagnosticReporter::new();
        let file_id = reporter.add_file("repl", input);
        let sema_diags = analyzer.analyze_static(&program, file_id);

        if !sema_diags.is_empty() {
            let mut error_messages = Vec::new();
            for diagnostic in &sema_diags {
                let mut buffer = Buffer::ansi();
                let config = term::Config::default();
                if let Err(_e) = term::emit(
                    &mut buffer,
                    &config,
                    &reporter.files,
                    &diagnostic.to_codespan_diagnostic(file_id),
                ) {
                    error_messages.push(format!("Semantic error: {}", diagnostic.message));
                    continue;
                }

                let output = String::from_utf8_lossy(buffer.as_slice()).to_string();
                error_messages.push(output);
            }

            return Ok(Some(error_messages.join("\n")));
        }

        let mut type_checker = TypeChecker::new();
        if let Err(failure) = type_checker.check_types(&program) {
            // A shared-budget breach (deadline/cancellation/resource) is fatal
            // for this command and must not be rendered as an ordinary type
            // diagnostic that the REPL might otherwise shrug off.
            let errors = match failure {
                TypeCheckError::Budget(exceeded) => {
                    return Ok(Some(format!("Error: {}", exceeded.message())));
                }
                TypeCheckError::Types(errors) => errors,
            };
            let mut error_messages = Vec::new();
            for error in &errors {
                let diagnostic = reporter.convert_type_error(file_id, error);

                let mut buffer = Buffer::ansi();
                let config = term::Config::default();
                if let Err(_e) = term::emit(
                    &mut buffer,
                    &config,
                    &reporter.files,
                    &diagnostic.to_codespan_diagnostic(file_id),
                ) {
                    error_messages.push(format!("Type error: {error}"));
                    continue;
                }

                let output = String::from_utf8_lossy(buffer.as_slice()).to_string();
                error_messages.push(output);
            }

            return Ok(Some(error_messages.join("\n")));
        }

        let mut result_output = None;

        if let Some(last_stmt) = program.statements.last() {
            match last_stmt {
                Statement::ExpressionStatement { .. } => {
                    let expr_program = Program {
                        statements: vec![last_stmt.clone()],
                    };

                    match self.interpreter.interpret(&expr_program).await {
                        Ok(value) => {
                            result_output = Some(format!("{value:?}"));
                        }
                        Err(errors) => {
                            let mut error_messages = Vec::new();
                            let mut reporter = DiagnosticReporter::new();
                            let file_id = reporter.add_file("repl", input);

                            for error in &errors {
                                let diagnostic = reporter.convert_runtime_error(file_id, error);

                                let mut buffer = Buffer::ansi();
                                let config = term::Config::default();
                                if let Err(_e) = term::emit(
                                    &mut buffer,
                                    &config,
                                    &reporter.files,
                                    &diagnostic.to_codespan_diagnostic(file_id),
                                ) {
                                    error_messages.push(format!("Runtime error: {error}"));
                                    continue;
                                }

                                let output = String::from_utf8_lossy(buffer.as_slice()).to_string();
                                error_messages.push(output);
                            }

                            result_output = Some(error_messages.join("\n"));
                        }
                    }
                }
                _ => match self.interpreter.interpret(&program).await {
                    Ok(_) => {}
                    Err(errors) => {
                        let mut error_messages = Vec::new();
                        let mut reporter = DiagnosticReporter::new();
                        let file_id = reporter.add_file("repl", input);

                        for error in &errors {
                            let diagnostic = reporter.convert_runtime_error(file_id, error);

                            let mut buffer = Buffer::ansi();
                            let config = term::Config::default();
                            if let Err(_e) = term::emit(
                                &mut buffer,
                                &config,
                                &reporter.files,
                                &diagnostic.to_codespan_diagnostic(file_id),
                            ) {
                                error_messages.push(format!("Runtime error: {error}"));
                                continue;
                            }

                            let output = String::from_utf8_lossy(buffer.as_slice()).to_string();
                            error_messages.push(output);
                        }

                        result_output = Some(error_messages.join("\n"));
                    }
                },
            }
        } else {
            match self.interpreter.interpret(&program).await {
                Ok(_) => {}
                Err(errors) => {
                    let mut error_messages = Vec::new();
                    let mut reporter = DiagnosticReporter::new();
                    let file_id = reporter.add_file("repl", input);

                    for error in &errors {
                        let diagnostic = reporter.convert_runtime_error(file_id, error);

                        let mut buffer = Buffer::ansi();
                        let config = term::Config::default();
                        if let Err(_e) = term::emit(
                            &mut buffer,
                            &config,
                            &reporter.files,
                            &diagnostic.to_codespan_diagnostic(file_id),
                        ) {
                            error_messages.push(format!("Runtime error: {error}"));
                            continue;
                        }

                        let output = String::from_utf8_lossy(buffer.as_slice()).to_string();
                        error_messages.push(output);
                    }

                    result_output = Some(error_messages.join("\n"));
                }
            }
        }

        Ok(result_output)
    }
}

pub async fn run_repl() -> RustylineResult<()> {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    #[cfg(feature = "dhat-ad-hoc")]
    let _profiler = dhat::Profiler::new_ad_hoc();

    let mut repl_state = ReplState::new();
    let mut rl = DefaultEditor::new()?;

    println!("WFL REPL - Type .help for commands or .exit to quit");

    loop {
        let prompt = if repl_state.in_multiline {
            "... "
        } else {
            "wfl> "
        };
        match rl.readline(prompt) {
            Ok(line) => {
                rl.add_history_entry(&line)?;

                // Fresh per-command budget (new deadline), and a handle so Ctrl-C
                // *during execution* cancels cooperatively. Ctrl-C while waiting
                // for input is still handled by rustyline (below).
                let budget = repl_state.reset_command_budget();
                // Install the command's budget as the current-thread budget for
                // the WHOLE pipeline — lexing, parsing, analysis, type checking,
                // and interpretation — not just interpretation. Otherwise the
                // front-end phases (which run before `interpret()` installs its
                // own guard) would see `ExecutionBudget::current() == None` and
                // skip every deadline/cancellation checkpoint.
                let _budget_guard = ExecutionBudget::enter(std::sync::Arc::clone(&budget));
                let outcome = {
                    let fut = repl_state.process_line(&line);
                    tokio::pin!(fut);
                    let mut cancelled = false;
                    loop {
                        tokio::select! {
                            r = &mut fut => break r,
                            _ = tokio::signal::ctrl_c(), if !cancelled => {
                                budget.cancel();
                                cancelled = true;
                                println!("^C — cancelling current command…");
                            }
                        }
                    }
                };

                match outcome {
                    Ok(Some(output)) => println!("{output}"),
                    Ok(None) => {} // No output needed
                    Err(error) => println!("Error: {error}"),
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {err:?}");
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clear_command() {
        let mut repl = ReplState::new();
        let result = repl.handle_repl_command(".clear");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CommandResult::ClearedScreen);
    }

    #[test]
    fn repl_resets_a_bounded_budget_per_command() {
        let mut repl = ReplState::new();
        let first = repl.reset_command_budget();
        // Each command runs under a wall-clock deadline (per-command), not the
        // old disabled-deadline session budget.
        assert!(
            first.limits().max_duration.is_some(),
            "per-command budget must carry a deadline"
        );
        let second = repl.reset_command_budget();
        assert!(
            !std::sync::Arc::ptr_eq(&first, &second),
            "each command must get a fresh budget instance"
        );
    }

    #[test]
    #[cfg(unix)]
    #[ignore] // This test manipulates stdout and should be run explicitly
    fn test_clear_command_with_closed_stdout() {
        use std::os::unix::io::{AsRawFd, FromRawFd};

        let mut repl = ReplState::new();

        let stdout_fd = std::io::stdout().as_raw_fd();
        let _stdout_dup = unsafe { std::fs::File::from_raw_fd(libc::dup(stdout_fd)) };
        assert!(unsafe { libc::close(stdout_fd) } == 0);

        let result = repl.handle_repl_command(".clear");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CommandResult::ClearedScreen);

        unsafe { libc::dup2(_stdout_dup.as_raw_fd(), stdout_fd) };
    }
}
