//! Managed authentication journeys through the compiled WFL binary and HTTP.

use std::fs::File;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use tempfile::TempDir;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

struct AuthServer {
    child: Child,
    dir: TempDir,
    base_url: String,
}

impl AuthServer {
    async fn start(body: &str) -> Self {
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .expect("reserve an ephemeral port")
            .local_addr()
            .expect("read ephemeral port")
            .port();
        let code = body.replace("PORT", &port.to_string());
        let tokens = lex_wfl_with_positions(&code);
        Parser::new(&tokens)
            .parse()
            .unwrap_or_else(|errors| panic!("HTTP fixture must parse: {errors:?}"));
        let dir = tempfile::tempdir().expect("create isolated server directory");
        std::fs::write(dir.path().join("server.wfl"), code).expect("write server fixture");
        std::fs::write(
            dir.path().join(".wflcfg"),
            "web_server_bind_address = 127.0.0.1\ntimeout_seconds = 60\n",
        )
        .expect("write server configuration");
        let stdout = File::create(dir.path().join("stdout.log")).expect("capture stdout");
        let stderr = File::create(dir.path().join("stderr.log")).expect("capture stderr");
        let child = Command::new(env!("CARGO_BIN_EXE_wfl"))
            .arg(dir.path().join("server.wfl"))
            .current_dir(dir.path())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .expect("launch the compiled WFL binary");
        let mut server = Self {
            child,
            dir,
            base_url: format!("http://127.0.0.1:{port}"),
        };
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(status) = server.child.try_wait().expect("inspect server process") {
                panic!(
                    "managed auth server exited before accepting HTTP ({status}): {}",
                    server.diagnostics()
                );
            }
            // A bare TCP readiness probe never enters the WFL request loop.
            if tokio::net::TcpStream::connect(("127.0.0.1", port))
                .await
                .is_ok()
            {
                return server;
            }
            assert!(
                Instant::now() < deadline,
                "server startup exceeded its deadline: {}",
                server.diagnostics()
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    fn diagnostics(&self) -> String {
        let stdout = std::fs::read_to_string(self.dir.path().join("stdout.log"))
            .expect("read captured stdout");
        let stderr = std::fs::read_to_string(self.dir.path().join("stderr.log"))
            .expect("read captured stderr");
        format!("{stdout}\n{stderr}")
    }

    async fn wait_for_clean_exit(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = self.child.try_wait().expect("inspect server exit") {
                assert!(status.success(), "server failed: {}", self.diagnostics());
                return;
            }
            assert!(
                Instant::now() < deadline,
                "server failed to shut down: {}",
                self.diagnostics()
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }
}

impl Drop for AuthServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("create localhost HTTP client")
}

#[tokio::test]
async fn managed_session_issues_secure_cookie_over_http() {
    let mut server = AuthServer::start(
        r#"
store sessions as create_session_store of 3600 and 900 and 100
store session_record as session_create of sessions and "account"
store cookie_value as session_cookie of session_record["id"]
create map response_headers:
    "Set-Cookie" is cookie_value
end map
listen on port PORT as auth_server
wait for request comes in on auth_server as req with timeout 20000
respond to req with session_record["csrf_token"] and headers response_headers
close server auth_server
"#,
    )
    .await;
    let response = client()
        .get(&server.base_url)
        .send()
        .await
        .expect("request a managed session");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let cookie = response
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .expect("session response must issue Set-Cookie")
        .to_str()
        .expect("ASCII session cookie");
    assert!(cookie.starts_with("__Host-wfl_session="));
    for attribute in ["Secure", "HttpOnly", "SameSite=Strict", "Path=/"] {
        assert!(
            cookie.split(';').any(|part| part.trim() == attribute),
            "missing required cookie attribute: {attribute}"
        );
    }
    assert!(!cookie.to_ascii_lowercase().contains("domain="));
    let csrf = response.text().await.expect("read CSRF token");
    assert!(csrf.len() >= 32, "session must issue an unpredictable token");
    server.wait_for_clean_exit().await;
}
