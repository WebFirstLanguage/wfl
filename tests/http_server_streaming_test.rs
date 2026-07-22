// Tests for streamed server responses (item 3):
//   start streaming response to <req> [with status <e>] [and content type <e>] as <out>
//   write line <value> to <out>     // frames a line (newline appended)
//   write chunk <value> to <out>    // raw bytes/text, verbatim
//   flush <out>
//   close <out>                     // ends the response body
//
// A WFL web server streams a response; a reqwest client reads it back.

use std::time::Duration;
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::Statement;

// ----------------------------- parser tests ------------------------------

fn parse_single_statement(code: &str) -> Statement {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser
        .parse()
        .unwrap_or_else(|e| panic!("Parse error for {code:?}: {e:?}"));
    assert_eq!(
        program.statements.len(),
        1,
        "Expected exactly one statement for {code:?}"
    );
    program.statements.into_iter().next().unwrap()
}

#[test]
fn test_start_streaming_response_parses() {
    let stmt = parse_single_statement(
        r#"start streaming response to req with status 200 and content type "application/x-ndjson" as out"#,
    );
    match stmt {
        Statement::StartStreamingResponseStatement {
            status,
            content_type,
            variable_name,
            ..
        } => {
            assert!(status.is_some());
            assert!(content_type.is_some());
            assert_eq!(variable_name, "out");
        }
        other => panic!("Expected StartStreamingResponseStatement, got {other:?}"),
    }
}

#[test]
fn test_content_type_variable_binds_correct_name() {
    // `content type <var>` where <var> is a bare identifier: the lexer merges it
    // into `type <var>`, so the parser must split the marker off and bind the
    // variable, not `type <var>` as one name.
    let stmt = parse_single_statement(
        r#"start streaming response to req with status 200 and content type ct as out"#,
    );
    match stmt {
        Statement::StartStreamingResponseStatement { content_type, .. } => match content_type {
            Some(wfl::parser::ast::Expression::Variable(name, _, _)) => assert_eq!(name, "ct"),
            other => panic!("Expected content type Variable(\"ct\"), got {other:?}"),
        },
        other => panic!("Expected StartStreamingResponseStatement, got {other:?}"),
    }
}

#[test]
fn test_write_line_parses() {
    let stmt = parse_single_statement(r#"write line payload to out"#);
    match stmt {
        Statement::StreamWriteStatement { is_line, .. } => assert!(is_line),
        other => panic!("Expected StreamWriteStatement, got {other:?}"),
    }
}

#[test]
fn test_write_chunk_parses() {
    let stmt = parse_single_statement(r#"write chunk payload to out"#);
    match stmt {
        Statement::StreamWriteStatement { is_line, .. } => assert!(!is_line),
        other => panic!("Expected StreamWriteStatement, got {other:?}"),
    }
}

#[test]
fn test_flush_parses() {
    let stmt = parse_single_statement("flush out");
    match stmt {
        Statement::FlushStreamStatement { .. } => {}
        other => panic!("Expected FlushStreamStatement, got {other:?}"),
    }
}

#[test]
fn test_write_bare_line_variable_to_file_still_parses() {
    // Backward compat: `write <var> to <file>` with a variable literally named
    // `line`/`chunk` must NOT be intercepted as a stream write (regression).
    for src in ["write line to out", "write chunk to out"] {
        let stmt = parse_single_statement(src);
        match stmt {
            Statement::WriteToStatement { .. } => {}
            other => panic!("Expected WriteToStatement for {src:?}, got {other:?}"),
        }
    }
}

// ----------------------------- runtime tests ------------------------------

fn start_server_thread(code: String) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let mut parser = Parser::new(&tokens);
            let ast = parser.parse().expect("Failed to parse WFL code");
            let mut interpreter = Interpreter::new();
            let _ = interpreter.interpret(&ast).await;
        });
    })
}

