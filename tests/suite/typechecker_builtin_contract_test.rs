use wfl::analyzer::Analyzer;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::{TypeCheckError, TypeChecker};

fn typecheck(source: &str) -> Result<(), TypeCheckError> {
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("test program should parse");
    TypeChecker::new().check_types(&program)
}

fn typecheck_like_cli(source: &str) -> Result<(), TypeCheckError> {
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("test program should parse");
    let mut analyzer = Analyzer::new();
    analyzer
        .analyze(&program)
        .expect("test program should pass semantic analysis");
    TypeChecker::with_analyzer(analyzer).check_types(&program)
}

#[test]
fn mutating_list_builtin_results_are_not_typed_as_lists() {
    for call in [
        "sort of values",
        "reverse_list of values",
        "unshift of values and 0",
        "insert_at of values and 0 and 9",
        "fill of values and 0",
    ] {
        let result = typecheck(&format!(
            r#"
store values as [3, 1, 2]
store mutation_result as {call}
store first as mutation_result[0]
"#
        ));

        let failure = match result {
            Err(failure) => failure,
            Ok(()) => panic!("{call} returns Nothing, so indexing it must be rejected"),
        };
        let diagnostics = failure.into_diagnostics();
        assert!(
            diagnostics
                .iter()
                .any(|error| error.message.contains("Cannot index into Nothing")),
            "expected the result of {call} to be Nothing, got: {diagnostics:?}"
        );
    }
}

#[test]
fn definite_nothing_does_not_satisfy_a_concrete_builtin_parameter() {
    let diagnostics = typecheck("store invalid as touppercase of nothing\n")
        .expect_err("the native uppercase implementation rejects Nothing")
        .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.message.contains("expected Text")),
        "expected the builtin contract to reject definite Nothing: {diagnostics:?}"
    );
}

#[test]
fn removing_list_builtins_return_an_element_not_a_list() {
    for call in [
        "pop of values",
        "shift of values",
        "remove_at of values and 0",
    ] {
        typecheck(&format!(
            r#"
store values as [3, 1, 2]
store removed as {call}
store adjusted as removed minus 1
"#
        ))
        .unwrap_or_else(|failure| {
            panic!(
                "{call} returns a dynamically typed list element, so numeric use must remain \
                 gradual: {:?}",
                failure.into_diagnostics()
            )
        });
    }
}

#[test]
fn date_time_builtins_preserve_their_runtime_value_types() {
    for (expression, expected_type) in [
        ("create_date of 2026 and 7 and 26", "Date"),
        ("parse_date of \"2026-07-26\" and \"%Y-%m-%d\"", "Date"),
        (
            "add_days of (create_date of 2026 and 7 and 26) and 1",
            "Date",
        ),
        ("create_time of 12 and 30 and 0", "Time"),
        ("create_datetime of 2026 and 7 and 26", "DateTime"),
        ("datetime_from_timestamp of 0", "DateTime"),
    ] {
        let failure = match typecheck(&format!("store invalid as ({expression}) minus 1\n")) {
            Err(failure) => failure,
            Ok(()) => panic!(
                "{expression} returns {expected_type}, so numeric subtraction must be rejected"
            ),
        };
        let diagnostics = failure.into_diagnostics();
        assert!(
            diagnostics
                .iter()
                .any(|error| error.message.contains(expected_type)),
            "expected {expression} to be reported as {expected_type}, got: {diagnostics:?}"
        );
    }
}

#[test]
fn optional_builtin_arguments_follow_the_runtime_arity_ranges() {
    for call in [
        "create_time of 12 and 30",
        "create_datetime of 2026 and 7 and 26 and 12 and 30 and 0",
        "call timestamp",
    ] {
        typecheck(&format!("store result as {call}\n")).unwrap_or_else(|failure| {
            panic!(
                "{call} is within the runtime builtin's accepted arity range: {:?}",
                failure.into_diagnostics()
            )
        });
    }
}

