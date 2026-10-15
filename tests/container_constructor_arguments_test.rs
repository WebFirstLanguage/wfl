//! Source-level constructor arguments for issue #786.

use std::{fs, process::Command};
use tempfile::TempDir;
use wfl::interpreter::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::{
    Parser,
    ast::{Program, Statement},
};
use wfl::typechecker::TypeChecker;

const CONSTRUCTOR: &str = r#"create container K:
    property value: Number
    action initialize needs amount: Number:
        change value to value plus amount
        this.configure()
    end
    action configure:
        change value to value times 2
    end
end
create new K with 3 as k:
    value is 0
end
display k.value
"#;

fn run(source: &str) -> (Option<i32>, String, String) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("main.wfl");
    fs::write(&path, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_wfl"))
        .arg(&path)
        .current_dir(dir.path())
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn type_errors(source: &str) -> Vec<String> {
    let program = Parser::new(&lex_wfl_with_positions(source))
        .parse()
        .unwrap();
    TypeChecker::new()
        .check_types(&program)
        .err()
        .map(|error| {
            error
                .into_diagnostics()
                .into_iter()
                .map(|d| d.message)
                .collect()
        })
        .unwrap_or_default()
}

fn parse(source: &str) -> Program {
    Parser::new(&lex_wfl_with_positions(source))
        .parse()
        .unwrap_or_else(|error| panic!("parse failed: {error:?}"))
}

fn container_with_rejecting_initialize() -> &'static str {
    r#"create container K:
    property value: Number
    action initialize needs amount: Number:
        check if amount is 0:
            call raise_error with "initialize failed: amount is 0. Use a nonzero amount."
        end check
        change value to amount
    end
end
"#
}

fn recover_after_failed_create(failed_create: &str, successful_create: &str) -> String {
    format!(
        r#"{container}
store after_catch as "unset"
store rebound as "no"
store rebound_value as 0
try:
    {failed_create}
    change after_catch to "unexpected_success"
when error:
    store seen as "missing"
    try:
        display k.value
        change seen to "bound"
    when error:
        change seen to "unbound"
    end try
    change after_catch to seen
    {successful_create}
    change rebound to "yes"
    change rebound_value to k.value
end try
display "after_catch=" with after_catch
display "rebound=" with rebound
display "rebound_value=" with rebound_value
"#,
        container = container_with_rejecting_initialize(),
        failed_create = failed_create,
        successful_create = successful_create,
    )
}

#[test]
fn parser_keeps_arguments_and_property_body() {
    let source = "create new K with 3 and 4 as k:\n    value is 0\nend\n";
    let program = Parser::new(&lex_wfl_with_positions(source))
        .parse()
        .unwrap();
    let Statement::ContainerInstantiation {
        arguments,
        property_initializers,
        ..
    } = &program.statements[0]
    else {
        panic!("expected instantiation")
    };
    assert_eq!(arguments.len(), 2);
    assert!(arguments.iter().all(|argument| argument.name.is_none()));
    assert_eq!(property_initializers.len(), 1);
}

#[test]
fn constructor_runs_after_property_body_with_this() {
    let (status, out, err) = run(CONSTRUCTOR);
    assert_eq!(status, Some(0), "{out}\n{err}");
    assert!(out.contains('6'), "{out}\n{err}");
}

#[test]
fn text_argument_containing_and_as_stays_one_argument() {
    let source = "create new K with \"a and b as c\" as k:\nend\n";
    let program = Parser::new(&lex_wfl_with_positions(source))
        .parse()
        .unwrap();
    let Statement::ContainerInstantiation { arguments, .. } = &program.statements[0] else {
        panic!("expected instantiation")
    };
    assert_eq!(arguments.len(), 1);
}

#[test]
fn constructor_arguments_accept_streaming_clause_names() {
    for source in [
        "create new K with 1 and headers as k:\nend\n",
        "create new K with 1 and content_type as k:\nend\n",
        "create new K with \"a\" with headers as k:\nend\n",
    ] {
        let program = Parser::new(&lex_wfl_with_positions(source))
            .parse()
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        let Statement::ContainerInstantiation { arguments, .. } = &program.statements[0] else {
            panic!("expected instantiation")
        };
        assert_eq!(
            arguments.len(),
            if source.contains(" and ") { 2 } else { 1 }
        );
    }
}

#[test]
fn existing_form_has_no_arguments() {
    let program = Parser::new(&lex_wfl_with_positions("create new K as k:\nend\n"))
        .parse()
        .unwrap();
    let Statement::ContainerInstantiation { arguments, .. } = &program.statements[0] else {
        panic!("expected instantiation")
    };
    assert!(arguments.is_empty());
}

