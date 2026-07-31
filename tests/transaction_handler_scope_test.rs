//! A transaction belongs to the handler that opened it (testing.md §11.3).
//!
//! Under `main loop concurrently:` every handler shares one `IoClient` and can
//! name the same global database handle. If open transactions were keyed by
//! handle alone, a handler that merely runs an ordinary `execute` on that handle
//! would be silently enrolled in a *different* handler's transaction — and its
//! write would then be committed or rolled back by that other request. That is a
//! cross-request data-integrity bug, and it is what this file pins shut.
//!
//! Driven over a real socket against a real file-backed SQLite database: the
//! interleaving only exists when two handler futures are genuinely in flight at
//! once, so a sequential test cannot observe it.

use std::path::{Path, PathBuf};
use std::time::Duration;

mod common;

/// A temp-file SQLite database that removes itself on drop. File-backed because
/// in-memory SQLite is capped at a single connection and so cannot exhibit the
/// pooled-handle behaviour under test.
struct TempDb {
    url: String,
    path: PathBuf,
}

impl TempDb {
    fn new(test_name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "wfl_tx_scope_{}_{}.db",
            test_name,
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let url = format!("sqlite://{}", path.display()).replace('\\', "/");
        Self { url, path }
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        for suffix in ["-wal", "-shm"] {
            let mut sidecar = self.path.clone().into_os_string();
            sidecar.push(suffix);
            let _ = std::fs::remove_file(Path::new(&sidecar));
        }
    }
}

fn start_server_thread(code: String) -> std::thread::JoinHandle<()> {
    // An explicit stack: the interpreter executes nested blocks by recursion, and
    // this server program is several blocks deep (concurrent main loop → check →
    // check → try → transaction). A spawned thread's default stack is 2 MB on
    // Linux and 1 MB on Windows, which this exceeds. The subject of the test is
    // transaction ownership, not stack depth, so give it room rather than
    // flattening the program into something that no longer reproduces the race.
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("build runtime");
            runtime.block_on(async move {
                let tokens = wfl::lexer::lex_wfl_with_positions(&code);
                let mut parser = wfl::parser::Parser::new(&tokens);
                let ast = parser.parse().expect("parse");
                let mut interpreter = wfl::Interpreter::new();
                if let Err(errors) = interpreter.interpret(&ast).await {
                    panic!("server interpreter failed: {errors:?}");
                }
            });
        })
        .expect("spawn server thread")
}

async fn wait_for_server(port: u16) {
    let addr = format!("127.0.0.1:{port}");
    for _ in 0..300 {
        if tokio::net::TcpStream::connect(&addr).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("server on {addr} did not become ready in time");
}

async fn shutdown(port: u16, server: std::thread::JoinHandle<()>) {
    let _ = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/shutdown"))
        .send()
        .await;
    match tokio::task::spawn_blocking(move || server.join()).await {
        Ok(Ok(())) => {}
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(join_err) => panic!("server join task failed: {join_err}"),
    }
}

/// `/tx` opens a transaction on the shared handle, waits (so the other request
/// is guaranteed to be interleaved inside it), writes a row, and then fails —
/// rolling its own work back. `/plain` writes a row through the very same
/// handle while that transaction is open.
///
/// The `/plain` row must survive: it was never part of the transaction.
#[tokio::test]
async fn an_unrelated_handler_is_not_enrolled_in_another_handlers_transaction() {
    let db = TempDb::new("not_enrolled");
    let url = &db.url;
    let port = common::free_tcp_port();

    let code = format!(
        r#"
        open database at "{url}" as db
        store made as execute db with "CREATE TABLE writes (tag TEXT)"
        listen on port {port} as srv
        main loop concurrently:
            wait for request comes in on srv as req with timeout 20000
            store p as req["path"]
            check if p is equal to "/shutdown":
                respond to req with "bye"
                close server srv
                break
            otherwise:
                check if p is equal to "/tx":
                    try:
                        in transaction on db:
                            store a as execute db with "INSERT INTO writes (tag) VALUES ('rolled-back')"
                            wait for 600 milliseconds
                            store boom as execute db with "INSERT INTO no_such_table (x) VALUES (1)"
                        end transaction
                    when error:
                        respond to req with "tx-rolled-back"
                    end try
                otherwise:
                    store b as execute db with "INSERT INTO writes (tag) VALUES ('kept')"
                    respond to req with "plain-written"
                end check
            end check
        end loop
    "#
    );

    let server = start_server_thread(code);
    wait_for_server(port).await;

    // Start the transactional request and let it get inside its block.
    let tx_url = format!("http://127.0.0.1:{port}/tx");
    let tx = tokio::spawn(async move {
        reqwest::Client::new()
            .get(&tx_url)
            .send()
            .await
            .expect("tx request failed")
            .text()
            .await
            .unwrap()
    });
    tokio::time::sleep(Duration::from_millis(200)).await;

    // ...then write through the same handle from a different handler.
    let plain = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/plain"))
        .send()
        .await
        .expect("plain request failed")
        .text()
        .await
        .unwrap();
    assert_eq!(plain, "plain-written");

    assert_eq!(tx.await.expect("tx task panicked"), "tx-rolled-back");

    shutdown(port, server).await;

    // The transaction's own row is gone; the unrelated handler's row remains.
    let rows = surviving_tags(url).await;
    assert!(
        rows.contains(&"kept".to_string()),
        "an unrelated handler's write was rolled back by another handler's \
         transaction — transactions must be scoped to the handler that opened \
         them; rows present: {rows:?}"
    );
    assert!(
        !rows.contains(&"rolled-back".to_string()),
        "the transaction's own write must still have been rolled back; rows present: {rows:?}"
    );
}

/// Read the table back with a fresh interpreter, after the server is gone.
async fn surviving_tags(url: &str) -> Vec<String> {
    let code = format!(
        r#"
open database at "{url}" as db
store rows as query db with "SELECT tag FROM writes ORDER BY tag"
"#
    );
    let tokens = wfl::lexer::lex_wfl_with_positions(&code);
    let mut parser = wfl::parser::Parser::new(&tokens);
    let ast = parser.parse().expect("parse");
    let mut interpreter = wfl::Interpreter::new();
    interpreter
        .interpret(&ast)
        .await
        .expect("read-back program");

    let rows = interpreter
        .global_env()
        .borrow()
        .get("rows")
        .expect("rows must exist");

    match rows {
        wfl::interpreter::value::Value::List(items) => items
            .borrow()
            .iter()
            .map(|row| match row {
                wfl::interpreter::value::Value::Object(map) => {
                    match map.borrow().get("tag").expect("tag column") {
                        wfl::interpreter::value::Value::Text(t) => t.to_string(),
                        other => panic!("unexpected tag value {other:?}"),
                    }
                }
                other => panic!("unexpected row {other:?}"),
            })
            .collect(),
        other => panic!("expected a list of rows, got {other:?}"),
    }
}