#[test]
fn action_form_builtin_calls_check_their_arguments() {
    let failure = typecheck("store invalid as call abs with \"not a number\"\n")
        .expect_err("abs must reject a concretely non-numeric argument");
    assert!(
        failure
            .into_diagnostics()
            .iter()
            .any(|error| error.message.contains("expected Number")),
        "expected a numeric argument diagnostic"
    );

    let failure = typecheck("store invalid as call abs with (1 minus \"x\")\n")
        .expect_err("type errors nested inside builtin arguments must not be skipped");
    assert!(
        failure
            .into_diagnostics()
            .iter()
            .any(|error| error.message.contains("Cannot perform Minus")),
        "expected the nested expression diagnostic"
    );
}

#[test]
fn production_pipeline_keeps_builtin_parameter_contracts() {
    for source in [
        "store invalid as abs of \"not a number\"\n",
        "store invalid as call abs with \"not a number\"\n",
    ] {
        let diagnostics = typecheck_like_cli(source)
            .expect_err("the CLI-style type checker must retain builtin contracts")
            .into_diagnostics();
        assert!(
            diagnostics
                .iter()
                .any(|error| error.message.contains("expected Number")),
            "expected a numeric builtin argument diagnostic, got: {diagnostics:?}"
        );
    }
}

#[test]
fn pattern_find_all_uses_the_runtime_two_argument_contract() {
    typecheck(
        "create pattern letters:\n    one or more letter\nend pattern\n\
         store hits as pattern_find_all of \"aba\" and letters\n",
    )
    .unwrap_or_else(|failure| {
        panic!(
            "the runtime accepts exactly two arguments: {:?}",
            failure.into_diagnostics()
        )
    });

    let diagnostics = typecheck(
        "create pattern letters:\n    one or more letter\nend pattern\n\
         store hits as pattern_find_all of \"aba\" and letters and 1\n",
    )
    .expect_err("the runtime rejects a third argument")
    .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.message.contains("expects 2 arguments")),
        "expected a two-argument arity diagnostic, got: {diagnostics:?}"
    );
}

#[test]
fn bare_zero_argument_builtins_have_their_runtime_result_types() {
    for source in [
        "store value as random plus 1\n",
        "store token_size as length of generate_csrf_token\n",
    ] {
        typecheck(source).unwrap_or_else(|failure| {
            panic!(
                "the runtime auto-invokes a bare zero-argument builtin: {:?}",
                failure.into_diagnostics()
            )
        });
    }

    let diagnostics = typecheck("store invalid as today minus 1\n")
        .expect_err("today produces a Date, which cannot be subtracted from a number")
        .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.message.contains("Date"))
            && diagnostics
                .iter()
                .all(|error| !error.message.contains("Function")),
        "today must be typed as its auto-invoked Date result, got: {diagnostics:?}"
    );
}

#[test]
fn implemented_builtins_reject_concrete_runtime_type_mismatches() {
    for call in [
        "min of \"x\" and 1",
        "sqrt of \"x\"",
        "random_between of \"low\" and 10",
        "path_join of \"root\" and 2",
        "create_time of \"12\" and 30",
        "timestamp of 1",
    ] {
        let diagnostics = typecheck(&format!("store invalid as {call}\n"))
            .expect_err("the runtime rejects this concrete argument type")
            .into_diagnostics();
        assert!(
            diagnostics
                .iter()
                .any(|error| error.message.contains("Argument")),
            "expected a builtin argument diagnostic for {call}, got: {diagnostics:?}"
        );
    }
}

#[test]
fn builtin_overloads_match_runtime_supported_value_kinds() {
    for call in [
        "indexof of \"abc\" and \"b\"",
        "index_of of \"abc\" and \"b\"",
    ] {
        typecheck(&format!("store position as {call}\n")).unwrap_or_else(|failure| {
            panic!(
                "text index lookup is supported at runtime: {:?}",
                failure.into_diagnostics()
            )
        });
    }

    typecheck(
        r#"
listen on port 8080 as srv
wait for request comes in on srv as req
store byte_count as length of body_bytes
"#,
    )
    .unwrap_or_else(|failure| {
        panic!(
            "runtime length accepts Binary values: {:?}",
            failure.into_diagnostics()
        )
    });
}

