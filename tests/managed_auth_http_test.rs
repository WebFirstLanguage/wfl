//! Managed authentication journeys through the compiled WFL binary and HTTP.

use std::fs::File;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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

const AUTH_APPLICATION: &str = r#"
store sessions as create_session_store of 3600 and 900 and 100
store attempts as create_account_rate_limiter of 2 and 60 and 2
listen on port PORT as auth_server
main loop concurrently:
    wait for request comes in on auth_server as req with timeout 20000
    store route_path as req["path"]
    check if route_path is equal to "/shutdown":
        respond to req with "bye"
        close server auth_server
        break
    end check
    check if route_path is equal to "/issue":
        store session_record as session_create of sessions and req["body"]
        store cookie_value as session_cookie of session_record["id"]
        store json_reply as stringify_json of session_record
        create map issue_headers:
            "Set-Cookie" is cookie_value
        end map
        respond to req with json_reply and content_type "application/json" and headers issue_headers
        continue
    end check
    check if route_path is equal to "/attempt":
        store allowed_attempt as account_rate_limit_allow of attempts and req["body"]
        check if allowed_attempt:
            respond to req with "attempt permitted"
        otherwise:
            respond to req with "attempt denied" and status 429
        end check
        continue
    end check
    store guarded as session_csrf_guard of sessions and req
    check if not guarded:
        respond to req with "denied" and status 403
        continue
    end check
    check if route_path is equal to "/rotate":
        store fresh_session as session_rotate of sessions and req["body"]
        check if fresh_session is nothing:
            respond to req with "revoked" and status 403
        otherwise:
            store fresh_cookie as session_cookie of fresh_session["id"]
            store fresh_json as stringify_json of fresh_session
            create map rotate_headers:
                "Set-Cookie" is fresh_cookie
            end map
            respond to req with fresh_json and content_type "application/json" and headers rotate_headers
        end check
        continue
    end check
    check if route_path is equal to "/logout":
        store revoked as session_revoke of sessions and req["body"]
        store revoked_json as stringify_json of revoked
        respond to req with revoked_json
        continue
    end check
    check if route_path is equal to "/revoke-account":
        store revoked_count as session_revoke_account of sessions and req["body"]
        store count_json as stringify_json of revoked_count
        respond to req with count_json
        continue
    end check
    check if route_path is equal to "/lookup":
        store account_name as session_lookup of sessions and req["body"]
        respond to req with account_name
        continue
    end check
    respond to req with "authorized"
end loop
"#;

struct Session {
    id: String,
    csrf: String,
    cookie: String,
}

impl Session {
    async fn from_response(response: reqwest::Response) -> Self {
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let cookie = response
            .headers()
            .get(reqwest::header::SET_COOKIE)
            .expect("issued session must include a cookie")
            .to_str()
            .expect("cookie is ASCII")
            .split(';')
            .next()
            .expect("cookie name and value")
            .to_string();
        let record: serde_json::Value = response.json().await.expect("session JSON");
        let id = record["id"].as_str().expect("session ID").to_string();
        let csrf = record["csrf_token"]
            .as_str()
            .expect("CSRF token")
            .to_string();
        assert_eq!(cookie, format!("__Host-wfl_session={id}"));
        Self { id, csrf, cookie }
    }

    fn request(
        &self,
        client: &reqwest::Client,
        server: &AuthServer,
        path: &str,
    ) -> reqwest::RequestBuilder {
        client
            .post(format!("{}{path}", server.base_url))
            .header(reqwest::header::COOKIE, &self.cookie)
            .header("x-csrf-token", &self.csrf)
    }
}

async fn issue(client: &reqwest::Client, server: &AuthServer, account: &str) -> Session {
    let response = client
        .post(format!("{}/issue", server.base_url))
        .body(account.to_string())
        .send()
        .await
        .expect("issue a session over HTTP");
    Session::from_response(response).await
}

async fn shutdown(client: &reqwest::Client, server: &mut AuthServer) {
    let response = client
        .get(format!("{}/shutdown", server.base_url))
        .send()
        .await
        .expect("request clean server shutdown");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(response.text().await.expect("shutdown response"), "bye");
    server.wait_for_clean_exit().await;
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
    assert!(
        csrf.len() >= 32,
        "session must issue an unpredictable token"
    );
    server.wait_for_clean_exit().await;
}

