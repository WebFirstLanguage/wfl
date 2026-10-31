//! Count and for-each loops report their diagnostics at the loop's opening
//! keyword.
//!
//! `parse_count_loop` and `parse_for_each_loop` used to store the position of
//! the token *after* `end count` / `end for`, so every diagnostic keyed on the
//! loop statement pointed past the end of the loop. A loop that ended the file
//! without a trailing newline had no next token at all and was placed at line
//! 0, column 0, which the renderer showed as the start of the file. The
//! interpreter's count-loop and for-each errors, the type checker's loop type
//! errors, the analyzer's nested count-loop error and the linter's nesting
//! warning all use that position.
//!
//! The end-to-end tests run the real `wfl` binary and read the location from
//! the rendered diagnostic (`main.wfl:LINE:COLUMN`). The CLI prints analyzer
//! and linter diagnostics without a location, so those are checked through
//! their APIs. Some messages quote the position in their text, and a program
//! can print a caught module error, so those and `--step` output are checked
//! through the binary as well.

use crate::common::{run_src, wfl_exe};
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::TempDir;
use wfl::analyzer::Analyzer;
use wfl::lexer::lex_wfl_with_positions;
use wfl::linter::Linter;
use wfl::parser::Parser;
use wfl::parser::ast::{Program, Statement};

/// The `(line, column)` of the first `main.wfl:LINE:COLUMN` rendered after
/// `message`, which is where that diagnostic points.
fn location_after(out: &str, message: &str) -> Option<(usize, usize)> {
    let rest = &out[out.find(message)? + message.len()..];
    let rest = &rest[rest.find("main.wfl:")? + "main.wfl:".len()..];
    let mut numbers = rest.split(':').map(|part| {
        part.chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse()
            .ok()
    });
    Some((numbers.next()??, numbers.next()??))
}

/// Runs `src` and asserts it fails, reporting each of `messages` at
/// `(line, column)`.
fn assert_errors_at(src: &str, messages: &[&str], line: usize, column: usize) {
    let (out, code) = run_src(src);
    assert_ne!(code, Some(0), "program should fail: {out}");
    for message in messages {
        assert!(out.contains(message), "expected {message:?} in: {out}");
        assert_eq!(
            location_after(&out, message),
            Some((line, column)),
            "{message:?} should point at the loop's opening keyword: {out}"
        );
    }
}

fn parse(src: &str) -> Program {
    let tokens = lex_wfl_with_positions(src);
    Parser::new(&tokens)
        .parse()
        .unwrap_or_else(|errors| panic!("{src:?} should parse: {errors:?}"))
}

