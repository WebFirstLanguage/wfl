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

/// Action parameters, loop variables, and container properties are registered
/// as immutable symbols in the analyzer even though they are not constants.
/// Appending to a list parameter has always been legal (`TestPrograms/
/// test_create_list_expression.wfl` relies on it) and must stay legal.
///
/// Each mutation below targets the parameter or loop variable itself, not some
/// other collection it is merely a value in. Only the bare-name statements this
/// change touches are exercised: `change`/`subtract`/`multiply`/`divide` desugar
/// to `Assignment`, whose long-standing `mutable: false` check already reports
/// these same bindings as constants — a pre-existing message wart that is out of
/// scope here and must not be mistaken for a regression from this test.
#[test]
fn list_parameters_and_loop_variables_are_not_constants() {
    let reports = constant_mutation_reports(
        r#"
define action called process_list with parameters list_param:
    add "processed" to list_param
    remove "processed" from list_param
    clear list_param
    give back list_param
end action

for each entry in [[1] and [2]]:
    add 3 to entry
    remove 1 from entry
    clear entry
end for
"#,
    );

    assert!(
        reports.is_empty(),
        "parameters and loop variables must not be reported as constants: {reports:?}"
    );
}

/// A constant created on both arms of a `check` is re-defined into the parent
/// scope under a new binding key by `promote_constant_marker`. Without that
/// migration `change` still reports (it reads the symbol's `mutable` flag,
/// which survives the merge) while `add`/`remove`/`clear` silently stop
/// reporting — reintroducing #671 one scope up.
///
/// This covers the branch-merge path only. The other migration site,
/// `pop_scope_promoting_except`, is not reachable from WFL source: a binding
/// declared inside a `try`/`catch` does not survive the statement at all — even
/// when declared on every path, a later reference reports
/// `Variable '<name>' is not defined` rather than resolving to a promoted key.
/// See the comment on that migration in `src/analyzer/mod.rs`.
#[test]
fn constants_promoted_out_of_a_branch_are_still_constants() {
    let scalar = r#"
check if yes:
    store new constant LIMIT as 10
otherwise:
    store new constant LIMIT as 20
end check
PLACEHOLDER
"#;

    for mutation in [
        "change LIMIT to 5",
        "add 1 to LIMIT",
        "subtract 1 from LIMIT",
        "multiply LIMIT by 2",
        "divide LIMIT by 2",
    ] {
        let reports = constant_mutation_reports(&scalar.replace("PLACEHOLDER", mutation));
        assert_eq!(
            reports.len(),
            1,
            "`{mutation}` on a promoted constant should be reported once: {reports:?}"
        );
    }

    // The list forms exercise the same merge through the other two bare-name
    // statements, which have no `Assignment` fallback to catch them.
    let list = r#"
check if yes:
    store new constant ROLES as ["admin"]
otherwise:
    store new constant ROLES as ["editor"]
end check
PLACEHOLDER
"#;

    for mutation in [
        "add \"guest\" to ROLES",
        "remove \"admin\" from ROLES",
        "clear ROLES",
    ] {
        let reports = constant_mutation_reports(&list.replace("PLACEHOLDER", mutation));
        assert_eq!(
            reports.len(),
            1,
            "`{mutation}` on a promoted constant list should be reported once: {reports:?}"
        );
    }
}

/// A binding declared inside a `try`/`catch` does not escape the statement, so
/// the `pop_scope_promoting_except` constant migration cannot be reached from
/// WFL source today. Pin that premise: if try-scoping ever changes so these
/// bindings do survive, this test fails and the migration needs real coverage.
#[test]
fn try_scoped_declarations_do_not_escape_the_statement() {
    let tokens = lex_wfl_with_positions(
        r#"
try:
    store new constant LIMIT as 3
catch:
    store new constant LIMIT as 5
end try
add 1 to LIMIT
"#,
    );
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("program should parse");
    let messages = match Analyzer::new().analyze(&program) {
        Ok(()) => Vec::new(),
        Err(errors) => errors.into_iter().map(|e| e.message).collect::<Vec<_>>(),
    };

    assert!(
        messages
            .iter()
            .any(|m| m.contains("'LIMIT' is not defined")),
        "a try-scoped declaration must not escape the statement: {messages:?}"
    );
}