#[tokio::test]
async fn csrf_guard_rejects_missing_wrong_and_ambiguous_http_credentials() {
    let mut server = AuthServer::start(AUTH_APPLICATION).await;
    let client = client();
    let alice = issue(&client, &server, "alice").await;
    let other = issue(&client, &server, "alice").await;
    let protected = format!("{}/protected", server.base_url);
    for request in [
        client.post(&protected),
        client.post(&protected).header("Cookie", &alice.cookie),
        client.post(&protected).header("x-csrf-token", &alice.csrf),
        client
            .post(&protected)
            .header("Cookie", &alice.cookie)
            .header("x-csrf-token", &other.csrf),
        client
            .post(&protected)
            .header("Cookie", format!("{}; {}", alice.cookie, alice.cookie))
            .header("x-csrf-token", &alice.csrf),
        client
            .post(&protected)
            .header("Cookie", &alice.cookie)
            .header("Cookie", &alice.cookie)
            .header("x-csrf-token", &alice.csrf),
        client
            .post(&protected)
            .header("Cookie", &alice.cookie)
            .header("x-csrf-token", &alice.csrf)
            .header("x-csrf-token", &alice.csrf),
        client
            .post(&protected)
            .header("Cookie", &alice.cookie)
            .header("x-csrf-token", "a".repeat(8192)),
    ] {
        let response = request.send().await.expect("send rejected auth request");
        assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
        assert_eq!(response.text().await.expect("denial response"), "denied");
    }
    for method in [
        reqwest::Method::GET,
        reqwest::Method::HEAD,
        reqwest::Method::OPTIONS,
    ] {
        let denied = client
            .request(method.clone(), &protected)
            .send()
            .await
            .expect("send unauthenticated safe method");
        assert_eq!(denied.status(), reqwest::StatusCode::FORBIDDEN);
        let allowed = client
            .request(method, &protected)
            .header("Cookie", &alice.cookie)
            .send()
            .await
            .expect("send authenticated safe method");
        assert_eq!(allowed.status(), reqwest::StatusCode::OK);
    }
    // HTTP permits obs-text octets in header values. If a request projection
    // cannot retain one losslessly, the guard must reject that request rather
    // than authorize using a silently truncated header map.
    let address = server.base_url.strip_prefix("http://").unwrap();
    let mut stream = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect for raw-header validation");
    let mut raw_request = format!(
        "POST /protected HTTP/1.1\r\nHost: localhost\r\nCookie: {}\r\nx-csrf-token: {}\r\nContent-Length: 0\r\nConnection: close\r\nX-Client-Metadata: ",
        alice.cookie, alice.csrf
    )
    .into_bytes();
    raw_request.extend_from_slice(b"\xff\r\n\r\n");
    stream
        .write_all(&raw_request)
        .await
        .expect("send raw-header request");
    let mut raw_response = Vec::new();
    tokio::time::timeout(
        Duration::from_secs(10),
        stream.read_to_end(&mut raw_response),
    )
    .await
    .expect("raw-header response deadline")
    .expect("read raw-header response");
    assert!(raw_response.starts_with(b"HTTP/1.1 403"));
    assert!(raw_response.ends_with(b"denied"));

    let allowed = alice
        .request(&client, &server, "/protected")
        .send()
        .await
        .expect("valid session and bound CSRF token must authorize POST");
    assert_eq!(allowed.status(), reqwest::StatusCode::OK);
    assert_eq!(
        allowed.text().await.expect("allowed response"),
        "authorized"
    );
    shutdown(&client, &mut server).await;
    let diagnostics = server.diagnostics();
    for secret in [&alice.id, &alice.csrf, &other.id, &other.csrf] {
        assert!(
            !diagnostics.contains(secret),
            "server logs must not include session credentials"
        );
    }
}

