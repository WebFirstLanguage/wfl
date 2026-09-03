use wfl::analyzer::{Analyzer, Symbol, SymbolKind};
use wfl::parser::ast::{
    Argument, DatabaseQueryKind, ExportType, Expression, Literal, Operator, PatternExpression,
    Program, Statement, Type, WriteMode, WsHandlerEvent,
};
use wfl::typechecker::{TypeCheckError, TypeChecker};

fn number(value: i64) -> Expression {
    Expression::Literal(Literal::Integer(value), 1, 1)
}

fn text(value: &str) -> Expression {
    Expression::Literal(Literal::String(value.into()), 1, 1)
}

fn boolean(value: bool) -> Expression {
    Expression::Literal(Literal::Boolean(value), 1, 1)
}

fn list(values: Vec<Expression>) -> Expression {
    Expression::Literal(Literal::List(values), 1, 1)
}

fn any_expression() -> Expression {
    Expression::ActionCall {
        name: "parse_json".to_string(),
        arguments: vec![argument(text("null"))],
        line: 1,
        column: 1,
    }
}

fn argument(value: Expression) -> Argument {
    Argument { name: None, value }
}

fn call(name: &str, arguments: Vec<Expression>) -> Expression {
    Expression::ActionCall {
        name: name.to_string(),
        arguments: arguments.into_iter().map(argument).collect(),
        line: 1,
        column: 1,
    }
}

fn typecheck(statements: Vec<Statement>) -> Result<(), TypeCheckError> {
    TypeChecker::new().check_types(&Program { statements })
}

fn typecheck_with_symbol(
    name: &str,
    symbol_type: Type,
    statements: Vec<Statement>,
) -> Result<(), TypeCheckError> {
    let mut analyzer = Analyzer::new();
    analyzer
        .define_symbol(Symbol {
            name: name.to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(symbol_type),
            line: 1,
            column: 1,
        })
        .expect("test symbol should be defined");
    TypeChecker::with_analyzer(analyzer).check_types(&Program { statements })
}

fn websocket_send_to(target: &str) -> Statement {
    Statement::SendWebSocketMessageStatement {
        message: text("hello"),
        target: Expression::Variable(target.to_string(), 1, 1),
        line: 1,
        column: 1,
    }
}

fn diagnostic_messages(statements: Vec<Statement>) -> Vec<String> {
    typecheck(statements)
        .expect_err("program should be rejected")
        .into_diagnostics()
        .into_iter()
        .map(|error| error.message)
        .collect()
}

#[test]
fn file_write_and_close_accept_runtime_supported_text_handles() {
    typecheck(vec![
        Statement::WriteFileStatement {
            file: text("output.txt"),
            content: text("hello"),
            mode: WriteMode::Overwrite,
            line: 1,
            column: 1,
        },
        Statement::CloseFileStatement {
            file: text("output.txt"),
            line: 2,
            column: 1,
        },
    ])
    .expect("legacy text paths/handles are accepted by the runtime");
}

#[test]
fn http_post_requires_text_data() {
    let messages = diagnostic_messages(vec![Statement::HttpPostStatement {
        url: text("https://example.invalid"),
        data: number(42),
        variable_name: "response".to_string(),
        line: 1,
        column: 1,
    }]);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("HTTP POST data") && message.contains("text")),
        "expected a data-type diagnostic, got {messages:?}"
    );
}

#[test]
fn streaming_response_requires_a_request_object() {
    let messages = diagnostic_messages(vec![Statement::StartStreamingResponseStatement {
        request: text("not a request"),
        status: None,
        content_type: None,
        headers: None,
        variable_name: "stream".to_string(),
        line: 1,
        column: 1,
    }]);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("request object")),
        "expected a request-object diagnostic, got {messages:?}"
    );
}

