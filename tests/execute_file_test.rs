// TDD tests for the `execute file` statement:
//   execute [wfl] file at <path> [with <request>] [and read output as <variable>]
//
// Executes another WFL file in-process with a nested interpreter, optionally
// passing HTTP request context and capturing the child's display/print output.

use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;
use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::{Expression, Statement};

fn parse_program(source: &str) -> Result<wfl::parser::ast::Program, String> {
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    parser.parse().map_err(|errors| {
        errors
            .first()
            .map(|e| e.message.clone())
            .unwrap_or_else(|| "unknown parse error".to_string())
    })
}

async fn run_program_in_dir(
    dir: &TempDir,
    source: &str,
) -> (
    Result<Value, Vec<wfl::interpreter::error::RuntimeError>>,
    Interpreter,
) {
    let main_file = dir.path().join("main.wfl");
    fs::write(&main_file, source).expect("Failed to write main file");

    let ast = parse_program(source).unwrap_or_else(|e| panic!("Parse failed: {e}"));
    let mut interpreter = Interpreter::new();
    interpreter.set_source_file(main_file);
    let result = interpreter.interpret(&ast).await;
    (result, interpreter)
}

fn get_global_text(interpreter: &Interpreter, name: &str) -> String {
    let env = interpreter.global_env().borrow();
    match env.values.get(name) {
        Some(Value::Text(text)) => text.to_string(),
        other => panic!("Expected Text variable '{name}', got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Parse tests
// ---------------------------------------------------------------------------

#[test]
fn test_parse_execute_file_minimal() {
    let program = parse_program(r#"execute file at "page.wfl""#).expect("Parse failed");
    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::ExecuteFileStatement {
            path,
            request,
            variable_name,
            ..
        } => {
            assert!(matches!(path, Expression::Literal(..)));
            assert!(request.is_none());
            assert!(variable_name.is_none());
        }
        other => panic!("Expected ExecuteFileStatement, got {other:?}"),
    }
}

#[test]
fn test_parse_execute_file_with_optional_wfl_keyword() {
    let program = parse_program(r#"execute wfl file at "page.wfl""#).expect("Parse failed");
    assert!(matches!(
        &program.statements[0],
        Statement::ExecuteFileStatement {
            request: None,
            variable_name: None,
            ..
        }
    ));
}

#[test]
fn test_parse_execute_file_with_output_capture() {
    let program = parse_program(r#"execute wfl file at "page.wfl" and read output as page_output"#)
        .expect("Parse failed");
    match &program.statements[0] {
        Statement::ExecuteFileStatement {
            request,
            variable_name,
            ..
        } => {
            assert!(request.is_none());
            assert_eq!(variable_name.as_deref(), Some("page_output"));
        }
        other => panic!("Expected ExecuteFileStatement, got {other:?}"),
    }
}

#[test]
fn test_parse_execute_file_full_form() {
    let program = parse_program(
        r#"execute wfl file at "page.wfl" with incoming_request and read output as page_output"#,
    )
    .expect("Parse failed");
    match &program.statements[0] {
        Statement::ExecuteFileStatement {
            request,
            variable_name,
            ..
        } => {
            assert!(
                matches!(request, Some(Expression::Variable(name, ..)) if name == "incoming_request")
            );
            assert_eq!(variable_name.as_deref(), Some("page_output"));
        }
        other => panic!("Expected ExecuteFileStatement, got {other:?}"),
    }
}

#[test]
fn test_parse_execute_file_missing_at_fails() {
    let result = parse_program(r#"execute file "page.wfl""#);
    assert!(result.is_err(), "Expected parse error without 'at'");
}

#[test]
fn test_parse_execute_command_still_works() {
    // Regression: dispatch must not break the existing execute command statement
    let program = parse_program(r#"execute command "echo" with arguments ["hi"] as cmd_result"#)
        .expect("Parse failed");
    assert!(matches!(
        &program.statements[0],
        Statement::ExecuteCommandStatement { .. }
    ));
}

// ---------------------------------------------------------------------------
// Output capture
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_execute_file_captures_display_output() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    fs::write(
        temp_dir.path().join("page.wfl"),
        "display \"line one\"\ndisplay \"line two\"\n",
    )
    .expect("Failed to write page file");

    let source = r#"execute wfl file at "page.wfl" and read output as page_output"#;
    let (result, interpreter) = run_program_in_dir(&temp_dir, source).await;
    result.unwrap_or_else(|e| panic!("Interpret failed: {e:?}"));

    assert_eq!(
        get_global_text(&interpreter, "page_output"),
        "line one\nline two\n"
    );
}

#[tokio::test]
async fn test_execute_file_captures_print_output() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    fs::write(
        temp_dir.path().join("page.wfl"),
        "store greeting as \"hello from print\"\nprint of greeting\n",
    )
    .expect("Failed to write page file");

    let source = r#"execute wfl file at "page.wfl" and read output as page_output"#;
    let (result, interpreter) = run_program_in_dir(&temp_dir, source).await;
    result.unwrap_or_else(|e| panic!("Interpret failed: {e:?}"));

    assert_eq!(
        get_global_text(&interpreter, "page_output"),
        "hello from print\n"
    );
}

#[tokio::test]
async fn test_execute_file_nested_capture() {
    // outer.wfl executes inner.wfl with capture and decorates its output;
    // the parent captures outer.wfl. Inner output must not leak.
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    fs::write(
        temp_dir.path().join("inner.wfl"),
        "display \"inner content\"\n",
    )
    .expect("Failed to write inner file");
    fs::write(
        temp_dir.path().join("outer.wfl"),
        concat!(
            "execute wfl file at \"inner.wfl\" and read output as inner_output\n",
            "display \"before\"\n",
            "display inner_output\n",
            "display \"after\"\n",
        ),
    )
    .expect("Failed to write outer file");

    let source = r#"execute wfl file at "outer.wfl" and read output as page_output"#;
    let (result, interpreter) = run_program_in_dir(&temp_dir, source).await;
    result.unwrap_or_else(|e| panic!("Interpret failed: {e:?}"));

    assert_eq!(
        get_global_text(&interpreter, "page_output"),
        "before\ninner content\n\nafter\n"
    );
}

// ---------------------------------------------------------------------------
// Request context
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_execute_file_passes_request_context() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    fs::write(
        temp_dir.path().join("page.wfl"),
        concat!(
            "display method\n",
            "display path\n",
            "display body\n",
            "display client_ip\n",
        ),
    )
    .expect("Failed to write page file");

    let main_file = temp_dir.path().join("main.wfl");
    let source =
        r#"execute wfl file at "page.wfl" with fake_request and read output as page_output"#;
    fs::write(&main_file, source).expect("Failed to write main file");

    let ast = parse_program(source).unwrap_or_else(|e| panic!("Parse failed: {e}"));
    let mut interpreter = Interpreter::new();
    interpreter.set_source_file(main_file);

    // Build a request object the way `wait for request` does
    {
        let mut headers = HashMap::new();
        headers.insert("user-agent".to_string(), Value::Text(Arc::from("wfl-test")));
        let mut props = HashMap::new();
        props.insert("method".to_string(), Value::Text(Arc::from("POST")));
        props.insert("path".to_string(), Value::Text(Arc::from("/hello")));
        props.insert("body".to_string(), Value::Text(Arc::from("name=World")));
        props.insert("client_ip".to_string(), Value::Text(Arc::from("127.0.0.1")));
        props.insert(
            "headers".to_string(),
            Value::Object(std::rc::Rc::new(std::cell::RefCell::new(headers))),
        );
        interpreter
            .global_env()
            .borrow_mut()
            .define(
                "fake_request",
                Value::Object(std::rc::Rc::new(std::cell::RefCell::new(props))),
            )
            .expect("Failed to define fake_request");
    }

    let result = interpreter.interpret(&ast).await;
    result.unwrap_or_else(|e| panic!("Interpret failed: {e:?}"));

    assert_eq!(
        get_global_text(&interpreter, "page_output"),
        "POST\n/hello\nname=World\n127.0.0.1\n"
    );
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_execute_file_missing_file_is_catchable() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let source = concat!(
        "store outcome as \"unset\"\n",
        "try:\n",
        "    execute wfl file at \"no_such_page.wfl\" and read output as page_output\n",
        "    change outcome to \"executed\"\n",
        "when file not found:\n",
        "    change outcome to \"caught missing file\"\n",
        "otherwise:\n",
        "    change outcome to \"wrong handler\"\n",
        "end try\n",
    );
    let (result, interpreter) = run_program_in_dir(&temp_dir, source).await;
    result.unwrap_or_else(|e| panic!("Interpret failed: {e:?}"));

    assert_eq!(
        get_global_text(&interpreter, "outcome"),
        "caught missing file"
    );
}

#[tokio::test]
async fn test_execute_file_child_parse_error_is_catchable() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    fs::write(
        temp_dir.path().join("broken.wfl"),
        "store as as as nonsense gibberish ===\n",
    )
    .expect("Failed to write broken file");

    let source = concat!(
        "store outcome as \"unset\"\n",
        "try:\n",
        "    execute wfl file at \"broken.wfl\" and read output as page_output\n",
        "    change outcome to \"executed\"\n",
        "when error:\n",
        "    change outcome to \"caught child error\"\n",
        "end try\n",
    );
    let (result, interpreter) = run_program_in_dir(&temp_dir, source).await;
    result.unwrap_or_else(|e| panic!("Interpret failed: {e:?}"));

    assert_eq!(
        get_global_text(&interpreter, "outcome"),
        "caught child error"
    );
}

#[tokio::test]
async fn test_execute_file_child_error_mentions_child_path() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    fs::write(
        temp_dir.path().join("broken.wfl"),
        "store as as as nonsense gibberish ===\n",
    )
    .expect("Failed to write broken file");

    let source = r#"execute wfl file at "broken.wfl" and read output as page_output"#;
    let (result, _interpreter) = run_program_in_dir(&temp_dir, source).await;
    let errors = result.expect_err("Expected child parse error to propagate");
    let message = &errors.first().expect("Expected at least one error").message;
    assert!(
        message.contains("broken.wfl"),
        "Error should mention the child file path, got: {message}"
    );
}

#[tokio::test]
async fn test_execute_file_depth_limit_prevents_infinite_recursion() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    // self_exec.wfl executes itself forever; the depth guard must stop it
    fs::write(
        temp_dir.path().join("self_exec.wfl"),
        "execute wfl file at \"self_exec.wfl\" and read output as nested_output\n",
    )
    .expect("Failed to write self-executing file");

    let source = r#"execute wfl file at "self_exec.wfl" and read output as page_output"#;
    let (result, _interpreter) = run_program_in_dir(&temp_dir, source).await;
    let errors = result.expect_err("Expected depth limit error");
    let message = &errors.first().expect("Expected at least one error").message;
    assert!(
        message.contains("depth"),
        "Error should mention nesting depth, got: {message}"
    );
}

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_execute_file_resolves_relative_to_parent_script() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let pages_dir = temp_dir.path().join("pages");
    fs::create_dir_all(&pages_dir).expect("Failed to create pages directory");
    fs::write(pages_dir.join("child.wfl"), "display \"from pages dir\"\n")
        .expect("Failed to write child file");

    // Run with CWD elsewhere: resolution must be relative to main.wfl's directory
    let source = r#"execute wfl file at "pages/child.wfl" and read output as page_output"#;
    let (result, interpreter) = run_program_in_dir(&temp_dir, source).await;
    result.unwrap_or_else(|e| panic!("Interpret failed: {e:?}"));

    assert_eq!(
        get_global_text(&interpreter, "page_output"),
        "from pages dir\n"
    );
}