#[test]
fn recognized_but_unimplemented_names_are_not_treated_as_callable_builtins() {
    for call in [
        "compile_pattern of \"a\"",
        "addmonths of today and 1",
        "list_directory of \".\"",
    ] {
        let diagnostics = typecheck(&format!("store invalid as {call}\n"))
            .expect_err("a name without a runtime implementation must fail before execution")
            .into_diagnostics();
        assert!(
            diagnostics
                .iter()
                .any(|error| error.message.contains("not implemented")),
            "expected an explicit unimplemented-builtin diagnostic for {call}, got: \
             {diagnostics:?}"
        );
    }

    let diagnostics = typecheck("store invalid as compile_pattern\n")
        .expect_err("a bare reference to an unimplemented builtin has no runtime value")
        .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.message.contains("not implemented")),
        "expected an explicit bare-reference diagnostic, got: {diagnostics:?}"
    );
}

#[test]
fn user_actions_can_use_names_reserved_for_future_builtins() {
    typecheck(
        r#"
define action called compile_pattern with parameters value as text:
    return value
end action

store result as compile_pattern of "ok"
store uppercase as touppercase of result
"#,
    )
    .unwrap_or_else(|failure| {
        panic!(
            "a real user action must win over an unimplemented reserved name: {:?}",
            failure.into_diagnostics()
        )
    });
}

#[test]
fn included_actions_can_use_names_reserved_for_future_builtins() {
    for call in [
        "compile_pattern of \"ok\"",
        "addmonths of today and 1",
        "list_directory of \".\"",
    ] {
        typecheck(&format!(
            "include from \"module.wfl\"\nstore result as {call}\ndisplay result\n"
        ))
        .unwrap_or_else(|failure| {
            panic!(
                "an include-exposed action must win over an unimplemented reserved name \
                 ({call}): {:?}",
                failure.into_diagnostics()
            )
        });
    }
}

#[test]
fn stored_builtin_references_preserve_runtime_contracts() {
    let diagnostics = typecheck(
        r#"
store magnitude as abs
store invalid as magnitude of "not a number"
"#,
    )
    .expect_err("a stored abs reference must retain its Number parameter")
    .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.message.contains("expected Number")),
        "expected the aliased builtin's numeric contract, got: {diagnostics:?}"
    );

    typecheck(
        r#"
store make_time as create_time
store clock as make_time of 12 and 30
store combine_paths as path_join
store combined as combine_paths of "root" and "child" and "file.txt"

listen on port 8080 as srv
wait for request comes in on srv as req
store measure as length
store byte_count as measure of body_bytes
"#,
    )
    .unwrap_or_else(|failure| {
        panic!(
            "stored optional, variadic, and overloaded builtins must keep their runtime \
             signatures: {:?}",
            failure.into_diagnostics()
        )
    });
}

#[test]
fn runtime_branded_builtin_types_do_not_accept_same_named_containers() {
    let diagnostics = typecheck(
        r#"
create container Date:
end
create new Date as date_container:
end
store rendered as format_date of date_container and "%Y-%m-%d"
"#,
    )
    .expect_err("a Date container is not the runtime's temporal Date value")
    .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.message.contains("format_date") && error.message.contains("Date")),
        "expected a branded Date contract diagnostic: {diagnostics:?}"
    );

    let diagnostics = typecheck(
        r#"
create container Date:
end
define action called render with parameters value as Date:
    return format_date of value and "%Y-%m-%d"
end action
"#,
    )
    .expect_err("a custom Date annotation is ambiguous when that container exists")
    .into_diagnostics();
    assert!(
        diagnostics.iter().any(|error| {
            error.message.contains("custom/container annotation")
                && error.message.contains("lowercase 'date'")
        }),
        "the rejection should explain how to request the temporal type: {diagnostics:?}"
    );

    let diagnostics = typecheck(
        r#"
create container DateTime:
end
define action called render with parameters value as DateTime:
    return format_datetime of value and "%Y-%m-%d"
end action
"#,
    )
    .expect_err("a custom DateTime annotation is ambiguous when that container exists")
    .into_diagnostics();
    assert!(
        diagnostics.iter().any(|error| {
            error.message.contains("no unambiguous DateTime spelling")
                && error.message.contains("rename the container")
        }),
        "DateTime guidance must not recommend lowercase datetime, which stays custom: \
         {diagnostics:?}"
    );
}
