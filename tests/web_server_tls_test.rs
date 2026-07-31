// Integration tests for HTTPS/TLS web server support:
//   listen on port N secured with certificate "..." and key "..." as server
//   listen on port N secured as server           (paths from config)
//   listen on port N redirecting to port M as server
//
// Self-signed certificates are generated per test with rcgen (localhost +
// 127.0.0.1 SANs); reqwest clients accept them via
// danger_accept_invalid_certs. Ports 8210-8219 (the bind-address tests use
// 8200-8203).

use reqwest::tls::Version as TlsVersion;
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
    // rcgen 0.14 renamed `CertifiedKey.key_pair` to `signing_key`.
    std::fs::write(&key_path, certified.signing_key.serialize_pem())
        .expect("Failed to write key.pem");

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

#[tokio::test]
async fn test_tls_listener_reports_occupied_port_without_panicking() {
    let port = 8220;
    let occupied = std::net::TcpListener::bind(("127.0.0.1", port))
        .expect("Failed to reserve the TLS test port");
    let temp_dir = tempfile::tempdir().unwrap();
    let (cert_path, key_path) = write_self_signed_cert(temp_dir.path());
    let code = format!(
        r#"listen on port {port} secured with certificate "{cert_path}" and key "{key_path}" as secure_server"#
    );
    let tokens = lex_wfl_with_positions(&code);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse().expect("Failed to parse");

    let mut interpreter = Interpreter::with_config(Arc::new(WflConfig::default()));
    let result = interpreter.interpret(&ast).await;

    drop(occupied);
    let errors = result.expect_err("An occupied TLS port should be a runtime error");
    let message = format!("{errors:?}");
    assert!(
        message.contains("Failed to start secure web server")
            && message.contains(&port.to_string()),
        "Error should identify the secure listener and occupied port, got: {message}"
    );
}

#[tokio::test]
async fn test_stalled_tls_handshake_does_not_block_protocol_matrix_or_peer_ip() {
    let port = 8221;
    let temp_dir = tempfile::tempdir().unwrap();
    let (cert_path, key_path) = write_self_signed_cert(temp_dir.path());
    let server_code = format!(
        r#"
        listen on port {port} secured with certificate "{cert_path}" and key "{key_path}" as secure_server
        wait for request comes in on secure_server as req1 with timeout 5000
        respond to req1 with client_ip of req1
        wait for request comes in on secure_server as req2 with timeout 5000
        respond to req2 with "http1"
        wait for request comes in on secure_server as req3 with timeout 5000
        respond to req3 with "tls12"
        wait for request comes in on secure_server as req4 with timeout 5000
        respond to req4 with "tls13"
        close server secure_server
    "#
    );

    let server_handle = start_server_with_config(server_code, WflConfig::default());
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Leave one TCP client connected without sending a TLS ClientHello. A
    // serial handshake in the accept loop would prevent every valid request
    // below from reaching the WFL request queue.
    let stalled_client = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("Failed to open stalled TLS client");

    let h2_response = insecure_client()
        .get(format!("https://127.0.0.1:{port}/peer"))
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .expect("A stalled handshake must not block a valid HTTP/2 client");
    assert_eq!(
        h2_response.version(),
        reqwest::Version::HTTP_2,
        "The secured listener must advertise h2 through ALPN"
    );
    assert_eq!(
        h2_response.text().await.unwrap(),
        "127.0.0.1",
        "The custom TLS transport must preserve the accepted peer address"
    );

    let http1_client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .http1_only()
        .build()
        .expect("Failed to build HTTP/1.1 client");
    let http1_response = http1_client
        .get(format!("https://127.0.0.1:{port}/http1"))
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .expect("The TLS listener must retain HTTP/1.1 fallback");
    assert_eq!(http1_response.version(), reqwest::Version::HTTP_11);
    assert_eq!(http1_response.text().await.unwrap(), "http1");

    let tls12_client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .min_tls_version(TlsVersion::TLS_1_2)
        .max_tls_version(TlsVersion::TLS_1_2)
        .build()
        .expect("Failed to build TLS 1.2 client");
    assert_eq!(
        tls12_client
            .get(format!("https://127.0.0.1:{port}/tls12"))
            .timeout(Duration::from_secs(3))
            .send()
            .await
            .expect("The secured listener must retain TLS 1.2")
            .text()
            .await
            .unwrap(),
        "tls12"
    );

    let tls13_client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .min_tls_version(TlsVersion::TLS_1_3)
        .max_tls_version(TlsVersion::TLS_1_3)
        .build()
        .expect("Failed to build TLS 1.3 client");
    assert_eq!(
        tls13_client
            .get(format!("https://127.0.0.1:{port}/tls13"))
            .timeout(Duration::from_secs(3))
            .send()
            .await
            .expect("The secured listener must retain TLS 1.3")
            .text()
            .await
            .unwrap(),
        "tls13"
    );

    drop(stalled_client);
    let _ = server_handle.join();
}

