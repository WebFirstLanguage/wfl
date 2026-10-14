// Regression tests: statements inside `wait for` that define a variable
// (open database / query / execute / open file) must not be double-defined
// by the analyzer. Previously the WaitForStatement arm pre-defined the
// variable and the inner statement's own analysis defined it again, so
// `wait for store rows as query db with ...` reported
// "Variable 'rows' has already been defined".

use wfl::analyzer::Analyzer;
use wfl::analyzer::static_analyzer::StaticAnalyzer;
use wfl::diagnostics::Severity;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

fn analyze_errors(code: &str) -> Vec<String> {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser
        .parse()
        .unwrap_or_else(|e| panic!("Failed to parse {code:?}: {e:?}"));

    let mut analyzer = Analyzer::new();
    analyzer
        .analyze_static(&program, 0)
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message)
        .collect()
}

#[test]
fn test_wait_for_query_does_not_double_define() {
    let errors = analyze_errors(
        r#"
open database at "sqlite::memory:" as db
wait for store rows as query db with "SELECT 1 AS x"
display rows
close database db
"#,
    );
    assert!(
        errors.is_empty(),
        "wait for query should not produce analyzer errors: {errors:?}"
    );
}

#[test]
fn test_wait_for_execute_does_not_double_define() {
    let errors = analyze_errors(
        r#"
open database at "sqlite::memory:" as db
wait for store result as execute db with "CREATE TABLE t (x INTEGER)"
display result
close database db
"#,
    );
    assert!(
        errors.is_empty(),
        "wait for execute should not produce analyzer errors: {errors:?}"
    );
}

#[test]
fn test_wait_for_open_database_does_not_double_define() {
    let errors = analyze_errors(
        r#"
wait for open database at "sqlite::memory:" as db
close database db
"#,
    );
    assert!(
        errors.is_empty(),
        "wait for open database should not produce analyzer errors: {errors:?}"
    );
}

#[test]
fn test_wait_for_open_file_does_not_double_define() {
    // Pre-existing instance of the same bug, fixed together.
    let errors = analyze_errors(
        r#"
wait for open file at "t.txt" for writing as fh
close file fh
"#,
    );
    assert!(
        errors.is_empty(),
        "wait for open file should not produce analyzer errors: {errors:?}"
    );
}

#[test]
fn test_duplicate_definition_still_detected() {
    // Removing the pre-define must not stop real duplicates being caught.
    let errors = analyze_errors(
        r#"
store rows as 1
store rows as 2
"#,
    );
    assert!(
        errors
            .iter()
            .any(|message| message.contains("already been defined")),
        "Real duplicate definitions must still error: {errors:?}"
    );
}