#[test]
fn command_and_process_arguments_require_text_or_list() {
    for statement in [
        Statement::ExecuteCommandStatement {
            command: text("tool"),
            arguments: Some(boolean(true)),
            variable_name: None,
            use_shell: false,
            line: 1,
            column: 1,
        },
        Statement::SpawnProcessStatement {
            command: text("tool"),
            arguments: Some(boolean(true)),
            variable_name: "process".to_string(),
            use_shell: false,
            line: 1,
            column: 1,
        },
    ] {
        let messages = diagnostic_messages(vec![statement]);
        assert!(
            messages
                .iter()
                .any(|message| message.contains("arguments") && message.contains("text or a list")),
            "expected an argument-type diagnostic, got {messages:?}"
        );
    }

    typecheck(vec![
        Statement::ExecuteCommandStatement {
            command: text("tool"),
            arguments: Some(text("--version")),
            variable_name: None,
            use_shell: false,
            line: 1,
            column: 1,
        },
        Statement::SpawnProcessStatement {
            command: text("tool"),
            arguments: Some(list(vec![text("--version")])),
            variable_name: "process".to_string(),
            use_shell: false,
            line: 1,
            column: 1,
        },
    ])
    .expect("runtime-supported text and list argument forms should typecheck");
}

#[test]
fn execute_file_request_requires_a_request_object() {
    let messages = diagnostic_messages(vec![Statement::ExecuteFileStatement {
        path: text("child.wfl"),
        request: Some(number(42)),
        variable_name: None,
        line: 1,
        column: 1,
    }]);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("request object")),
        "expected a request-object diagnostic, got {messages:?}"
    );
}

#[test]
fn websocket_send_validates_payload_and_target() {
    let messages = diagnostic_messages(vec![
        Statement::SendWebSocketMessageStatement {
            message: list(vec![]),
            target: any_expression(),
            line: 1,
            column: 1,
        },
        Statement::SendWebSocketMessageStatement {
            message: text("hello"),
            target: number(42),
            line: 2,
            column: 1,
        },
    ]);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("WebSocket message")),
        "expected a WebSocket payload diagnostic, got {messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("WebSocket connection")),
        "expected a WebSocket target diagnostic, got {messages:?}"
    );
}

#[test]
fn websocket_send_rejects_maps_without_text_connection_values() {
    let errors = typecheck_with_symbol(
        "connection",
        Type::Map(Box::new(Type::Text), Box::new(Type::Number)),
        vec![websocket_send_to("connection")],
    )
    .expect_err("Map<Text, Number> cannot contain a runtime text connection id")
    .into_diagnostics();

    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("WebSocket connection target")),
        "expected a WebSocket target diagnostic, got {errors:?}"
    );
}

#[test]
fn websocket_send_rejects_maps_without_text_keys() {
    let errors = typecheck_with_symbol(
        "connection",
        Type::Map(Box::new(Type::Number), Box::new(Type::Text)),
        vec![websocket_send_to("connection")],
    )
    .expect_err("a WebSocket connection object must use text keys")
    .into_diagnostics();

    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("WebSocket connection target")),
        "expected a WebSocket target diagnostic, got {errors:?}"
    );
}

#[test]
fn websocket_send_accepts_handler_connection_shapes_and_gradual_map_values() {
    typecheck(vec![
        Statement::ListenWebSocketStatement {
            port: number(8080),
            server_name: "server".to_string(),
            line: 1,
            column: 1,
        },
        Statement::WebSocketHandlerStatement {
            event: WsHandlerEvent::Connect,
            server: Expression::Variable("server".to_string(), 2, 1),
            binding: "connection".to_string(),
            body: vec![websocket_send_to("connection")],
            line: 2,
            column: 1,
        },
        Statement::WebSocketHandlerStatement {
            event: WsHandlerEvent::Message,
            server: Expression::Variable("server".to_string(), 3, 1),
            binding: "message_event".to_string(),
            body: vec![websocket_send_to("message_event")],
            line: 3,
            column: 1,
        },
    ])
    .expect("connect and message handler objects are valid WebSocket send targets");

    typecheck_with_symbol(
        "gradual_connection",
        Type::Map(Box::new(Type::Text), Box::new(Type::Any)),
        vec![websocket_send_to("gradual_connection")],
    )
    .expect("Map<Text, Any> must remain a gradual WebSocket target");
}