#[tokio::test]
async fn test_execute_file_dynamic_path_via_variable() {
    // A `with` directly after the path always passes request context, so
    // dynamically chosen pages are built into a variable first (see docs)
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let pages_dir = temp_dir.path().join("pages");
    fs::create_dir_all(&pages_dir).expect("Failed to create pages directory");
    fs::write(pages_dir.join("about.wfl"), "display \"about page\"\n")
        .expect("Failed to write page file");

    let source = concat!(
        "store page_name as \"about\"\n",
        "store page_path as \"pages/\" with page_name with \".wfl\"\n",
        "execute wfl file at page_path and read output as page_output\n",
    );
    let (result, interpreter) = run_program_in_dir(&temp_dir, source).await;
    result.unwrap_or_else(|e| panic!("Interpret failed: {e:?}"));

    assert_eq!(get_global_text(&interpreter, "page_output"), "about page\n");
}

// ---------------------------------------------------------------------------
// Pass-through (no capture clause): child output reaches stdout
// ---------------------------------------------------------------------------

mod test_helpers;

#[test]
fn test_execute_file_without_capture_passes_output_through() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let page_file = temp_dir.path().join("passthrough_page.wfl");
    fs::write(&page_file, "display \"PASSTHROUGH_MARKER\"\n").expect("Failed to write page file");

    // Forward slashes keep the embedded path lexable on Windows (backslashes
    // would be treated as escape sequences in the WFL string literal)
    let page_path = page_file.display().to_string().replace('\\', "/");
    let program = format!("execute wfl file at \"{page_path}\"\ndisplay \"MAIN_DONE\"\n");
    let output = test_helpers::run_wfl_program(&program, "test_execute_file_passthrough");
    test_helpers::assert_wfl_success_with_output(
        &output,
        &["PASSTHROUGH_MARKER", "MAIN_DONE"],
        &[],
    );
}

