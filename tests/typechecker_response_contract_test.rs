use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::{TypeCheckError, TypeChecker};

fn typecheck(source: &str) -> Result<(), TypeCheckError> {
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("test program should parse");
    TypeChecker::new().check_types(&program)
}

fn request_program(response: &str) -> String {
    format!(
        "listen on port 8080 as srv\n\
         wait for request comes in on srv as req\n\
         {response}\n"
    )
}

#[test]
fn response_content_accepts_every_runtime_scalar_type() {
    for content in ["42", "yes", "nothing", "\"text\""] {
        let source = request_program(&format!("respond to req with {content}"));
        typecheck(&source).unwrap_or_else(|failure| {
            panic!(
                "runtime-supported response content {content} must typecheck: {:?}",
                failure.into_diagnostics()
            )
        });
    }
}

#[test]
fn response_content_accepts_an_optional_runtime_scalar() {
    let source = r#"
define action called maybe_content with parameters enabled:
    check if enabled:
        return "ready"
    end check
end action

listen on port 8080 as srv
wait for request comes in on srv as req
store content_value as call maybe_content with no
respond to req with content_value
"#;
    typecheck(source).unwrap_or_else(|failure| {
        panic!(
            "both Text and Nothing are runtime-supported response values: {:?}",
            failure.into_diagnostics()
        )
    });
}

#[test]
fn response_content_rejects_runtime_unsupported_composites() {
    let source = request_program("respond to req with [1 and 2]");
    let diagnostics = typecheck(&source)
        .expect_err("composite response content fails at runtime")
        .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.message.contains("Response content")),
        "expected a response-content diagnostic, got: {diagnostics:?}"
    );

    let source = "listen on port 8080 as srv\n\
                  wait for request comes in on srv as req\n\
                  create map payload:\n\
                  \x20\x20\x20\x20key is \"value\"\n\
                  end map\n\
                  respond to req with payload\n";
    let diagnostics = typecheck(source)
        .expect_err("map response content fails at runtime")
        .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.message.contains("Response content")),
        "expected a response-content diagnostic, got: {diagnostics:?}"
    );
}

#[test]
fn response_request_operand_is_traversed_and_must_be_request_shaped() {
    let diagnostics = typecheck(r#"respond to "not a request" with "ok""#)
        .expect_err("a concrete text value is not a request object")
        .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.message.contains("request object")),
        "expected a request-object diagnostic, got: {diagnostics:?}"
    );

    let diagnostics = typecheck(r#"respond to (1 minus "x") with "ok""#)
        .expect_err("nested errors in the request expression must be visited")
        .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.message.contains("Cannot perform Minus")),
        "expected the nested request-expression diagnostic, got: {diagnostics:?}"
    );
}

#[test]
fn implicit_request_headers_have_a_reusable_map_type() {
    let source = request_program(r#"respond to req with "ok" and headers headers"#);
    typecheck(&source).unwrap_or_else(|failure| {
        panic!(
            "runtime request headers are a text-valued map: {:?}",
            failure.into_diagnostics()
        )
    });
}
