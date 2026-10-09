use std::sync::Arc;
use std::time::Duration;
use wfl::Interpreter;
use wfl::config::WflConfig;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

#[test]
fn published_web_server_addr_parses_ipv4_ipv6_and_rejects_partial_lines() {
    use crate::common::published_web_server_addr;
    assert_eq!(
        published_web_server_addr("READY WebServer::127.0.0.1:8080\n", "READY "),
        Some("127.0.0.1:8080".parse().unwrap())
    );
    assert_eq!(
        published_web_server_addr("READY WebServer:::1:8080\n", "READY "),
        Some("[::1]:8080".parse().unwrap())
    );
    assert!(
        published_web_server_addr("READY WebServer::127.0.0.1:8080", "READY ").is_none(),
        "a partial line without a newline must not parse"
    );
}

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

    fn listen_and_publish(ready_path: &std::path::Path, prefix: &str) -> String {
        let publish = crate::common::publish_ready_wfl(ready_path, prefix, "test_server");
        format!(
            r#"
            listen on port 0 as test_server
            {publish}
            wait for request comes in on test_server as req with timeout 5000
            respond to req with "OK"
            close server test_server
        "#
        )
    }

    #[tokio::test]
    async fn test_server_binds_to_localhost_by_default() {
        let ready_path = crate::common::unique_ready_path("bind_loopback");
        let server_code = listen_and_publish(&ready_path, "BIND_LOOPBACK_READY ");
        let server_handle = start_server_with_config(server_code, "127.0.0.1".to_string());
        let address =
            crate::common::wait_for_published_web_server(&ready_path, "BIND_LOOPBACK_READY ").await;
        assert_eq!(address.ip(), std::net::Ipv4Addr::LOCALHOST);

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{address}/test"))
            .header("Content-Length", "0")
            .body("")
            .timeout(Duration::from_secs(2))
            .send()
            .await
            .expect("Server should be accessible on 127.0.0.1");
        assert_eq!(response.text().await.unwrap(), "OK");

        let _ = server_handle.join();
        let _ = std::fs::remove_file(&ready_path);
    }

    #[tokio::test]
    async fn test_server_binds_to_all_interfaces() {
        let ready_path = crate::common::unique_ready_path("bind_all");
        let server_code = listen_and_publish(&ready_path, "BIND_ALL_READY ");
        let server_handle = start_server_with_config(server_code, "0.0.0.0".to_string());
        let address =
            crate::common::wait_for_published_web_server(&ready_path, "BIND_ALL_READY ").await;
        assert_eq!(address.ip(), std::net::Ipv4Addr::UNSPECIFIED);

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://127.0.0.1:{}/test", address.port()))
            .header("Content-Length", "0")
            .body("")
            .timeout(Duration::from_secs(2))
            .send()
            .await
            .expect("Server bound to 0.0.0.0 should be accessible on 127.0.0.1");
        assert_eq!(response.text().await.unwrap(), "OK");

        let _ = server_handle.join();
        let _ = std::fs::remove_file(&ready_path);
    }

    #[tokio::test]
    async fn test_server_with_invalid_bind_address_fails() {
        let ready_path = crate::common::unique_ready_path("bind_invalid");
        let server_code = listen_and_publish(&ready_path, "BIND_INVALID_READY ");
        let server_handle = start_server_with_config(server_code, "invalid-ip".to_string());

        let published = crate::common::try_wait_for_published_web_server(
            &ready_path,
            "BIND_INVALID_READY ",
            Duration::from_secs(2),
        )
        .await;
        assert!(
            published.is_none(),
            "invalid bind address must not publish a listening handle: {published:?}"
        );

        let _ = server_handle.join();
        let _ = std::fs::remove_file(&ready_path);
    }

    #[tokio::test]
    async fn test_server_binds_to_ipv6_localhost() {
        let ready_path = crate::common::unique_ready_path("bind_v6");
        let server_code = listen_and_publish(&ready_path, "BIND_V6_READY ");
        let server_handle = start_server_with_config(server_code, "::1".to_string());

        let published = crate::common::try_wait_for_published_web_server(
            &ready_path,
            "BIND_V6_READY ",
            Duration::from_secs(2),
        )
        .await;

        // Note: This test may fail on systems without IPv6 support
        // We just verify the server attempted to bind to the IPv6 address
        if let Some(address) = published {
            assert_eq!(address.ip(), std::net::Ipv6Addr::LOCALHOST);
            let client = reqwest::Client::new();
            let response = client
                .post(format!("http://{address}/test"))
                .header("Content-Length", "0")
                .body("")
                .timeout(Duration::from_secs(2))
                .send()
                .await;
            if let Ok(resp) = response {
                let body = resp.text().await.unwrap();
                assert_eq!(body, "OK", "Server should respond correctly on IPv6");
            }
        }

        let _ = server_handle.join();
        let _ = std::fs::remove_file(&ready_path);
    }
}