#[test]
fn websocket_broadcast_validates_payload() {
    let messages = diagnostic_messages(vec![
        Statement::ListenWebSocketStatement {
            port: number(8080),
            server_name: "server".to_string(),
            line: 1,
            column: 1,
        },
        Statement::BroadcastWebSocketMessageStatement {
            message: list(vec![]),
            server: Expression::Variable("server".to_string(), 2, 1),
            line: 2,
            column: 1,
        },
    ]);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("WebSocket message")),
        "expected a WebSocket payload diagnostic, got {messages:?}"
    );
}

#[test]
fn missing_header_result_requires_a_nothing_guard() {
    let header = Expression::HeaderAccess {
        header_name: "x-missing".to_string(),
        request: Box::new(Expression::Variable("request".to_string(), 1, 1)),
        line: 1,
        column: 1,
    };
    let adjusted = Expression::BinaryOperation {
        left: Box::new(header),
        operator: Operator::Minus,
        right: Box::new(number(1)),
        line: 1,
        column: 1,
    };

    let messages = typecheck_with_symbol(
        "request",
        Type::Custom("Request".to_string()),
        vec![Statement::ExpressionStatement {
            expression: adjusted,
            line: 1,
            column: 1,
        }],
    )
    .expect_err("a missing request header is still Nothing-capable")
    .into_diagnostics()
    .into_iter()
    .map(|error| error.message)
    .collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("Cannot perform Minus")),
        "HeaderAccess is Text or Nothing, not an unrestricted gradual value: {messages:?}"
    );
}

#[test]
fn header_fallback_preserves_the_map_value_type() {
    let mut analyzer = Analyzer::new();
    for (name, symbol_type) in [
        ("request", Type::Number),
        (
            "headers",
            Type::Map(Box::new(Type::Text), Box::new(Type::Number)),
        ),
    ] {
        analyzer
            .define_symbol(Symbol {
                name: name.to_string(),
                kind: SymbolKind::Variable { mutable: false },
                symbol_type: Some(symbol_type),
                line: 1,
                column: 1,
            })
            .expect("test symbol should be defined");
    }
    let header = Expression::HeaderAccess {
        header_name: "x-number".to_string(),
        request: Box::new(Expression::Variable("request".to_string(), 1, 1)),
        line: 1,
        column: 1,
    };
    let program = Program {
        statements: vec![
            Statement::VariableDeclaration {
                name: "value".to_string(),
                value: header,
                is_constant: false,
                line: 1,
                column: 1,
            },
            Statement::IfStatement {
                condition: Expression::BinaryOperation {
                    left: Box::new(Expression::Variable("value".to_string(), 2, 1)),
                    operator: Operator::NotEquals,
                    right: Box::new(Expression::Literal(Literal::Nothing, 2, 1)),
                    line: 2,
                    column: 1,
                },
                then_block: vec![Statement::ExpressionStatement {
                    expression: call(
                        "touppercase",
                        vec![Expression::Variable("value".to_string(), 3, 1)],
                    ),
                    line: 3,
                    column: 1,
                }],
                else_block: None,
                line: 2,
                column: 1,
            },
        ],
    };

    let diagnostics = TypeChecker::with_analyzer(analyzer)
        .check_types(&program)
        .expect_err("a Number fallback header must not narrow to Text")
        .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.found == Some(Type::Number)),
        "expected the runtime map value type to survive HeaderAccess: {diagnostics:?}"
    );
}

