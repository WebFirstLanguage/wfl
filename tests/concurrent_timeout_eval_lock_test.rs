// Red→Green regression for issue #642: `wait for request ... with timeout`
// must evaluate the timeout expression BEFORE locking the shared request
// receiver.
//
// The timeout clause takes an arbitrary WFL expression, and evaluating it can
// await (a user action that sleeps, does I/O, ...). If that evaluation happens
// while holding the receiver mutex shared by every concurrent handler, one
// handler's slow timeout expression parks the whole server: no sibling can
// dequeue a request until the expression finishes.
//
// Discriminating scenario: the first handler iteration to call `slow_timeout`
// flips a global flag and then sleeps 2s inside the expression. Siblings see
// the flipped flag and evaluate instantly.
//   - Buggy order (lock, then evaluate): the sleeping handler holds the
//     receiver lock, so a request sent during the sleep waits ~2s.
//   - Fixed order (evaluate, then lock): the sleeping handler holds no lock,
//     a sibling dequeues the request promptly (< 1s).

use std::time::{Duration, Instant};
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

mod common;

fn start_server_thread(code: String) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let mut parser = Parser::new(&tokens);
            let ast = parser.parse().expect("parse");
            let mut interpreter = Interpreter::new();
            // Surface an unexpected interpreter error as a thread panic so
            // `shutdown` re-raises it instead of the test silently passing.
            if let Err(errors) = interpreter.interpret(&ast).await {
                panic!("server interpreter failed: {errors:?}");
            }
        });
    })
}

/// Wait until the WFL server has actually bound `port` and is accepting
/// connections (bare TCP connect; warp accepts and closes it without
/// delivering an HTTP request to a handler).
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

/// Send `/shutdown` so the server closes and its loop breaks, then join.
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

#[tokio::test]
async fn test_slow_timeout_expression_does_not_stall_sibling_handlers() {
    let port = common::free_tcp_port();
    let code = format!(
        r#"
        store first as yes
        define action called slow_timeout:
            check if first is yes:
                change first to no
                wait for 2000 milliseconds
            end check
            give back 30000
        end action

        listen on port {port} as srv
        main loop concurrently:
            wait for request comes in on srv as req with timeout call slow_timeout
            store p as req["path"]
            check if p is equal to "/shutdown":
                respond to req with "bye"
                close server srv
                break
            otherwise:
                respond to req with "ok"
            end check
        end loop
    "#
    );
    let server = start_server_thread(code);
    wait_for_server(port).await;

    // Let the first handler iteration reach (and start sleeping inside) the
    // slow timeout expression before the request goes out.
    tokio::time::sleep(Duration::from_millis(200)).await;

    let t0 = Instant::now();
    let resp = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/ok"))
        .send()
        .await
        .expect("request failed");
    let elapsed = t0.elapsed();
    assert_eq!(resp.text().await.unwrap(), "ok");
    assert!(
        elapsed < Duration::from_millis(1000),
        "request was stalled behind a sibling's timeout-expression evaluation \
         holding the shared receiver lock ({elapsed:?})"
    );

    shutdown(port, server).await;
}
