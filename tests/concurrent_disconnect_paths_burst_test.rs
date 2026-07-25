//! Real-socket regression (maintainer re-review, P1 / #642): EVERY transport-confirmed
//! client disconnect must be classified as a cancellation, not a handler failure —
//! including the buffered `respond` send, the streaming-response head path (before the
//! head is sent), and the streaming write path — not only a cancelled upstream chunk
//! read.
//!
//! The concurrent loop breaks after `MAX_CONSECUTIVE_FAILURES` (256) consecutive
//! *structural* failures. Request-local outcomes (disconnects, wait timeouts, errors
//! after a request was accepted) must never feed that breaker. These bursts drive
//! more than 256 disconnects of each kind with no successful request in between; an
//! unrelated `/ping` must still be served afterward.
//!
//! Also: every client that is intended to exercise a path must actually connect and
//! reach that lifecycle point (no silent early-return that leaves the burst under the
//! breaker threshold), and an explicit handler-start barrier proves every intended
//! result was consumed before probing `/ping` (so a General-classified disconnect
//! cannot race past a premature success that resets the counter).

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, watch};
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

mod common;

/// Exactly fill the concurrent-handler cap. A second full wave cannot reach its
/// lifecycle checkpoint until every result from the first wave has been consumed.
const DISCONNECT_WAVE: usize = 256;
/// Both waves disconnect before `/ping`, so each path exercises 512 clients (>256).
const DISCONNECT_TOTAL: usize = DISCONNECT_WAVE * 2;
const _: () = assert!(DISCONNECT_TOTAL > 256);
/// The initial 256 handlers plus one replacement handler per consumed disconnect
/// across both waves. Observing this ordinal proves every one of the 512 intended
/// disconnect outcomes left `FuturesUnordered` before `/ping` is allowed to run.
const POST_DISCONNECT_BARRIER: usize = DISCONNECT_WAVE + DISCONNECT_TOTAL;
const WAVE_DEADLINE: Duration = Duration::from_secs(30);
/// The WFL handler waits this long after its checkpoint is released. That gives the
/// test time to close every downstream socket before `respond` / response-head send.
const POST_CHECKPOINT_DELAY_MS: u64 = 1_000;
const ITERATION_PROOF_DEADLINE: Duration = Duration::from_secs(20);

/// These cases deliberately fill the 256-handler cap. Rust's test harness otherwise
/// runs all four cases in this binary in parallel, multiplying the live socket/task
/// peak. Serializing the heavyweight cases keeps the test-host resource bound at one
/// full handler wave while preserving the real >256-request breaker proof.
static HEAVY_CASE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct CountingGate {
    port: u16,
    arrivals: mpsc::UnboundedReceiver<usize>,
    acknowledgements: mpsc::UnboundedReceiver<usize>,
    errors: mpsc::UnboundedReceiver<String>,
    release_wave: watch::Sender<usize>,
}

