// Regression tests for outbound HTTP requests that die on a *reused* keep-alive
// connection (issue: "connection reuse timeout").
//
// The failure in the wild: WFL's shared `reqwest::Client` pools keep-alive
// sockets for reqwest's default 90-second idle window, while the peer closes
// idle sockets far sooner (Node's `keepAliveTimeout` is 5s; many proxies sit at
// 5-15s). After an idle gap the next POST is written onto a socket the peer has
// already closed. hyper will not silently replay a non-idempotent request, so
// the send fails permanently — surfacing to the WFL program as
// "Failed to send HTTP POST request: error sending request for url (...)"
// instead of transparently reconnecting.
//
// A request that never received a response head demonstrably produced no
// observable effect the caller can see, so re-sending it once on a fresh
// connection is the correct recovery — that is what these tests pin down, for
// BOTH send sites (the buffered `read response` path and the `stream response`
// path), plus the bound that stops the retry from looping.
//
// The upstream here is a raw TCP server so the "peer closed the keep-alive
// socket" moment is deterministic rather than a timing race: it answers
// normally except for the request numbers it is told to drop, where it closes
// the socket without writing a response — exactly what the client observes when
// it writes onto a socket the peer had already decided to close.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

#[derive(Default)]
struct ServerStats {
    /// Requests fully read off the wire, across every connection.
    requests: AtomicUsize,
    /// Accepted TCP connections.
    connections: AtomicUsize,
    /// Requests that arrived on an already-used (kept-alive) connection.
    reused_connection_requests: AtomicUsize,
}

/// Which request numbers (1-based, counted across all connections) the upstream
/// should answer by closing the socket instead of writing a response.
#[derive(Clone)]
enum DropPolicy {
    Requests(Vec<usize>),
    All,
}

impl DropPolicy {
    fn drops(&self, request_number: usize) -> bool {
        match self {
            DropPolicy::Requests(numbers) => numbers.contains(&request_number),
            DropPolicy::All => true,
        }
    }
}

/// A keep-alive HTTP/1.1 upstream that serves `reply<N>\n` for request N, except
/// for the request numbers `policy` drops — those get the socket closed with no
/// response at all, standing in for a peer whose idle timeout fired.
async fn spawn_upstream(policy: DropPolicy) -> (String, Arc<ServerStats>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let stats = Arc::new(ServerStats::default());
    let server_stats = Arc::clone(&stats);

    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            server_stats.connections.fetch_add(1, Ordering::SeqCst);
            let policy = policy.clone();
            let conn_stats = Arc::clone(&server_stats);

            tokio::spawn(async move {
                let mut buf: Vec<u8> = Vec::new();
                let mut requests_on_this_connection = 0usize;

                loop {
                    // Read one complete request (head + Content-Length body).
                    let mut tmp = [0u8; 1024];
                    let header_end = loop {
                        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                            break Some(pos);
                        }
                        match socket.read(&mut tmp).await {
                            Ok(0) | Err(_) => break None,
                            Ok(n) => buf.extend_from_slice(&tmp[..n]),
                        }
                    };
                    let Some(header_end) = header_end else {
                        return; // client hung up between requests
                    };

                    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
                    let content_length = head
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())?
                        })
                        .unwrap_or(0);

                    let body_start = header_end + 4;
                    while buf.len() < body_start + content_length {
                        match socket.read(&mut tmp).await {
                            Ok(0) | Err(_) => return,
                            Ok(n) => buf.extend_from_slice(&tmp[..n]),
                        }
                    }
                    buf.drain(..body_start + content_length);

                    let number = conn_stats.requests.fetch_add(1, Ordering::SeqCst) + 1;
                    if requests_on_this_connection > 0 {
                        conn_stats
                            .reused_connection_requests
                            .fetch_add(1, Ordering::SeqCst);
                    }
                    requests_on_this_connection += 1;

                    if policy.drops(number) {
                        // The peer's idle timeout "fired": close without a
                        // response. Dropping the socket at the end of this task
                        // sends the FIN.
                        return;
                    }

                    let body = format!("reply{number}\n");
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{body}",
                        body.len()
                    );
                    if socket.write_all(response.as_bytes()).await.is_err() {
                        return;
                    }
                }
            });
        }
    });

    (format!("http://{addr}"), stats)
}

fn parse(code: &str) -> wfl::parser::ast::Program {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    parser
        .parse()
        .unwrap_or_else(|e| panic!("Parse error: {e:?}"))
}

async fn run_wfl(code: &str) -> Interpreter {
    let program = parse(code);
    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&program)
        .await
        .unwrap_or_else(|e| panic!("Runtime error: {e:?}"));
    interpreter
}

