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

    async fn serve_and_post(tag: &str, prefix: &str, respond_with: &str) -> reqwest::Response {
        let ready_path = crate::common::unique_ready_path(tag);
        let publish = crate::common::publish_ready_wfl(&ready_path, prefix, "test_server");
        let server_code = format!(
            r#"
            listen on port 0 as test_server
            {publish}
            wait for request comes in on test_server as req with timeout 10000
            respond to req with {respond_with}
            close server test_server
        "#
        );

        let server_handle = start_server_thread(server_code);
        let address = crate::common::wait_for_published_web_server(&ready_path, prefix).await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{address}/test"))
            .header("Content-Length", "0")
            .body("")
            .send()
            .await
            .expect("Failed to send request");

        let _ = server_handle.join();
        let _ = std::fs::remove_file(&ready_path);
        response
    }

    #[tokio::test]
    async fn test_content_length_ascii() {
        let response = serve_and_post("cl_ascii", "CL_ASCII_READY ", "\"Hello\"").await;

        let content_length = response
            .headers()
            .get("content-length")
            .expect("Content-Length header missing")
            .to_str()
            .expect("Invalid Content-Length value")
            .to_string();

        let body = response.text().await.expect("Failed to read body");
        assert_eq!(body, "Hello");
        assert_eq!(
            content_length, "5",
            "Content-Length should be 5 for 'Hello'"
        );
    }

    #[tokio::test]
    async fn test_content_length_unicode() {
        // "Hello, 世界!" = 7 + 3 + 3 + 1 = 14 bytes (not 10 characters)
        let response = serve_and_post("cl_unicode", "CL_UNICODE_READY ", "\"Hello, 世界!\"").await;

        let content_length = response
            .headers()
            .get("content-length")
            .expect("Content-Length header missing")
            .to_str()
            .expect("Invalid Content-Length value")
            .to_string();

        let body = response.text().await.expect("Failed to read body");
        assert_eq!(body, "Hello, 世界!");
        assert_eq!(
            content_length, "14",
            "Content-Length should be 14 bytes for 'Hello, 世界!' (UTF-8 encoding)"
        );
    }

    #[tokio::test]
    async fn test_content_length_empty() {
        let response = serve_and_post("cl_empty", "CL_EMPTY_READY ", "\"\"").await;

        let content_length = response
            .headers()
            .get("content-length")
            .expect("Content-Length header missing")
            .to_str()
            .expect("Invalid Content-Length value")
            .to_string();

        let body = response.text().await.expect("Failed to read body");
        assert_eq!(body, "");
        assert_eq!(
            content_length, "0",
            "Content-Length should be 0 for empty response"
        );
    }

    #[tokio::test]
    async fn test_content_length_large_content() {
        let large_content = "A".repeat(1000);
        let response = serve_and_post(
            "cl_large",
            "CL_LARGE_READY ",
            &format!("\"{large_content}\""),
        )
        .await;

        let content_length = response
            .headers()
            .get("content-length")
            .expect("Content-Length header missing")
            .to_str()
            .expect("Invalid Content-Length value")
            .to_string();

        assert_eq!(
            content_length, "1000",
            "Content-Length should be 1000 for 1000-character ASCII string"
        );
    }
}
