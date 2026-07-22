// Tests for `main loop concurrently:` (concurrent request handlers).
//
// Key properties:
//   - `main loop concurrently:` parses (concurrent = true); plain `main loop:`
//     stays serial (concurrent = false) — no silent upgrade.
//   - Concurrent: a slow handler does NOT block a fast sibling.
//   - Serial: a slow handler DOES block the next request (unchanged behavior).
//   - A handler that errors is contained; the server keeps serving.
//
// Each server exposes a `/shutdown` path that closes the server and breaks the
// loop, so the test can stop the server thread deterministically and join it.

use std::time::{Duration, Instant};
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::Statement;

fn parse_program(code: &str) -> Vec<Statement> {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    parser
        .parse()
        .unwrap_or_else(|e| panic!("Parse error: {e:?}"))
        .statements
}

#[test]
fn test_main_loop_concurrently_parses_as_concurrent() {
    let stmts = parse_program("main loop concurrently:\n    display \"x\"\nend loop");
    match &stmts[0] {
        Statement::MainLoop { concurrent, .. } => assert!(*concurrent),
        other => panic!("Expected MainLoop, got {other:?}"),
    }
}

#[test]
fn test_plain_main_loop_stays_serial() {
    let stmts = parse_program("main loop:\n    display \"x\"\nend loop");
    match &stmts[0] {
        Statement::MainLoop { concurrent, .. } => {
            assert!(!*concurrent, "plain main loop must remain serial")
        }
        other => panic!("Expected MainLoop, got {other:?}"),
    }
}

fn start_server_thread(code: String) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let mut parser = Parser::new(&tokens);
            let ast = parser.parse().expect("parse");
            let mut interpreter = Interpreter::new();
            let _ = interpreter.interpret(&ast).await;
        });
    })
}

fn server_code(port: u16, concurrently: bool) -> String {
    let marker = if concurrently { " concurrently" } else { "" };
    format!(
        r#"
        listen on port {port} as srv
        main loop{marker}:
            wait for request comes in on srv as req with timeout 20000
            store p as req["path"]
            check if p is equal to "/shutdown":
                respond to req with "bye"
                close server srv
                break
            otherwise:
                check if p is equal to "/slow":
                    wait for 500 milliseconds
                    respond to req with "slow"
                otherwise:
                    respond to req with "fast"
                end check
            end check
        end loop
    "#
    )
}

/// Send `/shutdown` so the server closes and its loop breaks, then join.
async fn shutdown(port: u16, server: std::thread::JoinHandle<()>) {
    let _ = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/shutdown"))
        .send()
        .await;
    let _ = tokio::task::spawn_blocking(move || server.join()).await;
}

#[tokio::test]
async fn test_concurrent_slow_handler_does_not_block_fast() {
    let port = 8341;
    let server = start_server_thread(server_code(port, true));
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();

    // Kick off the slow request first and give it a moment to be dequeued.
    let slow_url = format!("http://127.0.0.1:{port}/slow");
    let slow =
        tokio::spawn(async move { reqwest::Client::new().get(&slow_url).send().await.unwrap() });
    tokio::time::sleep(Duration::from_millis(80)).await;

    // The fast request must complete promptly even though /slow is mid-handler.
    let t0 = Instant::now();
    let fast = client
        .get(format!("http://127.0.0.1:{port}/fast"))
        .send()
        .await
        .expect("fast request failed");
    let fast_elapsed = t0.elapsed();
    let fast_body = fast.text().await.unwrap();

    assert_eq!(fast_body, "fast");
    assert!(
        fast_elapsed < Duration::from_millis(300),
        "fast request was blocked behind the slow handler ({fast_elapsed:?})"
    );

    let slow_resp = slow.await.expect("slow request task panicked");
    assert_eq!(slow_resp.text().await.unwrap(), "slow");

    shutdown(port, server).await;
}

