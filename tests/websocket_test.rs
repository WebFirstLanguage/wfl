//! End-to-end WebSocket integration test.
//!
//! Spawns the compiled `wfl` binary running an echo/broadcast WebSocket server,
//! connects real clients with `tokio-tungstenite`, and asserts the server drives
//! the `on websocket connect/message/disconnect` handler blocks and the
//! `send`/`broadcast`/`close server` statements correctly.

use std::process::Stdio;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// Spawns `wfl <program>` and returns the child, the port it announced, and the
/// temp dir holding the program (kept alive by the caller so it is cleaned up on
/// scope exit — including on panic).
async fn start_ws_server(program: &str) -> (Child, u16, TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let prog_path = dir.path().join("ws_server.wfl");
    std::fs::write(&prog_path, program).expect("write program");

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

    (child, port, dir)
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
            // tokio-tungstenite 0.30 carries text frames as `Utf8Bytes`.
            Message::Text(text) => return text.to_string(),
            Message::Ping(_) | Message::Pong(_) => continue,
            other => panic!("unexpected websocket frame: {other:?}"),
        }
    }
}

/// Waits until the connection is closed (a close frame, a clean stream end, or a
/// transport error all count).
async fn expect_closed(ws: &mut WsStream) {
    loop {
        let item = tokio::time::timeout(Duration::from_secs(10), ws.next())
            .await
            .expect("read timed out waiting for the connection to close");
        match item {
            None | Some(Ok(Message::Close(_))) | Some(Err(_)) => return,
            Some(Ok(_)) => continue, // ignore any data/control frames before close
        }
    }
}

// The outer `conn` / `body` bindings deliberately collide with the handler
// binding and a message property name: the handler binding must shadow the outer
// variable (runtime), and `body of msg` must resolve as a property read even
// though `body` also names an outer variable (analysis).
const ECHO_PROGRAM: &str = r#"
store conn as "outer connection"
store body as "outer body"

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

on websocket connect to ws_server as conn:
    send websocket message "ready" to conn
end on

on websocket message from ws_server as msg:
    broadcast websocket message body of msg to ws_server
end on

wait for 30 seconds
"#;

const DISCONNECT_PROGRAM: &str = r#"
listen for websockets on port 0 as ws_server

on websocket connect to ws_server as conn:
    send websocket message "ready" to conn
end on

on websocket disconnect from ws_server as conn:
    broadcast websocket message "a client left" to ws_server
end on

wait for 30 seconds
"#;

const CLOSE_SERVER_PROGRAM: &str = r#"
listen for websockets on port 0 as ws_server

on websocket connect to ws_server as conn:
    send websocket message "ready" to conn
end on

on websocket message from ws_server as msg:
    close server ws_server
end on

wait for 30 seconds
"#;

#[tokio::test]
async fn websocket_connect_and_echo() {
    let (mut child, port, _dir) = start_ws_server(ECHO_PROGRAM).await;
    let mut ws = connect(port).await;

    // The connect handler greets the client first.
    assert_eq!(next_text(&mut ws).await, "welcome");

    // The message handler echoes with a prefix, proving inbound content reaches
    // the handler (`body of msg`) and `send ... to msg` replies to the sender.
    ws.send(Message::Text("hello".into()))
        .await
        .expect("send hello");
    assert_eq!(next_text(&mut ws).await, "Echo: hello");

    ws.send(Message::Text("again".into()))
        .await
        .expect("send again");
    assert_eq!(next_text(&mut ws).await, "Echo: again");

    let _ = ws.close(None).await;
    let _ = child.kill().await;
}

#[tokio::test]
async fn websocket_broadcast_reaches_all_clients() {
    let (mut child, port, _dir) = start_ws_server(BROADCAST_PROGRAM).await;

    let mut a = connect(port).await;
    let mut b = connect(port).await;

    // Wait for the server to register both clients (deterministic — no sleeps):
    // each receives its connect greeting before the broadcast is sent.
    assert_eq!(next_text(&mut a).await, "ready");
    assert_eq!(next_text(&mut b).await, "ready");

    a.send(Message::Text("ping".into()))
        .await
        .expect("send ping");

    // Both clients (including the sender) receive the broadcast.
    assert_eq!(next_text(&mut a).await, "ping");
    assert_eq!(next_text(&mut b).await, "ping");

    let _ = a.close(None).await;
    let _ = b.close(None).await;
    let _ = child.kill().await;
}

#[tokio::test]
async fn websocket_disconnect_handler_fires() {
    let (mut child, port, _dir) = start_ws_server(DISCONNECT_PROGRAM).await;

    let mut a = connect(port).await;
    let mut b = connect(port).await;

    assert_eq!(next_text(&mut a).await, "ready");
    assert_eq!(next_text(&mut b).await, "ready");

    // When A leaves, the disconnect handler broadcasts to the remaining clients.
    a.close(None).await.expect("close a");
    assert_eq!(next_text(&mut b).await, "a client left");

    let _ = b.close(None).await;
    let _ = child.kill().await;
}

#[tokio::test]
async fn websocket_close_server_closes_connections() {
    let (mut child, port, _dir) = start_ws_server(CLOSE_SERVER_PROGRAM).await;

    let mut ws = connect(port).await;
    assert_eq!(next_text(&mut ws).await, "ready");

    // Asking the server to close should tear the client's connection down.
    ws.send(Message::Text("shutdown".into()))
        .await
        .expect("send shutdown");
    expect_closed(&mut ws).await;

    let _ = child.kill().await;
}
