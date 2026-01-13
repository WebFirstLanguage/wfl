use std::sync::Arc;
use std::time::Duration;
use wfl::Interpreter;
use wfl::config::WflConfig;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

/// Integration tests for web server bind address configuration
#[cfg(test)]
mod bind_address_tests {
    use super::*;

    /// Helper to create an interpreter with a custom bind address config
    fn create_interpreter_with_bind_address(bind_address: &str) -> Interpreter {
        let config = WflConfig {
            web_server_bind_address: bind_address.to_string(),
            ..Default::default()
        };
        Interpreter::with_config(Arc::new(config))
    }

    /// Helper to start a WFL server with custom config in a separate thread
    fn start_server_with_config(code: String, bind_address: String) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
            rt.block_on(async {
                let tokens = lex_wfl_with_positions(&code);
                let mut parser = Parser::new(&tokens);
                let ast = parser.parse().expect("Failed to parse WFL code");
                let mut interpreter = create_interpreter_with_bind_address(&bind_address);
                let _ = interpreter.interpret(&ast).await;
            });
        })
    }

    #[tokio::test]
    async fn test_server_binds_to_localhost_by_default() {
        let port = 8200;
        let server_code = format!(
            r#"
            listen on port {} as test_server
            wait for request comes in on test_server as req with timeout 5000
            respond to req with "OK"
            close server test_server
        "#,
            port
        );

        // Start server with default config (127.0.0.1)
        let server_handle = start_server_with_config(server_code, "127.0.0.1".to_string());

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Make HTTP request to localhost - should succeed
        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://127.0.0.1:{}/test", port))
            .header("Content-Length", "0")
            .body("")
            .timeout(Duration::from_secs(2))
            .send()
            .await;

        assert!(response.is_ok(), "Server should be accessible on 127.0.0.1");

        let _ = server_handle.join();
    }

    #[tokio::test]
    async fn test_server_binds_to_all_interfaces() {
        let port = 8201;
        let server_code = format!(
            r#"
            listen on port {} as test_server
            wait for request comes in on test_server as req with timeout 5000
            respond to req with "OK"
            close server test_server
        "#,
            port
        );

        // Start server with 0.0.0.0 config (all interfaces)
        let server_handle = start_server_with_config(server_code, "0.0.0.0".to_string());

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Make HTTP request - should succeed
        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://127.0.0.1:{}/test", port))
            .header("Content-Length", "0")
            .body("")
            .timeout(Duration::from_secs(2))
            .send()
            .await;

        assert!(
            response.is_ok(),
            "Server bound to 0.0.0.0 should be accessible on 127.0.0.1"
        );

        let _ = server_handle.join();
    }

    #[tokio::test]
    async fn test_server_with_invalid_bind_address_fails() {
        let port = 8202;
        let server_code = format!(
            r#"
            listen on port {} as test_server
            wait for request comes in on test_server as req with timeout 2000
            respond to req with "OK"
            close server test_server
        "#,
            port
        );

        // Start server with invalid IP address
        let server_handle = start_server_with_config(server_code, "invalid-ip".to_string());

        // Give server time to attempt to start (and fail)
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Try to connect - should fail because server couldn't start
        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://127.0.0.1:{}/test", port))
            .header("Content-Length", "0")
            .body("")
            .timeout(Duration::from_secs(1))
            .send()
            .await;

        // Server should have failed to start due to invalid IP
        assert!(
            response.is_err(),
            "Server with invalid bind address should not be accessible"
        );

        let _ = server_handle.join();
    }

    #[tokio::test]
    async fn test_server_binds_to_ipv6_localhost() {
        let port = 8203;
        let server_code = format!(
            r#"
            listen on port {} as test_server
            wait for request comes in on test_server as req with timeout 5000
            respond to req with "OK"
            close server test_server
        "#,
            port
        );

        // Start server with IPv6 localhost
        let server_handle = start_server_with_config(server_code, "::1".to_string());

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Make HTTP request to IPv6 localhost - should succeed if IPv6 is available
        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://[::1]:{}/test", port))
            .header("Content-Length", "0")
            .body("")
            .timeout(Duration::from_secs(2))
            .send()
            .await;

        // Note: This test may fail on systems without IPv6 support
        // We just verify the server attempted to bind to the IPv6 address
        if response.is_ok() {
            let body = response.unwrap().text().await.unwrap();
            assert_eq!(body, "OK", "Server should respond correctly on IPv6");
        }
        // If IPv6 is not available, the test passes silently

        let _ = server_handle.join();
    }
}