#[test]
fn pattern_find_result_requires_a_nothing_guard() {
    let result = Expression::PatternFind {
        text: Box::new(text("abc")),
        pattern: Box::new(Expression::Literal(
            Literal::Pattern("letter".to_string()),
            1,
            1,
        )),
        line: 1,
        column: 1,
    };
    let messages = diagnostic_messages(vec![Statement::ExpressionStatement {
        expression: Expression::BinaryOperation {
            left: Box::new(result),
            operator: Operator::Minus,
            right: Box::new(number(1)),
            line: 1,
            column: 1,
        },
        line: 1,
        column: 1,
    }]);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("Cannot perform Minus")),
        "pattern find is a match map or Nothing, not unrestricted Any: {messages:?}"
    );
}

#[test]
fn method_calls_on_gradual_values_defer_but_still_visit_arguments() {
    typecheck(vec![Statement::ExpressionStatement {
        expression: Expression::MethodCall {
            object: Box::new(any_expression()),
            method: "runtime_method".to_string(),
            arguments: vec![argument(number(1))],
            line: 1,
            column: 1,
        },
        line: 1,
        column: 1,
    }])
    .expect("method dispatch on Any must defer to runtime");

    let invalid_argument = Expression::BinaryOperation {
        left: Box::new(number(1)),
        operator: Operator::Minus,
        right: Box::new(text("wrong")),
        line: 2,
        column: 1,
    };
    let messages = diagnostic_messages(vec![Statement::ExpressionStatement {
        expression: Expression::MethodCall {
            object: Box::new(any_expression()),
            method: "runtime_method".to_string(),
            arguments: vec![argument(invalid_argument)],
            line: 2,
            column: 1,
        },
        line: 2,
        column: 1,
    }]);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("Cannot perform Minus")),
        "gradual dispatch must not hide nested argument errors: {messages:?}"
    );
    assert!(
        !messages
            .iter()
            .any(|message| message.contains("Cannot call method")),
        "Any itself must not produce a false non-container error: {messages:?}"
    );
}

#[test]
fn prior_expression_errors_do_not_cascade_through_property_and_method_access() {
    let failed_object = Expression::BinaryOperation {
        left: Box::new(number(1)),
        operator: Operator::Minus,
        right: Box::new(text("wrong")),
        line: 1,
        column: 1,
    };
    let property = Expression::PropertyAccess {
        object: Box::new(failed_object),
        property: "anything".to_string(),
        line: 1,
        column: 1,
    };
    let messages = diagnostic_messages(vec![Statement::ExpressionStatement {
        expression: Expression::MethodCall {
            object: Box::new(property),
            method: "anything".to_string(),
            arguments: vec![],
            line: 1,
            column: 1,
        },
        line: 1,
        column: 1,
    }]);
    assert_eq!(
        messages
            .iter()
            .filter(|message| message.contains("Cannot perform Minus"))
            .count(),
        1,
        "the root expression error should be reported once: {messages:?}"
    );
    assert!(
        !messages.iter().any(|message| {
            message.contains("Cannot access property") || message.contains("Cannot call method")
        }),
        "Error must propagate without secondary member-access errors: {messages:?}"
    );
}