/// An outer constant stays a constant across an intervening `try`, which is the
/// case that would break if the try analysis clobbered `constant_bindings`.
#[test]
fn outer_constants_survive_an_intervening_try() {
    let reports = constant_mutation_reports(
        r#"
store new constant OUTER as 1
try:
    display "attempting"
catch:
    display "failed"
end try
add 1 to OUTER
"#,
    );

    assert_eq!(
        reports.len(),
        1,
        "an outer constant must still be reported after a try: {reports:?}"
    );
}

/// Container-method parameters and container properties are registered
/// immutable on a different code path than plain action parameters, so they get
/// their own guard.
#[test]
fn container_members_are_not_constants() {
    let reports = constant_mutation_reports(
        r#"
create container Messages:
    property entries: List

    action enqueue needs incoming: List:
        add "queued" to incoming
        add "queued" to entries
        clear entries
    end
end
"#,
    );

    assert!(
        reports.is_empty(),
        "container properties and method parameters must not be reported as constants: {reports:?}"
    );
}

/// Issue #673: `push with <list> and <value>` carries its target as an
/// `Expression` (`Statement::PushStatement { list: Expression, .. }`), not the
/// `list_name: String` the #671 check resolves, so `push` slipped past the
/// constant check entirely — no analyzer report, no runtime error, the
/// interpreter mutating the list in place through its `Rc<RefCell<..>>`.
#[test]
fn push_onto_a_constant_list_is_rejected() {
    let reports = constant_mutation_reports(
        r#"
store new constant ROLES as ["admin"]
push with ROLES and "guest"
"#,
    );

    assert_eq!(
        reports.len(),
        1,
        "`push with CONST and value` should be reported by the analyzer: {reports:?}"
    );
    assert!(
        reports[0].contains("ROLES"),
        "report should name the constant: {reports:?}"
    );
}

/// An indexed push target is still a write through the constant binding, so it
/// draws the same report — named for the *root* binding the write reaches, not
/// for the element expression.
#[test]
fn push_onto_an_indexed_constant_target_names_the_root_binding() {
    let reports = constant_mutation_reports(
        r#"
store new constant CONFIG as [["a"]]
push with CONFIG[0] and "b"
"#,
    );

    assert_eq!(
        reports.len(),
        1,
        "`push with CONST[0] and value` should be reported by the analyzer: {reports:?}"
    );
    assert!(
        reports[0].contains("CONFIG"),
        "report should name the root binding: {reports:?}"
    );
}

/// A push target that bottoms out in a call result or a literal has no root
/// binding at all, so there is nothing to test for constness and nothing to
/// report. Pushing onto a temporary was always legal and stays legal.
#[test]
fn push_targets_without_a_root_binding_are_accepted() {
    let reports = constant_mutation_reports(
        r#"
store new constant SOURCE as ["admin"]
push with (unique of SOURCE) and "guest"
push with ["literal"] and "guest"
"#,
    );

    assert!(
        reports.is_empty(),
        "push targets with no root binding must not be reported: {reports:?}"
    );
}

#[test]
fn push_onto_a_mutable_list_is_accepted() {
    let reports = constant_mutation_reports(
        r#"
store items as ["a"]
store nested as [["a"]]
push with items and "b"
push with nested[0] and "b"
"#,
    );

    assert!(
        reports.is_empty(),
        "mutable push targets must not be reported as constants: {reports:?}"
    );
}

/// Action parameters and loop variables are immutable *symbols* that are not
/// constants (see `list_parameters_and_loop_variables_are_not_constants`), and
/// pushing onto a list parameter has always been legal. Guard the #671 shapes
/// against the new write-target walk widening into them.
#[test]
fn push_onto_a_parameter_or_loop_variable_is_accepted() {
    let reports = constant_mutation_reports(
        r#"
define action called process_list with parameters list_param:
    push with list_param and "processed"
    give back list_param
end action

for each entry in [[1] and [2]]:
    push with entry and 3
end for
"#,
    );

    assert!(
        reports.is_empty(),
        "parameters and loop variables must not be reported as constants: {reports:?}"
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