#[tokio::test]
async fn test_tls_configuration_rejects_malformed_and_mismatched_keys() {
    let malformed_dir = tempfile::tempdir().unwrap();
    let (cert_path, malformed_key_path) = write_self_signed_cert(malformed_dir.path());
    std::fs::write(&malformed_key_path, "not a private key")
        .expect("Failed to replace the test key");

    let malformed_code = format!(
        r#"listen on port 8222 secured with certificate "{cert_path}" and key "{malformed_key_path}" as secure_server"#
    );
    let tokens = lex_wfl_with_positions(&malformed_code);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse().expect("Failed to parse malformed-key case");
    let mut interpreter = Interpreter::with_config(Arc::new(WflConfig::default()));
    let malformed_result = interpreter.interpret(&ast).await;
    let malformed_message = format!(
        "{:?}",
        malformed_result.expect_err("Malformed private key should be rejected")
    );
    assert!(
        malformed_message.contains("contains no private key"),
        "Malformed-key error should explain the PEM requirement, got: {malformed_message}"
    );

    let cert_dir = tempfile::tempdir().unwrap();
    let key_dir = tempfile::tempdir().unwrap();
    let (cert_path, _) = write_self_signed_cert(cert_dir.path());
    let (_, unrelated_key_path) = write_self_signed_cert(key_dir.path());
    let mismatch_code = format!(
        r#"listen on port 8223 secured with certificate "{cert_path}" and key "{unrelated_key_path}" as secure_server"#
    );
    let tokens = lex_wfl_with_positions(&mismatch_code);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse().expect("Failed to parse mismatched-key case");
    let mut interpreter = Interpreter::with_config(Arc::new(WflConfig::default()));
    let mismatch_result = interpreter.interpret(&ast).await;
    let mismatch_message = format!(
        "{:?}",
        mismatch_result.expect_err("Certificate/key mismatch should be rejected")
    );
    assert!(
        mismatch_message.contains("Failed to start secure web server"),
        "Certificate/key mismatch should be a secure-listener error, got: {mismatch_message}"
    );
}

#[tokio::test]
async fn test_close_server_stops_tls_listener() {
    let port = 8224;
    let temp_dir = tempfile::tempdir().unwrap();
    let (cert_path, key_path) = write_self_signed_cert(temp_dir.path());
    let server_code = format!(
        r#"
        listen on port {port} secured with certificate "{cert_path}" and key "{key_path}" as secure_server
        close server secure_server
    "#
    );

    start_server_with_config(server_code, WflConfig::default())
        .join()
        .expect("TLS server program panicked");
    tokio::time::sleep(Duration::from_millis(100)).await;

    let result = insecure_client()
        .get(format!("https://127.0.0.1:{port}/after-close"))
        .timeout(Duration::from_secs(1))
        .send()
        .await;
    assert!(
        result.is_err(),
        "A closed TLS server must not accept a new connection"
    );
}
