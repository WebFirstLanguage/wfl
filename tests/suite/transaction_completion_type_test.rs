//! A transaction block passes its body's value through, like `try:` does.
//!
//! `execute_transaction_statement` returns the body's last value directly, and
//! the block shares the enclosing scope for exactly that reason. The type
//! checker has to agree: if it records the block's completion type as `Nothing`,
//! an action whose body ends in a transaction is inferred to return nothing, and
//! every later use of that result is reported as a type error on a program that
//! runs correctly.

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::TypeChecker;

fn type_errors(source: &str) -> Vec<String> {
    let tokens = lex_wfl_with_positions(source);
    let program = Parser::new(&tokens).parse().expect("program must parse");
    let mut checker = TypeChecker::new();
    match checker.check_types(&program) {
        Ok(()) => Vec::new(),
        Err(error) => vec![format!("{error:?}")],
    }
}

/// The value produced inside the block is the action's result, so arithmetic on
/// it is valid.
#[test]
fn an_action_ending_in_a_transaction_returns_the_blocks_value() {
    let errors = type_errors(
        r#"
open database at "sqlite::memory:" as db
define action called row_count:
    in transaction on db:
        store rows as query db with "SELECT x FROM t"
        length of rows
    end transaction
end action

store n as call row_count
store doubled as n times 2
display doubled
"#,
    );

    assert!(
        errors.is_empty(),
        "the transaction block must pass its body's value through, as `try:` does; got: {errors:?}"
    );
}

/// The same shape written with `try:` is the reference behaviour the block is
/// documented to mirror. If this ever fails, the comparison above is moot.
#[test]
fn the_same_shape_with_try_is_accepted() {
    let errors = type_errors(
        r#"
define action called row_count:
    try:
        store rows as [1 and 2]
        length of rows
    when error:
        0
    end try
end action

store n as call row_count
store doubled as n times 2
display doubled
"#,
    );

    assert!(
        errors.is_empty(),
        "baseline: `try:` already propagates its completion type; got: {errors:?}"
    );
}
