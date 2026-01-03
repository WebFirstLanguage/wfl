use std::time::Duration;
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

// Integration tests for Content-Length header verification
#[cfg(test)]
mod content_length_tests {
    use super::*;

    /// Helper to start a WFL server in a separate thread with its own runtime
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
    async fn test_content_length_ascii() {
        let port = 8098;
        let server_code = format!(
            r#"
            listen on port {} as test_server
            wait for request comes in on test_server as req with timeout 10000
            respond to req with "Hello"
            close server test_server
        "#,
            port
        );

        // Start server in separate thread
        let server_handle = start_server_thread(server_code);

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Make HTTP POST request with Content-Length: 0 (required by warp body filter)
        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://127.0.0.1:{}/test", port))
            .header("Content-Length", "0")
            .body("")
            .send()
            .await
            .expect("Failed to send request");

        // Get headers before consuming body
        let content_length = response
            .headers()
            .get("content-length")
            .expect("Content-Length header missing")
            .to_str()
            .expect("Invalid Content-Length value")
            .to_string();

        // Verify body matches
        let body = response.text().await.expect("Failed to read body");
        assert_eq!(body, "Hello");

        // Verify Content-Length header
        assert_eq!(
            content_length, "5",
            "Content-Length should be 5 for 'Hello'"
        );

        // Wait for server thread to finish
        let _ = server_handle.join();
    }

    #[tokio::test]
    async fn test_content_length_unicode() {
        let port = 8099;
        // "Hello, 世界!" = 7 + 3 + 3 + 1 = 14 bytes (not 10 characters)
        let server_code = format!(
            r#"
            listen on port {} as test_server
            wait for request comes in on test_server as req with timeout 10000
            respond to req with "Hello, 世界!"
            close server test_server
        "#,
            port
        );

        // Start server in separate thread
        let server_handle = start_server_thread(server_code);

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Make HTTP POST request
        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://127.0.0.1:{}/test", port))
            .header("Content-Length", "0")
            .body("")
            .send()
            .await
            .expect("Failed to send request");

        // Get headers before consuming body
        let content_length = response
            .headers()
            .get("content-length")
            .expect("Content-Length header missing")
            .to_str()
            .expect("Invalid Content-Length value")
            .to_string();

        // Verify body matches
        let body = response.text().await.expect("Failed to read body");
        assert_eq!(body, "Hello, 世界!");

        // Verify Content-Length header - should be 14 bytes, not 10 characters
        assert_eq!(
            content_length, "14",
            "Content-Length should be 14 bytes for 'Hello, 世界!' (UTF-8 encoding)"
        );

        // Wait for server thread to finish
        let _ = server_handle.join();
    }

    #[tokio::test]
    async fn test_content_length_empty() {
        let port = 8100;
        let server_code = format!(
            r#"
            listen on port {} as test_server
            wait for request comes in on test_server as req with timeout 10000
            respond to req with ""
            close server test_server
        "#,
            port
        );

        // Start server in separate thread
        let server_handle = start_server_thread(server_code);

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Make HTTP POST request
        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://127.0.0.1:{}/test", port))
            .header("Content-Length", "0")
            .body("")
            .send()
            .await
            .expect("Failed to send request");

        // Get headers before consuming body
        let content_length = response
            .headers()
            .get("content-length")
            .expect("Content-Length header missing")
            .to_str()
            .expect("Invalid Content-Length value")
            .to_string();

        // Verify body is empty
        let body = response.text().await.expect("Failed to read body");
        assert_eq!(body, "");

        // Verify Content-Length header is 0 for empty response
        assert_eq!(
            content_length, "0",
            "Content-Length should be 0 for empty response"
        );

        // Wait for server thread to finish
        let _ = server_handle.join();
    }

    #[tokio::test]
    async fn test_content_length_large_content() {
        let port = 8101;
        // Create a larger response to verify byte counting at scale
        let large_content = "A".repeat(1000);
        let server_code = format!(
            r#"
            listen on port {} as test_server
            wait for request comes in on test_server as req with timeout 10000
            respond to req with "{}"
            close server test_server
        "#,
            port, large_content
        );

        // Start server in separate thread
        let server_handle = start_server_thread(server_code);

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Make HTTP POST request
        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://127.0.0.1:{}/test", port))
            .header("Content-Length", "0")
            .body("")
            .send()
            .await
            .expect("Failed to send request");

        // Get headers before consuming body
        let content_length = response
            .headers()
            .get("content-length")
            .expect("Content-Length header missing")
            .to_str()
            .expect("Invalid Content-Length value")
            .to_string();

        // Verify Content-Length header
        assert_eq!(
            content_length, "1000",
            "Content-Length should be 1000 for 1000-character ASCII string"
        );

        // Wait for server thread to finish
        let _ = server_handle.join();
    }
}
