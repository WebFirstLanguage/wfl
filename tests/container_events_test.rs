//! End-to-end coverage for container events (issue #685).
//!
//! Before this suite, `event` and `trigger` parsed but the handler side was
//! unreachable: the `on ...` handler statement could not be written at all, its
//! body was dropped by the parser, `trigger` never parsed arguments, and
//! registration wrote to a different place than the one `trigger` read. A
//! program could therefore `trigger` all day and look successful while running
//! nothing. Every test here asserts an *observable effect* of a handler having
//! run — never merely that the program did not crash.
//!
//! The programs run through the real `wfl` binary, so the whole pipeline
//! (lexer → parser → analyzer → type checker → interpreter) is exercised.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Absolute path to the `wfl` binary built for this integration-test run.
fn wfl_exe() -> &'static str {
    env!("CARGO_BIN_EXE_wfl")
}

/// Run inline WFL source in a fresh temp dir, returning (stdout+stderr, exit code).
fn run_src(src: &str) -> (String, Option<i32>) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("main.wfl");
    fs::write(&path, src).unwrap();
    let output = Command::new(wfl_exe())
        .arg(&path)
        .output()
        .expect("failed to execute WFL");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    drop(dir);
    (combined, output.status.code())
}

/// Assert the program ran cleanly, surfacing the captured output on failure.
fn assert_ran_clean(out: &str, code: Option<i32>, what: &str) {
    assert_eq!(code, Some(0), "{what} should exit 0, got:\n{out}");
    assert!(
        !out.contains("error["),
        "{what} should report no diagnostics, got:\n{out}"
    );
}

#[test]
fn handler_registered_on_an_instance_runs_when_a_method_triggers() {
    let (out, code) = run_src(
        r#"
create container Button:
    property label: Text
    event on_click
    action click:
        display "clicking"
        trigger on_click
    end
end

create new Button as btn:
    label is "Submit"
end

on on_click of btn:
    display "handler ran"
end on

btn.click()
"#,
    );

    assert_ran_clean(&out, code, "an instance event handler");
    assert!(
        out.contains("handler ran"),
        "the registered handler must actually run, got:\n{out}"
    );
}

#[test]
fn trigger_arguments_bind_to_the_events_needs_parameters() {
    let (out, code) = run_src(
        r#"
create container Slider:
    event on_change needs old_value: Number, new_value: Number
    action set_to needs v: Number:
        trigger on_change with 10 and v
    end
end

create new Slider as s:
end

on on_change of s:
    display "changed " with old_value with " -> " with new_value
end on

s.set_to(42)
"#,
    );

    assert_ran_clean(&out, code, "an event with needs parameters");
    assert!(
        out.contains("changed 10 -> 42"),
        "trigger arguments must bind to the event's declared parameters, got:\n{out}"
    );
}

#[test]
fn parameters_without_a_trigger_argument_bind_to_nothing() {
    let (out, code) = run_src(
        r#"
event tick needs n: Number

on tick:
    display "n is " with n
end on

trigger tick
"#,
    );

    assert_ran_clean(&out, code, "a trigger with no arguments");
    assert!(
        out.contains("n is nothing"),
        "an unsupplied event parameter must bind to nothing, got:\n{out}"
    );
}

#[test]
fn every_registered_handler_runs_in_registration_order() {
    let (out, code) = run_src(
        r#"
create container Bell:
    event ring
    action strike:
        trigger ring
    end
end

create new Bell as b:
end

on ring of b:
    display "first"
end on

on ring of b:
    display "second"
end on

b.strike()
"#,
    );

    assert_ran_clean(&out, code, "multiple handlers");
    let first = out.find("first").expect("the first handler must run");
    let second = out.find("second").expect("the second handler must run");
    assert!(
        first < second,
        "handlers must run in registration order, got:\n{out}"
    );
}

#[test]
fn handlers_are_registered_per_instance_not_per_container() {
    let (out, code) = run_src(
        r#"
create container Bell:
    property name: Text
    event ring
    action strike:
        display "striking " with name
        trigger ring
    end
end

create new Bell as loud:
    name is "loud"
end
create new Bell as quiet:
    name is "quiet"
end

on ring of loud:
    display "only loud handles this"
end on

quiet.strike()
loud.strike()
"#,
    );

    assert_ran_clean(&out, code, "per-instance handlers");
    assert_eq!(
        out.matches("only loud handles this").count(),
        1,
        "a handler registered on one instance must not fire for another, got:\n{out}"
    );
    // Ordering proves the one run belonged to `loud`, not to `quiet`.
    let quiet_strike = out.find("striking quiet").expect("quiet must be struck");
    let handled = out.find("only loud handles this").expect("handler must run");
    assert!(
        quiet_strike < handled,
        "the handler must have run for loud, after quiet was struck, got:\n{out}"
    );
}

