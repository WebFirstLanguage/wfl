//! Analyzer coverage for constant immutability across every mutation form.
//!
//! Regression tests for issue #671: `add <value> to CONST` parses to
//! `AddToListStatement` (the target's type is unknown at parse time), and the
//! analyzer never checked that statement's target for constness. Only the
//! `Assignment`-shaped mutations (`change`, `subtract`, `multiply`, `divide`)
//! were reported, so a program mutating a constant five ways got four reports.

use wfl::analyzer::Analyzer;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

/// Analyze `source` and return every "Cannot modify constant" report.
fn constant_mutation_reports(source: &str) -> Vec<String> {
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("program should parse");

    let mut analyzer = Analyzer::new();
    match analyzer.analyze(&program) {
        Ok(()) => Vec::new(),
        Err(errors) => errors
            .into_iter()
            .map(|error| error.message)
            .filter(|message| message.contains("Cannot modify constant"))
            .collect(),
    }
}

#[test]
fn add_to_constant_is_rejected_on_its_own() {
    let reports = constant_mutation_reports(
        r#"
store new constant MAX_SIZE as 100
add 10 to MAX_SIZE
"#,
    );

    assert_eq!(
        reports.len(),
        1,
        "`add ... to CONST` should be reported by the analyzer: {reports:?}"
    );
    assert!(
        reports[0].contains("MAX_SIZE"),
        "report should name the constant: {reports:?}"
    );
}

#[test]
fn every_mutation_form_of_a_constant_is_reported() {
    let reports = constant_mutation_reports(
        r#"
store new constant MAX_SIZE as 100
change MAX_SIZE to 200
add 10 to MAX_SIZE
subtract 5 from MAX_SIZE
multiply MAX_SIZE by 2
divide MAX_SIZE by 2
"#,
    );

    assert_eq!(
        reports.len(),
        5,
        "each of the five constant mutations should be reported once: {reports:?}"
    );
}

#[test]
fn list_mutation_statements_reject_constant_targets() {
    let reports = constant_mutation_reports(
        r#"
store new constant FIXED_LIST as [1 and 2]
add 3 to FIXED_LIST
remove 1 from FIXED_LIST
clear FIXED_LIST
"#,
    );

    assert_eq!(
        reports.len(),
        3,
        "add/remove/clear against a constant list should each be reported: {reports:?}"
    );
}

#[test]
fn mutable_targets_are_still_accepted() {
    let reports = constant_mutation_reports(
        r#"
store total as 100
store items as [1 and 2]
change total to 200
add 10 to total
subtract 5 from total
multiply total by 2
divide total by 2
add 3 to items
remove 1 from items
clear items
"#,
    );

    assert!(
        reports.is_empty(),
        "mutable bindings must not be reported as constants: {reports:?}"
    );
}