fn get_text(interpreter: &Interpreter, name: &str) -> String {
    match interpreter.global_env().borrow().get(name) {
        Some(Value::Text(t)) => t.to_string(),
        other => panic!("Expected '{name}' to be text, got {other:?}"),
    }
}

/// The buffered path: `open url ... and read response`.
///
/// Two POSTs through one interpreter (hence one pooled client). The second one
/// lands on the connection the first one left in the pool, and the peer closes
/// it instead of answering — the program must still get its reply.
#[tokio::test]
async fn buffered_post_survives_a_peer_closed_keepalive_socket() {
    let (url, stats) = spawn_upstream(DropPolicy::Requests(vec![2])).await;

    let code = format!(
        r#"
        open url at "{url}" with method "POST" and body "first" and read response as resp1
        store body1 as resp1["body"]
        open url at "{url}" with method "POST" and body "second" and read response as resp2
        store body2 as resp2["body"]
        store status2 as resp2["status"]
        "#
    );

    let interpreter = run_wfl(&code).await;

    assert_eq!(get_text(&interpreter, "body1"), "reply1\n");
    // Request 2 was dropped by the peer; the retry is request 3.
    assert_eq!(get_text(&interpreter, "body2"), "reply3\n");
    match interpreter.global_env().borrow().get("status2") {
        Some(Value::Number(n)) => assert_eq!(n, 200.0),
        other => panic!("Expected numeric status, got {other:?}"),
    }

    assert_eq!(
        stats.requests.load(Ordering::SeqCst),
        3,
        "expected the dropped request to be re-sent exactly once"
    );
    assert!(
        stats.connections.load(Ordering::SeqCst) >= 2,
        "the retry must go out on a fresh connection"
    );
    assert!(
        stats.reused_connection_requests.load(Ordering::SeqCst) >= 1,
        "the second request should have reused the pooled keep-alive socket — \
         without reuse this test is not exercising the defect"
    );
}

/// The streaming path: `open url ... and stream response`. This is the one an
/// SSE/LLM proxy uses, and the one that produced the reported 500.
#[tokio::test]
async fn streaming_post_survives_a_peer_closed_keepalive_socket() {
    let (url, stats) = spawn_upstream(DropPolicy::Requests(vec![2])).await;

    let code = format!(
        r#"
        open url at "{url}" with method "POST" and body "warm" and read response as resp1
        store body1 as resp1["body"]
        open url at "{url}" with method "POST" and body "stream" and stream response as up
        wait for next line from up as line1
        store first_line as line1
        "#
    );

    let interpreter = run_wfl(&code).await;

    assert_eq!(get_text(&interpreter, "body1"), "reply1\n");
    assert_eq!(get_text(&interpreter, "first_line"), "reply3");

    assert_eq!(
        stats.requests.load(Ordering::SeqCst),
        3,
        "expected the dropped stream request to be re-sent exactly once"
    );
    assert!(
        stats.reused_connection_requests.load(Ordering::SeqCst) >= 1,
        "the streaming request should have reused the pooled keep-alive socket"
    );
}

/// The retry is bounded: an upstream that closes *every* request must produce a
/// runtime error after exactly one re-send, not an unbounded replay loop.
#[tokio::test]
async fn a_permanently_closing_upstream_is_retried_once_and_then_fails() {
    let (url, stats) = spawn_upstream(DropPolicy::All).await;

    let code = format!(
        r#"
        open url at "{url}" with method "POST" and body "doomed" and read response as resp
        "#
    );

    let program = parse(&code);
    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&program).await;

    let errors = result.expect_err("a peer that never answers must surface an error");
    let message = errors
        .iter()
        .map(|e| e.message.clone())
        .collect::<Vec<_>>()
        .join("; ");
    assert!(
        message.contains("Failed to send HTTP POST request"),
        "unexpected error: {message}"
    );
    assert_eq!(
        stats.requests.load(Ordering::SeqCst),
        2,
        "the send should be attempted exactly twice (initial + one retry)"
    );
}

/// A response that *did* arrive is never re-sent: a 500 from the upstream is
/// the program's to see, not something to replay behind its back.
#[tokio::test]
async fn an_answered_request_is_never_re_sent() {
    let (url, stats) = spawn_upstream(DropPolicy::Requests(vec![])).await;

    let code = format!(
        r#"
        open url at "{url}" with method "POST" and body "once" and read response as resp
        store body1 as resp["body"]
        "#
    );

    let interpreter = run_wfl(&code).await;
    assert_eq!(get_text(&interpreter, "body1"), "reply1\n");
    assert_eq!(
        stats.requests.load(Ordering::SeqCst),
        1,
        "a request that got a response head must be sent exactly once"
    );
}
