//! End-to-end HTTP coverage for the WFL port of laravel/laravel (13.x).
//!
//! Spawns the real `wfl` binary on `examples/laravel-app/app.wfl` (copied to a
//! temp dir so the listen port can be a free OS port) and drives `/`, `/up`,
//! `/robots.txt`, an unknown path, and `POST /` over a real socket.

use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

mod common;

fn example_app_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/laravel-app")
}

fn stage_app(port: u16) -> tempfile::TempDir {
    let src = example_app_dir();
    let dir = tempfile::tempdir().expect("tempdir for laravel-app port");
    for name in ["app.wfl", "routes.wfl", "views.wfl", "user.wfl", "quotes.wfl"] {
        let content = std::fs::read_to_string(src.join(name))
            .unwrap_or_else(|e| panic!("read {}: {e}", src.join(name).display()));
        let content = content.replace(
            "store listen_port as 8000",
            &format!("store listen_port as {port}"),
        );
        std::fs::write(dir.path().join(name), content).expect("write staged app file");
    }
    let public = dir.path().join("public");
    std::fs::create_dir_all(&public).expect("create public/");
    std::fs::copy(src.join("public/robots.txt"), public.join("robots.txt"))
        .expect("copy robots.txt");
    if src.join(".wflcfg").exists() {
        std::fs::copy(src.join(".wflcfg"), dir.path().join(".wflcfg")).expect("copy .wflcfg");
    }
    dir
}

async fn start_app(port: u16) -> (Child, tempfile::TempDir) {
    let dir = stage_app(port);
    let program = dir.path().join("app.wfl");
    let mut child = Command::new(common::wfl_exe())
        .arg(&program)
        .current_dir(dir.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn wfl laravel-app");

    let stdout = child.stdout.take().expect("child stdout");
    let mut lines = BufReader::new(stdout).lines();
    tokio::time::timeout(Duration::from_secs(60), async {
        while let Ok(Some(line)) = lines.next_line().await {
            if line.contains("listening on port") {
                return;
            }
        }
        panic!("laravel-app server exited before announcing its listen port");
    })
    .await
    .expect("timed out waiting for laravel-app to listen");

    (child, dir)
}

async fn get(port: u16, path: &str) -> reqwest::Response {
    reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}{path}"))
        .send()
        .await
        .unwrap_or_else(|e| panic!("GET {path}: {e}"))
}

async fn post(port: u16, path: &str) -> reqwest::Response {
    reqwest::Client::new()
        .post(format!("http://127.0.0.1:{port}{path}"))
        .send()
        .await
        .unwrap_or_else(|e| panic!("POST {path}: {e}"))
}

#[tokio::test]
async fn welcome_page_returns_200_html() {
    let port = common::free_tcp_port();
    let (_child, _dir) = start_app(port).await;

    let response = get(port, "/").await;
    assert_eq!(response.status(), 200);
    let body = response.text().await.expect("welcome body");
    assert!(
        body.contains("Your starter app is ready"),
        "welcome body should identify the starter app, got: {body}"
    );
    assert!(
        body.contains("WFL starter"),
        "welcome body should name the WFL starter, got: {body}"
    );
}

#[tokio::test]
async fn health_endpoint_reports_application_up() {
    let port = common::free_tcp_port();
    let (_child, _dir) = start_app(port).await;

    let response = get(port, "/up").await;
    assert_eq!(response.status(), 200);
    let body = response.text().await.expect("health body");
    assert!(
        body.contains("Application up"),
        "health body should report Application up, got: {body}"
    );
}

#[tokio::test]
async fn robots_txt_matches_laravel_skeleton() {
    let port = common::free_tcp_port();
    let (_child, _dir) = start_app(port).await;

    let response = get(port, "/robots.txt").await;
    assert_eq!(response.status(), 200);
    let body = response.text().await.expect("robots body");
    assert!(body.contains("User-agent: *"), "got: {body}");
    assert!(body.contains("Disallow:"), "got: {body}");
}

#[tokio::test]
async fn unknown_path_returns_404() {
    let port = common::free_tcp_port();
    let (_child, _dir) = start_app(port).await;

    let response = get(port, "/no-such-route").await;
    assert_eq!(response.status(), 404);
    let body = response.text().await.expect("404 body");
    assert!(body.contains("Not Found"), "got: {body}");
}

#[tokio::test]
async fn post_to_welcome_returns_405() {
    let port = common::free_tcp_port();
    let (_child, _dir) = start_app(port).await;

    let response = post(port, "/").await;
    assert_eq!(response.status(), 405);
    let body = response.text().await.expect("405 body");
    assert!(body.contains("Method Not Allowed"), "got: {body}");
}
