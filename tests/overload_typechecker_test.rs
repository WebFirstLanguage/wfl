//! Typechecker tests for user-defined action overloading: each overload's
//! inferred return type must flow to call sites that statically resolve to
//! it, and statically ambiguous (deferred) calls must not produce spurious
//! errors.
//!
//! Return types are always inferred from action bodies — WFL has no working
//! return-type annotation syntax (the lexer folds `name returns text` into a
//! single multi-word identifier), so the overload machinery leans on the
//! post-body inference from issue #575.

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::TypeChecker;

fn typecheck_errors(code: &str) -> Vec<String> {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser
        .parse()
        .unwrap_or_else(|e| panic!("Failed to parse {code:?}: {e:?}"));

    let mut checker = TypeChecker::new();
    match checker.check_types(&program) {
        Ok(()) => Vec::new(),
        Err(err) => err
            .into_diagnostics()
            .into_iter()
            .map(|e| e.message)
            .collect(),
    }
}

fn assert_clean(code: &str) {
    let errors = typecheck_errors(code);
    assert!(
        errors.is_empty(),
        "expected clean typecheck, got: {errors:?}"
    );
}

#[test]
fn return_type_per_overload_number_path() {
    // `f of 1` resolves to the number overload (inferred return: Number), so
    // arithmetic on its result typechecks.
    assert_clean(
        r#"
define action called f with parameters x as number:
    return x plus 1
end action

define action called f with parameters x as text:
    return "text result"
end action

store a as f of 1
store b as a plus 1
display b
"#,
    );
}

#[test]
fn return_type_per_overload_text_misuse_rejected() {
    // `f of "s"` resolves to the text overload (inferred return: Text);
    // multiplying its result must be a type error.
    let errors = typecheck_errors(
        r#"
define action called f with parameters x as number:
    return x plus 1
end action

define action called f with parameters x as text:
    return "text result"
end action

store c as f of "s"
store d as c times 2
display d
"#,
    );
    assert!(
        !errors.is_empty(),
        "using the text overload's result as a number must be rejected"
    );
}

#[test]
fn common_return_when_deferred() {
    // Both overloads return text, so even a deferred (dynamic-arg) call has
    // a known Text result that concatenation accepts.
    assert_clean(
        r#"
define action called f with parameters x as number:
    return "n"
end action

define action called f with parameters x as text:
    return "t"
end action

define action called g with parameters v:
    store r as f of v
    store s as r with "!"
    display s
end action
"#,
    );
}

#[test]
fn divergent_return_when_deferred_does_not_cascade() {
    // Overloads disagree on return type; a deferred call infers Unknown and
    // any downstream use stays diagnostic-free.
    assert_clean(
        r#"
define action called f with parameters x as number:
    return 1
end action

define action called f with parameters x as text:
    return "t"
end action

define action called g with parameters v:
    store r as f of v
    display r
end action
"#,
    );
}

#[test]
fn forward_reference_to_later_overload() {
    // PASS 1 registers all top-level signatures before checking, so a call
    // that statically resolves to a later-defined overload is clean.
    assert_clean(
        r#"
define action called f with parameters x as number:
    return 1
end action

define action called probe:
    return f of "hello"
end action

define action called f with parameters x as text:
    return "t"
end action
"#,
    );
}
