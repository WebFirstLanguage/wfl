use crate::analyzer::Analyzer;
use crate::analyzer::static_analyzer::StaticAnalyzer;
use crate::diagnostics::{DiagnosticReporter, Severity, Span, WflDiagnostic};
use crate::exec::budget::ExecutionBudget;
use crate::interpreter::Interpreter;
use crate::interpreter::value::Value;
use crate::lexer::{lex_wfl_with_positions_checked, token::TokenWithPosition};
use crate::parser::{Parser, ast::Statement};
use crate::typechecker::{TypeCheckError, TypeChecker, TypeError};
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
    /// Every submission that passed static analysis and was executed this
    /// session, in order. Re-analysing `session_inputs + new submission` as one
    /// program is what lets the (otherwise whole-program) analyzer and type
    /// checker see names, actions, and their declared types from earlier lines,
    /// so cross-line references resolve and cross-line type/security contracts
    /// are still enforced — the interpreter alone cannot do the latter.
    session_inputs: Vec<String>,
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
            session_inputs: Vec::new(),
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

    /// Clamp a byte-offset span onto valid UTF-8 char boundaries within
    /// `source`. Spans index the source `String`; a `start + 1` end (produced by
    /// the `convert_*` helpers and the synthesized-label path alike) can land in
    /// the middle of a multi-byte character, and codespan then slices a
    /// non-char-boundary range and panics. This snaps `start` down and `end` up
    /// to the enclosing character (widening never shrinking), covering the last
    /// character when the offset was at EOF.
    fn snap_span_to_char_boundaries(source: &str, span: Span) -> Span {
        let len = source.len();
        let mut start = span.start.min(len);
        let mut end = span.end.min(len).max(start);
        if start == end {
            // Empty span: cover one character so the caret is visible. At EOF
            // there is nothing to the right, so step back onto the last char.
            if start == len && len > 0 {
                start -= 1;
            } else if end < len {
                end += 1;
            }
        }
        while start > 0 && !source.is_char_boundary(start) {
            start -= 1;
        }
        while end < len && !source.is_char_boundary(end) {
            end += 1;
        }
        Span { start, end }
    }

    /// Render one WFL diagnostic to a coloured, Elm-style block (source
    /// snippet, caret, and note), returning the string the REPL prints.
    ///
    /// Parse/type/runtime diagnostics already carry a labelled span; analyzer
    /// diagnostics only carry a line/column, so a span is synthesized from it —
    /// that way *every* REPL diagnostic follows the same "point at the source"
    /// convention (WFL Fundamental #4: clear, actionable errors). Every label
    /// span (synthesized or pre-existing) is then snapped to UTF-8 char
    /// boundaries so codespan never slices mid-codepoint on multi-byte source.
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
            diag.labels.push((
                Span {
                    start: offset,
                    end: offset + 1,
                },
                "here".to_string(),
            ));
        }

        // Normalize ALL label spans (synthesized or from `convert_*`) to valid
        // char boundaries, then drop any that collapse to empty (e.g. an empty
        // source) so they never render a zero-width caret.
        if let Ok(file) = reporter.files.get(file_id) {
            let source = file.source();
            for (span, _) in diag.labels.iter_mut() {
                *span = Self::snap_span_to_char_boundaries(source, *span);
            }
        }
        diag.labels.retain(|(span, _)| span.start < span.end);

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

        // --- Session-aware static analysis -----------------------------------
        // Run the SAME analyze -> type-check -> execute pipeline `wfl <file>`
        // uses, but over `session_inputs + this submission` as one program, so
        // names/actions/types defined on earlier lines are in scope. Only this
        // submission's diagnostics are reported (see `static_check_submission`).
        // A genuine semantic error (undefined name, insecure `random_seed`
        // seeding, an undefined reference inside an action body) blocks
        // execution just as it aborts a file; type diagnostics are advisory, as
        // in the file pipeline.
        if self.static_check_submission(input, &mut reporter, file_id, &mut messages) {
            return Ok(Some(messages.join("\n")));
        }

        // --- Execution -------------------------------------------------------
        // Execute ONLY this submission — the earlier submissions already ran
        // against the persistent interpreter, so re-running them would repeat
        // their output and side effects. Echo the final value only when the last
        // statement is a bare expression (`interpret` returns the value of the
        // last executed statement); `store`/`change`/`display` produce no echo.
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

        // This submission passed static analysis and was executed, so it joins
        // the session's static context for later submissions. It is recorded on
        // execution *attempt*, not only on success, on purpose: recording only
        // fully-successful submissions would drop the bindings a partially-run
        // construct DID establish in the interpreter, so a later reference to
        // one would be wrongly reported as "undefined" — reintroducing exactly
        // the false-positive blocking this change removes. The opposite drift (a
        // fully-failed `store` whose binding never took effect is still "known"
        // to later analysis) is benign: the reference simply raises a clear
        // undefined error at run time rather than being blocked.
        self.session_inputs.push(input.to_string());

        if messages.is_empty() {
            Ok(None)
        } else {
            Ok(Some(messages.join("\n")))
        }
    }

    /// Analyse `session_inputs + submission` as a single program and report only
    /// the diagnostics that fall inside the new submission, translated back to
    /// the submission's own line numbers. Returns `true` when a fatal error was
    /// found (the submission must not run).
    ///
    /// This is what makes REPL static analysis session-aware: a variable/action
    /// defined on an earlier line is in scope, so it is not a false "undefined";
    /// and self-contained contracts the interpreter cannot enforce — the
    /// insecure-RNG security lint, undefined references inside a not-yet-called
    /// action body, cross-line type mismatches — are still checked. Semantic
    /// errors are fatal (as they abort a file); type diagnostics are advisory
    /// (as the file pipeline prints them and continues).
    fn static_check_submission(
        &self,
        input: &str,
        reporter: &mut DiagnosticReporter,
        file_id: usize,
        messages: &mut Vec<String>,
    ) -> bool {
        // Build the accumulated source. `base_line` is the number of lines in
        // the prefix (0 for the first submission), so the new submission's first
        // line sits at `base_line + 1` in the combined source — the offset used
        // to keep only this submission's diagnostics and shift them back.
        let prefix = self.session_inputs.join("\n");
        let (combined, base_line) = if prefix.is_empty() {
            (input.to_string(), 0usize)
        } else {
            (
                format!("{prefix}\n{input}"),
                prefix.matches('\n').count() + 1,
            )
        };

        // Re-analysing the whole session each command is O(n²) over its length.
        // A per-command budget deadline already bounds the work, but cap the
        // accumulated source at a generous size so a pathologically long session
        // degrades to analysing just this submission (warnings only) instead of
        // getting slow. Realistic interactive sessions never approach this.
        const MAX_SESSION_ANALYSIS_BYTES: usize = 256 * 1024;
        if combined.len() > MAX_SESSION_ANALYSIS_BYTES {
            return self.static_check_isolated(input, reporter, file_id, messages);
        }

        // Each prior submission parsed on its own, so the concatenation parses
        // too; if it somehow does not, fall back to analysing this submission
        // alone (no earlier-line context, warnings only — never a false fatal).
        let Ok(tokens) = lex_wfl_with_positions_checked(&combined) else {
            return self.static_check_isolated(input, reporter, file_id, messages);
        };
        let mut parser = Parser::new(&tokens);
        let Ok(combined_program) = parser.parse() else {
            return self.static_check_isolated(input, reporter, file_id, messages);
        };

        // A scratch file over the combined source. Diagnostics reference it, but
        // are rendered against the caller's submission `reporter`/`file_id`
        // after their line numbers are shifted back by `base_line`.
        let mut combined_reporter = DiagnosticReporter::new();
        let combined_file = combined_reporter.add_file("repl", &combined);

        let mut analyzer = Analyzer::new();
        let mut fatal = false;
        for diagnostic in &analyzer.analyze_static(&combined_program, combined_file) {
            if !Self::belongs_to_submission(diagnostic.line, base_line)
                || Self::is_repl_ignorable_semantic(diagnostic)
            {
                continue;
            }
            let mut adjusted = diagnostic.clone();
            adjusted.line = adjusted.line.saturating_sub(base_line);
            messages.push(Self::render_diagnostic(reporter, file_id, &adjusted));
            if adjusted.severity == Severity::Error {
                fatal = true;
            }
        }
        // A fatal semantic error aborts before type checking, exactly as a file.
        if fatal {
            return true;
        }

        // Type checking is advisory in the file pipeline (`wfl <file>` prints
        // type diagnostics and keeps going); mirror that here. A shared-budget
        // breach is still fatal.
        let mut type_checker = TypeChecker::with_analyzer(analyzer);
        if let Err(failure) = type_checker.check_types(&combined_program) {
            match failure {
                TypeCheckError::Budget(exceeded) => {
                    messages.push(format!("Error: {}", exceeded.message()));
                    return true;
                }
                TypeCheckError::Types(errors) => {
                    let action_params = type_checker.get_action_parameters().clone();
                    for error in &errors {
                        if !Self::belongs_to_submission(error.line, base_line)
                            || Self::is_repl_ignorable_type_error(error, &action_params)
                        {
                            continue;
                        }
                        let mut adjusted = error.clone();
                        adjusted.line = adjusted.line.saturating_sub(base_line);
                        let diagnostic = reporter.convert_type_error(file_id, &adjusted);
                        messages.push(Self::render_diagnostic(reporter, file_id, &diagnostic));
                    }
                }
            }
        }
        false
    }

    /// Fallback used only if the accumulated source unexpectedly fails to parse:
    /// analyse the submission alone. Without earlier-line context a cross-line
    /// reference can look undefined, so this NEVER blocks (returns `false`) — the
    /// interpreter catches genuine errors at run time. It still *reports* every
    /// non-ignorable diagnostic (advisory, any severity) rather than silently
    /// dropping errors; a stray false positive here is only reachable in the
    /// near-impossible case where the concatenated session does not parse.
    fn static_check_isolated(
        &self,
        input: &str,
        reporter: &mut DiagnosticReporter,
        file_id: usize,
        messages: &mut Vec<String>,
    ) -> bool {
        let Ok(tokens) = lex_wfl_with_positions_checked(input) else {
            return false;
        };
        let mut parser = Parser::new(&tokens);
        let Ok(program) = parser.parse() else {
            return false;
        };
        let mut scratch = DiagnosticReporter::new();
        let scratch_file = scratch.add_file("repl", input);
        let mut analyzer = Analyzer::new();
        for diagnostic in &analyzer.analyze_static(&program, scratch_file) {
            if !Self::is_repl_ignorable_semantic(diagnostic) {
                messages.push(Self::render_diagnostic(reporter, file_id, diagnostic));
            }
        }
        false
    }

    /// A diagnostic belongs to the new submission when it sits on a line after
    /// the accumulated prefix. Position-less diagnostics (line 0) are kept so a
    /// genuine but unlocated error is never silently dropped.
    fn belongs_to_submission(line: usize, base_line: usize) -> bool {
        line == 0 || line > base_line
    }

    /// Semantic diagnostics that are correct for a file but wrong to enforce in
    /// an interactive session.
    fn is_repl_ignorable_semantic(diagnostic: &WflDiagnostic) -> bool {
        // "Unused" is a false positive interactively — a binding is available to
        // a *future* submission. Re-`store`/re-defining a name is allowed in the
        // REPL (the interpreter reassigns), so "already defined" must not block
        // continuing the session.
        diagnostic.code == "ANALYZE-UNUSED"
            || diagnostic.message.contains("already been defined")
            || diagnostic.message.contains("already defined")
    }

    /// Type diagnostics to drop in the REPL: the same action-parameter and
    /// duplicate-symbol false positives `wfl <file>` filters, plus re-definition
    /// (allowed interactively).
    fn is_repl_ignorable_type_error(
        error: &TypeError,
        action_params: &std::collections::HashSet<String>,
    ) -> bool {
        if error.message.starts_with("Variable '") && error.message.ends_with("' is not defined") {
            let name = error
                .message
                .trim_start_matches("Variable '")
                .trim_end_matches("' is not defined");
            if action_params.contains(name) {
                return true;
            }
        }
        error.message.contains("already been defined") || error.message.contains("already defined")
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

    /// Lex a snippet through the same entry point the REPL uses before asking
    /// whether the input is complete (`lex_wfl_with_positions_checked`). With no
    /// budget scoped — as in these tests — the checked lexer skips its budget
    /// checks and behaves like the plain lexer, so a small snippet never errors.
    fn tokens_of(src: &str) -> Vec<TokenWithPosition> {
        lex_wfl_with_positions_checked(src).expect("test snippet should lex cleanly")
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
    async fn undefined_variable_is_reported() {
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

    // --- Session-aware static analysis (maintainer review on PR #617) ------
    // The REPL analyses `earlier submissions + this one` as one program, so the
    // static checks the interpreter cannot do (security lint, definition-time
    // reference checks, cross-line type contracts) still run, while genuine
    // session definitions are not false "undefined".

    #[tokio::test]
    async fn insecure_rng_is_blocked_even_when_referencing_an_earlier_line_variable() {
        // The security lint runs against the whole session, so referencing a
        // variable defined on an earlier line no longer makes the analyzer bail
        // before the lint (which was the reported gap).
        let mut repl = ReplState::new();
        assert_eq!(
            repl.process_line("store note as \"session\"")
                .await
                .unwrap(),
            None
        );
        for line in [
            "define action called mk:",
            "display note", // references the earlier-line variable
            "random_seed of 1",
            "give back secure_random_bytes of 16",
        ] {
            assert_eq!(repl.process_line(line).await.unwrap(), None);
        }
        let out = repl
            .process_line("end action")
            .await
            .unwrap()
            .expect("the security lint must still fire");
        assert!(
            out.contains("random_seed must not be used"),
            "expected the insecure-RNG error, got: {out}"
        );
        assert!(!repl.interpreter.global_env().borrow().has("mk"));
    }

    #[tokio::test]
    async fn undefined_reference_inside_an_action_body_is_blocked_at_definition() {
        // Defining an action stores its body without running it, so without
        // definition-time validation a typo would only surface when the action
        // is finally called. The analyzer must catch it now.
        let mut repl = ReplState::new();
        for line in [
            "define action called broken:",
            "display misspelled_variable",
        ] {
            assert_eq!(repl.process_line(line).await.unwrap(), None);
        }
        let out = repl
            .process_line("end action")
            .await
            .unwrap()
            .expect("an undefined reference in the body must be reported");
        assert!(
            out.contains("misspelled_variable") && out.to_lowercase().contains("defined"),
            "expected an undefined-variable error, got: {out}"
        );
        assert!(!repl.interpreter.global_env().borrow().has("broken"));
    }

    #[tokio::test]
    async fn earlier_session_variable_is_usable_inside_a_definition() {
        // The converse of the previous test: a reference that *is* defined on an
        // earlier line must resolve, so the definition is accepted and runs.
        let mut repl = ReplState::new();
        assert_eq!(
            repl.process_line("store greeting as \"hello\"")
                .await
                .unwrap(),
            None
        );
        for line in [
            "define action called sayit:",
            "display greeting",
            "end action",
        ] {
            assert_eq!(repl.process_line(line).await.unwrap(), None);
        }
        assert!(repl.interpreter.global_env().borrow().has("sayit"));
        // And calling it runs cleanly (no "not defined").
        assert_eq!(repl.process_line("call sayit").await.unwrap(), None);
    }

    #[tokio::test]
    async fn self_recursive_action_is_accepted() {
        // The action's own name is in scope inside its body (signature pass), so
        // a self-reference is not a false "undefined" at definition time.
        let mut repl = ReplState::new();
        for line in [
            "define action called countdown with n:",
            "call countdown with n",
            "end action",
        ] {
            assert_eq!(repl.process_line(line).await.unwrap(), None);
        }
        assert!(repl.interpreter.global_env().borrow().has("countdown"));
    }

    #[tokio::test]
    async fn cross_line_type_mismatch_is_surfaced() {
        // The type checker sees `n: Number` from the earlier line, so assigning
        // text to it is reported — advisory, matching the `wfl <file>` pipeline,
        // so the command still runs.
        let mut repl = ReplState::new();
        assert_eq!(repl.process_line("store n as 5").await.unwrap(), None);
        let out = repl
            .process_line("change n to \"hello\"")
            .await
            .unwrap()
            .expect("a cross-line type mismatch must be surfaced");
        assert!(
            out.contains("incompatible") && out.contains("Number"),
            "expected an incompatible-assignment type error, got: {out}"
        );
    }

    #[tokio::test]
    async fn re_storing_a_variable_is_allowed() {
        // Re-defining a name is normal in a REPL (the interpreter reassigns), so
        // the file-only "already defined" error must not block continuation.
        let mut repl = ReplState::new();
        assert_eq!(repl.process_line("store x as 5").await.unwrap(), None);
        assert_eq!(repl.process_line("store x as 10").await.unwrap(), None);
        assert_eq!(
            repl.process_line("x").await.unwrap(),
            Some("10".to_string())
        );
    }

    #[tokio::test]
    async fn syntax_error_is_reported() {
        let mut repl = ReplState::new();
        let out = repl.process_line("store as 5").await.unwrap();
        assert!(out.is_some(), "a syntax error must be surfaced");
    }

    #[test]
    fn render_diagnostic_snaps_span_to_char_boundaries() {
        // A label-less diagnostic whose position falls on a multi-byte
        // character must not make codespan slice a non-char-boundary range
        // (which panics). "café" is 5 bytes; column 4 is the start of the 2-byte
        // 'é', so a naive `start + 1` span would split it.
        let mut reporter = DiagnosticReporter::new();
        let file_id = reporter.add_file("repl", "café");
        let diag = WflDiagnostic::new(
            Severity::Warning,
            "unreachable code",
            None::<String>,
            "ANALYZE-UNREACHABLE",
            file_id,
            1,
            4,
            None,
        );
        // Must not panic, must carry the message, and must render the caret
        // (the source-location header) rather than the caret-less fallback.
        let rendered = ReplState::render_diagnostic(&mut reporter, file_id, &diag);
        assert!(
            rendered.contains("unreachable code") && rendered.contains("repl:1:"),
            "expected a caret-rendered diagnostic, got: {rendered}"
        );
    }

    #[test]
    fn render_diagnostic_snaps_preexisting_label_span_to_char_boundaries() {
        // Parse/type/runtime diagnostics arrive with a span already attached
        // (from the `convert_*` helpers, typically `start..start + 1`). If that
        // end lands mid-codepoint on multi-byte source, codespan panics — so the
        // normalization must apply to pre-existing labels too, not only
        // synthesized ones. "café" is 5 bytes; a 3..4 span splits the 2-byte 'é'.
        let mut reporter = DiagnosticReporter::new();
        let file_id = reporter.add_file("repl", "café");
        let diag =
            WflDiagnostic::error("boom").with_primary_label(Span { start: 3, end: 4 }, "here");
        // Must not panic and must still carry the message.
        let rendered = ReplState::render_diagnostic(&mut reporter, file_id, &diag);
        assert!(
            rendered.contains("boom"),
            "expected the message in the rendered diagnostic, got: {rendered}"
        );
    }

    #[test]
    fn render_diagnostic_points_at_source_even_at_eof() {
        // A diagnostic whose column lands at EOF (one past the last byte) must
        // still get a caret on the last character, not render caret-less.
        let mut reporter = DiagnosticReporter::new();
        let file_id = reporter.add_file("repl", "hi");
        let diag = WflDiagnostic::new(
            Severity::Warning,
            "watch out",
            None::<String>,
            "ANALYZE-SHADOW",
            file_id,
            1,
            3, // column past the last character of "hi"
            None,
        );
        let rendered = ReplState::render_diagnostic(&mut reporter, file_id, &diag);
        // A caret label makes codespan emit the source-location header
        // (`repl:1:2`, pointing at the last character); without a label the
        // message would render alone with no location.
        assert!(
            rendered.contains("repl:1:"),
            "expected a caret pointing at the source at EOF, got: {rendered}"
        );
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
    async fn multiline_if_block_buffers_across_lines_and_runs_on_close() {
        let mut repl = ReplState::new();
        // Opener and body are buffered — the REPL waits and runs nothing yet.
        assert_eq!(repl.process_line("check if yes:").await.unwrap(), None);
        assert!(repl.in_multiline, "REPL should be waiting for more input");
        assert_eq!(
            repl.process_line("store inside_block as 7").await.unwrap(),
            None
        );
        assert!(repl.in_multiline);
        assert!(
            !repl.interpreter.global_env().borrow().has("inside_block"),
            "the buffered block must not run before it is closed"
        );
        // Closing the block completes the buffered input and runs it once,
        // cleanly (no error → `Ok(None)`), which defines the block's variable.
        assert_eq!(repl.process_line("end check").await.unwrap(), None);
        assert!(!repl.in_multiline, "block is closed, multiline should end");
        assert!(
            repl.interpreter.global_env().borrow().has("inside_block"),
            "closing the block should execute its body exactly once"
        );
    }
}
