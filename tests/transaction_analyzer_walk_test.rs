//! The analyzer has three AST walkers that must all know about every
//! block-bearing statement. A statement that only one of them understands is a
//! silent hole: code inside the block becomes invisible to the other two.
//!
//! For `in transaction on db: ... end transaction` the stakes are not cosmetic.
//! `check_insecure_rng_seeding` is a security lint that blocks a program which
//! seeds the general-purpose RNG and then performs a cryptographic operation;
//! if its walker cannot see into the block, moving the calls inside one is
//! enough to evade it.

use wfl::analyzer::Analyzer;
use wfl::analyzer::static_analyzer::{StaticAnalyzer, rng_security_ingredients};
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

fn parse(source: &str) -> wfl::parser::ast::Program {
    let tokens = lex_wfl_with_positions(source);
    Parser::new(&tokens)
        .parse()
        .expect("test program must parse")
}

/// The lint fires on this program when the calls sit at the top level.
#[test]
fn insecure_rng_seeding_is_detected_at_the_top_level() {
    let program = parse(
        r#"
store seeded as random_seed of 42
store token as hash_password of "hunter2"
"#,
    );
    let found = rng_security_ingredients(&program);
    assert!(found.seed_site.is_some(), "random_seed must be seen");
    assert!(found.security_site.is_some(), "hash_password must be seen");
}

/// ...and must still fire when they are moved inside a transaction block.
#[test]
fn insecure_rng_seeding_is_detected_inside_a_transaction_block() {
    let program = parse(
        r#"
open database at "sqlite::memory:" as db
in transaction on db:
    store seeded as random_seed of 42
    store token as hash_password of "hunter2"
end transaction
"#,
    );
    let found = rng_security_ingredients(&program);
    assert!(
        found.seed_site.is_some(),
        "a random_seed call inside a transaction block must not escape the security lint"
    );
    assert!(
        found.security_site.is_some(),
        "a crypto call inside a transaction block must not escape the security lint"
    );
}

/// Calls in a database statement's operands are collected too — the handle, the
/// SQL text, and any bound parameters are ordinary expressions, so a
/// security-sensitive call can hide in one whether or not a transaction block is
/// involved.
#[test]
fn calls_in_database_statement_operands_are_collected() {
    let program = parse(
        r#"
open database at "sqlite::memory:" as db
store seeded as random_seed of 42
store rows as query db with "SELECT 1" and parameters [hash_password of "hunter2"]
"#,
    );
    let found = rng_security_ingredients(&program);
    assert!(found.seed_site.is_some(), "random_seed must be seen");
    assert!(
        found.security_site.is_some(),
        "a crypto call inside a bound parameter must not escape the security lint"
    );
}

/// ...and the same holds inside a transaction block, where both walks apply.
#[test]
fn calls_in_database_operands_inside_a_transaction_are_collected() {
    let program = parse(
        r#"
open database at "sqlite::memory:" as db
store seeded as random_seed of 42
in transaction on db:
    store rows as query db with "SELECT 1" and parameters [hash_password of "hunter2"]
end transaction
"#,
    );
    let found = rng_security_ingredients(&program);
    assert!(found.seed_site.is_some());
    assert!(
        found.security_site.is_some(),
        "a crypto call in a database parameter inside a transaction must still be seen"
    );
}

/// The unused-variable walk must see declarations inside the block. Its failure
/// mode is a false negative — a variable the block declares and nobody uses is
/// simply never tracked, so it is never reported.
#[test]
fn an_unused_variable_declared_inside_a_transaction_block_is_reported() {
    let program = parse(
        r#"
open database at "sqlite::memory:" as db
in transaction on db:
    store forgotten_receipt as 7
end transaction
"#,
    );
    let analyzer = Analyzer::new();
    let diagnostics = analyzer.check_unused_variables(&program, 0);

    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("forgotten_receipt")),
        "a variable declared inside a transaction block and never used must still be \
         reported as unused; got: {diagnostics:?}"
    );
}

/// ...and one that *is* used afterwards must not be reported, so fixing the
/// false negative does not introduce a false positive.
#[test]
fn a_used_variable_declared_inside_a_transaction_block_is_not_reported() {
    let program = parse(
        r#"
open database at "sqlite::memory:" as db
in transaction on db:
    store receipt_id as 7
end transaction
display receipt_id
"#,
    );
    let analyzer = Analyzer::new();
    let diagnostics = analyzer.check_unused_variables(&program, 0);

    let unused: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.message.contains("receipt_id"))
        .collect();
    assert!(
        unused.is_empty(),
        "receipt_id is used after the block, so it must not be reported: {unused:?}"
    );
}