#[test]
fn named_constructor_arguments_are_rejected() {
    let error = Parser::new(&lex_wfl_with_positions(
        "create new K with amount: 3 as k:\nend\n",
    ))
    .parse()
    .unwrap_err();
    assert!(format!("{error:?}").contains("positional"), "{error:?}");
}

#[test]
fn misplaced_constructor_arguments_are_rejected_with_guidance() {
    let error = Parser::new(&lex_wfl_with_positions("create new K as k with 3:\nend\n"))
        .parse()
        .unwrap_err();
    assert!(format!("{error:?}").contains("before 'as'"), "{error:?}");
}

#[test]
fn parenthesized_constructor_arguments_are_rejected() {
    let error = Parser::new(&lex_wfl_with_positions("create new K(3) as k:\nend\n"))
        .parse()
        .unwrap_err();
    assert!(format!("{error:?}").contains("'as'"), "{error:?}");
}

#[test]
fn constructor_argument_can_concatenate_text() {
    let source = r#"create container K:
    property buf: Text
    action initialize needs message: Text:
        change buf to message
    end
end
create new K with "a" with "b" as k:
    buf is ""
end
display k.buf
"#;
    let (status, out, err) = run(source);
    assert_eq!(status, Some(0), "{out}\n{err}");
    assert!(out.contains("ab"), "{out}\n{err}");
}

#[test]
fn concatenation_does_not_swallow_the_next_constructor_argument() {
    let source = r#"create container K:
    property buf: text
    action initialize needs first: text, second: text:
        change buf to first with second
    end
end
create new K with "a" with "b" and "c" as k:
    buf is ""
end
display k.buf
"#;
    let program = Parser::new(&lex_wfl_with_positions(source))
        .parse()
        .unwrap();
    let Statement::ContainerInstantiation { arguments, .. } = &program.statements[1] else {
        panic!("expected instantiation")
    };
    assert_eq!(arguments.len(), 2);
    let (status, out, err) = run(source);
    assert_eq!(status, Some(0), "{out}\n{err}");
    assert!(out.contains("abc"), "{out}\n{err}");
}

#[test]
fn inherited_initialize_is_checked_and_runs() {
    let source = r#"create container Parent:
    property value: Number
    action initialize needs amount: Number:
        change value to amount
    end
end
create container Child extends Parent:
end
create new Child with 7 as child:
    value is 0
end
display child.value
"#;
    assert!(type_errors(source).is_empty(), "{:?}", type_errors(source));
    let (status, out, err) = run(source);
    assert_eq!(status, Some(0), "{out}\n{err}");
    assert!(out.contains('7'), "{out}\n{err}");
}

#[test]
fn child_override_wins_over_parent_signature_and_dispatch() {
    let source = r#"create container Parent:
    property value: Number
    action initialize needs label: Text:
        change value to 99
    end
end
create container Child extends Parent:
    action initialize needs amount: Number:
        change value to amount times 2
    end
end
create new Child with 4 as child:
    value is 0
end
display child.value
"#;
    assert!(type_errors(source).is_empty(), "{:?}", type_errors(source));
    let (status, out, err) = run(source);
    assert_eq!(status, Some(0), "{out}\n{err}");
    assert!(out.contains('8'), "{out}\n{err}");
}

#[test]
fn missing_initialize_names_container_and_action() {
    let errors = type_errors("create container K:\nend\ncreate new K with 3 as k:\nend\n");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("K") && e.contains("initialize")),
        "{errors:?}"
    );
}

#[test]
fn wrong_constructor_count_and_type_name_container_and_action() {
    let definition =
        "create container K:\n    action initialize needs amount: Number:\n    end\nend\n";
    for (creation, needle) in [
        (
            "create new K with 3 and 4 as k:\nend\n",
            "expects 1 arguments",
        ),
        ("create new K with \"bad\" as k:\nend\n", "expects Number"),
    ] {
        let errors = type_errors(&format!("{definition}{creation}"));
        assert!(
            errors
                .iter()
                .any(|e| e.contains("K") && e.contains("initialize") && e.contains(needle)),
            "{errors:?}"
        );
    }
}

#[test]
fn unresolved_parent_does_not_claim_initialize_is_missing() {
    let source = "create container Child extends IncludedParent:\nend\ncreate new Child with 3 as child:\nend\n";
    let errors = type_errors(source);
    assert!(
        !errors
            .iter()
            .any(|e| e.contains("has no initialize method")),
        "{errors:?}"
    );
}

