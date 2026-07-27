// Regression tests for outbound response streaming against a *paced* upstream:
// one that trickles chunks onto a kept-alive connection over time, the way a
// real model endpoint streams SSE/NDJSON. The existing http_stream_test.rs
// upstream writes its whole body in one shot with Content-Length and
// `Connection: close`, so bytes are already buffered by the time the program
// reads; these tests cover the complementary shape — data that arrives while
// `wait for next line` is already parked — which is the entire point of
// `stream response` (see Docs/04-advanced-features/interoperability.md,
// "an upstream that emits output progressively").
//
// Risk class R3 (streaming/lifecycle, testing.md §11.3): proves ordering and
// wakeups for parked reads, clean EOF via the chunked terminator (NOT via
// connection close — the server holds the socket open afterwards, as a
// keep-alive upstream does), and bounded wall-clock completion.

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

/// One chunked-encoding frame for `payload` + `\n`.
fn chunk_frame(payload: &str) -> String {
    let line = format!("{payload}\n");
    format!("{:x}\r\n{}\r\n", line.len(), line)
}

const CHUNKED_HEAD: &[u8] =
    b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\n\r\n";

async fn run_wfl(code: &str) -> Interpreter {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser
        .parse()
        .unwrap_or_else(|e| panic!("Parse error: {e:?}"));
    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&program)
        .await
        .unwrap_or_else(|e| panic!("Runtime error: {e:?}"));
    interpreter
}

fn get_var(interpreter: &Interpreter, name: &str) -> Value {
    interpreter
        .global_env()
        .borrow()
        .get(name)
        .unwrap_or_else(|| panic!("Variable '{name}' not found"))
}

fn get_number(interpreter: &Interpreter, name: &str) -> f64 {
    match get_var(interpreter, name) {
        Value::Number(n) => n,
        other => panic!("Expected '{name}' to be a number, got {other:?}"),
    }
}

fn get_text(interpreter: &Interpreter, name: &str) -> String {
    match get_var(interpreter, name) {
        Value::Text(t) => t.to_string(),
        other => panic!("Expected '{name}' to be text, got {other:?}"),
    }
}