#[test]
fn temporal_values_support_the_runtime_comparison_contract() {
    for (name, left, right) in [
        (
            "Date",
            call("create_date", vec![number(2026), number(7), number(26)]),
            call("create_date", vec![number(2026), number(7), number(27)]),
        ),
        ("Time", call("now", vec![]), call("now", vec![])),
        (
            "DateTime",
            call("datetime_now", vec![]),
            call("datetime_now", vec![]),
        ),
    ] {
        typecheck(vec![Statement::ExpressionStatement {
            expression: Expression::BinaryOperation {
                left: Box::new(left),
                operator: Operator::LessThan,
                right: Box::new(right),
                line: 1,
                column: 1,
            },
            line: 1,
            column: 1,
        }])
        .unwrap_or_else(|failure| {
            panic!(
                "same-kind {name} ordering is implemented by the runtime: {:?}",
                failure.into_diagnostics()
            )
        });
    }

    typecheck(vec![Statement::ExpressionStatement {
        expression: Expression::BinaryOperation {
            left: Box::new(call("today", vec![])),
            operator: Operator::Equals,
            right: Box::new(call("now", vec![])),
            line: 1,
            column: 1,
        },
        line: 1,
        column: 1,
    }])
    .expect("runtime equality is total across unlike temporal values");

    typecheck(vec![
        Statement::CreateListStatement {
            name: "dates".to_string(),
            initial_values: vec![call("today", vec![])],
            line: 1,
            column: 1,
        },
        Statement::ExpressionStatement {
            expression: Expression::BinaryOperation {
                left: Box::new(Expression::Variable("dates".to_string(), 2, 1)),
                operator: Operator::Contains,
                right: Box::new(call("now", vec![])),
                line: 2,
                column: 1,
            },
            line: 2,
            column: 1,
        },
    ])
    .expect("runtime list membership returns false for an unlike temporal needle");
}

#[test]
fn response_statements_reject_ordinary_maps_but_execute_file_defers_shape() {
    let make_map = || Statement::MapCreation {
        name: "request_like".to_string(),
        entries: vec![],
        line: 1,
        column: 1,
    };
    let request_like = || Expression::Variable("request_like".to_string(), 2, 1);

    let messages = diagnostic_messages(vec![
        make_map(),
        Statement::RespondStatement {
            request: request_like(),
            content: text("ok"),
            status: None,
            content_type: None,
            headers: None,
            set_session: None,
            clear_session: false,
            line: 2,
            column: 1,
        },
        Statement::StartStreamingResponseStatement {
            request: request_like(),
            status: None,
            content_type: None,
            headers: None,
            variable_name: "stream".to_string(),
            line: 3,
            column: 1,
        },
    ]);
    assert!(
        messages
            .iter()
            .filter(|message| message.contains("request object"))
            .count()
            >= 2,
        "ordinary maps have no pending response sender: {messages:?}"
    );

    typecheck(vec![
        make_map(),
        Statement::ExecuteFileStatement {
            path: text("child.wfl"),
            request: Some(request_like()),
            variable_name: None,
            line: 2,
            column: 1,
        },
    ])
    .expect("execute-file validates the fields of a map-shaped request at runtime");
}

#[test]
fn header_access_rejects_a_scalar_without_request_headers_in_scope() {
    let messages = diagnostic_messages(vec![Statement::ExpressionStatement {
        expression: Expression::HeaderAccess {
            header_name: "x-test".to_string(),
            request: Box::new(number(1)),
            line: 1,
            column: 1,
        },
        line: 1,
        column: 1,
    }]);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("Header access requires")),
        "expected a missing request/header-scope diagnostic: {messages:?}"
    );
}

#[test]
fn binary_writes_require_an_open_file_handle() {
    let messages = diagnostic_messages(vec![Statement::WriteBinaryStatement {
        content: any_expression(),
        target: text("literal-path.bin"),
        line: 1,
        column: 1,
    }]);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("open File handle")),
        "a literal path is not a runtime binary handle: {messages:?}"
    );
}

#[test]
fn database_parameter_lists_reject_known_composite_elements() {
    let messages = diagnostic_messages(vec![
        Statement::OpenDatabaseStatement {
            url: text("sqlite::memory:"),
            variable_name: "db".to_string(),
            line: 1,
            column: 1,
        },
        Statement::MapCreation {
            name: "payload".to_string(),
            entries: vec![("key".to_string(), text("value"))],
            line: 2,
            column: 1,
        },
        Statement::CreateListStatement {
            name: "params".to_string(),
            initial_values: vec![Expression::Variable("payload".to_string(), 3, 1)],
            line: 3,
            column: 1,
        },
        Statement::DatabaseQueryStatement {
            db: Expression::Variable("db".to_string(), 4, 1),
            sql: text("select ?"),
            parameters: Some(Expression::Variable("params".to_string(), 4, 1)),
            variable_name: "rows".to_string(),
            kind: DatabaseQueryKind::Query,
            line: 4,
            column: 1,
        },
    ]);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("SQL scalar")),
        "known map elements cannot be bound as SQL parameters: {messages:?}"
    );
}

