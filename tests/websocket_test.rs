//! End-to-end WebSocket integration test.
//!
//! Spawns the compiled `wfl` binary running an echo/broadcast WebSocket server,
//! connects real clients with `tokio-tungstenite`, and asserts the server drives
//! the `on websocket connect/message` handler blocks and the `send`/`broadcast`
//! statements correctly.

use std::process::Stdio;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// Spawns `wfl <program>` and returns the child plus the port it announced.
async fn start_ws_server(program: &str) -> (Child, u16) {
    let dir = tempfile::tempdir().expect("tempdir");
    let prog_path = dir.path().join("ws_server.wfl");
    std::fs::write(&prog_path, program).expect("write program");
    // Keep the temp dir alive for the process lifetime by leaking it; the OS
    // reclaims it when the test process exits.
    std::mem::forget(dir);

    let mut child = Command::new(env!("CARGO_BIN_EXE_wfl"))
        .arg(&prog_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn wfl binary");

    let stdout = child.stdout.take().expect("child stdout");
    let mut lines = BufReader::new(stdout).lines();

    let port = tokio::time::timeout(Duration::from_secs(60), async {
        while let Ok(Some(line)) = lines.next_line().await {
            const MARKER: &str = "listening on port ";
            if let Some(idx) = line.rfind(MARKER)
                && let Ok(port) = line[idx + MARKER.len()..].trim().parse::<u16>()
            {
                return Some(port);
            }
        }
        None
    })
    .await
    .expect("timed out waiting for the websocket server to start")
    .expect("server did not announce a port");

    (child, port)
}

async fn connect(port: u16) -> WsStream {
    let url = format!("ws://127.0.0.1:{port}");
    let (ws, _response) = tokio::time::timeout(Duration::from_secs(10), connect_async(&url))
        .await
        .expect("connect timed out")
        .expect("websocket handshake failed");
    ws
}

/// Reads the next text frame, skipping control frames.
async fn next_text(ws: &mut WsStream) -> String {
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(10), ws.next())
            .await
            .expect("read timed out")
            .expect("stream ended unexpectedly")
            .expect("websocket error");
        match msg {
            Message::Text(text) => return text,
            Message::Ping(_) | Message::Pong(_) => continue,
            other => panic!("unexpected websocket frame: {other:?}"),
        }
    }
}

const ECHO_PROGRAM: &str = r#"
listen for websockets on port 0 as ws_server

on websocket connect to ws_server as conn:
    send websocket message "welcome" to conn
end on

on websocket message from ws_server as msg:
    send websocket message "Echo: " with body of msg to msg
end on

wait for 30 seconds
"#;

const BROADCAST_PROGRAM: &str = r#"
listen for websockets on port 0 as ws_server

on websocket message from ws_server as msg:
    broadcast websocket message body of msg to ws_server
end on

wait for 30 seconds
"#;

#[tokio::test]
async fn websocket_connect_and_echo() {
    let (mut child, port) = start_ws_server(ECHO_PROGRAM).await;
    let mut ws = connect(port).await;

    // The connect handler greets the client first.
    assert_eq!(next_text(&mut ws).await, "welcome");

    // The message handler echoes with a prefix, proving inbound content reaches
    // the handler (`body of msg`) and `send ... to msg` replies to the sender.
    ws.send(Message::Text("hello".to_string()))
        .await
        .expect("send hello");
    assert_eq!(next_text(&mut ws).await, "Echo: hello");

    ws.send(Message::Text("again".to_string()))
        .await
        .expect("send again");
    assert_eq!(next_text(&mut ws).await, "Echo: again");

    let _ = ws.close(None).await;
    let _ = child.kill().await;
}

#[tokio::test]
async fn websocket_broadcast_reaches_all_clients() {
    let (mut child, port) = start_ws_server(BROADCAST_PROGRAM).await;

    let mut a = connect(port).await;
    let mut b = connect(port).await;

    // Give the second connection a moment to register before broadcasting.
    tokio::time::sleep(Duration::from_millis(200)).await;

    a.send(Message::Text("ping".to_string()))
        .await
        .expect("send ping");

    // Both clients (including the sender) receive the broadcast.
    assert_eq!(next_text(&mut a).await, "ping");
    assert_eq!(next_text(&mut b).await, "ping");

    let _ = a.close(None).await;
    let _ = b.close(None).await;
    let _ = child.kill().await;
}
