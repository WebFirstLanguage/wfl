// Integration tests for issue #597 web-server gaps:
// 1. Query string exposed as `query` / `query of req`
// 2. Request property access inside actions (header / path / method / body of req)
// 3. parse_multipart of body and content_type

use std::time::Duration;
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

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

#[tokio::test]
async fn test_query_string_accessible_on_request() {
    let port = 8131;
    let server_code = format!(
        r#"
        listen on port {port} as test_server
        wait for request comes in on test_server as req with timeout 10000
        store raw as query of req
        store params as parse_query_string of raw
        store page as params["page"]
        store reply as "page=" with page
        respond to req with reply
        close server test_server
    "#
    );

    let server_handle = start_server_thread(server_code);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{port}/blog?page=2&q=hello"))
        .send()
        .await
        .expect("Failed to send request");

    let body = response.text().await.expect("Failed to read body");
    assert_eq!(
        body, "page=2",
        "query of req should expose the raw query string for parse_query_string"
    );

    let _ = server_handle.join();
}

#[tokio::test]
async fn test_bare_query_variable_in_request_loop() {
    let port = 8132;
    let server_code = format!(
        r#"
        listen on port {port} as test_server
        wait for request comes in on test_server as req with timeout 10000
        respond to req with query
        close server test_server
    "#
    );

    let server_handle = start_server_thread(server_code);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{port}/search?q=wfl"))
        .send()
        .await
        .expect("Failed to send request");

    let body = response.text().await.expect("Failed to read body");
    assert_eq!(
        body, "q=wfl",
        "bare `query` variable should be the raw query string"
    );

    let _ = server_handle.join();
}

#[tokio::test]
async fn test_empty_query_string_is_empty_text() {
    let port = 8133;
    let server_code = format!(
        r#"
        listen on port {port} as test_server
        wait for request comes in on test_server as req with timeout 10000
        check if query is equal to "":
            respond to req with "empty"
        otherwise:
            respond to req with query
        end check
        close server test_server
    "#
    );

    let server_handle = start_server_thread(server_code);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{port}/"))
        .send()
        .await
        .expect("Failed to send request");

    let body = response.text().await.expect("Failed to read body");
    assert_eq!(body, "empty", "missing query string should be empty text");

    let _ = server_handle.join();
}

#[tokio::test]
async fn test_header_path_method_body_accessible_inside_action() {
    let port = 8134;
    let server_code = format!(
        r#"
        define action called handle with parameters req:
            store ua as header "User-Agent" of req
            store p as path of req
            store m as method of req
            store b as body of req
            store reply as m with " " with p with " " with ua with " " with b
            respond to req with reply
        end action

        listen on port {port} as test_server
        wait for request comes in on test_server as req with timeout 10000
        call handle with req
        close server test_server
    "#
    );

    let server_handle = start_server_thread(server_code);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://127.0.0.1:{port}/submit"))
        .header("User-Agent", "wfl-action-test")
        .body("payload")
        .send()
        .await
        .expect("Failed to send request");

    let body = response.text().await.expect("Failed to read body");
    assert_eq!(
        body, "POST /submit wfl-action-test payload",
        "header/path/method/body of req must resolve inside actions from the passed request object"
    );

    let _ = server_handle.join();
}

#[tokio::test]
async fn test_body_size_limit_rejects_oversized_request() {
    let port = 8135;
    let server_code = format!(
        r#"
        listen on port {port} as test_server
        wait for request comes in on test_server as req with timeout 5000
        respond to req with "accepted"
        close server test_server
    "#
    );

    let server_handle = start_server_thread(server_code);
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Default limit is 1 MiB; send just over 1 MiB
    let large_body = vec![b'x'; 1_048_576 + 1];
    let client = reqwest::Client::new();
    let result = client
        .post(format!("http://127.0.0.1:{port}/upload"))
        .body(large_body)
        .send()
        .await;

    // Warp rejects oversized bodies; the client should see an error or non-success
    match result {
        Ok(response) => {
            assert!(
                !response.status().is_success(),
                "oversized body should not be accepted with default 1 MiB limit, got {}",
                response.status()
            );
        }
        Err(_) => {
            // Connection reset / reject is also fine — the body was not processed
        }
    }

    // Clean up: send a small request so the server can exit if it is still waiting,
    // otherwise the timeout will end the wait.
    let _ = client.get(format!("http://127.0.0.1:{port}/")).send().await;
    let _ = server_handle.join();
}