#[test]
fn a_handler_sees_the_scope_it_was_registered_in() {
    let (out, code) = run_src(
        r#"
create container Bell:
    event ring
    action strike:
        trigger ring
    end
end

create new Bell as b:
end

store greeting as "captured from the registration scope"

on ring of b:
    display greeting
end on

b.strike()
"#,
    );

    assert_ran_clean(&out, code, "a closing-over handler");
    assert!(
        out.contains("captured from the registration scope"),
        "a handler body must see the variables in scope where it was registered, got:\n{out}"
    );
    assert!(
        !out.contains("Unused variable 'greeting'"),
        "a variable read only by a handler body must not be reported unused, got:\n{out}"
    );
}

#[test]
fn a_handler_can_mutate_state_visible_to_the_rest_of_the_program() {
    let (out, code) = run_src(
        r#"
create container Bell:
    event ring
    action strike:
        trigger ring
    end
end

create new Bell as b:
end

store rings as []

on ring of b:
    push with rings and "ding"
end on

b.strike()
b.strike()
b.strike()

display "rang " with length of rings with " times"
"#,
    );

    assert_ran_clean(&out, code, "a state-mutating handler");
    assert!(
        out.contains("rang 3 times"),
        "each trigger must run the handler exactly once, got:\n{out}"
    );
}

#[test]
fn events_are_inherited_by_extending_containers() {
    let (out, code) = run_src(
        r#"
create container Base:
    event started
    action begin:
        trigger started
    end
end

create container Derived extends Base:
    action go:
        trigger started
    end
end

create new Derived as d:
end

on started of d:
    display "derived started"
end on

d.begin()
d.go()
"#,
    );

    assert_ran_clean(&out, code, "an inherited event");
    assert_eq!(
        out.matches("derived started").count(),
        2,
        "an inherited event must dispatch from both inherited and own actions, got:\n{out}"
    );
}

#[test]
fn a_bare_handler_attaches_to_a_top_level_event() {
    let (out, code) = run_src(
        r#"
event data_ready needs payload: Text

on data_ready:
    display "got " with payload
end on

trigger data_ready with "the goods"
"#,
    );

    assert_ran_clean(&out, code, "a top-level event");
    assert!(
        out.contains("got the goods"),
        "the bare `on <event>:` form must attach to an event in scope, got:\n{out}"
    );
}

#[test]
fn a_handler_registered_during_dispatch_runs_on_the_next_trigger() {
    // Registration mutates the very handler list being iterated, so this also
    // guards against a borrow panic in the shared handler storage.
    let (out, code) = run_src(
        r#"
create container Bell:
    event ring
    action strike:
        trigger ring
    end
end

create new Bell as b:
end

on ring of b:
    display "outer"
    on ring of b:
        display "added during dispatch"
    end on
end on

b.strike()
display "--- second strike ---"
b.strike()
"#,
    );

    assert_ran_clean(&out, code, "a handler registered during dispatch");
    let marker = out
        .find("--- second strike ---")
        .expect("both strikes must happen");
    let added = out
        .find("added during dispatch")
        .expect("the newly registered handler must run on a later trigger");
    assert!(
        added > marker,
        "a handler added during dispatch must not run in that same dispatch, got:\n{out}"
    );
}

#[test]
fn a_self_triggering_handler_hits_the_recursion_ceiling_instead_of_crashing() {
    let (out, code) = run_src(
        r#"
event loopy

on loopy:
    trigger loopy
end on

trigger loopy
"#,
    );

    assert_eq!(
        code,
        Some(1),
        "runaway event recursion must fail cleanly, got:\n{out}"
    );
    assert!(
        out.contains("Maximum call depth"),
        "runaway event recursion must report the recursion ceiling rather than \
         overflowing the native stack, got:\n{out}"
    );
}

#[test]
fn registering_a_handler_for_an_unknown_event_is_an_error() {
    let (out, code) = run_src(
        r#"
create container Button:
    event on_click
end

create new Button as btn:
end

on no_such_event of btn:
    display "never"
end on
"#,
    );

    assert_ne!(code, Some(0), "an unknown event must fail, got:\n{out}");
    assert!(
        out.contains("Event 'no_such_event' not found in container 'Button'"),
        "the error must name the event and the container, got:\n{out}"
    );
    assert!(
        !out.contains("never"),
        "the handler body must not run, got:\n{out}"
    );
}

#[test]
fn registering_a_handler_on_a_non_container_is_an_error() {
    let (out, code) = run_src(
        r#"
store x as 5

on thing of x:
    display "never"
end on
"#,
    );

    assert_ne!(code, Some(0), "a non-container source must fail, got:\n{out}");
    assert!(
        out.contains("Cannot add event handler to non-container value"),
        "the error must explain the source is not a container, got:\n{out}"
    );
}

