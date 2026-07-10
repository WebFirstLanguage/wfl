//! Regression tests for the `wfl --test` test-framework result accounting.
//!
//! These guard two bugs in the `TestBlock` error handler:
//!
//! 1. A test that fails due to a *runtime* error (not an assertion) was recorded
//!    in the failures list but never counted in `failed_tests`. The summary showed
//!    `Failed: 0` and the process exited 0, so CI treated a crashing test as passing.
//!
//! 2. A failing *assertion* was recorded twice. The guard meant to skip
//!    already-recorded assertion failures compared the `Display` string (prefixed
//!    with "Runtime error at line ...:") against `"Assertion failed:"`, which never
//!    matched, so every assertion failure was pushed to the failures list a second
//!    time.

use wfl::interpreter::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

async fn run_tests(code: &str) -> wfl::interpreter::TestResults {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse test program");

    let mut interpreter = Interpreter::new();
    interpreter.set_test_mode(true);
    interpreter
        .interpret(&program)
        .await
        .expect("Test-mode programs catch in-test errors and should not error overall");

    interpreter.get_test_results()
}

/// A runtime error inside a test body must count as a failed test and appear
/// exactly once in the failures list (Bug #1).
#[tokio::test]
async fn runtime_error_in_test_is_counted_as_failure() {
    let code = r#"
describe "runtime errors":
    test "passing test":
        expect 1 to equal 1
    end test

    test "runtime error test":
        store x as undefined_variable_that_does_not_exist
    end test
end describe
"#;

    let results = run_tests(code).await;

    assert_eq!(results.total_tests, 2, "two tests should run");
    assert_eq!(results.passed_tests, 1, "one test passes");
    assert_eq!(
        results.failed_tests, 1,
        "the runtime-error test must count as a failure (was 0 before the fix)"
    );
    assert_eq!(
        results.total_tests,
        results.passed_tests + results.failed_tests,
        "total must equal passed + failed"
    );
    assert_eq!(
        results.failures.len(),
        1,
        "the runtime error should be recorded exactly once"
    );
}

/// A failing assertion must be recorded exactly once, not duplicated (Bug #2).
#[tokio::test]
async fn failing_assertion_is_recorded_once() {
    let code = r#"
describe "assertions":
    test "failing assertion":
        expect 5 to equal 6
    end test
end describe
"#;

    let results = run_tests(code).await;

    assert_eq!(results.total_tests, 1);
    assert_eq!(results.passed_tests, 0);
    assert_eq!(results.failed_tests, 1, "one assertion failure");
    assert_eq!(
        results.failures.len(),
        1,
        "a failing assertion must be recorded exactly once, not duplicated"
    );
}

/// Mixed suite: passing tests, assertion failures, and runtime errors must all
/// be counted consistently so that total == passed + failed and the failures
/// list has one entry per failing test.
#[tokio::test]
async fn mixed_results_are_counted_consistently() {
    let code = r#"
describe "mixed":
    test "pass one":
        expect 2 plus 2 to equal 4
    end test

    test "assertion fail":
        expect 1 to equal 2
    end test

    test "runtime fail":
        store y as some_missing_variable
    end test

    test "pass two":
        expect "hi" to equal "hi"
    end test
end describe
"#;

    let results = run_tests(code).await;

    assert_eq!(results.total_tests, 4);
    assert_eq!(results.passed_tests, 2);
    assert_eq!(results.failed_tests, 2);
    assert_eq!(
        results.total_tests,
        results.passed_tests + results.failed_tests,
        "total must equal passed + failed"
    );
    assert_eq!(
        results.failures.len(),
        2,
        "exactly one failure entry per failing test"
    );
}

/// Only the first failing assertion in a test runs (the test stops on first
/// failure), so a test with several failing assertions still counts once.
#[tokio::test]
async fn only_first_failing_assertion_recorded_per_test() {
    let code = r#"
describe "short circuit":
    test "multiple failing assertions":
        expect 1 to equal 2
        expect 3 to equal 4
        expect 5 to equal 6
    end test
end describe
"#;

    let results = run_tests(code).await;

    assert_eq!(results.total_tests, 1);
    assert_eq!(results.failed_tests, 1, "one failed test, not three");
    assert_eq!(
        results.failures.len(),
        1,
        "only the first failing assertion is recorded"
    );
}