/// An HTTP checkpoint shared by the two request waves. Each handler opens the
/// checkpoint before its response operation. The mock reports all arrivals, then
/// withholds the HTTP head until the test releases that wave. This proves all 256
/// handlers reached the intended path without relying on sleeps or successful TCP
/// writes alone.
async fn spawn_counting_gate() -> CountingGate {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind counting gate");
    let port = listener.local_addr().expect("counting gate address").port();
    let next_checkpoint_ordinal = Arc::new(AtomicUsize::new(0));
    let next_ack_ordinal = Arc::new(AtomicUsize::new(0));
    let (arrival_tx, arrivals) = mpsc::unbounded_channel();
    let (ack_tx, acknowledgements) = mpsc::unbounded_channel();
    let (error_tx, errors) = mpsc::unbounded_channel();
    let (release_wave, release_guard) = watch::channel(0usize);

    tokio::spawn(async move {
        // Keep one receiver alive between waves so `release_wave.send()` cannot
        // fail in the brief interval after the prior wave's connections close.
        let release_guard = release_guard;
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            let next_checkpoint_ordinal = Arc::clone(&next_checkpoint_ordinal);
            let next_ack_ordinal = Arc::clone(&next_ack_ordinal);
            let arrival_tx = arrival_tx.clone();
            let ack_tx = ack_tx.clone();
            let error_tx = error_tx.clone();
            let mut release_rx = release_guard.clone();
            tokio::spawn(async move {
                let result = async {
                    let head = read_http_head(&mut sock).await?;
                    let request = String::from_utf8_lossy(&head);
                    let path = request
                        .lines()
                        .next()
                        .and_then(|line| line.split_whitespace().nth(1))
                        .ok_or_else(|| {
                            "counting gate received a malformed request line".to_string()
                        })?;
                    match path {
                        "/checkpoint" => {
                            let ordinal =
                                next_checkpoint_ordinal.fetch_add(1, Ordering::Relaxed) + 1;
                            let wave = ((ordinal - 1) / DISCONNECT_WAVE) + 1;
                            arrival_tx
                                .send(ordinal)
                                .map_err(|_| "checkpoint arrival receiver dropped".to_string())?;
                            release_rx
                                .wait_for(|released_wave| *released_wave >= wave)
                                .await
                                .map_err(|_| "counting-gate release sender dropped".to_string())?;
                            sock.write_all(
                                b"HTTP/1.1 200 OK\r\n\
                                  Content-Length: 0\r\n\
                                  Connection: close\r\n\r\n",
                            )
                            .await
                            .map_err(|error| format!("write checkpoint response: {error}"))?;
                            sock.flush()
                                .await
                                .map_err(|error| format!("flush checkpoint response: {error}"))
                        }
                        "/ack" => {
                            let ordinal = next_ack_ordinal.fetch_add(1, Ordering::Relaxed) + 1;
                            // Deliberately leave this chunked response unfinished. The
                            // handler reads the marker and explicitly closes its
                            // outbound stream; observing EOF below is an exact
                            // handler-side acknowledgement that the earlier
                            // `/checkpoint` open returned and the local post-checkpoint
                            // wait is now the next operation.
                            sock.write_all(
                                b"HTTP/1.1 200 OK\r\n\
                                  Content-Type: text/plain\r\n\
                                  Transfer-Encoding: chunked\r\n\
                                  Connection: keep-alive\r\n\r\n\
                                  1\r\nA\r\n",
                            )
                            .await
                            .map_err(|error| format!("write acknowledgement marker: {error}"))?;
                            sock.flush().await.map_err(|error| {
                                format!("flush acknowledgement marker: {error}")
                            })?;
                            let deadline = tokio::time::Instant::now() + WAVE_DEADLINE;
                            let mut byte = [0u8; 1];
                            loop {
                                let read = tokio::time::timeout_at(deadline, sock.read(&mut byte))
                                    .await
                                    .map_err(|_| {
                                        format!(
                                            "handler did not close acknowledgement stream {ordinal}"
                                        )
                                    })?
                                    .map_err(|error| {
                                        format!("read acknowledgement close {ordinal}: {error}")
                                    })?;
                                if read == 0 {
                                    break;
                                }
                            }
                            ack_tx
                                .send(ordinal)
                                .map_err(|_| "handler acknowledgement receiver dropped".to_string())
                        }
                        other => Err(format!("unexpected counting-gate path {other:?}")),
                    }
                }
                .await;
                if let Err(error) = result {
                    let _ = error_tx.send(error);
                }
            });
        }
    });

    CountingGate {
        port,
        arrivals,
        acknowledgements,
        errors,
        release_wave,
    }
}

async fn read_http_head(sock: &mut tokio::net::TcpStream) -> Result<Vec<u8>, String> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut head = Vec::new();
    let mut buf = [0u8; 512];
    loop {
        let n = tokio::time::timeout_at(deadline, sock.read(&mut buf))
            .await
            .map_err(|_| "timed out waiting for HTTP head".to_string())?
            .map_err(|error| format!("read HTTP head: {error}"))?;
        if n == 0 {
            return Err("connection closed before the complete HTTP head".to_string());
        }
        head.extend_from_slice(&buf[..n]);
        if head.windows(4).any(|window| window == b"\r\n\r\n") {
            return Ok(head);
        }
        if head.len() > 16 * 1024 {
            return Err("HTTP head exceeded 16 KiB".to_string());
        }
    }
}