/// Wait until the WFL server has bound `port` and is accepting connections,
/// instead of a fixed sleep that flakes on a loaded CI runner (spurious
/// `Connection refused` when binding takes longer than the guess). A bare TCP
/// connect that drops immediately is a safe readiness probe.
async fn wait_for_server(port: u16) {
    let addr = format!("127.0.0.1:{port}");
    for _ in 0..300 {
        if tokio::net::TcpStream::connect(&addr).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("server on {addr} did not become ready in time");
}

#[tokio::test]
async fn test_streamed_response_lines_and_headers() {
    let port = 8231;
    let server_code = format!(
        r#"
        listen on port {port} as s
        wait for request comes in on s as req with timeout 10000
        start streaming response to req with status 200 and content type "application/x-ndjson" as out
        write line "alpha" to out
        write line "beta" to out
        flush out
        write line "gamma" to out
        close out
        close server s
    "#
    );

    let server_handle = start_server_thread(server_code);
    wait_for_server(port).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{port}/stream"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status().as_u16(), 200);
    let ct = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert_eq!(ct, "application/x-ndjson");

    let body = response.text().await.expect("Failed to read body");
    assert_eq!(body, "alpha\nbeta\ngamma\n");

    let _ = server_handle.join();
}

#[tokio::test]
async fn test_write_after_close_does_not_reach_client() {
    // Writing after `close out` is a catchable error and does NOT reach the
    // client: the client sees only the bytes written before close.
    let port = 8233;
    let server_code = format!(
        r#"
        listen on port {port} as s
        wait for request comes in on s as req with timeout 10000
        start streaming response to req with status 200 and content type "text/plain" as out
        write line "before" to out
        close out
        try:
            write line "after" to out
        catch:
            display "write after close correctly errored"
        end try
        close server s
    "#
    );

    let server_handle = start_server_thread(server_code);
    wait_for_server(port).await;

    let response = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/x"))
        .send()
        .await
        .expect("request failed");
    let body = response.text().await.unwrap();
    assert_eq!(
        body, "before\n",
        "writes after close must not reach the client"
    );

    let _ = server_handle.join();
}

#[tokio::test]
async fn test_stream_auto_closes_when_handler_ends_without_close() {
    // Lifecycle guarantee (spec item 5): a handler that starts a stream and ends
    // WITHOUT `close out` must still finalize the client's body on the way out.
    // Otherwise the sender lingers in the interpreter's stream table, the body is
    // never terminated, and the client hangs forever (and the table leaks).
    let port = 8234;
    let server_code = format!(
        r#"
        listen on port {port} as s
        main loop:
            wait for request comes in on s as req with timeout 20000
            store p as req["path"]
            check if p is equal to "/shutdown":
                respond to req with "bye"
                close server s
                break
            otherwise:
                start streaming response to req with status 200 and content type "text/plain" as out
                write line "hello" to out
            end check
        end loop
    "#
    );

    let server_handle = start_server_thread(server_code);
    wait_for_server(port).await;

    let response = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/x"))
        .send()
        .await
        .expect("request failed");
    assert_eq!(response.status().as_u16(), 200);

    // Reading the body must COMPLETE (the stream was auto-closed). If the handler
    // leaked the stream, this read hangs — the timeout turns that into a failure
    // instead of a stuck test.
    let body = tokio::time::timeout(Duration::from_secs(5), response.text())
        .await
        .expect("body did not finish — stream was not auto-closed when the handler ended")
        .expect("failed to read body");
    assert_eq!(body, "hello\n");

    // Stop the server.
    let _ = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/shutdown"))
        .send()
        .await;
    let _ = server_handle.join();
}

#[tokio::test]
async fn test_streamed_response_write_chunk_verbatim() {
    let port = 8232;
    let server_code = format!(
        r#"
        listen on port {port} as s
        wait for request comes in on s as req with timeout 10000
        start streaming response to req with status 201 and content type "text/plain" as out
        write chunk "one" to out
        write chunk "two" to out
        close out
        close server s
    "#
    );

    let server_handle = start_server_thread(server_code);
    wait_for_server(port).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{port}/raw"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status().as_u16(), 201);
    let body = response.text().await.expect("Failed to read body");
    // write chunk does not append newlines.
    assert_eq!(body, "onetwo");

    let _ = server_handle.join();
}