#[tokio::test]
async fn test_concurrent_handler_error_does_not_kill_server() {
    // A handler that errors mid-iteration (here: responding twice) must be
    // contained — the concurrent loop keeps serving other requests.
    let port = 8342;
    let code = format!(
        r#"
        listen on port {port} as srv
        main loop concurrently:
            wait for request comes in on srv as req with timeout 20000
            store p as req["path"]
            check if p is equal to "/shutdown":
                respond to req with "bye"
                close server srv
                break
            otherwise:
                check if p is equal to "/boom":
                    respond to req with "boom-ok"
                    respond to req with "this second respond errors"
                otherwise:
                    respond to req with "ok"
                end check
            end check
        end loop
    "#
    );
    let server = start_server_thread(code);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();

    // The erroring handler still delivered its first response.
    let boom = client
        .get(format!("http://127.0.0.1:{port}/boom"))
        .send()
        .await
        .expect("boom request failed");
    assert_eq!(boom.text().await.unwrap(), "boom-ok");

    // The server survived the caught error and keeps serving.
    let ok = client
        .get(format!("http://127.0.0.1:{port}/ok"))
        .send()
        .await
        .expect("follow-up request failed");
    assert_eq!(ok.text().await.unwrap(), "ok");

    shutdown(port, server).await;
}

#[tokio::test]
async fn test_concurrent_handlers_do_not_share_count_loop_state() {
    // Per-handler run-state isolation (P1 #1). Two concurrent handlers each run
    // a `count` loop that yields (via `wait for`) mid-iteration and then reads
    // `count`. The interpreter's count-loop state (`current_count`,
    // `in_count_loop`) is a single shared field; without per-poll isolation, one
    // handler's `count` bleeds into the other across the yield.
    //
    // The two ranges are disjoint (1..5 vs 100..104), so any cross-contamination
    // is unmistakable: with isolation each handler observes only its own range.
    let port = 8344;
    let code = format!(
        r#"
        listen on port {port} as srv
        main loop concurrently:
            wait for request comes in on srv as req with timeout 20000
            store p as req["path"]
            check if p is equal to "/shutdown":
                respond to req with "bye"
                close server srv
                break
            otherwise:
                check if p is equal to "/a":
                    store seen as ""
                    count from 1 to 5:
                        wait for 80 milliseconds
                        change seen to seen with count with "-"
                    end count
                    respond to req with seen
                otherwise:
                    store seen as ""
                    count from 100 to 104:
                        wait for 80 milliseconds
                        change seen to seen with count with "-"
                    end count
                    respond to req with seen
                end check
            end check
        end loop
    "#
    );
    let server = start_server_thread(code);
    tokio::time::sleep(Duration::from_millis(300)).await;

    let a_url = format!("http://127.0.0.1:{port}/a");
    let b_url = format!("http://127.0.0.1:{port}/b");
    // Fire both at once so their count loops interleave on the single thread.
    let a = tokio::spawn(async move {
        reqwest::Client::new()
            .get(&a_url)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    });
    let b = tokio::spawn(async move {
        reqwest::Client::new()
            .get(&b_url)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    });

    let a_body = a.await.expect("/a task panicked");
    let b_body = b.await.expect("/b task panicked");

    assert_eq!(
        a_body, "1-2-3-4-5-",
        "/a handler observed a count from outside its own loop (shared count-loop state)"
    );
    assert_eq!(
        b_body, "100-101-102-103-104-",
        "/b handler observed a count from outside its own loop (shared count-loop state)"
    );

    shutdown(port, server).await;
}

#[tokio::test]
async fn test_serial_slow_handler_blocks_next() {
    let port = 8343;
    let server = start_server_thread(server_code(port, false));
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();

    let slow_url = format!("http://127.0.0.1:{port}/slow");
    let slow =
        tokio::spawn(async move { reqwest::Client::new().get(&slow_url).send().await.unwrap() });
    tokio::time::sleep(Duration::from_millis(80)).await;

    // On the serial loop the fast request cannot be handled until the slow
    // handler finishes, so it is delayed by roughly the slow handler's duration.
    let t0 = Instant::now();
    let fast = client
        .get(format!("http://127.0.0.1:{port}/fast"))
        .send()
        .await
        .expect("fast request failed");
    let fast_elapsed = t0.elapsed();
    let fast_body = fast.text().await.unwrap();

    assert_eq!(fast_body, "fast");
    assert!(
        fast_elapsed > Duration::from_millis(300),
        "serial main loop should have blocked the fast request behind the slow one ({fast_elapsed:?})"
    );

    let slow_resp = slow.await.expect("slow request task panicked");
    assert_eq!(slow_resp.text().await.unwrap(), "slow");

    shutdown(port, server).await;
}