#[tokio::test]
async fn rotation_logout_and_account_revocation_take_effect_over_http() {
    let mut server = AuthServer::start(AUTH_APPLICATION).await;
    let client = client();
    let original = issue(&client, &server, "alice").await;
    let second = issue(&client, &server, "alice").await;
    let bob = issue(&client, &server, "bob").await;

    // A denied mutation must leave the original credentials usable.
    let denied_rotation = client
        .post(format!("{}/rotate", server.base_url))
        .header("Cookie", &original.cookie)
        .body(original.id.clone())
        .send()
        .await
        .expect("attempt rotation without CSRF");
    assert_eq!(denied_rotation.status(), reqwest::StatusCode::FORBIDDEN);
    let fresh = Session::from_response(
        original
            .request(&client, &server, "/rotate")
            .body(original.id.clone())
            .send()
            .await
            .expect("rotate session with valid credentials"),
    )
    .await;
    assert_ne!(fresh.id, original.id);
    assert_ne!(fresh.csrf, original.csrf);
    for (cookie, csrf) in [
        (&original.cookie, &original.csrf),
        (&original.cookie, &fresh.csrf),
        (&fresh.cookie, &original.csrf),
    ] {
        let response = client
            .post(format!("{}/protected", server.base_url))
            .header("Cookie", cookie)
            .header("x-csrf-token", csrf)
            .send()
            .await
            .expect("send stale credentials after rotation");
        assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    }
    let lookup = fresh
        .request(&client, &server, "/lookup")
        .body(fresh.id.clone())
        .send()
        .await
        .expect("look up rotated session");
    assert_eq!(lookup.status(), reqwest::StatusCode::OK);
    assert_eq!(lookup.text().await.expect("authenticated account"), "alice");

    let revoked = fresh
        .request(&client, &server, "/logout")
        .body(fresh.id.clone())
        .send()
        .await
        .expect("logout");
    assert_eq!(revoked.status(), reqwest::StatusCode::OK);
    assert_eq!(revoked.text().await.expect("logout result"), "true");
    let stale = fresh
        .request(&client, &server, "/protected")
        .send()
        .await
        .expect("replay logged-out session");
    assert_eq!(stale.status(), reqwest::StatusCode::FORBIDDEN);

    let third = issue(&client, &server, "alice").await;
    let revoked_account = second
        .request(&client, &server, "/revoke-account")
        .body("alice")
        .send()
        .await
        .expect("revoke all remaining account sessions");
    assert_eq!(revoked_account.status(), reqwest::StatusCode::OK);
    assert_eq!(
        revoked_account
            .json::<f64>()
            .await
            .expect("revocation count"),
        2.0
    );
    for session in [&second, &third] {
        let response = session
            .request(&client, &server, "/protected")
            .send()
            .await
            .expect("replay account-revoked credentials");
        assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    }
    let unaffected = bob
        .request(&client, &server, "/protected")
        .send()
        .await
        .expect("another account remains authenticated");
    assert_eq!(unaffected.status(), reqwest::StatusCode::OK);
    shutdown(&client, &mut server).await;
}

#[tokio::test]
async fn simultaneous_http_attempts_share_account_limits_and_capacity() {
    let mut server = AuthServer::start(AUTH_APPLICATION).await;
    let client = client();
    let requests = (0..8).map(|_| {
        client
            .post(format!("{}/attempt", server.base_url))
            .body("alice")
            .send()
    });
    let results = futures_util::future::join_all(requests).await;
    let statuses = results
        .into_iter()
        .map(|result| result.expect("concurrent login attempt response").status())
        .collect::<Vec<_>>();
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == reqwest::StatusCode::OK)
            .count(),
        2,
        "concurrent handlers must share the same account budget"
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == reqwest::StatusCode::TOO_MANY_REQUESTS)
            .count(),
        6
    );
    for (account, expected) in [
        ("bob", reqwest::StatusCode::OK),
        ("eve", reqwest::StatusCode::TOO_MANY_REQUESTS),
        ("alice", reqwest::StatusCode::TOO_MANY_REQUESTS),
        ("bob", reqwest::StatusCode::OK),
        ("bob", reqwest::StatusCode::TOO_MANY_REQUESTS),
    ] {
        let response = client
            .post(format!("{}/attempt", server.base_url))
            .body(account)
            .send()
            .await
            .expect("account attempt at capacity");
        assert_eq!(response.status(), expected);
    }
    shutdown(&client, &mut server).await;
}

#[tokio::test]
async fn simultaneous_rotations_only_issue_one_successor_session() {
    let mut server = AuthServer::start(AUTH_APPLICATION).await;
    let client = client();
    let original = issue(&client, &server, "alice").await;
    let requests = (0..2).map(|_| {
        original
            .request(&client, &server, "/rotate")
            .body(original.id.clone())
            .send()
    });
    let responses = futures_util::future::join_all(requests).await;
    let mut successor = None;
    let mut denied = 0;
    for response in responses {
        let response = response.expect("concurrent rotation response");
        if response.status() == reqwest::StatusCode::OK {
            assert!(successor.is_none(), "a session may only rotate once");
            successor = Some(Session::from_response(response).await);
        } else {
            assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
            denied += 1;
        }
    }
    assert_eq!(denied, 1);
    let successor = successor.expect("one rotation must succeed");
    let valid = successor
        .request(&client, &server, "/protected")
        .send()
        .await
        .expect("the successor session is usable");
    assert_eq!(valid.status(), reqwest::StatusCode::OK);
    let replay = original
        .request(&client, &server, "/protected")
        .send()
        .await
        .expect("attempt to replay the original session");
    assert_eq!(replay.status(), reqwest::StatusCode::FORBIDDEN);
    shutdown(&client, &mut server).await;
}
