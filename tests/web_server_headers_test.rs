use std::net::TcpListener;
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
                let _ = interpreter.interpret(&ast).await;
            });
        })
    }

    #[tokio::test]
    async fn test_custom_headers() {
        // Use an ephemeral port to avoid conflicts
        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to ephemeral port");
        let port = listener.local_addr().unwrap().port();
        drop(listener); // Release the port so WFL server can bind to it

        let server_code = format!(
            r#"
            listen on port {} as test_server
            wait for request comes in on test_server as req with timeout 10000

            # Create a map for headers (using new string key syntax)
            # Cannot use 'headers' as name because it is reserved for incoming request headers
            create map response_headers:
                "X-Custom-Header" is "MyValue"
                "X-Another-Header" is "AnotherValue"
                # This should be ignored by security check
                "Content-Type" is "application/x-hack"
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
        tokio::time::sleep(Duration::from_millis(500)).await;

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

        // Verify that Content-Type was NOT overridden (should default to text/plain)
        let content_type = response
            .headers()
            .get("content-type")
            .expect("Content-Type missing")
            .to_str()
            .expect("Invalid content-type value")
            .to_string();

        assert_eq!(content_type, "text/plain");

        // Verify body matches
        let body = response.text().await.expect("Failed to read body");
        assert_eq!(body, "Response with headers");

        // Wait for server thread to finish
        let _ = server_handle.join();
    }
}
