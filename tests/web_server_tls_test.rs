// Integration tests for HTTPS/TLS web server support:
//   listen on port N secured with certificate "..." and key "..." as server
//   listen on port N secured as server           (paths from config)
//   listen on port N redirecting to port M as server
//
// Self-signed certificates are generated per test with rcgen (localhost +
// 127.0.0.1 SANs); reqwest clients accept them via
// danger_accept_invalid_certs. Ports 8210-8219 (the bind-address tests use
// 8200-8203).

use std::sync::Arc;
use std::time::Duration;
use wfl::Interpreter;
use wfl::config::WflConfig;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

/// Writes a fresh self-signed certificate + key pair into `dir` and returns
/// their paths.
fn write_self_signed_cert(dir: &std::path::Path) -> (String, String) {
    let certified =
        rcgen::generate_simple_self_signed(vec!["localhost".to_string(), "127.0.0.1".to_string()])
            .expect("Failed to generate self-signed certificate");

    let cert_path = dir.join("cert.pem");
    let key_path = dir.join("key.pem");
    std::fs::write(&cert_path, certified.cert.pem()).expect("Failed to write cert.pem");
    std::fs::write(&key_path, certified.key_pair.serialize_pem()).expect("Failed to write key.pem");

    // Forward slashes: these paths are embedded in WFL string literals, and
    // Windows backslashes would be rejected by the WFL lexer. Windows file
    // APIs accept forward-slash paths.
    (
        cert_path.to_string_lossy().replace('\\', "/"),
        key_path.to_string_lossy().replace('\\', "/"),
    )
}

/// Runs a WFL program in its own thread + runtime, like the bind-address tests.
fn start_server_with_config(code: String, config: WflConfig) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let mut parser = Parser::new(&tokens);
            let ast = parser.parse().expect("Failed to parse WFL code");
            let mut interpreter = Interpreter::with_config(Arc::new(config));
            let _ = interpreter.interpret(&ast).await;
        });
    })
}

fn insecure_client() -> reqwest::Client {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to build client")
}

#[tokio::test]
async fn test_https_server_serves_requests() {
    let port = 8210;
    let temp_dir = tempfile::tempdir().unwrap();
    let (cert_path, key_path) = write_self_signed_cert(temp_dir.path());

    let server_code = format!(
        r#"
        listen on port {port} secured with certificate "{cert_path}" and key "{key_path}" as secure_server
        wait for request comes in on secure_server as req with timeout 5000
        respond to req with "Hello over HTTPS"
        close server secure_server
    "#
    );

    let server_handle = start_server_with_config(server_code, WflConfig::default());
    tokio::time::sleep(Duration::from_millis(500)).await;

    let response = insecure_client()
        .get(format!("https://127.0.0.1:{port}/"))
        .timeout(Duration::from_secs(3))
        .send()
        .await;

    let response = response.expect("HTTPS request to secured server should succeed");
    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "Hello over HTTPS");

    let _ = server_handle.join();
}

#[tokio::test]
async fn test_plain_http_to_tls_port_fails() {
    let port = 8211;
    let temp_dir = tempfile::tempdir().unwrap();
    let (cert_path, key_path) = write_self_signed_cert(temp_dir.path());

    let server_code = format!(
        r#"
        listen on port {port} secured with certificate "{cert_path}" and key "{key_path}" as secure_server
        wait for request comes in on secure_server as req with timeout 3000
        respond to req with "unreachable"
        close server secure_server
    "#
    );

    let server_handle = start_server_with_config(server_code, WflConfig::default());
    tokio::time::sleep(Duration::from_millis(500)).await;

    let response = insecure_client()
        .get(format!("http://127.0.0.1:{port}/"))
        .timeout(Duration::from_secs(2))
        .send()
        .await;

    assert!(
        response.is_err(),
        "Plain HTTP request to a TLS port should fail"
    );

    let _ = server_handle.join();
}

#[tokio::test]
async fn test_redirect_server_returns_301_with_location() {
    let http_port = 8212;
    let https_port = 8213;

    // The redirect is answered natively by the server, so the program never
    // sees a request; the timed-out wait just keeps the server alive long
    // enough for the test to hit it.
    let server_code = format!(
        r#"
        listen on port {http_port} redirecting to port {https_port} as redirect_server
        wait for request comes in on redirect_server as req with timeout 4000
        close server redirect_server
    "#
    );

    let server_handle = start_server_with_config(server_code, WflConfig::default());
    tokio::time::sleep(Duration::from_millis(500)).await;

    let response = insecure_client()
        .get(format!("http://127.0.0.1:{http_port}/some/path?x=1&y=2"))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .expect("Redirect server should answer");

    assert_eq!(response.status(), 301, "Expected 301 Moved Permanently");
    let location = response
        .headers()
        .get("location")
        .expect("Location header missing")
        .to_str()
        .unwrap();
    assert_eq!(
        location,
        format!("https://127.0.0.1:{https_port}/some/path?x=1&y=2"),
        "Location should preserve host, path and query, swapping scheme and port"
    );

    let _ = server_handle.join();
}

