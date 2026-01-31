use std::time::Duration;
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

// Integration tests for custom headers in response
#[cfg(test)]
mod headers_tests {
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
                if let Err(e) = interpreter.interpret(&ast).await {
                    eprintln!("Interpreter error: {:?}", e);
                }
            });
        })
    }

    #[tokio::test]
    async fn test_custom_headers() {
        let port = 8110;
        let server_code = format!(
            r#"
            listen on port {} as test_server
            wait for request comes in on test_server as req with timeout 10000

            # Create a map for headers (using new string key syntax)
            # Cannot use 'headers' as name because it is reserved for incoming request headers
            create map response_headers:
                "X-Custom-Header" is "MyValue"
                "X-Another-Header" is "AnotherValue"
            end map

            respond to req with "Response with headers" and headers response_headers

            # Wait a bit to ensure response is sent before closing server
            wait for 500 milliseconds

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

        // Verify headers
        let custom_header = response
            .headers()
            .get("x-custom-header")
            .expect("X-Custom-Header missing")
            .to_str()
            .expect("Invalid header value")
            .to_string();

        let another_header = response
            .headers()
            .get("x-another-header")
            .expect("X-Another-Header missing")
            .to_str()
            .expect("Invalid header value")
            .to_string();

        assert_eq!(custom_header, "MyValue");
        assert_eq!(another_header, "AnotherValue");

        // Verify body matches
        let body = response.text().await.expect("Failed to read body");
        assert_eq!(body, "Response with headers");

        // Wait for server thread to finish
        let _ = server_handle.join();
    }
}