// ---------------------------------------------------------------------------
// End-to-end: web server executes a page and serves its output (PHP-style)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_web_server_serves_executed_wfl_page() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    fs::write(
        temp_dir.path().join("dynamic_page.wfl"),
        concat!(
            "display \"<h1>Hello from WFL</h1>\"\n",
            "display \"<p>You requested \" with path with \"</p>\"\n",
        ),
    )
    .expect("Failed to write page file");

    let port: u16 = 58123;
    let server_file = temp_dir.path().join("server.wfl");
    let server_code = format!(
        concat!(
            "listen on port {} as test_server\n",
            "wait for request comes in on test_server as incoming_request with timeout 10000\n",
            "execute wfl file at \"dynamic_page.wfl\" with incoming_request and read output as page_output\n",
            "respond to incoming_request with page_output and content_type \"text/html\"\n",
            "close server test_server\n",
        ),
        port
    );
    fs::write(&server_file, &server_code).expect("Failed to write server file");

    // Run the server on a dedicated thread with its own runtime (interpreter is !Send)
    let server_handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&server_code);
            let mut parser = Parser::new(&tokens);
            let ast = parser.parse().expect("Failed to parse server code");
            let mut interpreter = Interpreter::new();
            interpreter.set_source_file(server_file);
            if let Err(e) = interpreter.interpret(&ast).await {
                panic!("Server program failed: {e:?}");
            }
        });
    });

    // Wait for the server to accept connections. Probe with a raw TCP connect:
    // an HTTP probe would consume the server's single `wait for request`.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        match tokio::net::TcpStream::connect(("127.0.0.1", port)).await {
            Ok(_) => break,
            Err(_) if std::time::Instant::now() < deadline => {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
            Err(e) => panic!("Server did not start listening within 10s: {e}"),
        }
    }

    let response = reqwest::get(format!("http://127.0.0.1:{port}/welcome"))
        .await
        .expect("Failed to send request");
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .expect("Missing content-type")
            .to_str()
            .unwrap(),
        "text/html"
    );
    let body = response.text().await.expect("Failed to read body");
    assert!(
        body.contains("<h1>Hello from WFL</h1>"),
        "Body should contain page heading, got: {body}"
    );
    assert!(
        body.contains("You requested /welcome"),
        "Page should see the request path, got: {body}"
    );

    server_handle.join().expect("Server thread panicked");
}