struct IterationCounter {
    port: u16,
    arrivals: mpsc::UnboundedReceiver<usize>,
    errors: mpsc::UnboundedReceiver<String>,
}

/// Count a handler-start request and return an empty response immediately. A
/// concurrent handler calls this before waiting for a request. Since the handler
/// then remains parked in `wait for request`, each later start is observable proof
/// that the outer loop consumed one prior handler result and refilled its slot.
async fn spawn_iteration_counter() -> IterationCounter {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind iteration counter");
    let port = listener
        .local_addr()
        .expect("iteration counter address")
        .port();
    let ordinal = Arc::new(AtomicUsize::new(0));
    let (arrival_tx, arrivals) = mpsc::unbounded_channel();
    let (error_tx, errors) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            let ordinal = Arc::clone(&ordinal);
            let arrival_tx = arrival_tx.clone();
            let error_tx = error_tx.clone();
            tokio::spawn(async move {
                let result = async {
                    let head = read_http_head(&mut sock).await?;
                    let request = String::from_utf8_lossy(&head);
                    if !request.starts_with("GET /tick ") {
                        return Err(format!(
                            "unexpected iteration-counter request: {:?}",
                            request.lines().next()
                        ));
                    }
                    let current = ordinal.fetch_add(1, Ordering::Relaxed) + 1;
                    arrival_tx
                        .send(current)
                        .map_err(|_| "iteration arrival receiver dropped".to_string())?;
                    sock.write_all(
                        b"HTTP/1.1 200 OK\r\n\
                          Content-Length: 0\r\n\
                          Connection: close\r\n\r\n",
                    )
                    .await
                    .map_err(|error| format!("write iteration response: {error}"))?;
                    sock.flush()
                        .await
                        .map_err(|error| format!("flush iteration response: {error}"))
                }
                .await;
                if let Err(error) = result {
                    let _ = error_tx.send(error);
                }
            });
        }
    });

    IterationCounter {
        port,
        arrivals,
        errors,
    }
}

async fn wait_for_proven_handler_iterations(
    counter: &mut IterationCounter,
    expected: usize,
    context: &str,
) {
    let deadline = tokio::time::Instant::now() + ITERATION_PROOF_DEADLINE;
    let mut seen = HashSet::with_capacity(expected);
    while seen.len() < expected {
        let ordinal = tokio::time::timeout_at(deadline, async {
            tokio::select! {
                ordinal = counter.arrivals.recv() => {
                    ordinal.expect("iteration arrival channel closed")
                }
                error = counter.errors.recv() => {
                    panic!(
                        "iteration counter failed before proving {expected} {context} \
                         handler starts: {}",
                        error.expect("iteration counter error channel closed")
                    )
                }
            }
        })
        .await
        .unwrap_or_else(|_| {
            panic!(
                "observed only {} of {expected} {context} handler starts; the \
                 {expected}th start is required to prove the loop consumed every \
                 intended result before the liveness probe",
                seen.len()
            )
        });
        assert!(
            seen.insert(ordinal),
            "iteration counter duplicated handler-start ordinal {ordinal}"
        );
    }
}

async fn wait_for_gate_arrivals(
    arrivals: &mut mpsc::UnboundedReceiver<usize>,
    errors: &mut mpsc::UnboundedReceiver<String>,
    wave: usize,
    context: &str,
) {
    let first = (wave - 1) * DISCONNECT_WAVE + 1;
    let last = wave * DISCONNECT_WAVE;
    let deadline = tokio::time::Instant::now() + WAVE_DEADLINE;
    let mut seen = HashSet::with_capacity(DISCONNECT_WAVE);
    while seen.len() < DISCONNECT_WAVE {
        let ordinal = tokio::time::timeout_at(deadline, async {
            tokio::select! {
                ordinal = arrivals.recv() => {
                    ordinal.expect("counting-gate arrival channel closed")
                }
                error = errors.recv() => {
                    panic!(
                        "{context} counting gate failed while waiting for wave {wave}: {}",
                        error.expect("counting-gate error channel closed")
                    )
                }
            }
        })
        .await
        .unwrap_or_else(|_| {
            panic!(
                "only {} of {DISCONNECT_WAVE} {context} handlers reached the \
                 counting checkpoint in wave {wave}",
                seen.len()
            )
        });
        assert!(
            (first..=last).contains(&ordinal),
            "unexpected counting-gate ordinal {ordinal} while waiting for wave {wave} \
             ({first}..={last})"
        );
        assert!(
            seen.insert(ordinal),
            "duplicate counting-gate arrival ordinal {ordinal}"
        );
    }
}

