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

/// Spawn an upstream that answers 200 with `Transfer-Encoding: chunked`, then
/// writes one chunk per line with `delay_ms` between chunks, sends the chunked
/// terminator, and KEEPS THE CONNECTION OPEN (no `Connection: close`, no
/// shutdown) so end-of-body is only observable from the chunked framing.
async fn spawn_paced_chunked_server(lines: Vec<&'static str>, delay_ms: u64) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut tmp = [0u8; 4096];
        // Drain the request head (single read is enough for a bodyless GET).
        let _ = socket.read(&mut tmp).await;
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\n\r\n",
            )
            .await
            .unwrap();
        socket.flush().await.unwrap();
        for line in lines {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            let payload = format!("{line}\n");
            let frame = format!("{:x}\r\n{}\r\n", payload.len(), payload);
            socket.write_all(frame.as_bytes()).await.unwrap();
            socket.flush().await.unwrap();
        }
        socket.write_all(b"0\r\n\r\n").await.unwrap();
        socket.flush().await.unwrap();
        // Keep-alive: hold the socket open long past the test window. The
        // client must reach EOF from the zero-length chunk alone.
        tokio::time::sleep(Duration::from_secs(60)).await;
        drop(socket);
    });
    format!("http://{addr}")
}

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

/// A single chunk that arrives ~200ms AFTER the program has parked in
/// `wait for next line` must wake the read and be delivered.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_next_line_wakes_for_chunk_arriving_while_parked() {
    let url = spawn_paced_chunked_server(vec!["late-arrival"], 200).await;
    let code = format!(
        r#"
open url at "{url}" and stream response as up
wait for next line from up as first_line
store got as "" with first_line
wait for next line from up as second_line
check if second_line is nothing:
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
    assert_eq!(get_text(&interpreter, "got"), "late-arrival");
    assert_eq!(get_text(&interpreter, "eof_ok"), "yes");
}

/// Many paced chunks (the real SSE shape: ~30ms cadence on one kept-alive
/// connection) must ALL be delivered, in order, ending in a clean EOF.
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
    let expected = lines.len() as f64;
    let url = spawn_paced_chunked_server(lines, 30).await;
    let code = format!(
        r#"
open url at "{url}" and stream response as up
store line_count as 0
store last_line as ""
count from 1 to 1000:
    wait for next line from up as l
    check if l is nothing:
        break
    otherwise:
        add 1 to line_count
        change last_line to l
    end check
end count
close up
"#
    );

    let interpreter = tokio::time::timeout(Duration::from_secs(15), run_wfl(&code))
        .await
        .expect("stream read stalled mid-body: paced chunks were never delivered");
    assert_eq!(get_number(&interpreter, "line_count"), expected);
    assert_eq!(get_text(&interpreter, "last_line"), "data: ten");
}
