//! Source-level constructor arguments for issue #786.

use std::{fs, process::Command};
use tempfile::TempDir;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::{Parser, ast::Statement};

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