#[test]
fn database_parameter_lists_accept_optional_scalar_elements() {
    typecheck_with_symbol(
        "params",
        Type::List(Box::new(Type::Optional(Box::new(Type::Text)))),
        vec![
            Statement::OpenDatabaseStatement {
                url: text("sqlite::memory:"),
                variable_name: "db".to_string(),
                line: 1,
                column: 1,
            },
            Statement::DatabaseQueryStatement {
                db: Expression::Variable("db".to_string(), 2, 1),
                sql: text("select ?"),
                parameters: Some(Expression::Variable("params".to_string(), 2, 1)),
                variable_name: "rows".to_string(),
                kind: DatabaseQueryKind::Query,
                line: 2,
                column: 1,
            },
        ],
    )
    .expect("both Text and Nothing are valid SQL parameter values");
}

#[test]
fn gradual_function_calls_still_visit_every_argument() {
    let invalid_argument = Expression::BinaryOperation {
        left: Box::new(number(1)),
        operator: Operator::Minus,
        right: Box::new(text("wrong")),
        line: 1,
        column: 1,
    };
    let messages = diagnostic_messages(vec![Statement::ExpressionStatement {
        expression: Expression::FunctionCall {
            function: Box::new(any_expression()),
            arguments: vec![argument(invalid_argument)],
            line: 1,
            column: 1,
        },
        line: 1,
        column: 1,
    }]);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("Cannot perform Minus")),
        "a gradual callee must not hide argument errors: {messages:?}"
    );
    assert!(
        !messages
            .iter()
            .any(|message| message.contains("not a function")),
        "Any callability is deferred to runtime: {messages:?}"
    );
}

#[test]
fn describe_setup_is_shared_but_test_locals_and_describe_locals_do_not_escape() {
    let declaration = |name: &str| Statement::VariableDeclaration {
        name: name.to_string(),
        value: number(1),
        is_constant: false,
        line: 1,
        column: 1,
    };
    let display = |name: &str| Statement::DisplayStatement {
        value: Expression::Variable(name.to_string(), 1, 1),
        line: 1,
        column: 1,
    };

    typecheck(vec![Statement::DescribeBlock {
        description: "scope".to_string(),
        setup: Some(vec![declaration("setup_value")]),
        teardown: Some(vec![display("setup_value")]),
        tests: vec![
            Statement::TestBlock {
                description: "first".to_string(),
                body: vec![display("setup_value")],
                line: 1,
                column: 1,
            },
            Statement::TestBlock {
                description: "second".to_string(),
                body: vec![display("setup_value")],
                line: 1,
                column: 1,
            },
        ],
        line: 1,
        column: 1,
    }])
    .expect("setup bindings are visible to each test and teardown");

    let messages = diagnostic_messages(vec![
        Statement::DescribeBlock {
            description: "scope".to_string(),
            setup: Some(vec![declaration("setup_value")]),
            teardown: None,
            tests: vec![
                Statement::TestBlock {
                    description: "first".to_string(),
                    body: vec![declaration("test_only")],
                    line: 1,
                    column: 1,
                },
                Statement::TestBlock {
                    description: "second".to_string(),
                    body: vec![display("test_only")],
                    line: 1,
                    column: 1,
                },
            ],
            line: 1,
            column: 1,
        },
        display("setup_value"),
    ]);
    assert!(
        messages.iter().any(|message| message.contains("test_only")),
        "one test's locals must not leak into another: {messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("setup_value")),
        "describe-level setup locals must not escape the describe: {messages:?}"
    );
}