/// A chunk that arrives strictly AFTER the program has consumed everything
/// sent so far must wake the parked read and be delivered.
///
/// Synchronization (rather than a bare wall-clock delay racing interpreter
/// startup): the upstream sends a `ready` line immediately, and holds the
/// late chunk until the program — having consumed `ready` — signals progress
/// by hitting the server's second endpoint (`/go`). Only then, after a fixed
/// pause, is the late chunk written. The program also timestamps the parked
/// read on both sides, and the test asserts it genuinely waited: if the late
/// chunk had been buffered before the read began (broken-wakeup false pass),
/// the read would return instantly and the parked-duration assertion would
/// fail the test rather than let it pass vacuously.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_next_line_wakes_for_chunk_arriving_while_parked() {
    const LATE_DELAY_MS: u64 = 300;
    // Generous slack under LATE_DELAY_MS: proves the read parked, without
    // flaking on scheduling jitter.
    const MIN_PARKED_MS: f64 = 150.0;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        // Connection A: the stream. Head + `ready` line immediately.
        let (mut stream_sock, _) = listener.accept().await.unwrap();
        let mut tmp = [0u8; 4096];
        let _ = stream_sock.read(&mut tmp).await;
        stream_sock.write_all(CHUNKED_HEAD).await.unwrap();
        stream_sock
            .write_all(chunk_frame("ready").as_bytes())
            .await
            .unwrap();
        stream_sock.flush().await.unwrap();

        // Connection B: `/go` — the program signals it has consumed `ready`
        // and is about to park in the next read.
        let (mut go_sock, _) = listener.accept().await.unwrap();
        let _ = go_sock.read(&mut tmp).await;
        go_sock
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            .await
            .unwrap();
        go_sock.shutdown().await.ok();

        // Now — and only now — pause, then deliver the late chunk and the
        // chunked terminator. The socket stays open (keep-alive): EOF must
        // come from the zero-length chunk alone.
        tokio::time::sleep(Duration::from_millis(LATE_DELAY_MS)).await;
        stream_sock
            .write_all(chunk_frame("late-arrival").as_bytes())
            .await
            .unwrap();
        stream_sock.write_all(b"0\r\n\r\n").await.unwrap();
        stream_sock.flush().await.unwrap();
        tokio::time::sleep(Duration::from_secs(60)).await;
        drop(stream_sock);
    });
    let url = format!("http://{addr}");

    let code = format!(
        r#"
open url at "{url}/stream" and stream response as up
wait for next line from up as ready_line
store got_ready as "" with ready_line
open url at "{url}/go" and read content as go_ack
store parked_at as current time in milliseconds
wait for next line from up as late_line
store woke_at as current time in milliseconds
store got_late as "" with late_line
store parked_ms as woke_at minus parked_at
wait for next line from up as eof_line
check if eof_line is nothing:
    store eof_ok as "yes"
otherwise:
    store eof_ok as "no"
end check
close up
"#
    );

    let interpreter = tokio::time::timeout(Duration::from_secs(10), run_wfl(&code))
        .await
        .expect("stream read stalled: chunk arriving while parked was never delivered");
    assert_eq!(get_text(&interpreter, "got_ready"), "ready");
    assert_eq!(get_text(&interpreter, "go_ack"), "ok");
    assert_eq!(get_text(&interpreter, "got_late"), "late-arrival");
    assert_eq!(get_text(&interpreter, "eof_ok"), "yes");
    let parked_ms = get_number(&interpreter, "parked_ms");
    assert!(
        parked_ms >= MIN_PARKED_MS,
        "read returned after only {parked_ms}ms — the late chunk was already \
         buffered, so this run never exercised a parked-read wakeup"
    );
}

/// Many paced chunks (the real SSE shape: ~30ms cadence on one kept-alive
/// connection) must ALL be delivered — the exact sequence, in order — ending
/// in a clean EOF from the chunked terminator.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_next_line_delivers_every_paced_chunk_then_eof() {
    let lines: Vec<&'static str> = vec![
        "data: one",
        "data: two",
        "data: three",
        "data: four",
        "data: five",
        "data: six",
        "data: seven",
        "data: eight",
        "data: nine",
        "data: ten",
    ];
    let expected_count = lines.len() as f64;
    let expected_sequence = lines
        .iter()
        .map(|l| format!("{l}|"))
        .collect::<Vec<_>>()
        .join("");

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut tmp = [0u8; 4096];
        let _ = socket.read(&mut tmp).await;
        socket.write_all(CHUNKED_HEAD).await.unwrap();
        socket.flush().await.unwrap();
        for line in lines {
            tokio::time::sleep(Duration::from_millis(30)).await;
            socket
                .write_all(chunk_frame(line).as_bytes())
                .await
                .unwrap();
            socket.flush().await.unwrap();
        }
        socket.write_all(b"0\r\n\r\n").await.unwrap();
        socket.flush().await.unwrap();
        // Keep-alive: hold the socket open long past the test window.
        tokio::time::sleep(Duration::from_secs(60)).await;
        drop(socket);
    });
    let url = format!("http://{addr}");

    let code = format!(
        r#"
open url at "{url}" and stream response as up
store line_count as 0
store all_lines as ""
count from 1 to 1000:
    wait for next line from up as l
    check if l is nothing:
        break
    otherwise:
        add 1 to line_count
        change all_lines to all_lines with l with "|"
    end check
end count
close up
"#
    );

    let interpreter = tokio::time::timeout(Duration::from_secs(15), run_wfl(&code))
        .await
        .expect("stream read stalled mid-body: paced chunks were never delivered");
    assert_eq!(get_number(&interpreter, "line_count"), expected_count);
    assert_eq!(get_text(&interpreter, "all_lines"), expected_sequence);
}
