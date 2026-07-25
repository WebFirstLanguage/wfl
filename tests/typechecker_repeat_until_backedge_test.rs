//! Red→Green regression for issue #642: the type checker must walk a
//! `repeat until` loop the way the runtime executes it — body FIRST, then the
//! condition, in the same scope — mirroring the `while`/`repeat while`
//! fixed-point treatment added in #641.
//!
//! Runtime (`Statement::RepeatUntilLoop`) always runs the body once before
//! evaluating the condition, and evaluates the condition in the same
//! environment. Checking the condition first leaves body-introduced bindings
//! `Unknown` in the condition, silently missing real type errors.

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::TypeChecker;

fn typecheck(code: &str) -> Result<(), String> {
    let tokens = lex_wfl_with_positions(code);
    let program = Parser::new(&tokens).parse().expect("parse");
    TypeChecker::new()
        .check_types(&program)
        .map_err(|errors| format!("{errors:?}"))
}

#[test]
fn repeat_until_condition_sees_body_retyped_binding() {
    // The body ALWAYS runs before the condition, and the body retypes `out`
    // from Number to a response stream — so the condition's numeric
    // comparison is a guaranteed runtime type error the checker must
    // surface. Checking the condition first sees the stale outer Number and
    // misses it.
    let code = "store out as 5\n\
                repeat until out is greater than 3:\n\
                \x20\x20\x20\x20start streaming response to \"req\" with status 200 as out\n\
                end repeat\n";
    assert!(
        typecheck(code).is_err(),
        "the condition must be checked AFTER the body (runtime order), so the \
         body's retyping of `out` to a response stream is visible to the \
         numeric comparison"
    );
}

#[test]
fn repeat_until_with_consistent_types_still_checks_cleanly() {
    // The happy path must stay green under body-first checking.
    let code = "store counter as 0\n\
                repeat until counter is greater than 3:\n\
                \x20\x20\x20\x20change counter to counter plus 1\n\
                end repeat\n\
                display counter\n";
    let result = typecheck(code);
    assert!(
        result.is_ok(),
        "a well-typed repeat-until loop must not be rejected: {result:?}"
    );
}

#[test]
fn repeat_until_break_path_does_not_force_post_body_condition_types() {
    // Runtime exits at `break` WITHOUT evaluating the condition, so a body
    // that retypes a binding and then always breaks is runtime-valid even if
    // the condition would mistype against the retyped binding. The checker
    // must not reject it — type errors are fatal inside `load module`, so a
    // false positive here breaks working modules (PR #643 review).
    let code = "store out as 5\n\
                repeat until out is greater than 3:\n\
                \x20\x20\x20\x20start streaming response to \"req\" with status 200 as out\n\
                \x20\x20\x20\x20break\n\
                end repeat\n";
    let result = typecheck(code);
    assert!(
        result.is_ok(),
        "a retype-then-break body never reaches the condition at runtime and \
         must not be reported against post-body types: {result:?}"
    );
}
