// Regression test for header access against a real HTTP request.
// HTTP header names are case-insensitive and warp normalizes them to
// lowercase, so `header "User-Agent" of req` must find the "user-agent"
// entry. Previously the lookup was exact-match and returned nothing for
// every canonically-spelled header name.

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
async fn test_header_access_is_case_insensitive() {
    let port = 8121;
    let server_code = format!(
        r#"
        listen on port {port} as test_server
        wait for request comes in on test_server as req with timeout 10000
        store agent as header "User-Agent" of req
        store agent_text as "Agent: " with agent
        respond to req with agent_text
        close server test_server
    "#
    );

    let server_handle = start_server_thread(server_code);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{port}/"))
        .header("User-Agent", "wfl-header-test")
        .send()
        .await
        .expect("Failed to send request");

    let body = response.text().await.expect("Failed to read body");
    assert_eq!(
        body, "Agent: wfl-header-test",
        "header \"User-Agent\" should resolve the lowercase 'user-agent' entry"
    );

    let _ = server_handle.join();
}

#[tokio::test]
async fn test_missing_header_is_nothing() {
    let port = 8122;
    let server_code = format!(
        r#"
        listen on port {port} as test_server
        wait for request comes in on test_server as req with timeout 10000
        store custom as header "X-Custom-Header" of req
        check if custom is nothing:
            respond to req with "missing"
        otherwise:
            respond to req with "present"
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
    assert_eq!(body, "missing", "absent header should compare as nothing");

    let _ = server_handle.join();
}