#[test]
fn initialize_error_inside_try_leaves_name_unbound_and_allows_recreate() {
    let source = recover_after_failed_create(
        "create new K with 0 as k:\n        value is 0\n    end",
        "create new K with 3 as k:\n        value is 0\n    end",
    );
    let (status, out, err) = run(&source);
    assert_eq!(status, Some(0), "{out}\n{err}");
    assert!(
        out.contains("after_catch=unbound"),
        "failed initialize must not leave k bound for the catch handler\n{out}\n{err}"
    );
    assert!(
        out.contains("rebound=yes"),
        "re-creating k after a failed initialize must succeed\n{out}\n{err}"
    );
    assert!(
        out.contains("rebound_value=3"),
        "the recovered instance must be the successful construction\n{out}\n{err}"
    );
    assert!(
        !err.contains("already been defined"),
        "the catch handler must not see a stale k binding\n{out}\n{err}"
    );
}

#[test]
fn argument_error_inside_try_leaves_name_unbound_and_allows_recreate() {
    let source = recover_after_failed_create(
        "create new K with 1 divided by 0 as k:\n        value is 0\n    end",
        "create new K with 3 as k:\n        value is 0\n    end",
    );
    let (status, out, err) = run(&source);
    assert_eq!(status, Some(0), "{out}\n{err}");
    assert!(
        out.contains("after_catch=unbound"),
        "a throwing constructor argument must not leave k bound for the catch handler\n{out}\n{err}"
    );
    assert!(
        out.contains("rebound=yes"),
        "re-creating k after a throwing constructor argument must succeed\n{out}\n{err}"
    );
    assert!(
        out.contains("rebound_value=3"),
        "the recovered instance must be the successful construction\n{out}\n{err}"
    );
    assert!(
        !err.contains("already been defined"),
        "the catch handler must not see a stale k binding\n{out}\n{err}"
    );
}

#[test]
fn successful_construction_still_binds_and_runs_initialize() {
    let (status, out, err) = run(CONSTRUCTOR);
    assert_eq!(status, Some(0), "{out}\n{err}");
    assert!(out.contains('6'), "{out}\n{err}");
}

#[test]
fn no_arg_form_does_not_call_initialize() {
    let source = format!(
        r#"{container}
create new K as k:
    value is 4
end
display k.value
"#,
        container = container_with_rejecting_initialize()
    );
    let (status, out, err) = run(&source);
    assert_eq!(status, Some(0), "{out}\n{err}");
    assert!(out.contains('4'), "{out}\n{err}");
}

#[tokio::test]
async fn failed_initialize_leaves_the_environment_unchanged() {
    let failed = format!(
        r#"{container}
create new K with 0 as k:
    value is 0
end
"#,
        container = container_with_rejecting_initialize()
    );
    let mut interpreter = Interpreter::new();
    let errors = interpreter
        .interpret(&parse(&failed))
        .await
        .expect_err("initialize must raise when amount is 0");
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("amount is 0")),
        "{errors:?}"
    );
    assert!(
        interpreter.global_env().borrow().get("k").is_none(),
        "a failed initialize must not bind k"
    );

    let recovered = interpreter
        .interpret(&parse("create new K with 3 as k:\n    value is 0\nend\n"))
        .await
        .unwrap_or_else(|errors| panic!("re-create after failed initialize: {errors:?}"));
    let Some(wfl::interpreter::value::Value::ContainerInstance(k)) =
        interpreter.global_env().borrow().get("k")
    else {
        panic!("re-create must bind k; last value was {recovered:?}");
    };
    assert_eq!(
        k.borrow().properties.get("value"),
        Some(&wfl::interpreter::value::Value::Number(3.0))
    );
}

#[tokio::test]
async fn failed_constructor_argument_leaves_the_environment_unchanged() {
    let failed = format!(
        r#"{container}
create new K with 1 divided by 0 as k:
    value is 0
end
"#,
        container = container_with_rejecting_initialize()
    );
    let mut interpreter = Interpreter::new();
    let errors = interpreter
        .interpret(&parse(&failed))
        .await
        .expect_err("argument evaluation must raise");
    assert!(
        errors
            .iter()
            .any(|error| error.message.to_lowercase().contains("zero")),
        "{errors:?}"
    );
    assert!(
        interpreter.global_env().borrow().get("k").is_none(),
        "a throwing constructor argument must not bind k"
    );

    interpreter
        .interpret(&parse("create new K with 3 as k:\n    value is 0\nend\n"))
        .await
        .unwrap_or_else(|errors| panic!("re-create after throwing argument: {errors:?}"));
    let Some(wfl::interpreter::value::Value::ContainerInstance(k)) =
        interpreter.global_env().borrow().get("k")
    else {
        panic!("re-create must bind k");
    };
    assert_eq!(
        k.borrow().properties.get("value"),
        Some(&wfl::interpreter::value::Value::Number(3.0))
    );
}
