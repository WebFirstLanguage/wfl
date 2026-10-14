//! Source-level constructor arguments for issue #786.

use std::{fs, process::Command};
use tempfile::TempDir;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::{Parser, ast::Statement};
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
        assert_eq!(arguments.len(), if source.contains(" and ") { 2 } else { 1 });
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
