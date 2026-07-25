// Red→Green regression for issue #642: module/include loading context must be
// handler-local under `main loop concurrently:`.
//
// `current_source_file` (relative-path resolution base) and `loading_stack`
// (circular-dependency detection) are interpreter-global. A handler that
// awaits mid-include leaves its module on the shared stack and its module's
// path installed as the resolution base, so a concurrent sibling:
//   1. including the same module falsely trips "Circular dependency detected";
//   2. including its own (existing) module resolves the relative path against
//      the *other* handler's module directory and fails with "not found".

use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

mod common;

fn start_server_thread(code: String, source_file: Option<PathBuf>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let tokens = lex_wfl_with_positions(&code);
            let mut parser = Parser::new(&tokens);
            let ast = parser.parse().expect("parse");
            let mut interpreter = Interpreter::new();
            if let Some(path) = source_file {
                interpreter.set_source_file(path);
            }
            if let Err(errors) = interpreter.interpret(&ast).await {
                panic!("server interpreter failed: {errors:?}");
            }
        });
    })
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

fn wfl_path(path: &std::path::Path) -> String {
    path.display().to_string().replace('\\', "/")
}

#[tokio::test]
async fn test_concurrent_includes_of_same_module_do_not_false_cycle() {
    let dir = TempDir::new().expect("tempdir");
    // Top-level wait keeps the module load in flight so a sibling's include of
    // the same module overlaps it.
    let module = dir.path().join("shared_mod.wfl");
    fs::write(
        &module,
        "wait for 400 milliseconds\nstore shared_value as \"included-ok\"\n",
    )
    .expect("write module");
    let module_path = wfl_path(&module);

    let port = common::free_tcp_port();
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
                include from "{module_path}"
                respond to req with shared_value
            end check
        end loop
    "#
    );
    let server = start_server_thread(code, None);
    wait_for_server(port).await;

    // Overlap two loads of the same module.
    let a_url = format!("http://127.0.0.1:{port}/a");
    let a = tokio::spawn(async move { reqwest::Client::new().get(&a_url).send().await.unwrap() });
    tokio::time::sleep(Duration::from_millis(100)).await;
    let b = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/b"))
        .send()
        .await
        .expect("/b request failed");

    let b_status = b.status().as_u16();
    let b_body = b.text().await.unwrap();
    assert_eq!(
        (b_status, b_body.as_str()),
        (200, "included-ok"),
        "a sibling's in-flight include of the same module was misreported as \
         a circular dependency"
    );

    let a_resp = a.await.expect("/a task panicked");
    let a_status = a_resp.status().as_u16();
    let a_body = a_resp.text().await.unwrap();
    assert_eq!((a_status, a_body.as_str()), (200, "included-ok"));

    shutdown(port, server).await;
}

#[tokio::test]
async fn test_relative_include_resolves_against_own_handler_context() {
    let dir = TempDir::new().expect("tempdir");
    let main_file = dir.path().join("main.wfl");
    fs::write(&main_file, "// server main\n").expect("write main");

    // /a's module lives in a subdirectory and stays in flight (top-level wait),
    // installing that subdirectory as the shared resolution base while parked.
    let subdir = dir.path().join("subdir");
    fs::create_dir(&subdir).expect("mkdir subdir");
    fs::write(
        subdir.join("slow_mod.wfl"),
        "wait for 400 milliseconds\nstore slow_marker as \"slow-ok\"\n",
    )
    .expect("write slow module");

    // /b's module lives next to main.wfl and is included by RELATIVE path, so
    // it only resolves if the base is /b's own context (main.wfl's directory),
    // not /a's parked module directory.
    fs::write(
        dir.path().join("sibling.wfl"),
        "store sibling_value as \"sibling-ok\"\n",
    )
    .expect("write sibling module");

    let port = common::free_tcp_port();
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
                    include from "subdir/slow_mod.wfl"
                    respond to req with slow_marker
                otherwise:
                    include from "sibling.wfl"
                    respond to req with sibling_value
                end check
            end check
        end loop
    "#
    );
    let server = start_server_thread(code, Some(main_file));
    wait_for_server(port).await;

    let a_url = format!("http://127.0.0.1:{port}/a");
    let a = tokio::spawn(async move { reqwest::Client::new().get(&a_url).send().await.unwrap() });
    tokio::time::sleep(Duration::from_millis(100)).await;
    let b = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/b"))
        .send()
        .await
        .expect("/b request failed");

    let b_status = b.status().as_u16();
    let b_body = b.text().await.unwrap();
    assert_eq!(
        (b_status, b_body.as_str()),
        (200, "sibling-ok"),
        "/b's relative include resolved against a sibling handler's module \
         directory instead of its own context"
    );

    let a_resp = a.await.expect("/a task panicked");
    let a_status = a_resp.status().as_u16();
    let a_body = a_resp.text().await.unwrap();
    assert_eq!((a_status, a_body.as_str()), (200, "slow-ok"));

    shutdown(port, server).await;
}
