use wfl::interpreter::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::TypeChecker;

fn diagnostics(source: &str) -> Vec<String> {
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("test program should parse");
    TypeChecker::new()
        .check_types(&program)
        .expect_err("the concrete runtime binding must make subtraction invalid")
        .into_diagnostics()
        .into_iter()
        .map(|error| error.message)
        .collect()
}

fn assert_invalid_minus(source: &str, expected_type: &str) {
    let errors = diagnostics(source);
    assert!(
        errors.iter().any(|message| {
            message.contains("Cannot perform Minus") && message.contains(expected_type)
        }),
        "expected subtraction to see the runtime binding as {expected_type}, got: {errors:?}"
    );
}

#[test]
fn action_local_io_results_keep_their_concrete_types() {
    assert_invalid_minus(
        r#"
define action called inspect_file:
    open file at "unused.txt" and read content as contents
    store invalid as contents minus 1
end action
"#,
        "Text",
    );

    assert_invalid_minus(
        r#"
define action called inspect_http:
    open url at "https://example.com" and read content as reply
    store invalid as reply minus 1
end action
"#,
        "Text",
    );

    assert_invalid_minus(
        r#"
define action called inspect_execution:
    execute wfl file at "unused.wfl" and read output as page_text
    store invalid as page_text minus 1
end action
"#,
        "Text",
    );
}

#[test]
fn action_local_collection_and_calendar_bindings_keep_their_concrete_types() {
    assert_invalid_minus(
        r#"
define action called inspect_list:
    create list items:
        add 1
    end list
    store invalid as items minus 1
end action
"#,
        "List",
    );

    assert_invalid_minus(
        r#"
define action called inspect_map:
    create map details:
        label is "value"
    end map
    store invalid as details minus 1
end action
"#,
        "Map",
    );

    assert_invalid_minus(
        r#"
define action called inspect_date:
    create date due
    store invalid as due minus 1
end action
"#,
        "Date",
    );

    assert_invalid_minus(
        r#"
define action called inspect_time:
    create time started
    store invalid as started minus 1
end action
"#,
        "Time",
    );

    // The explicit forms are pass-through bindings at runtime, not coercions.
    assert_invalid_minus(
        r#"
define action called inspect_explicit_calendar_values:
    create date label_date as "not coerced"
    store invalid_date as label_date minus 1
end action
"#,
        "Text",
    );
    assert_invalid_minus(
        r#"
define action called inspect_explicit_clock_values:
    create time label_time as "not coerced"
    store invalid_time as label_time minus 1
end action
"#,
        "Text",
    );
}

#[test]
fn request_bindings_are_recreated_in_runtime_loop_scopes() {
    assert_invalid_minus(
        r#"
listen on port 8080 as srv
main loop:
    wait for request comes in on srv as req
    store invalid as body minus 1
    break
end loop
"#,
        "Text",
    );
}

#[test]
fn action_local_patterns_keep_their_pattern_type() {
    assert_invalid_minus(
        r#"
define action called inspect_pattern:
    create pattern local_pattern:
        "x"
    end pattern
    store invalid as local_pattern minus 1
end action
"#,
        "Pattern",
    );
}

#[test]
fn websocket_server_and_event_bindings_keep_their_runtime_types() {
    assert_invalid_minus(
        r#"
define action called inspect_websocket_server:
    listen for websockets on port 0 as ws_server
    store invalid as ws_server minus 1
end action
"#,
        "Text",
    );

    assert_invalid_minus(
        r#"
listen for websockets on port 0 as ws_server
on websocket message from ws_server as message_event:
    store invalid as message_event minus 1
end on
"#,
        "Map",
    );

    for event in ["connect to", "disconnect from"] {
        let source = format!(
            "listen for websockets on port 0 as ws_server\n\
             on websocket {event} ws_server as connection:\n\
                 store invalid as connection[\"id\"] minus 1\n\
             end on\n"
        );
        assert_invalid_minus(&source, "Text");
    }
}

#[tokio::test]
async fn explicit_method_local_binders_shadow_same_named_properties_at_runtime() {
    for source in [
        r#"
create container Worker:
    property item: Number defaults 0

    action run:
        for each item in ["ok"]:
            display touppercase of item
        end for
    end
end

create new Worker as worker:
end
store result as worker.run()
"#,
        r#"
create container Worker:
    property items: Number defaults 0

    action run:
        create list items:
            add "ok"
        end list
        display length of items
    end
end

create new Worker as worker:
end
store result as worker.run()
"#,
    ] {
        let tokens = lex_wfl_with_positions(source);
        let mut parser = Parser::new(&tokens);
        let program = parser.parse().expect("test program should parse");
        TypeChecker::new()
            .check_types(&program)
            .unwrap_or_else(|failure| {
                panic!(
                    "the explicit local binding should shadow the property statically: {:?}",
                    failure.into_diagnostics()
                )
            });

        let mut interpreter = Interpreter::new();
        interpreter
            .interpret(&program)
            .await
            .unwrap_or_else(|errors| {
                panic!("the runtime must create the same local binding: {errors:?}")
            });
    }
}

#[tokio::test]
async fn nested_declarations_shadow_same_named_properties_at_runtime() {
    for source in [
        r#"
create container Worker:
    property helper: Number defaults 0

    action run:
        define action called helper:
            return "ok"
        end action
        display call helper
    end
end

create new Worker as worker:
end
store result as worker.run()
"#,
        r#"
create container Worker:
    property LocalType: Number defaults 0

    action run:
        create container LocalType:
        end
    end
end

create new Worker as worker:
end
store result as worker.run()
"#,
        r#"
create container Worker:
    property LocalContract: Number defaults 0

    action run:
        create interface LocalContract
    end
end

create new Worker as worker:
end
store result as worker.run()
"#,
        r#"
create container Worker:
    static property helper: Number defaults 0

    static action run:
        define action called helper:
            return "ok"
        end action
        display call helper
    end
end

store result as Worker.run()
"#,
    ] {
        let tokens = lex_wfl_with_positions(source);
        let mut parser = Parser::new(&tokens);
        let program = parser.parse().expect("test program should parse");
        TypeChecker::new()
            .check_types(&program)
            .unwrap_or_else(|failure| {
                panic!(
                    "the nested declaration should shadow the property statically: {:?}",
                    failure.into_diagnostics()
                )
            });

        let mut interpreter = Interpreter::new();
        interpreter
            .interpret(&program)
            .await
            .unwrap_or_else(|errors| {
                panic!("the runtime must create the same local declaration: {errors:?}")
            });
    }
}