async fn wait_for_handler_acknowledgements(
    acknowledgements: &mut mpsc::UnboundedReceiver<usize>,
    errors: &mut mpsc::UnboundedReceiver<String>,
    wave: usize,
    context: &str,
) {
    let first = (wave - 1) * DISCONNECT_WAVE + 1;
    let last = wave * DISCONNECT_WAVE;
    let deadline = tokio::time::Instant::now() + WAVE_DEADLINE;
    let mut seen = HashSet::with_capacity(DISCONNECT_WAVE);
    while seen.len() < DISCONNECT_WAVE {
        let ordinal = tokio::time::timeout_at(deadline, async {
            tokio::select! {
                ordinal = acknowledgements.recv() => {
                    ordinal.expect("handler acknowledgement channel closed")
                }
                error = errors.recv() => {
                    panic!(
                        "{context} counting gate failed while waiting for acknowledgements \
                         in wave {wave}: {}",
                        error.expect("counting-gate error channel closed")
                    )
                }
            }
        })
        .await
        .unwrap_or_else(|_| {
            panic!(
                "only {} of {DISCONNECT_WAVE} {context} handlers acknowledged the \
                 completed checkpoint in wave {wave}",
                seen.len()
            )
        });
        assert!(
            (first..=last).contains(&ordinal),
            "unexpected handler acknowledgement ordinal {ordinal} while waiting for wave {wave}"
        );
        assert!(
            seen.insert(ordinal),
            "duplicate handler acknowledgement ordinal {ordinal}"
        );
    }
}

struct ProxyServer {
    thread: std::thread::JoinHandle<()>,
    abort: Option<tokio::sync::oneshot::Sender<()>>,
}

fn start_proxy_server(code: String) -> ProxyServer {
    let (abort, abort_rx) = tokio::sync::oneshot::channel();
    let thread = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let ast = Parser::new(&tokens).parse().expect("parse");
            let mut interp = Interpreter::new();
            tokio::select! {
                result = interp.interpret(&ast) => {
                    if let Err(errors) = result {
                        panic!("server interpreter failed: {errors:?}");
                    }
                }
                _ = abort_rx => {
                    // Test cleanup: dropping the interpret future closes listeners,
                    // pending responses, and response streams.
                }
            }
        });
    });
    ProxyServer {
        thread,
        abort: Some(abort),
    }
}

