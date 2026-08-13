// TDD tests for interface contracts (Red first).
//
// Interfaces previously parsed only as bare declarations (`create interface X`)
// with no body and no enforcement: a container could claim `implements X` for
// any X and nothing ever checked conformance. These tests pin down the wired-up
// behavior:
//   * `create interface Name:` bodies with `requires action ...` signatures
//   * backward-compatible bare `create interface Name` (empty contract)
//   * `extends` between interfaces (requirements accumulate)
//   * runtime rejection of a container whose `implements` list is unsatisfied
//   * inherited container methods satisfy interface requirements

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::Statement;

mod test_helpers;
use test_helpers::*;

fn parse(source: &str) -> Result<wfl::parser::ast::Program, Vec<wfl::parser::ast::ParseError>> {
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    parser.parse()
}

// === Parser: interface bodies ===

#[test]
fn interface_body_with_required_actions_parses() {
    let source = r#"
create interface Drawable:
    requires action draw
    requires action get_area: Number
    requires action resize needs w: Number, h: Number
end
"#;
    let program = parse(source).expect("interface body should parse");
    let stmt = program
        .statements
        .iter()
        .find(|s| matches!(s, Statement::InterfaceDefinition { .. }))
        .expect("expected an InterfaceDefinition statement");

    if let Statement::InterfaceDefinition {
        name,
        required_actions,
        ..
    } = stmt
    {
        assert_eq!(name, "Drawable");
        assert_eq!(
            required_actions.len(),
            3,
            "all three required actions should be captured"
        );
        let draw = required_actions
            .iter()
            .find(|a| a.name == "draw")
            .expect("draw signature");
        assert!(draw.parameters.is_empty());
        assert!(draw.return_type.is_none());

        let get_area = required_actions
            .iter()
            .find(|a| a.name == "get_area")
            .expect("get_area signature");
        assert!(get_area.return_type.is_some(), "return type recorded");

        let resize = required_actions
            .iter()
            .find(|a| a.name == "resize")
            .expect("resize signature");
        assert_eq!(resize.parameters.len(), 2, "parameters recorded");
    } else {
        unreachable!();
    }
}

#[test]
fn bare_interface_still_parses_as_empty_contract() {
    let source = r#"
create interface Drawable

create container Rectangle implements Drawable:
    property width: Number

    action draw:
        display "drawing"
    end
end
"#;
    let program = parse(source).expect("bare interface must keep parsing (backward compat)");
    let stmt = program
        .statements
        .iter()
        .find(|s| matches!(s, Statement::InterfaceDefinition { .. }))
        .expect("expected an InterfaceDefinition statement");
    if let Statement::InterfaceDefinition {
        required_actions, ..
    } = stmt
    {
        assert!(required_actions.is_empty());
    }
}

#[test]
fn interface_extends_parses() {
    let source = r#"
create interface Drawable:
    requires action draw
end

create interface Shape extends Drawable:
    requires action get_area: Number
end
"#;
    let program = parse(source).expect("interface extends should parse");
    let shape = program
        .statements
        .iter()
        .find_map(|s| match s {
            Statement::InterfaceDefinition { name, extends, .. } if name == "Shape" => {
                Some(extends.clone())
            }
            _ => None,
        })
        .expect("Shape interface parsed");
    assert_eq!(shape, vec!["Drawable".to_string()]);
}

#[test]
fn interface_requires_rejects_missing_action_keyword() {
    let source = r#"
create interface Broken:
    requires draw
end
"#;
    assert!(
        parse(source).is_err(),
        "'requires' without 'action' should be a parse error"
    );
}

// === Runtime: conformance enforcement ===

#[test]
fn container_satisfying_interface_runs() {
    let program = r#"
create interface Drawable:
    requires action draw
    requires action get_area: Number
end

create container Rectangle implements Drawable:
    property width: Number
    property height: Number

    action draw:
        display "Drawing rectangle: " with width with " x " with height
    end

    action get_area: Number
        return width times height
    end
end

create new Rectangle as rect:
    width is 10
    height is 5
end

rect.draw()
store area as rect.get_area()
display "Area: " with area
"#;
    let output = run_wfl_program(program, "iface_conformant");
    assert_wfl_success_with_output(
        &output,
        &["Drawing rectangle: 10 x 5", "Area: 50"],
        &["error"],
    );
}

#[test]
fn container_missing_required_action_fails_at_runtime() {
    let program = r#"
create interface Drawable:
    requires action draw
    requires action get_area: Number
end

create container Circle implements Drawable:
    property radius: Number

    action draw:
        display "Drawing circle"
    end
end

display "should not get here"
"#;
    let output = run_wfl_program(program, "iface_missing_action");
    assert!(
        !output.status.success(),
        "a container missing a required action must fail, got stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Circle") && stderr.contains("Drawable") && stderr.contains("get_area"),
        "error should name the container, interface, and missing action; got: {stderr}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("should not get here"),
        "program must not continue past the unsatisfied contract"
    );
}

#[test]
fn container_with_wrong_arity_fails_at_runtime() {
    let program = r#"
create interface Resizable:
    requires action resize needs w: Number, h: Number
end

create container Box implements Resizable:
    property size: Number

    action resize needs s: Number:
        store size as s
    end
end

display "should not get here"
"#;
    let output = run_wfl_program(program, "iface_wrong_arity");
    assert!(
        !output.status.success(),
        "an arity mismatch against the interface must fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("resize"),
        "error should name the mismatched action; got: {stderr}"
    );
}

#[test]
fn inherited_method_satisfies_interface() {
    let program = r#"
create interface Greeter:
    requires action greet
end

create container Person:
    property name: Text

    action greet:
        display "Hello, I am " with name
    end
end

create container Employee extends Person implements Greeter:
    property job_title: Text
end

create new Employee as bob:
    name is "Bob"
    job_title is "Developer"
end

bob.greet()
"#;
    let output = run_wfl_program(program, "iface_inherited");
    assert_wfl_success_with_output(&output, &["Hello, I am Bob"], &["error"]);
}

#[test]
fn extended_interface_requirements_are_enforced() {
    let program = r#"
create interface Drawable:
    requires action draw
end

create interface Shape extends Drawable:
    requires action get_area: Number
end

create container Blob implements Shape:
    property size: Number

    action get_area: Number
        return size times size
    end
end

display "should not get here"
"#;
    let output = run_wfl_program(program, "iface_extends_enforced");
    assert!(
        !output.status.success(),
        "requirements inherited from an extended interface must be enforced"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("draw"),
        "error should name the missing inherited requirement; got: {stderr}"
    );
}

#[test]
fn implementing_unknown_interface_fails_at_runtime() {
    let program = r#"
create container Widget implements NoSuchInterface:
    property id: Number

    action ping:
        display "pong"
    end
end

display "should not get here"
"#;
    let output = run_wfl_program(program, "iface_unknown");
    assert!(
        !output.status.success(),
        "implementing an undefined interface must fail"
    );
}

#[test]
fn bare_interface_accepts_any_implementer_at_runtime() {
    // Backward compatibility: existing programs use `create interface X` with
    // no body; that is an empty contract every container satisfies.
    let program = r#"
create interface Marker

create container Anything implements Marker:
    property id: Number

    action ping:
        display "pong"
    end
end

create new Anything as a:
    id is 1
end

a.ping()
"#;
    let output = run_wfl_program(program, "iface_bare_ok");
    assert_wfl_success_with_output(&output, &["pong"], &["error"]);
}
