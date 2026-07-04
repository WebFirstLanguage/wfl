// Integration tests for serving and receiving BINARY content over the WFL
// web server (issue #573). Verifies bytes survive end-to-end: file -> read
// binary -> respond -> warp -> client, and client -> request body -> body_bytes.

use std::time::Duration;
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

/// Start a WFL server in a separate thread with its own Tokio runtime.
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

/// A 1024-byte fixture spanning every byte value 0x00..=0xFF (repeated), which
/// contains invalid-UTF-8 sequences (e.g. lone 0xFF/0x80). Any lossy String
/// round-trip would corrupt it, so byte-identical output proves losslessness.
fn binary_fixture() -> Vec<u8> {
    let mut bytes = Vec::with_capacity(1024);
    for _ in 0..4 {
        bytes.extend(0u8..=255);
    }
    bytes
}

/// Absolute path (forward slashes) to a fresh temp file, safe to embed in a
/// WFL string literal on any platform.
fn temp_path(tag: &str) -> String {
    let mut p = std::env::temp_dir();
    p.push(format!("wfl_binary_test_{tag}.bin"));
    p.to_string_lossy().replace('\\', "/")
}

#[tokio::test]
async fn test_serve_binary_font_bytes_lossless() {
    let port = 8112;
    let fixture = binary_fixture();
    let path = temp_path(&format!("serve_{port}"));
    std::fs::write(&path, &fixture).expect("write fixture");

    let server_code = format!(
        r#"
        listen on port {port} as test_server
        wait for request comes in on test_server as req with timeout 10000
        open file at "{path}" for reading binary as f
        store payload as read binary from f
        close file f
        respond to req with payload and content_type "font/ttf"
        close server test_server
    "#
    );

    let server_handle = start_server_thread(server_code);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://127.0.0.1:{port}/font"))
        .header("Content-Length", "0")
        .body("")
        .send()
        .await
        .expect("Failed to send request");

    let content_type = response
        .headers()
        .get("content-type")
        .expect("Content-Type header missing")
        .to_str()
        .unwrap()
        .to_string();
    let content_length = response
        .headers()
        .get("content-length")
        .expect("Content-Length header missing")
        .to_str()
        .unwrap()
        .to_string();

    let body = response.bytes().await.expect("read body");

    assert_eq!(content_type, "font/ttf");
    assert_eq!(
        content_length,
        fixture.len().to_string(),
        "Content-Length must equal the exact byte count"
    );
    assert_eq!(
        body.as_ref(),
        fixture.as_slice(),
        "served bytes must be byte-identical to the source file"
    );

    let _ = server_handle.join();
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn test_serve_binary_defaults_to_octet_stream() {
    let port = 8113;
    let fixture = binary_fixture();
    let path = temp_path(&format!("serve_{port}"));
    std::fs::write(&path, &fixture).expect("write fixture");

    // No content_type clause -> binary content should default to
    // application/octet-stream (not text/plain).
    let server_code = format!(
        r#"
        listen on port {port} as test_server
        wait for request comes in on test_server as req with timeout 10000
        open file at "{path}" for reading binary as f
        store payload as read binary from f
        close file f
        respond to req with payload
        close server test_server
    "#
    );

    let server_handle = start_server_thread(server_code);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://127.0.0.1:{port}/asset"))
        .header("Content-Length", "0")
        .body("")
        .send()
        .await
        .expect("Failed to send request");

    let content_type = response
        .headers()
        .get("content-type")
        .expect("Content-Type header missing")
        .to_str()
        .unwrap()
        .to_string();
    let body = response.bytes().await.expect("read body");

    assert_eq!(content_type, "application/octet-stream");
    assert_eq!(body.as_ref(), fixture.as_slice());

    let _ = server_handle.join();
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn test_inbound_binary_body_roundtrip() {
    let port = 8114;
    let fixture = binary_fixture();

    // Echo the raw request bytes straight back via body_bytes. If the inbound
    // path were still text-only, non-UTF-8 bytes would be mangled.
    let server_code = format!(
        r#"
        listen on port {port} as test_server
        wait for request comes in on test_server as req with timeout 10000
        respond to req with body_bytes and content_type "application/octet-stream"
        close server test_server
    "#
    );

    let server_handle = start_server_thread(server_code);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://127.0.0.1:{port}/upload"))
        .body(fixture.clone())
        .send()
        .await
        .expect("Failed to send request");

    let body = response.bytes().await.expect("read body");
    assert_eq!(
        body.as_ref(),
        fixture.as_slice(),
        "echoed request body must be byte-identical (inbound binary preserved)"
    );

    let _ = server_handle.join();
}

/// Text responses must be entirely unchanged by the bytes migration.
#[tokio::test]
async fn test_text_response_unchanged() {
    let port = 8115;
    let server_code = format!(
        r#"
        listen on port {port} as test_server
        wait for request comes in on test_server as req with timeout 10000
        respond to req with "Hello, 世界!"
        close server test_server
    "#
    );

    let server_handle = start_server_thread(server_code);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://127.0.0.1:{port}/text"))
        .header("Content-Length", "0")
        .body("")
        .send()
        .await
        .expect("Failed to send request");

    let content_length = response
        .headers()
        .get("content-length")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let body = response.text().await.expect("read body");

    assert_eq!(body, "Hello, 世界!");
    assert_eq!(content_length, "14", "UTF-8 byte length preserved");

    let _ = server_handle.join();
}
