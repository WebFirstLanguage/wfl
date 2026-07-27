use wfl::analyzer::Analyzer;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::{Parser, ast::Program};
use wfl::typechecker::TypeChecker;

fn parse(source: &str) -> Program {
    let tokens = lex_wfl_with_positions(source);
    Parser::new(&tokens)
        .parse()
        .expect("test program should parse")
}

#[test]
fn a_failed_run_does_not_poison_the_next_typecheck() {
    let mut checker = TypeChecker::new();
    assert!(
        checker
            .check_types(&parse("store bad as 1 minus \"x\"\n"))
            .is_err()
    );
    assert!(
        checker.check_types(&Program::new()).is_ok(),
        "per-run diagnostics must be cleared"
    );
}

#[test]
fn independent_runs_do_not_reuse_program_symbols() {
    let mut checker = TypeChecker::new();
    checker
        .check_types(&parse("store value as 1\n"))
        .expect("first program should typecheck");
    checker
        .check_types(&parse("store value as 2\n"))
        .expect("the next program has an independent top-level scope");
}

#[test]
fn with_analyzer_is_preanalyzed_for_one_run_only() {
    let first = parse("store first_only as 1\n");
    let mut analyzer = Analyzer::new();
    analyzer
        .analyze(&first)
        .expect("first program should analyze");
    let mut checker = TypeChecker::with_analyzer(analyzer);
    checker
        .check_types(&first)
        .expect("the supplied analyzer belongs to the first run");

    assert!(
        checker.check_types(&parse("display first_only\n")).is_err(),
        "a reused with_analyzer checker must analyze the second independent program"
    );
}

#[test]
fn analyzer_reuse_does_not_reuse_program_symbols() {
    let mut analyzer = Analyzer::new();
    analyzer
        .analyze(&parse("store stale as 1\n"))
        .expect("first program should analyze");

    assert!(
        analyzer.analyze(&parse("display stale\n")).is_err(),
        "the next independent program must not resolve a prior binding"
    );
}

#[test]
fn analyzer_reuse_clears_prior_diagnostics() {
    let mut analyzer = Analyzer::new();
    assert!(analyzer.analyze(&parse("display missing_name\n")).is_err());
    assert!(
        analyzer.analyze(&Program::new()).is_ok(),
        "an empty second program must not inherit the first run's errors"
    );
}
