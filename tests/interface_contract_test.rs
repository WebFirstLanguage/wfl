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

// === Static/runtime parity for extends-chain names (PR #686 review) ===

#[test]
fn interface_extending_unknown_interface_fails_at_runtime() {
    let program = r#"
create interface Child extends NotAnInterface:
    requires action ping
end

create container Widget implements Child:
    property id: Number

    action ping:
        display "pong"
    end
end

display "should not get here"
"#;
    let output = run_wfl_program(program, "iface_extends_unknown_runtime");
    assert!(
        !output.status.success(),
        "an interface extending an undefined name must fail when implemented"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("NotAnInterface"),
        "error should name the unknown parent interface; got: {stderr}"
    );
}

#[test]
fn typechecker_reports_unknown_extended_interface() {
    // The static check must not silently skip extends-chain names the way it
    // does for direct `implements` entries (those are reported separately):
    // runtime rejects this program, so `wfl --analyze`-level tooling must too.
    use wfl::typechecker::TypeChecker;

    let source = r#"
create interface Child extends NotAnInterface:
    requires action ping
end

create container Widget implements Child:
    property id: Number

    action ping:
        display "pong"
    end
end
"#;
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("test program should parse");
    let diagnostics = TypeChecker::new()
        .check_types(&program)
        .expect_err("extending an undefined interface must be a static error")
        .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.message.contains("NotAnInterface")),
        "expected a diagnostic naming the unknown parent interface: {diagnostics:?}"
    );
}

#[test]
fn typechecker_reports_non_interface_extended_name() {
    use wfl::typechecker::TypeChecker;

    let source = r#"
create container NotReallyAnInterface:
    property id: Number
end

create interface Child extends NotReallyAnInterface:
    requires action ping
end

create container Widget implements Child:
    property id: Number

    action ping:
        display "pong"
    end
end
"#;
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("test program should parse");
    let diagnostics = TypeChecker::new()
        .check_types(&program)
        .expect_err("extending a container instead of an interface must be a static error")
        .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.message.contains("NotReallyAnInterface")
                && error.message.contains("not an interface")),
        "expected a diagnostic naming the non-interface parent: {diagnostics:?}"
    );
}

#[test]
fn typechecker_skips_conformance_when_parent_chain_unresolvable() {
    // A container whose extends-parent the analyzer cannot resolve (e.g. a
    // parent supplied by `include from`) must not produce a false "missing
    // required action" diagnostic — the parent may well provide the action.
    // The unresolved parent itself is already reported separately.
    use wfl::typechecker::TypeChecker;

    let source = r#"
create interface Greeter:
    requires action greet
end

create container Employee extends UnresolvableParent implements Greeter:
    property job_title: Text
end
"#;
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("test program should parse");
    let diagnostics = match TypeChecker::new().check_types(&program) {
        Ok(()) => Vec::new(),
        Err(e) => e.into_diagnostics(),
    };
    assert!(
        !diagnostics
            .iter()
            .any(|error| error.message.contains("does not satisfy interface")),
        "an unresolvable parent chain must suppress conformance diagnostics, got: {diagnostics:?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|error| error.message.contains("UnresolvableParent")),
        "the unresolved parent itself should still be reported: {diagnostics:?}"
    );
}

#[test]
fn typechecker_reports_interface_return_type_mismatch() {
    use wfl::typechecker::TypeChecker;

    let source = r#"
create interface Measurable:
    requires action get_area: Number
end

create container Card implements Measurable:
    property label: Text

    action get_area: Text
        return label
    end
end
"#;
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("test program should parse");
    let diagnostics = TypeChecker::new()
        .check_types(&program)
        .expect_err(
            "a matching-arity action with an incompatible return type must be a static error",
        )
        .into_diagnostics();
    assert!(
        diagnostics.iter().any(|error| {
            error.message.contains("get_area") && error.message.contains("return")
        }),
        "expected a return-type conformance diagnostic for get_area: {diagnostics:?}"
    );
}

#[test]
fn static_action_does_not_satisfy_interface() {
    // Interface contracts are instance contracts: a static action with the
    // right name must not satisfy `requires action`.
    let program = r#"
create interface Drawable:
    requires action draw
end

create container Chart implements Drawable:
    property title: Text

    static action draw:
        display "static draw"
    end
end

display "should not get here"
"#;
    let output = run_wfl_program(program, "iface_static_not_satisfying");
    assert!(
        !output.status.success(),
        "a static action must not satisfy an instance interface requirement"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("draw"),
        "error should name the missing instance action; got: {stderr}"
    );
}

