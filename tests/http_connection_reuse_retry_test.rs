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
use std::time::Duration;

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

/// How the upstream misbehaves.
#[derive(Clone)]
enum DropPolicy {
    /// Close the socket, with no response, on these request numbers (1-based,
    /// counted across every connection).
    Requests(Vec<usize>),
    /// Close the socket on every request.
    All,
    /// Answer everything, but close a kept-alive connection once it has been
    /// idle this long — what Node's `keepAliveTimeout` does at 5 seconds.
    IdleAfter(Duration),
}

impl DropPolicy {
    fn drops(&self, request_number: usize) -> bool {
        match self {
            DropPolicy::Requests(numbers) => numbers.contains(&request_number),
            DropPolicy::All => true,
            DropPolicy::IdleAfter(_) => false,
        }
    }

    fn idle_timeout(&self) -> Option<Duration> {
        match self {
            DropPolicy::IdleAfter(duration) => Some(*duration),
            _ => None,
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
                let idle_timeout = policy.idle_timeout();
                let mut buf: Vec<u8> = Vec::new();
                let mut requests_on_this_connection = 0usize;

                loop {
                    // Read one complete request (head + Content-Length body).
                    let mut tmp = [0u8; 1024];
                    let header_end = loop {
                        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                            break Some(pos);
                        }
                        // Between requests, an idle connection may time out and
                        // be closed — the client is not told.
                        let read = match idle_timeout {
                            Some(limit) if buf.is_empty() => {
                                match tokio::time::timeout(limit, socket.read(&mut tmp)).await {
                                    Ok(result) => result,
                                    Err(_elapsed) => break None,
                                }
                            }
                            _ => socket.read(&mut tmp).await,
                        };
                        match read {
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

/// The shape the defect was reported in: a peer whose keep-alive window is
/// shorter than the gap the program leaves between two calls (Node's 5s default
/// versus a chat turn spent talking to somebody else).
///
/// Unlike the tests above this one is time-based, so it does not pin *which*
/// recovery ran — the pool may drop the expired socket before the second call,
/// or hand it over and let the re-send recover. It pins the outcome that
/// matters either way: an idle gap past the peer's keep-alive window still
/// yields a reply rather than a spurious send failure. Both calls run through
/// one interpreter, so they share one connection pool.
#[tokio::test]
async fn an_idle_gap_past_the_peers_keepalive_still_gets_a_reply() {
    let (url, stats) = spawn_upstream(DropPolicy::IdleAfter(Duration::from_millis(300))).await;

    let first = parse(&format!(
        r#"open url at "{url}" with method "POST" and body "first" and read response as resp
           store body1 as resp["body"]"#
    ));
    let second = parse(&format!(
        r#"open url at "{url}" with method "POST" and body "second" and read response as later
           store body2 as later["body"]"#
    ));

    let mut interpreter = Interpreter::new();
    interpreter.interpret(&first).await.expect("first call");
    assert_eq!(get_text(&interpreter, "body1"), "reply1\n");

    // Long enough that the peer has certainly closed the socket it was keeping
    // alive for us.
    tokio::time::sleep(Duration::from_millis(900)).await;

    interpreter
        .interpret(&second)
        .await
        .unwrap_or_else(|e| panic!("call after the peer's keep-alive expired: {e:?}"));
    assert_eq!(get_text(&interpreter, "body2"), "reply2\n");
    assert!(
        stats.connections.load(Ordering::SeqCst) >= 2,
        "the second call must end up on a live connection"
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
