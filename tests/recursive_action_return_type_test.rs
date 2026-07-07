//! Regression tests for issue #590: a self-recursive action's result must not be
//! typed `Nothing` inside its own body.
//!
//! When an action calls itself and uses the recursive result inside its own body
//! (e.g. indexes it), the type checker used to seed the action's provisional
//! return type as `Nothing`. The body is type-checked *before* the real return
//! type is inferred (#575's ordering), so the self-reference resolved to
//! `Nothing`, producing a false `Cannot index into Nothing` diagnostic. Seeding
//! the provisional type as `Unknown` (which degrades gracefully after #589) fixes
//! this while post-body inference still records the concrete return type.

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::TypeChecker;

/// Type-check `code` and assert it produces zero diagnostics. Asserting a fully
/// clean result (rather than only the absence of one error substring) is a
/// tighter regression guard: it catches both a re-introduced "Cannot index into
/// Nothing" error and any new spurious diagnostic on the same recursive path.
fn assert_typechecks_clean(code: &str) {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");

    let mut type_checker = TypeChecker::new();
    let result = type_checker.check_types(&program);

    assert!(
        result.is_ok(),
        "Self-recursive action should type-check clean; got: {:?}",
        result.err()
    );
}

/// Issue #590: a self-recursive call whose result is indexed directly inside the
/// body must not raise a false "Cannot index into Nothing" diagnostic.
#[test]
fn test_self_recursive_action_result_not_typed_nothing() {
    assert_typechecks_clean(
        r#"
define action called other with parameters n:
    create map m:
        "val" is n
    end map
    return m
end action

define action called p_unary with parameters n:
    check if n is greater than 0:
        store r as p_unary of (n minus 1)
        return other of (r["val"])
    end check
    return other of n
end action

display (p_unary of 3)["val"]
"#,
    );
}

/// A self-recursive action that negates its recursive result before indexing
/// (the Scribe `scribe_p_unary` shape) must also type-check clean.
#[test]
fn test_self_recursive_action_negating_result_typechecks_clean() {
    assert_typechecks_clean(
        r#"
define action called other with parameters n:
    create map m:
        "val" is n
    end map
    return m
end action

define action called p_unary with parameters n:
    check if n is greater than 0:
        store r as p_unary of (n minus 1)
        return other of (0 minus (r["val"]))
    end check
    return other of n
end action

display (p_unary of 3)["val"]
"#,
    );
}