fn position(stmt: &Statement) -> (usize, usize) {
    match stmt {
        Statement::CountLoop { line, column, .. } | Statement::ForEachLoop { line, column, .. } => {
            (*line, *column)
        }
        other => panic!("expected a count or for-each loop, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Parser: the statement records the opening keyword
// ---------------------------------------------------------------------------

#[test]
fn count_loop_records_the_count_keyword_position() {
    let program = parse("count from 1 to 3:\n    display count\nend count\n");
    assert_eq!(position(&program.statements[0]), (1, 1));
}

#[test]
fn nested_count_loop_records_its_own_count_keyword_position() {
    let program = parse(
        "count from 1 to 2 as outer:\n    count from 1 to 2 as inner:\n        display inner\n    end count\nend count\n",
    );
    assert_eq!(position(&program.statements[0]), (1, 1));
    let Statement::CountLoop { body, .. } = &program.statements[0] else {
        panic!("expected a count loop");
    };
    assert_eq!(position(&body[0]), (2, 5));
}

#[test]
fn count_loop_at_end_of_file_records_the_count_keyword_position() {
    // No trailing newline: there is no token after `end count`.
    let program = parse("display 0\ncount from 1 to 3:\n    display count\nend count");
    assert_eq!(position(&program.statements[1]), (2, 1));
}

#[test]
fn for_each_loop_records_the_for_keyword_position() {
    let program =
        parse("store items as [1, 2]\nfor each item in items:\n    display item\nend for\n");
    assert_eq!(position(&program.statements[1]), (2, 1));
}

#[test]
fn truncated_loops_are_parse_errors_not_panics() {
    // Both parsers now hold their opening keyword token; malformed or cut-off
    // loops must still come back as parse errors.
    for src in [
        "count",
        "count from",
        "count from 1 to",
        "count from 1 to 3 by",
        "count from 1 to 3 as",
        "count from 1 to 3:",
        "count from 1 to 3:\n    display count\nend",
        "for",
        "for each",
        "for each item in",
        "for each item in [1]:",
        "for each item in [1]:\n    display item\nend",
    ] {
        let tokens = lex_wfl_with_positions(src);
        assert!(
            Parser::new(&tokens).parse().is_err(),
            "{src:?} should be a parse error"
        );
    }
}

#[test]
fn for_each_loop_at_end_of_file_records_the_for_keyword_position() {
    // No trailing newline: there is no token after `end for`.
    let program =
        parse("store items as [1, 2]\nfor each item in items:\n    display item\nend for");
    assert_eq!(position(&program.statements[1]), (2, 1));
}

// ---------------------------------------------------------------------------
// Runtime errors raised by the loop itself (real binary)
// ---------------------------------------------------------------------------

#[test]
fn negative_step_error_points_at_the_count_keyword() {
    assert_errors_at(
        "count from 1 to 5 by -1:\n    display count\nend count\n",
        &["Count loop step must be a positive number"],
        1,
        1,
    );
}

#[test]
fn zero_step_error_in_a_nested_block_points_at_the_count_keyword() {
    assert_errors_at(
        "check if yes:\n    count from 1 to 3 by 0:\n        display count\n    end count\nend check\n",
        &["Count loop step must be a positive number"],
        2,
        5,
    );
}

#[test]
fn step_error_at_end_of_file_points_at_the_count_keyword() {
    // No trailing newline: the old position was line 0, column 0.
    assert_errors_at(
        "display \"start\"\ncount from 5 down to 1 by -1:\n    display count\nend count",
        &["Count loop step must be a positive number"],
        2,
        1,
    );
}

#[test]
fn stalled_counter_error_points_at_the_count_keyword() {
    // Past 2^53 a step of 1 cannot move the counter.
    assert_errors_at(
        "display \"start\"\ncount from 9007199254740992 to 9007199254740994:\n    display count\nend count\n",
        &["is too small to move the counter"],
        2,
        1,
    );
}

#[test]
fn non_numeric_start_errors_point_at_the_count_keyword() {
    // The type checker warns, then the run fails on the same loop.
    assert_errors_at(
        "display \"start\"\ncount from \"a\" to 3:\n    display count\nend count\n",
        &[
            "Start value in count loop must be a number",
            "Count loop requires numeric start and end values",
        ],
        2,
        1,
    );
}

#[test]
fn non_collection_errors_point_at_the_for_keyword() {
    // The type checker warns, then the run fails on the same loop.
    assert_errors_at(
        "store n as 5\nfor each item in n:\n    display item\nend for\n",
        &[
            "Collection in for-each loop must be a list or map",
            "Cannot iterate over Number",
        ],
        2,
        1,
    );
}

// ---------------------------------------------------------------------------
// Analyzer and linter diagnostics keyed on the loop statement
// ---------------------------------------------------------------------------

#[test]
fn nested_count_loop_name_clash_points_at_the_inner_count_keyword() {
    let program = parse(
        "count from 1 to 2:\n    count from 1 to 2:\n        display count\n    end count\nend count\n",
    );
    let errors = Analyzer::new()
        .analyze(&program)
        .expect_err("nested loops sharing `count` must be rejected");
    let clash = errors
        .iter()
        .find(|error| error.message.contains("Nested count loops"))
        .unwrap_or_else(|| panic!("expected the nested-loop error, got {errors:?}"));
    assert_eq!((clash.line, clash.column), (2, 5));
}

#[test]
fn nesting_depth_warnings_point_at_each_loop_keyword() {
    let src = "count from 1 to 2 as outer:\n    for each item in [1, 2]:\n        count from 1 to 2 as inner:\n            display inner\n        end count\n    end for\nend count\n";
    let program = parse(src);
    let mut linter = Linter::new();
    linter.set_max_nesting_depth(1);
    let (diagnostics, _) = linter.lint(&program, src, "main.wfl");
    let positions: Vec<_> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "LINT-COMPLEX")
        .map(|diagnostic| (diagnostic.line, diagnostic.column))
        .collect();
    assert_eq!(positions, vec![(2, 5), (3, 9)]);
}

// ---------------------------------------------------------------------------
// Positions quoted inside message text, and `--step` output (real binary)
// ---------------------------------------------------------------------------

/// Runs `main.wfl` from `files` (name, source) in a fresh temp dir, feeding
/// `stdin`. Returns (stdout, stderr, exit code).
fn run_files(files: &[(&str, &str)], args: &[&str], stdin: &str) -> (String, String, Option<i32>) {
    let dir = TempDir::new().expect("tempdir");
    for (name, source) in files {
        fs::write(dir.path().join(name), source).unwrap();
    }
    let mut child = Command::new(wfl_exe())
        .args(args)
        .arg(dir.path().join("main.wfl"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to execute WFL");
    // A child that exits before reading closes the pipe; the assertions on
    // its output report that more clearly than a BrokenPipe panic would.
    let _ = child.stdin.take().unwrap().write_all(stdin.as_bytes());
    let output = child.wait_with_output().unwrap();
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code(),
    )
}

#[test]
fn loop_variable_redefinition_names_the_loop_line() {
    let (stdout, stderr, code) = run_files(
        &[(
            "main.wfl",
            "display \"start\"\ncount from 1 to 3 as i:\n    store i as 5\nend count\n",
        )],
        &[],
        "",
    );
    let out = format!("{stdout}{stderr}");
    assert_eq!(
        code,
        Some(3),
        "redefining the loop variable is a semantic error: {out}"
    );
    assert!(
        out.contains("Variable 'i' has already been defined at line 2."),
        "the message should name the loop's line: {out}"
    );
}

#[test]
fn caught_module_error_quotes_the_loop_position() {
    // A program can catch a module's analysis error and print its text, which
    // quotes the loop's position.
    let (stdout, stderr, code) = run_files(
        &[
            (
                "badmod.wfl",
                "count from 1 to 2:\n    count from 1 to 2:\n        display count\n    end count\nend count\n",
            ),
            (
                "main.wfl",
                "try:\n    load module from \"badmod.wfl\"\nwhen error:\n    display \"caught: \" with error_message\nend try\n",
            ),
        ],
        &[],
        "",
    );
    assert_eq!(code, Some(0), "the error is caught: {stdout}{stderr}");
    assert!(
        stdout.contains("Semantic error at line 2, column 5: Nested count loops"),
        "the caught message should quote the inner loop's position: {stdout}{stderr}"
    );
}

#[test]
fn caught_executed_file_error_quotes_the_loop_line() {
    // `execute file` re-raises a runtime error from the file it ran with that
    // error's line in the text, and a program can catch and print it.
    let (stdout, stderr, code) = run_files(
        &[
            (
                "steps.wfl",
                "display \"a\"\ncount from 1 to 5 by -1:\n    display count\nend count\n",
            ),
            (
                "main.wfl",
                "try:\n    execute file at \"steps.wfl\"\nwhen error:\n    display \"caught: \" with error_message\nend try\n",
            ),
        ],
        &[],
        "",
    );
    assert_eq!(code, Some(0), "the error is caught: {stdout}{stderr}");
    assert!(
        stdout.contains("(line 2): Count loop step must be a positive number"),
        "the caught message should name the loop's line: {stdout}{stderr}"
    );
}

#[test]
fn step_mode_labels_loops_with_their_first_line() {
    let (stdout, stderr, code) = run_files(
        &[(
            "main.wfl",
            "display \"start\"\ncount from 1 to 2:\n    display count\nend count\nstore items as [1]\nfor each item in items:\n    display item\nend for\n",
        )],
        &["--step"],
        // Answer every `continue (y/n)?` prompt; more than the program asks.
        // The final `n` stops the run rather than hanging if it ever asks
        // more: at end of input the prompt repeats forever.
        &format!("{}n\n", "y\n".repeat(50)),
    );
    assert_eq!(code, Some(0), "{stdout}{stderr}");
    let labels: Vec<&str> = stdout
        .match_indices("Line ")
        .map(|(at, _)| &stdout[at..])
        .filter_map(|rest| {
            let label = rest.split_once(" {").map_or(rest, |(label, _)| label);
            (label.ends_with("CountLoop") || label.ends_with("ForEachLoop")).then_some(label)
        })
        .collect();
    assert_eq!(
        labels,
        vec!["Line 2: CountLoop", "Line 6: ForEachLoop"],
        "{stdout}"
    );
}
