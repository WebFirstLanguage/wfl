// Red→Green regression for issue #642: concurrent handlers must inherit the
// LIVE recursion depth at loop entry, not the run's base depth.
//
// `main loop concurrently:` can run nested inside user actions. Each handler's
// run state previously seeded `call_depth` from `base_call_depth` (the run
// entry, 0 for a top-level program), discarding the enclosing action frames
// that are still live beneath the loop. With 256 handlers each granted a full
// `max_call_depth` budget on top of uncounted real frames, the depth ceiling
// no longer bounds the native stack — the exact overflow the limit exists to
// prevent. (`execute file` already seeds its child with the parent's live
// depth; this aligns the concurrent loop with it.)
//
// Setup: max_call_depth = 10; the loop runs 3 action frames deep
// (level_a -> level_b -> serve); the handler recursion peaks at 9 frames of
// its own. 9 < 10 passes standalone, but 3 + 9 = 12 must exceed the limit.

use std::sync::Arc;
use std::time::Duration;
use wfl::Interpreter;
use wfl::config::WflConfig;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

mod common;

fn start_server_thread(code: String) -> std::thread::JoinHandle<()> {
    // Generous native stack: WFL call levels are stack-hungry in debug builds,
    // and on the BUGGY path the handler runs its full 9 recursion levels (the
    // depth limit never fires). The test must reach its assertion and fail on
    // content — not abort the process with a native stack overflow.
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("runtime");
            rt.block_on(async {
                let tokens = lex_wfl_with_positions(&code);
                let mut parser = Parser::new(&tokens);
                let ast = parser.parse().expect("parse");
                let config = WflConfig {
                    max_call_depth: 10,
                    ..WflConfig::default()
                };
                let mut interpreter = Interpreter::with_config(Arc::new(config));
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

#[tokio::test]
async fn test_handler_recursion_budget_includes_enclosing_action_frames() {
    let port = common::free_tcp_port();
    let code = format!(
        r#"
        define action called recurse with parameters n:
            check if n is greater than 0:
                store unused as recurse of n minus 1
            end check
            give back n
        end action

        define action called serve:
            main loop concurrently:
                wait for request comes in on srv as req with timeout 20000
                store p as req["path"]
                check if p is equal to "/shutdown":
                    respond to req with "bye"
                    close server srv
                    break
                otherwise:
                    store depth_outcome as "not-limited"
                    try:
                        store unused_r as recurse of 8
                    when error:
                        change depth_outcome to "depth-limited"
                    end try
                    respond to req with depth_outcome
                end check
            end loop
        end action

        define action called level_b:
            store serve_result as call serve
        end action

        define action called level_a:
            store level_b_result as call level_b
        end action

        listen on port {port} as srv
        store run_result as call level_a
    "#
    );
    let server = start_server_thread(code);
    wait_for_server(port).await;

    let resp = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/deep"))
        .send()
        .await
        .expect("request failed");
    let body = resp.text().await.unwrap();
    assert_eq!(
        body, "depth-limited",
        "handler recursion accounting ignored the live action frames \
         beneath the concurrent loop (expected 3 enclosing + 9 handler \
         frames to exceed max_call_depth 10)"
    );

    shutdown(port, server).await;
}
