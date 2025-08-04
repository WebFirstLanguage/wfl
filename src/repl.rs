use crate::analyzer::Analyzer;
use crate::analyzer::static_analyzer::StaticAnalyzer;
use crate::config::WflConfig;
use crate::diagnostics::DiagnosticReporter;
use crate::interpreter::Interpreter;
use crate::lexer::{lex_wfl_with_positions, token::TokenWithPosition};
use crate::parser::{
    Parser,
    ast::{Program, Statement},
};
use crate::typechecker::TypeChecker;
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
        let config = WflConfig::default();
        let interpreter = Interpreter::with_timeout(config.timeout_seconds);

        ReplState {
            interpreter,
            input_buffer: String::new(),
            in_multiline: false,
            history: Vec::new(),
        }
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

        if !self.input_buffer.is_empty() {
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
        if let Err(errors) = type_checker.check_types(&program) {
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

                match repl_state.process_line(&line).await {
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
