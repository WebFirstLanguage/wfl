use crate::analyzer::Analyzer;
use crate::analyzer::static_analyzer::StaticAnalyzer;
use crate::diagnostics::{DiagnosticReporter, Severity, Span, WflDiagnostic};
use crate::exec::budget::ExecutionBudget;
use crate::interpreter::Interpreter;
use crate::interpreter::value::Value;
use crate::lexer::{lex_wfl_with_positions_checked, token::TokenWithPosition};
use crate::parser::{Parser, ast::Statement};
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
        // Lex under the command's scoped budget: a Ctrl-C cancellation (or a
        // deadline breach) mid-paste surfaces as a typed error instead of a
        // truncated stream that multiline detection would misread as incomplete.
        let tokens = lex_wfl_with_positions_checked(&input).map_err(|e| e.message())?;

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
            Err(errors) => errors.iter().any(Self::error_means_more_input_needed),
            Ok(_) => false, // Successfully parsed, input is complete
        }
    }

    /// Decide whether a parse error means "keep reading — the user is
    /// mid-construct" rather than "this is genuinely wrong".
    ///
    /// The parser has no single "unexpected EOF" error type; each construct
    /// phrases running out of tokens in its own way ("Unexpected end of
    /// input …", "…, found end of input", and — for action bodies — a bare
    /// "Expected 'end' after action body" with no EOF marker at all). Matching
    /// on those strings case-sensitively is what made `check if …:` and
    /// `define action …:` blocks fail to continue onto the next line. Two
    /// case-insensitive signals cover every block form:
    ///   1. the parser explicitly ran out of tokens (`end of input`), or
    ///   2. it is still waiting for a block terminator (`expected 'end`) and
    ///      did **not** stop on a concrete token (no `found …`) — i.e. it hit
    ///      EOF, so more lines can still complete the block.
    ///
    /// A real mid-stream mistake reports `… found <Token>` and is therefore
    /// surfaced immediately instead of silently swallowed.
    fn error_means_more_input_needed(error: &crate::parser::ast::ParseError) -> bool {
        let message = error.message.to_lowercase();
        message.contains("end of input")
            || (message.contains("expected 'end") && !message.contains("found"))
    }

    /// Render one WFL diagnostic to a coloured, Elm-style block (source
    /// snippet, caret, and note), returning the string the REPL prints.
    ///
    /// Parse/type/runtime diagnostics already carry a labelled span, so they
    /// render with a caret as-is. Analyzer diagnostics only carry a
    /// line/column, so a one-character span is synthesized from it here — that
    /// way *every* REPL diagnostic follows the same "point at the source"
    /// convention (WFL Fundamental #4: clear, actionable errors) instead of
    /// some showing a bare message with no location.
    fn render_diagnostic(
        reporter: &mut DiagnosticReporter,
        file_id: usize,
        diagnostic: &WflDiagnostic,
    ) -> String {
        let mut diag = diagnostic.clone();
        if diag.labels.is_empty()
            && diag.line >= 1
            && let Some(offset) = reporter.line_col_to_offset(file_id, diag.line, diag.column)
        {
            // `line_col_to_offset` may return an offset one past the last byte
            // (a column at end-of-line / EOF). Clamp the one-character span into
            // the source, because codespan fails to emit on an end offset past
            // the file length — which would silently drop the caret.
            let source_len = reporter
                .files
                .get(file_id)
                .map(|file| file.source().len())
                .unwrap_or(0);
            let start = offset.min(source_len.saturating_sub(1));
            let end = (start + 1).min(source_len);
            if start < end {
                diag.labels.push((Span { start, end }, "here".to_string()));
            }
        }

        let mut buffer = Buffer::ansi();
        let config = term::Config::default();
        match term::emit_to_write_style(
            &mut buffer,
            &config,
            &reporter.files,
            &diag.to_codespan_diagnostic(file_id),
        ) {
            Ok(()) => String::from_utf8_lossy(buffer.as_slice()).to_string(),
            Err(_) => {
                let severity = match diag.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Note => "note",
                    Severity::Help => "help",
                };
                format!(
                    "{severity}: {} (at line {}, column {})",
                    diag.message, diag.line, diag.column
                )
            }
        }
    }

    async fn process_complete_input(&mut self, input: &str) -> Result<Option<String>, String> {
        // Apply the same source-size ceiling the CLI uses, so pasting an
        // oversized blob into the REPL is refused before it is lexed/parsed.
        if let Err(exceeded) = self.interpreter.budget().check_source_bytes(input.len()) {
            return Err(exceeded.message());
        }

        let tokens = lex_wfl_with_positions_checked(input).map_err(|e| e.message())?;

        let mut reporter = DiagnosticReporter::new();
        let file_id = reporter.add_file("repl", input);

        // --- Parse -----------------------------------------------------------
        // A syntax error is genuinely fatal for this line: there is no AST to
        // run. (Incomplete multiline input was already routed away by
        // `is_input_incomplete`, so anything that reaches here is meant to be
        // complete.)
        let mut parser = Parser::new(&tokens);
        let program = match parser.parse() {
            Ok(prog) => prog,
            Err(errors) => {
                let rendered: Vec<String> = errors
                    .iter()
                    .map(|error| {
                        let diagnostic = reporter.convert_parse_error(file_id, error);
                        Self::render_diagnostic(&mut reporter, file_id, &diagnostic)
                    })
                    .collect();
                return Ok(Some(rendered.join("\n")));
            }
        };

        if program.statements.is_empty() {
            return Ok(None);
        }

        // Messages accumulated across the analysis and run phases.
        let mut messages: Vec<String> = Vec::new();

        // --- Static analysis -------------------------------------------------
        // The REPL keeps ONE persistent interpreter for the whole session, but
        // the analyzer and type checker are whole-program tools that only ever
        // see this single line. Some of their diagnostics are false positives
        // against the live session; others are perfectly valid. So they are
        // filtered by diagnostic code, not blanket-dropped:
        //
        //   * `ANALYZE-SEMANTIC` — context-dependent name resolution (undefined
        //     name, "not an action", already-defined, undefined-inside-`try`).
        //     A name defined on an earlier line looks undefined here, so these
        //     are dropped: the interpreter, which owns the real environment,
        //     re-checks them at run time and reports genuine ones (this is what
        //     made `display x` after `store x as 5` wrongly fail).
        //   * `ANALYZE-UNUSED` — a stored binding is available to the next line,
        //     so "unused" is always a false positive interactively. Dropped.
        //   * everything else is *self-contained* within the single line and is
        //     kept exactly as `wfl <file>` treats it: advisory warnings
        //     (unreachable code, dead branch, shadowing, inconsistent returns)
        //     are shown, and any error — notably the `ANALYZE-SECURITY` lint for
        //     seeding the RNG in crypto/auth code — is shown AND blocks
        //     execution. Such lints depend only on this line's own code, never
        //     on session state, so there is no reason to weaken them.
        //
        // The type checker is skipped because its diagnostics are the same
        // context-dependent name/type kind that would be noise on every line
        // referring back to the session.
        let mut analyzer = Analyzer::new();
        let mut has_fatal_error = false;
        for diagnostic in &analyzer.analyze_static(&program, file_id) {
            if matches!(
                diagnostic.code.as_str(),
                "ANALYZE-SEMANTIC" | "ANALYZE-UNUSED"
            ) {
                continue;
            }
            messages.push(Self::render_diagnostic(&mut reporter, file_id, diagnostic));
            if diagnostic.severity == Severity::Error {
                has_fatal_error = true;
            }
        }
        // A self-contained fatal analysis error (e.g. a security-policy
        // violation) must stop the command before it runs, just like the CLI.
        if has_fatal_error {
            return Ok(Some(messages.join("\n")));
        }

        // --- Execution -------------------------------------------------------
        // Run the WHOLE program (so every statement's side effects happen), and
        // echo the final value only when the last statement is a bare
        // expression — `interpret` returns the value of the last executed
        // statement. `store`/`change`/`display` statements produce no echo.
        let echo_value = matches!(
            program.statements.last(),
            Some(Statement::ExpressionStatement { .. })
        );
        match self.interpreter.interpret(&program).await {
            Ok(value) => {
                // Suppress the echo for void results (`nothing`/null) so a call
                // to an action that only has side effects — e.g. `call greet
                // with "World"` — does not print a stray `null` under its output.
                //
                // Echo with `Display`, not `Debug`, so the value reads the way
                // WFL itself presents it (`yes`/`no` for booleans, unquoted
                // text, `[1, 2, 3]` for lists) — consistent with `display` and
                // WFL's natural-language principle, instead of Rust's `"..."`
                // / `true` debug spelling.
                if echo_value && !matches!(value, Value::Nothing | Value::Null) {
                    messages.push(format!("{value}"));
                }
            }
            Err(errors) => {
                for error in &errors {
                    let diagnostic = reporter.convert_runtime_error(file_id, error);
                    messages.push(Self::render_diagnostic(&mut reporter, file_id, &diagnostic));
                }
            }
        }

        if messages.is_empty() {
            Ok(None)
        } else {
            Ok(Some(messages.join("\n")))
        }
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
                let outcome = {
                    // Scope the command's budget as the TASK-local current budget
                    // for the WHOLE pipeline — lexing, parsing, analysis, type
                    // checking, and interpretation all run inside `process_line`,
                    // so each front-end phase sees `ExecutionBudget::current()`.
                    // Task-local (not a thread-local guard held across `.await`)
                    // so an interleaved task can never observe this command's
                    // budget or restore stale state; `budget` itself is retained
                    // so the Ctrl-C arm can still cancel it cooperatively.
                    let fut = ExecutionBudget::scope(
                        std::sync::Arc::clone(&budget),
                        repl_state.process_line(&line),
                    );
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

    /// Lex a snippet the way the REPL does before asking whether it is complete.
    fn tokens_of(src: &str) -> Vec<TokenWithPosition> {
        crate::lexer::lex_wfl_with_positions(src)
    }

    // --- The reported bug --------------------------------------------------
    // Storing a variable used to print an "unused variable" warning and, worse,
    // the warning made the REPL discard the whole command so the variable was
    // never stored. A bare `store` must now produce no output *and* persist.

    #[tokio::test]
    async fn store_persists_and_prints_nothing() {
        let mut repl = ReplState::new();
        let out = repl.process_line("store x as 5").await.unwrap();
        assert_eq!(out, None, "`store` must not print a warning or any echo");
        assert!(
            repl.interpreter.global_env().borrow().has("x"),
            "the variable must actually be stored in the session"
        );
    }

    #[tokio::test]
    async fn variable_defined_on_an_earlier_line_is_usable_later() {
        let mut repl = ReplState::new();
        repl.process_line("store x as 5").await.unwrap();
        // A bare reference echoes the value — proving the per-line analyzer no
        // longer reports the session variable as "not defined".
        let echo = repl.process_line("x").await.unwrap();
        assert_eq!(echo, Some("5".to_string()));
    }

    #[tokio::test]
    async fn action_defined_earlier_can_be_called_later_without_null_echo() {
        let mut repl = ReplState::new();
        for line in [
            "define action called greet with name:",
            "display \"Hello \" with name",
            "end action",
        ] {
            assert_eq!(repl.process_line(line).await.unwrap(), None);
        }
        // The call resolves against the session (not "not an action") and its
        // void return value is not echoed as a stray `null`.
        let out = repl
            .process_line("call greet with \"World\"")
            .await
            .unwrap();
        assert_eq!(out, None);
    }

    // --- Error reporting still fires for genuine mistakes ------------------

    #[tokio::test]
    async fn self_contained_security_lint_still_blocks() {
        // Seeding the RNG inside crypto/auth code is a self-contained security
        // violation (it does not depend on session state), so it must stay
        // fatal in the REPL — not get filtered out with the context-dependent
        // name-resolution diagnostics.
        let mut repl = ReplState::new();
        for line in [
            "define action called make_token:",
            "random_seed of 1",
            "give back secure_random_bytes of 16",
        ] {
            assert_eq!(repl.process_line(line).await.unwrap(), None);
        }
        let out = repl
            .process_line("end action")
            .await
            .unwrap()
            .expect("the security lint must be reported");
        assert!(
            out.contains("random_seed must not be used"),
            "expected the insecure-RNG security error, got: {out}"
        );
        // Execution was blocked, so the action was never defined.
        assert!(!repl.interpreter.global_env().borrow().has("make_token"));
    }

    #[tokio::test]
    async fn undefined_variable_is_reported_as_a_runtime_error() {
        let mut repl = ReplState::new();
        let out = repl
            .process_line("display genuinely_missing")
            .await
            .unwrap()
            .expect("a genuine typo must still be reported");
        assert!(
            out.contains("genuinely_missing") && out.to_lowercase().contains("defined"),
            "expected an undefined-variable error, got: {out}"
        );
    }

    #[tokio::test]
    async fn syntax_error_is_reported() {
        let mut repl = ReplState::new();
        let out = repl.process_line("store as 5").await.unwrap();
        assert!(out.is_some(), "a syntax error must be surfaced");
    }

    // --- Expression echo uses WFL's own value spelling --------------------

    #[tokio::test]
    async fn expression_result_is_echoed() {
        let mut repl = ReplState::new();
        assert_eq!(
            repl.process_line("2 plus 3").await.unwrap(),
            Some("5".to_string())
        );
    }

    #[tokio::test]
    async fn boolean_echo_uses_yes_no_not_true_false() {
        let mut repl = ReplState::new();
        assert_eq!(
            repl.process_line("5 is greater than 3").await.unwrap(),
            Some("yes".to_string())
        );
    }

    // --- Multiline block detection ----------------------------------------
    // Every `end`-terminated block must be recognised as "keep reading" so it
    // can be typed across several lines (previously only some were).

    #[test]
    fn incomplete_blocks_request_more_input() {
        let repl = ReplState::new();
        for opener in [
            "check if yes:",
            "count from 1 to 3:",
            "for each item in items:",
            "repeat while yes:",
            "define action called f with n:",
        ] {
            assert!(
                repl.is_input_incomplete(&tokens_of(opener)),
                "`{opener}` should be treated as incomplete (awaiting its `end`)"
            );
        }
    }

    #[test]
    fn complete_statements_are_not_treated_as_incomplete() {
        let repl = ReplState::new();
        for complete in ["store x as 5", "display \"hi\"", "2 plus 3"] {
            assert!(
                !repl.is_input_incomplete(&tokens_of(complete)),
                "`{complete}` is a complete statement and should run immediately"
            );
        }
    }

    #[tokio::test]
    async fn multiline_if_block_runs_only_once_closed() {
        let mut repl = ReplState::new();
        assert_eq!(repl.process_line("check if yes:").await.unwrap(), None);
        assert!(repl.in_multiline, "REPL should be waiting for more input");
        assert_eq!(repl.process_line("display \"inside\"").await.unwrap(), None);
        assert!(repl.in_multiline);
        // Closing the block completes the buffered input and clears multiline.
        repl.process_line("end check").await.unwrap();
        assert!(!repl.in_multiline, "block is closed, multiline should end");
    }
}