#[test]
fn implementing_a_container_instead_of_an_interface_fails_at_runtime() {
    let program = r#"
create container NotAnInterface:
    property id: Number
end

create container Widget implements NotAnInterface:
    property id: Number
end

display "should not get here"
"#;
    let output = run_wfl_program(program, "iface_not_an_interface");
    assert!(
        !output.status.success(),
        "implementing a non-interface must fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("is not an interface"),
        "error should explain the target is not an interface; got: {stderr}"
    );
}

#[test]
fn typed_parameter_and_return_type_colon_boundary() {
    // Pin the colon grammar for interface signatures: the first colon after a
    // parameter name annotates the parameter; a further colon sets the
    // required return type.
    let source = r#"
create interface Sizer:
    requires action set_size needs value: Number
    requires action scaled_size needs factor: Number: Number
end
"#;
    let program = parse(source).expect("both colon forms should parse");
    let stmt = program
        .statements
        .iter()
        .find(|s| matches!(s, Statement::InterfaceDefinition { .. }))
        .expect("expected an InterfaceDefinition statement");
    if let Statement::InterfaceDefinition {
        required_actions, ..
    } = stmt
    {
        let set_size = required_actions
            .iter()
            .find(|a| a.name == "set_size")
            .expect("set_size signature");
        assert_eq!(set_size.parameters.len(), 1);
        assert!(
            set_size.parameters[0].param_type.is_some(),
            "first colon annotates the parameter"
        );
        assert!(
            set_size.return_type.is_none(),
            "no second colon means no required return type"
        );

        let scaled = required_actions
            .iter()
            .find(|a| a.name == "scaled_size")
            .expect("scaled_size signature");
        assert_eq!(scaled.parameters.len(), 1);
        assert!(scaled.parameters[0].param_type.is_some());
        assert!(
            scaled.return_type.is_some(),
            "second colon sets the required return type"
        );
    }
}

#[test]
fn unterminated_interface_body_reports_real_position() {
    let source = "create interface Broken:\n    requires action draw\n";
    let errors = parse(source).expect_err("missing 'end' must be a parse error");
    assert!(
        errors
            .iter()
            .any(|e| e.line > 0 && e.message.contains("interface body")),
        "the unterminated-body diagnostic should carry a real source position: {errors:?}"
    );
}

// === Fixer round-trip (PR #686 review): --fix output must re-parse ===

#[test]
fn fixer_roundtrip_preserves_interface_bodies() {
    use wfl::fixer::CodeFixer;

    let source = r#"create interface Drawable:
    requires action draw
    requires action get_area: Number
    requires action resize needs w: Number, h: Number
end
"#;
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("interface body should parse");

    let (fixed_code, _) = CodeFixer::new().fix(&program, source);

    let fixed_tokens = lex_wfl_with_positions(&fixed_code);
    let mut fixed_parser = Parser::new(&fixed_tokens);
    let reparsed = fixed_parser
        .parse()
        .unwrap_or_else(|e| panic!("fixer output must re-parse, got {e:?}\noutput:\n{fixed_code}"));

    let stmt = reparsed
        .statements
        .iter()
        .find(|s| matches!(s, Statement::InterfaceDefinition { .. }))
        .expect("re-parsed program should still contain the interface");
    if let Statement::InterfaceDefinition {
        required_actions, ..
    } = stmt
    {
        assert_eq!(
            required_actions.len(),
            3,
            "all required actions must survive the fix round-trip:\n{fixed_code}"
        );
        let resize = required_actions
            .iter()
            .find(|a| a.name == "resize")
            .expect("resize survives round-trip");
        assert_eq!(resize.parameters.len(), 2);
        let get_area = required_actions
            .iter()
            .find(|a| a.name == "get_area")
            .expect("get_area survives round-trip");
        assert!(get_area.return_type.is_some());
    }
}

#[test]
fn fixer_roundtrip_preserves_bare_interfaces() {
    use wfl::fixer::CodeFixer;

    let source = "create interface Marker\n";
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("bare interface should parse");

    let (fixed_code, _) = CodeFixer::new().fix(&program, source);

    let fixed_tokens = lex_wfl_with_positions(&fixed_code);
    let mut fixed_parser = Parser::new(&fixed_tokens);
    let reparsed = fixed_parser.parse().unwrap_or_else(|e| {
        panic!("fixer output for a bare interface must re-parse, got {e:?}\noutput:\n{fixed_code}")
    });
    assert!(
        reparsed
            .statements
            .iter()
            .any(|s| matches!(s, Statement::InterfaceDefinition { .. })),
        "bare interface must survive the fix round-trip:\n{fixed_code}"
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