#[test]
fn exports_require_a_definition_owned_by_the_current_scope() {
    let action = |name: &str, body: Vec<Statement>| Statement::ActionDefinition {
        name: name.to_string(),
        parameters: vec![],
        body,
        return_type: None,
        line: 1,
        column: 1,
    };
    let export = |export_type, name: &str| Statement::ExportStatement {
        export_type,
        name: name.to_string(),
        line: 2,
        column: 1,
    };

    let messages = diagnostic_messages(vec![
        action("outer_action", vec![]),
        Statement::VariableDeclaration {
            name: "OUTER_CONSTANT".to_string(),
            value: number(1),
            is_constant: true,
            line: 1,
            column: 1,
        },
        Statement::ContainerDefinition {
            name: "OuterContainer".to_string(),
            extends: None,
            implements: vec![],
            properties: vec![],
            methods: vec![],
            events: vec![],
            static_properties: vec![],
            static_methods: vec![],
            line: 1,
            column: 1,
        },
        action(
            "attempt_exports",
            vec![
                export(ExportType::Action, "outer_action"),
                export(ExportType::Constant, "OUTER_CONSTANT"),
                export(ExportType::Container, "OuterContainer"),
            ],
        ),
    ]);

    for name in ["outer_action", "OUTER_CONSTANT", "OuterContainer"] {
        assert!(
            messages.iter().any(|message| message.contains(name)),
            "parent-scope export of {name} must be rejected: {messages:?}"
        );
    }
}

#[test]
fn exports_accept_definitions_owned_by_the_current_scope() {
    let action = |name: &str, body: Vec<Statement>| Statement::ActionDefinition {
        name: name.to_string(),
        parameters: vec![],
        body,
        return_type: None,
        line: 1,
        column: 1,
    };
    let export = |export_type, name: &str| Statement::ExportStatement {
        export_type,
        name: name.to_string(),
        line: 2,
        column: 1,
    };

    typecheck(vec![action(
        "owner",
        vec![
            Statement::VariableDeclaration {
                name: "LOCAL_CONSTANT".to_string(),
                value: number(1),
                is_constant: true,
                line: 1,
                column: 1,
            },
            action("local_action", vec![]),
            Statement::ContainerDefinition {
                name: "LocalContainer".to_string(),
                extends: None,
                implements: vec![],
                properties: vec![],
                methods: vec![],
                events: vec![],
                static_properties: vec![],
                static_methods: vec![],
                line: 1,
                column: 1,
            },
            export(ExportType::Constant, "LOCAL_CONSTANT"),
            export(ExportType::Action, "local_action"),
            export(ExportType::Container, "LocalContainer"),
        ],
    )])
    .expect("definitions owned by the active scope are exportable");
}

#[test]
fn pattern_backreferences_require_an_earlier_capture() {
    let valid_pattern = PatternExpression::Sequence(vec![
        PatternExpression::Capture {
            name: "word".to_string(),
            pattern: Box::new(PatternExpression::Literal("hello".to_string())),
        },
        PatternExpression::Backreference("word".to_string()),
    ]);
    typecheck(vec![Statement::PatternDefinition {
        name: "valid".to_string(),
        pattern: valid_pattern,
        line: 1,
        column: 1,
    }])
    .expect("a backreference to an earlier capture is valid");

    let messages = diagnostic_messages(vec![Statement::PatternDefinition {
        name: "invalid".to_string(),
        pattern: PatternExpression::Backreference("missing".to_string()),
        line: 1,
        column: 1,
    }]);
    assert!(
        messages.iter().any(|message| {
            message.contains("Backreference")
                && message.contains("missing")
                && message.contains("undefined")
        }),
        "an undefined backreference must fail before runtime: {messages:?}"
    );
}