#[tokio::test]
async fn test_bare_secured_uses_config_paths() {
    let port = 8214;
    let temp_dir = tempfile::tempdir().unwrap();
    let (cert_path, key_path) = write_self_signed_cert(temp_dir.path());

    let server_code = format!(
        r#"
        listen on port {port} secured as secure_server
        wait for request comes in on secure_server as req with timeout 5000
        respond to req with "Config-driven TLS"
        close server secure_server
    "#
    );

    let config = WflConfig {
        web_server_tls_cert_file: Some(cert_path),
        web_server_tls_key_file: Some(key_path),
        ..Default::default()
    };
    let server_handle = start_server_with_config(server_code, config);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let response = insecure_client()
        .get(format!("https://127.0.0.1:{port}/"))
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .expect("HTTPS request using config-supplied cert should succeed");
    assert_eq!(response.text().await.unwrap(), "Config-driven TLS");

    let _ = server_handle.join();
}

#[tokio::test]
async fn test_missing_certificate_file_is_actionable_error() {
    let code = r#"listen on port 8217 secured with certificate "/nonexistent/cert.pem" and key "/nonexistent/key.pem" as secure_server"#;
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse().expect("Failed to parse");

    let mut interpreter = Interpreter::with_config(Arc::new(WflConfig::default()));
    let result = interpreter.interpret(&ast).await;

    let errors = result.expect_err("Missing certificate file should be a runtime error");
    let message = format!("{errors:?}");
    assert!(
        message.contains("/nonexistent/cert.pem"),
        "Error should name the missing certificate file, got: {message}"
    );
}

#[tokio::test]
async fn test_bare_secured_without_config_is_actionable_error() {
    let code = r#"listen on port 8218 secured as secure_server"#;
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse().expect("Failed to parse");

    // Default config has no TLS paths
    let mut interpreter = Interpreter::with_config(Arc::new(WflConfig::default()));
    let result = interpreter.interpret(&ast).await;

    let errors = result.expect_err("Bare 'secured' without config paths should error");
    let message = format!("{errors:?}");
    assert!(
        message.contains("web_server_tls_cert_file"),
        "Error should point at the .wflcfg settings, got: {message}"
    );
}

#[tokio::test]
async fn test_out_of_range_redirect_target_port_is_error() {
    let code = r#"listen on port 8219 redirecting to port 0 as bad_redirect"#;
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse().expect("Failed to parse");

    let mut interpreter = Interpreter::with_config(Arc::new(WflConfig::default()));
    let result = interpreter.interpret(&ast).await;

    let errors = result.expect_err("Out-of-range redirect target port should be a runtime error");
    let message = format!("{errors:?}");
    assert!(
        message.contains("between 1 and 65535"),
        "Error should explain the valid port range, got: {message}"
    );
}

#[tokio::test]
async fn test_dual_http_and_https_servers() {
    let http_port = 8215;
    let https_port = 8216;
    let temp_dir = tempfile::tempdir().unwrap();
    let (cert_path, key_path) = write_self_signed_cert(temp_dir.path());

    let server_code = format!(
        r#"
        listen on port {http_port} as http_server
        listen on port {https_port} secured with certificate "{cert_path}" and key "{key_path}" as secure_server
        wait for request comes in on http_server as req with timeout 5000
        respond to req with "HTTP OK"
        wait for request comes in on secure_server as req2 with timeout 5000
        respond to req2 with "HTTPS OK"
        close server http_server
        close server secure_server
    "#
    );

    let server_handle = start_server_with_config(server_code, WflConfig::default());
    tokio::time::sleep(Duration::from_millis(500)).await;

    let client = insecure_client();

    // The program serves one request per server, HTTP first.
    let http_response = client
        .get(format!("http://127.0.0.1:{http_port}/"))
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .expect("HTTP server should answer");
    assert_eq!(http_response.text().await.unwrap(), "HTTP OK");

    let https_response = client
        .get(format!("https://127.0.0.1:{https_port}/"))
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .expect("HTTPS server should answer");
    assert_eq!(https_response.text().await.unwrap(), "HTTPS OK");

    let _ = server_handle.join();
}
