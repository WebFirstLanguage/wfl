//! Real-socket regression for P1 (#4): a cancelled/dropped `interpret()` future
//! must still close the outbound stream handles the run opened.
//!
//! Handler-exit cleanup (concurrent `IsolatedHandler::drop`, serial-loop/program
//! cleanup sites) closes outbound handles on normal control-flow exits. But if the
//! whole `interpret()` future is DROPPED (an embedder cancels it) while a handle
//! sits idle in `IoClient.stream_handles` — opened, not currently inside a read —
//! none of those sites run, and (with the interpreter kept alive, e.g. a reused
//! REPL) the upstream request leaks until the interpreter itself is dropped. An
//! RAII guard tied to the run must drop those handles when the future unwinds.

use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

/// Upstream: send a chunked head immediately, then STALL. Signal when the proxy
/// drops the connection (a blocking read returns 0/Err at peer close).
async fn spawn_head_then_stall_upstream() -> (u16, tokio::sync::oneshot::Receiver<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock upstream");
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        if let Ok((mut sock, _)) = listener.accept().await {
            let mut buf = [0u8; 1024];
            let _ = sock.read(&mut buf).await; // request head
            let head = "HTTP/1.1 200 OK\r\n\
                        Content-Type: text/plain\r\n\
                        Transfer-Encoding: chunked\r\n\r\n";
            let _ = sock.write_all(head.as_bytes()).await;
            let _ = sock.flush().await;
            loop {
                match sock.read(&mut buf).await {
                    Ok(0) | Err(_) => {
                        let _ = tx.send(());
                        return;
                    }
                    Ok(_) => {}
                }
            }
        }
    });
    (port, rx)
}

#[tokio::test]
async fn test_dropped_interpret_future_closes_outbound_handle() {
    let (port, mut upstream_closed) = spawn_head_then_stall_upstream().await;

    // Open an outbound stream, then sit in a long wait WITHOUT reading it — the
    // handle is parked in `IoClient.stream_handles`, not held inside an in-flight
    // read (dropping a read future would itself drop the handle and mask the bug).
    let code = format!(
        r#"open url at "http://127.0.0.1:{port}/" and stream response as up
wait for 5000 milliseconds"#
    );
    let tokens = lex_wfl_with_positions(&code);
    let program = Parser::new(&tokens).parse().expect("parse");

    let mut interp = Interpreter::new();
    {
        let fut = interp.interpret(&program);
        tokio::pin!(fut);
        // Drive the run long enough to open the stream and enter the wait, then
        // let `fut` drop at the end of this scope (the interpret future is
        // cancelled). The interpreter itself stays alive below.
        let _ = tokio::time::timeout(Duration::from_millis(800), fut.as_mut()).await;
    }

    // The dropped future must have released the outbound handle (RAII), so the
    // upstream is cancelled and the mock observes its connection close — even
    // though `interp` is still alive.
    tokio::time::timeout(Duration::from_secs(3), &mut upstream_closed)
        .await
        .expect(
            "upstream was not closed after the interpret() future was dropped — \
             the outbound handle leaked until interpreter teardown",
        )
        .expect("upstream close sender dropped");

    // Keep the interpreter alive until after the assertion, so the close was the
    // RAII guard's doing and not the interpreter being torn down.
    drop(interp);
}