async fn wait_for_server(port: u16) {
    let addr = format!("127.0.0.1:{port}");
    for _ in 0..300 {
        if tokio::net::TcpStream::connect(&addr).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("server on {addr} did not become ready");
}

async fn hold_client_until_disconnect(
    port: u16,
    path: &'static str,
    mut disconnect: watch::Receiver<bool>,
) -> Result<(), String> {
    let mut sock = tokio::time::timeout(
        Duration::from_secs(5),
        tokio::net::TcpStream::connect(("127.0.0.1", port)),
    )
    .await
    .map_err(|_| format!("timed out connecting to {path}"))?
    .map_err(|error| format!("connect {path}: {error}"))?;
    let req = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
    sock.write_all(req.as_bytes())
        .await
        .map_err(|error| format!("send {path} request: {error}"))?;
    sock.flush()
        .await
        .map_err(|error| format!("flush {path} request: {error}"))?;
    disconnect
        .wait_for(|should_disconnect| *should_disconnect)
        .await
        .map_err(|_| format!("{path} disconnect signal sender dropped"))?;
    drop(sock);
    Ok(())
}

async fn disconnect_after_stream_head(port: u16, path: &'static str) -> Result<(), String> {
    let mut sock = tokio::time::timeout(
        Duration::from_secs(5),
        tokio::net::TcpStream::connect(("127.0.0.1", port)),
    )
    .await
    .map_err(|_| format!("timed out connecting to {path}"))?
    .map_err(|error| format!("connect {path}: {error}"))?;
    let req = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
    sock.write_all(req.as_bytes())
        .await
        .map_err(|error| format!("send {path} request: {error}"))?;
    sock.flush()
        .await
        .map_err(|error| format!("flush {path} request: {error}"))?;
    let head = read_http_head(&mut sock).await?;
    let head = String::from_utf8_lossy(&head);
    assert!(
        head.starts_with("HTTP/1.1 200"),
        "{path} must reach a successful streaming response head before disconnect; \
         got {head:?}"
    );
    drop(sock);
    Ok(())
}

fn spawn_gated_client_wave(
    port: u16,
    path: &'static str,
) -> (
    watch::Sender<bool>,
    Vec<tokio::task::JoinHandle<Result<(), String>>>,
) {
    let (disconnect, disconnect_rx) = watch::channel(false);
    let mut tasks = Vec::with_capacity(DISCONNECT_WAVE);
    for _ in 0..DISCONNECT_WAVE {
        let disconnect_rx = disconnect_rx.clone();
        tasks.push(tokio::spawn(async move {
            hold_client_until_disconnect(port, path, disconnect_rx).await
        }));
    }
    (disconnect, tasks)
}

fn spawn_stream_client_wave(
    port: u16,
    path: &'static str,
) -> Vec<tokio::task::JoinHandle<Result<(), String>>> {
    (0..DISCONNECT_WAVE)
        .map(|_| tokio::spawn(disconnect_after_stream_head(port, path)))
        .collect()
}

async fn join_client_wave(
    tasks: Vec<tokio::task::JoinHandle<Result<(), String>>>,
    wave: usize,
    context: &str,
) {
    assert_eq!(
        tasks.len(),
        DISCONNECT_WAVE,
        "each {context} wave must contain exactly {DISCONNECT_WAVE} clients"
    );
    tokio::time::timeout(WAVE_DEADLINE, async move {
        for (index, task) in tasks.into_iter().enumerate() {
            let result = task
                .await
                .unwrap_or_else(|error| panic!("{context} client task {index} panicked: {error}"));
            result.unwrap_or_else(|error| {
                panic!("{context} client task {index} failed in wave {wave}: {error}")
            });
        }
    })
    .await
    .unwrap_or_else(|_| panic!("{context} client joins timed out in wave {wave}"));
}

/// Drive two full checkpointed waves. All 256 first-wave handlers are held inside
/// the checkpoint simultaneously. The second wave cannot put all 256 handlers into
/// that checkpoint until the loop has consumed every first-wave result. After this
/// returns, the separate handler-start barrier proves the second-wave results were
/// consumed too, before `/ping` is sent.
async fn drive_two_gated_disconnect_waves(
    port: u16,
    path: &'static str,
    gate: &mut CountingGate,
    context: &str,
) {
    for wave in 1..=2 {
        let (disconnect, clients) = spawn_gated_client_wave(port, path);
        wait_for_gate_arrivals(&mut gate.arrivals, &mut gate.errors, wave, context).await;
        gate.release_wave
            .send(wave)
            .expect("counting-gate release receiver stays alive");
        wait_for_handler_acknowledgements(
            &mut gate.acknowledgements,
            &mut gate.errors,
            wave,
            context,
        )
        .await;
        disconnect
            .send(true)
            .expect("all gated clients remain alive until explicitly disconnected");
        join_client_wave(clients, wave, context).await;
    }
}

/// The streaming-head variant needs no auxiliary checkpoint: receiving a valid
/// response head is itself the lifecycle proof. As above, all 256 second-wave heads
/// can only arrive after every first-wave disconnect result was consumed.
async fn drive_two_stream_disconnect_waves(port: u16, path: &'static str, context: &str) {
    for wave in 1..=2 {
        let clients = spawn_stream_client_wave(port, path);
        join_client_wave(clients, wave, context).await;
    }
}

async fn assert_ping_survives(port: u16, context: &str) {
    let ping = tokio::time::timeout(
        Duration::from_secs(10),
        reqwest::Client::new()
            .get(format!("http://127.0.0.1:{port}/ping"))
            .send(),
    )
    .await
    .unwrap_or_else(|_| panic!("`/ping` timed out after the {context} burst — loop torn down"))
    .expect("`/ping` request failed");
    assert_eq!(
        ping.status().as_u16(),
        200,
        "`/ping` should be served after the {context} burst"
    );
    assert_eq!(ping.text().await.unwrap(), "pong");
}

async fn shutdown(port: u16, mut server: ProxyServer) {
    let _ = tokio::time::timeout(
        Duration::from_secs(10),
        reqwest::Client::new()
            .get(format!("http://127.0.0.1:{port}/shutdown"))
            .send(),
    )
    .await;
    let graceful = tokio::time::timeout(Duration::from_secs(10), async {
        while !server.thread.is_finished() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .is_ok();
    if !graceful {
        let _ = server
            .abort
            .take()
            .expect("proxy abort signal is sent at most once")
            .send(());
        tokio::time::timeout(Duration::from_secs(5), async {
            while !server.thread.is_finished() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("proxy server ignored both graceful shutdown and forced test cleanup");
    }
    match server.thread.join() {
        Ok(()) => {}
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

#[tokio::test]
async fn test_disconnect_before_buffered_respond_does_not_kill_the_loop() {
    let _heavy_case = HEAVY_CASE_LOCK.lock().await;
    let mut gate = spawn_counting_gate().await;
    let mut iterations = spawn_iteration_counter().await;
    let port = common::free_tcp_port();
    // `/slow` reaches the counting checkpoint, waits after its release, then
    // responds. The test disconnects every client during that wait, so the buffered
    // `respond` send fails (or the pending entry is sibling-pruned). That must be a
    // cancellation, not a structural failure.
    let code = format!(
        r#"
        listen on port {port} as srv
        main loop concurrently:
            open url at "http://127.0.0.1:{counter_port}/tick" and stream response as iteration
            close iteration
            wait for request comes in on srv as req with timeout 30000
            store p as req["path"]
            check if p is equal to "/ping":
                respond to req with "pong"
            otherwise:
                check if p is equal to "/shutdown":
                    respond to req with "bye"
                    close server srv
                    break
                otherwise:
                    open url at "http://127.0.0.1:{gate_port}/checkpoint" and stream response as checkpoint
                    close checkpoint
                    open url at "http://127.0.0.1:{gate_port}/ack" and stream response as acknowledgement
                    wait for next chunk from acknowledgement as acknowledged
                    close acknowledgement
                    wait for {POST_CHECKPOINT_DELAY_MS} milliseconds
                    respond to req with "late"
                end check
            end check
        end loop
    "#,
        gate_port = gate.port,
        counter_port = iterations.port,
    );
    let server = start_proxy_server(code);
    wait_for_server(port).await;
    drive_two_gated_disconnect_waves(port, "/slow", &mut gate, "buffered-respond disconnect").await;
    wait_for_proven_handler_iterations(
        &mut iterations,
        POST_DISCONNECT_BARRIER,
        "buffered-disconnect",
    )
    .await;
    assert_ping_survives(port, "buffered-respond disconnect").await;
    shutdown(port, server).await;
}

#[tokio::test]
async fn test_disconnect_before_stream_write_does_not_kill_the_loop() {
    let _heavy_case = HEAVY_CASE_LOCK.lock().await;
    let mut iterations = spawn_iteration_counter().await;
    let port = common::free_tcp_port();
    // `/stream` sends the head, waits (the client reads the head then disconnects),
    // then writes — the write send fails. That must be a cancellation, not a failure.
    let code = format!(
        r#"
        listen on port {port} as srv
        main loop concurrently:
            open url at "http://127.0.0.1:{counter_port}/tick" and stream response as iteration
            close iteration
            wait for request comes in on srv as req with timeout 30000
            store p as req["path"]
            check if p is equal to "/ping":
                respond to req with "pong"
            otherwise:
                check if p is equal to "/shutdown":
                    respond to req with "bye"
                    close server srv
                    break
                otherwise:
                    start streaming response to req with status 200 and content type "text/plain" as out
                    wait for 300 milliseconds
                    store payload as "0123456789"
                    count from 1 to 9:
                        store payload as payload with payload
                    end count
                    count from 1 to 400:
                        write chunk payload to out
                    end count
                    close out
                end check
            end check
        end loop
    "#,
        counter_port = iterations.port,
    );
    let server = start_proxy_server(code);
    wait_for_server(port).await;
    drive_two_stream_disconnect_waves(port, "/stream", "stream-write disconnect").await;
    wait_for_proven_handler_iterations(
        &mut iterations,
        POST_DISCONNECT_BARRIER,
        "stream-write-disconnect",
    )
    .await;
    assert_ping_survives(port, "stream-write disconnect").await;
    shutdown(port, server).await;
}

#[tokio::test]
async fn test_disconnect_before_streaming_head_does_not_kill_the_loop() {
    let _heavy_case = HEAVY_CASE_LOCK.lock().await;
    let mut gate = spawn_counting_gate().await;
    let mut iterations = spawn_iteration_counter().await;
    let port = common::free_tcp_port();
    // Client disconnects *before* the streaming head is sent (no head read). The
    // handler reaches the counting checkpoint, parks after its release, then reaches
    // `start streaming response` with a missing/closed pending entry — must be
    // Cancelled, not a structural General that trips the breaker after >256 instances.
    let code = format!(
        r#"
        listen on port {port} as srv
        main loop concurrently:
            open url at "http://127.0.0.1:{counter_port}/tick" and stream response as iteration
            close iteration
            wait for request comes in on srv as req with timeout 30000
            store p as req["path"]
            check if p is equal to "/ping":
                respond to req with "pong"
            otherwise:
                check if p is equal to "/shutdown":
                    respond to req with "bye"
                    close server srv
                    break
                otherwise:
                    open url at "http://127.0.0.1:{gate_port}/checkpoint" and stream response as checkpoint
                    close checkpoint
                    open url at "http://127.0.0.1:{gate_port}/ack" and stream response as acknowledgement
                    wait for next chunk from acknowledgement as acknowledged
                    close acknowledgement
                    wait for {POST_CHECKPOINT_DELAY_MS} milliseconds
                    start streaming response to req with status 200 and content type "text/plain" as out
                    write line "late" to out
                    close out
                end check
            end check
        end loop
    "#,
        gate_port = gate.port,
        counter_port = iterations.port,
    );
    let server = start_proxy_server(code);
    wait_for_server(port).await;
    drive_two_gated_disconnect_waves(port, "/prehead", &mut gate, "pre-streaming-head disconnect")
        .await;
    wait_for_proven_handler_iterations(
        &mut iterations,
        POST_DISCONNECT_BARRIER,
        "pre-streaming-head-disconnect",
    )
    .await;
    assert_ping_survives(port, "pre-streaming-head disconnect").await;
    shutdown(port, server).await;
}

#[tokio::test]
async fn test_repeated_wait_timeouts_do_not_kill_the_loop() {
    let _heavy_case = HEAVY_CASE_LOCK.lock().await;
    let mut counter = spawn_iteration_counter().await;
    let port = common::free_tcp_port();
    // Finite `wait for request ... with timeout` that repeatedly expires with no
    // client traffic must not trip the structural breaker. After many idle
    // timeouts a real `/ping` must still be served.
    let code = format!(
        r#"
        listen on port {port} as srv
        main loop concurrently:
            open url at "http://127.0.0.1:{counter_port}/tick" and stream response as tick
            close tick
            wait for request comes in on srv as req with timeout 1
            store p as req["path"]
            check if p is equal to "/ping":
                respond to req with "pong"
            otherwise:
                check if p is equal to "/shutdown":
                    respond to req with "bye"
                    close server srv
                    break
                otherwise:
                    respond to req with "ok"
                end check
            end check
        end loop
    "#,
        counter_port = counter.port,
    );
    let server = start_proxy_server(code);
    wait_for_server(port).await;
    // The loop initially starts 256 handlers. It can start handler 512 only after
    // consuming 256 finite request-wait expiries and refilling once more. A buggy
    // structural classifier breaks while consuming expiry 256 and tops out at 511,
    // so this is an observable threshold proof rather than an elapsed-time proxy.
    wait_for_proven_handler_iterations(&mut counter, DISCONNECT_TOTAL, "finite-timeout").await;
    assert_ping_survives(port, "repeated wait timeouts").await;
    shutdown(port, server).await;
}