#[test]
fn a_bare_handler_for_an_event_that_is_not_in_scope_is_an_error() {
    let (out, code) = run_src(
        r#"
on nothing_here:
    display "never"
end on
"#,
    );

    assert_ne!(code, Some(0), "an unknown event must fail, got:\n{out}");
    assert!(
        out.contains("Event 'nothing_here' not found"),
        "the error must name the missing event, got:\n{out}"
    );
}

#[test]
fn an_unclosed_handler_block_is_a_parse_error() {
    let (out, code) = run_src(
        r#"
event tick

on tick:
    display "never"
"#,
    );

    assert_ne!(code, Some(0), "an unclosed handler must fail, got:\n{out}");
    assert!(
        out.contains("end on"),
        "the parse error must point at the missing 'end on', got:\n{out}"
    );
}

/// The parser must keep the handler's statements. Before the fix,
/// `parse_event_handler` returned `handler_body: Vec::new()` unconditionally,
/// which this asserts against directly rather than through observed output.
#[test]
fn the_parser_keeps_the_handler_body_and_trigger_arguments() {
    use wfl::lexer::lex_wfl_with_positions;
    use wfl::parser::Parser;
    use wfl::parser::ast::Statement;

    let source = r#"
create container Button:
    event on_click needs which: Text
    action click:
        trigger on_click with "left"
    end
end

create new Button as btn:
end

on on_click of btn:
    display "one"
    display "two"
end on
"#;

    let tokens = lex_wfl_with_positions(source);
    let program = Parser::new(&tokens)
        .parse()
        .expect("the handler grammar must parse");

    let handler = program
        .statements
        .iter()
        .find_map(|statement| match statement {
            Statement::EventHandler {
                event_name,
                handler_body,
                ..
            } => Some((event_name, handler_body)),
            _ => None,
        })
        .expect("the program must contain an event handler");

    assert_eq!(handler.0, "on_click");
    assert_eq!(
        handler.1.len(),
        2,
        "both statements under the handler must be attached to it"
    );

    // `trigger <event> with <args>` inside the container action.
    let container_events = program
        .statements
        .iter()
        .find_map(|statement| match statement {
            Statement::ContainerDefinition { methods, .. } => Some(methods),
            _ => None,
        })
        .expect("the program must contain a container definition");
    let trigger_arg_count = container_events
        .iter()
        .filter_map(|method| match method {
            Statement::ActionDefinition { body, .. } => Some(body),
            _ => None,
        })
        .flatten()
        .find_map(|statement| match statement {
            Statement::EventTrigger { arguments, .. } => Some(arguments.len()),
            _ => None,
        })
        .expect("the action must contain an event trigger");
    assert_eq!(
        trigger_arg_count, 1,
        "`trigger <event> with <value>` must record its argument"
    );
}

/// The declaration grammar is shared between the in-container and statement
/// forms, so a top-level `event` accepts `needs` parameters too.
#[test]
fn a_top_level_event_declaration_keeps_its_needs_parameters() {
    use wfl::lexer::lex_wfl_with_positions;
    use wfl::parser::Parser;
    use wfl::parser::ast::Statement;

    let tokens = lex_wfl_with_positions("event resized needs width: Number, height: Number\n");
    let program = Parser::new(&tokens)
        .parse()
        .expect("a top-level event declaration must parse");

    let parameters = program
        .statements
        .iter()
        .find_map(|statement| match statement {
            Statement::EventDefinition { parameters, .. } => Some(parameters),
            _ => None,
        })
        .expect("the program must contain an event definition");

    let names: Vec<&str> = parameters.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["width", "height"]);
}

/// `--lint --fix` reprints the whole program from the AST, so event syntax has
/// to survive a round trip rather than being emitted as Rust debug output.
#[test]
fn the_fixer_reprints_event_syntax_so_it_parses_again() {
    use wfl::fixer::CodeFixer;
    use wfl::lexer::lex_wfl_with_positions;
    use wfl::parser::Parser;

    let source = r#"event resized needs width: Number
on resized:
    display width
end on
trigger resized with 800
"#;

    let tokens = lex_wfl_with_positions(source);
    let program = Parser::new(&tokens).parse().expect("source must parse");

    let (fixed, _summary) = CodeFixer::new().fix(&program, source);

    let refixed_tokens = lex_wfl_with_positions(&fixed);
    Parser::new(&refixed_tokens)
        .parse()
        .unwrap_or_else(|errors| panic!("fixer output must parse again:\n{fixed}\n{errors:?}"));

    for expected in ["event resized needs width", "on resized:", "end on", "trigger resized with"]
    {
        assert!(
            fixed.contains(expected),
            "fixer output should contain {expected:?}, got:\n{fixed}"
        );
    }
}
