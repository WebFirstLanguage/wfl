//! Regression for the admission-cap deadlock (issue #611, P1-2).
//!
//! When the global in-flight admission cap fills with requests the handler
//! dequeued but never answered, the route tasks eventually hit their response
//! timeout — and the freed admission slots MUST reopen WITHOUT needing another
//! admitted request first. Previously each dequeued request's admission guard
//! was parked in the interpreter's pending map and released only when a *later*
//! dequeued request was inserted (which pruned closed entries). Once the cap was
//! full no new request could be admitted to trigger that prune, so every slot
//! stayed pinned and the server was permanently wedged. The admission guard now
//! rides with the warp transport task, so a response timeout (or disconnect)
//! releases the slot independently of any future admission.

use std::sync::Arc;
use std::time::Duration;
use wfl::Interpreter;
use wfl::config::WflConfig;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

/// Start a WFL server on its own thread + runtime with a custom config.
fn start_server(code: String, config: WflConfig) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let ast = Parser::new(&tokens).parse().expect("parse server code");
            let mut interpreter = Interpreter::with_config(Arc::new(config));
            let _ = interpreter.interpret(&ast).await;
        });
    })
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("build client")
}

#[tokio::test]
async fn admission_reopens_after_pending_requests_time_out() {
    let port = 8241;

    // Cap in-flight admission at 2 and shed a dequeued-but-unanswered request
    // after 1s. The handler dequeues requests in a loop and NEVER responds, so
    // each admitted request pins its slot until its route task times out.
    let config = WflConfig {
        web_server_request_queue_bound: 2,
        web_server_response_timeout_seconds: 1,
        ..Default::default()
    };

    let server_code = format!(
        "\
listen on port {port} as test_server
count from 1 to 100:
    wait for request comes in on test_server as req with timeout 30000
end count
"
    );

    let _server = start_server(server_code, config);
    // Let the listener bind.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let url = format!("http://127.0.0.1:{port}/");

    // Fill both admission slots with requests the handler dequeues but never
    // answers. Each client stays connected and receives a 504 when its route
    // task times out (~1s) — the TIMEOUT path, not a client disconnect.
    let mut fillers = Vec::new();
    for _ in 0..2 {
        let url = url.clone();
        fillers.push(tokio::spawn(async move {
            client().get(&url).send().await.map(|r| r.status().as_u16())
        }));
    }

    // Wait past the 1s response timeout so both route tasks have shed their
    // requests and released their slots.
    tokio::time::sleep(Duration::from_secs(3)).await;
    for f in fillers {
        let status = f.await.expect("filler task").expect("filler response");
        assert_eq!(
            status, 504,
            "a dequeued-but-unanswered request should time out with 504"
        );
    }

    // With the fix, admission has reopened WITHOUT another request being admitted
    // first, so this request reaches the server (and, since the handler still
    // never answers, itself times out with 504). Before the fix every slot
    // stayed pinned and this would be shed with 503.
    let third = client()
        .get(&url)
        .send()
        .await
        .expect("third request should reach the server, not be shed");
    assert_ne!(
        third.status().as_u16(),
        503,
        "admission must reopen after pending requests time out (503 = still wedged)"
    );
    assert_eq!(
        third.status().as_u16(),
        504,
        "the admitted third request should itself time out with 504"
    );
}
