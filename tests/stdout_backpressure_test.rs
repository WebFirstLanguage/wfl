//! Real stdout pipes must not monopolize the concurrent interpreter.
mod common;

use std::fs;
use std::future::Future;
use std::process::Stdio;
use std::time::Duration;
use tempfile::TempDir;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};

const FLOOD_BYTES: usize = 1_000_000;

async fn bounded<T>(future: impl Future<Output = T>, message: &str) -> T {
    tokio::time::timeout(Duration::from_secs(5), future)
        .await
        .expect(message)
}

fn start(dir: &TempDir, source: &str, timeout: &str) -> Child {
    let path = dir.path().join("main.wfl");
    fs::write(&path, source).unwrap();
    Command::new(common::wfl_exe())
        .args(["--execution-timeout", timeout])
        .arg(path)
        .current_dir(dir.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap()
}

async fn ready(port: u16) {
    bounded(
        async {
            loop {
                if tokio::net::TcpStream::connect(("127.0.0.1", port))
                    .await
                    .is_ok()
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        },
        "server did not become ready",
    )
    .await;
}

fn server_source(port: u16, emitter: &str) -> String {
    let flood = "x".repeat(FLOOD_BYTES);
    format!(
        r#"
store flood as "{flood}"
listen on port {port} as srv
main loop concurrently:
    wait for request comes in on srv as req with timeout 20000
    store p as req["path"]
    check if p is equal to "/shutdown":
        respond to req with "bye"
        close server srv
        break
    otherwise:
        check if p is equal to "/slow":
            {emitter}
            call write_stdout with "done"
            respond to req with "slow"
        otherwise:
            check if p is equal to "/queued":
                create file at "queued.txt"
                call write_stdout with "abandoned"
                respond to req with "queued"
            otherwise:
                respond to req with "fast"
            end check
        end check
    end check
end loop
"#
    )
}

async fn response(client: &reqwest::Client, port: u16, path: &str) -> String {
    bounded(
        async {
            client
                .get(format!("http://127.0.0.1:{port}{path}"))
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .text()
                .await
                .unwrap()
        },
        "unrelated request stalled behind stdout backpressure",
    )
    .await
}

async fn check_sibling_and_order(emitter: &str, newline: bool) {
    let dir = TempDir::new().unwrap();
    let port = common::free_tcp_port();
    let mut child = start(&dir, &server_source(port, emitter), "30");
    ready(port).await;
    let client = reqwest::Client::new();
    let slow_client = client.clone();
    let slow = tokio::spawn(async move { response(&slow_client, port, "/slow").await });
    let mut stdout = child.stdout.take().unwrap();
    let mut first = [0];
    // The first byte proves the handler entered its write. Stop draining while
    // the remaining payload exceeds pipe capacity; no timing guess is needed.
    bounded(stdout.read_exact(&mut first), "handler never wrote stdout")
        .await
        .unwrap();
    assert_eq!(first, [b'x']);
    assert_eq!(response(&client, port, "/fast").await, "fast");
    assert!(
        !slow.is_finished(),
        "write returned before the pipe drained"
    );

    let mut remaining = vec![0; FLOOD_BYTES - 1 + usize::from(newline) + 4];
    bounded(stdout.read_exact(&mut remaining), "output did not resume")
        .await
        .unwrap();
    let mut expected = vec![b'x'; FLOOD_BYTES - 1];
    if newline {
        expected.push(b'\n');
    }
    expected.extend_from_slice(b"done");
    assert_eq!(remaining, expected, "output was reordered or corrupted");
    assert_eq!(slow.await.unwrap(), "slow");
    assert_eq!(response(&client, port, "/shutdown").await, "bye");
    assert!(
        bounded(child.wait(), "server did not stop")
            .await
            .unwrap()
            .success()
    );
    let mut stderr = Vec::new();
    bounded(
        child.stderr.take().unwrap().read_to_end(&mut stderr),
        "stderr stayed open",
    )
    .await
    .unwrap();
    assert!(stderr.is_empty(), "{}", String::from_utf8_lossy(&stderr));
}

#[tokio::test]
async fn stalled_write_stdout_yields_and_resumes_in_order() {
    check_sibling_and_order("call write_stdout with flood", false).await;
}

#[tokio::test]
async fn stalled_print_yields_and_resumes_in_order() {
    check_sibling_and_order("call print with flood", true).await;
}

#[tokio::test]
async fn stalled_display_yields_and_resumes_in_order() {
    check_sibling_and_order("display flood", true).await;
}

#[tokio::test]
async fn shutdown_cancels_queued_output_without_waiting_for_the_pipe() {
    let dir = TempDir::new().unwrap();
    let port = common::free_tcp_port();
    let mut child = start(
        &dir,
        &server_source(port, "call write_stdout with flood"),
        "30",
    );
    ready(port).await;
    let client = reqwest::Client::new();
    let slow_client = client.clone();
    let slow = tokio::spawn(async move { response(&slow_client, port, "/slow").await });
    let mut stdout = child.stdout.take().unwrap();
    bounded(stdout.read_exact(&mut [0]), "handler never wrote stdout")
        .await
        .unwrap();
    let queued_client = client.clone();
    let queued = tokio::spawn(async move { response(&queued_client, port, "/queued").await });
    bounded(
        async {
            while !dir.path().join("queued.txt").exists() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        },
        "queued handler could not run while stdout was stalled",
    )
    .await;
    assert_eq!(response(&client, port, "/shutdown").await, "bye");
    assert!(
        bounded(child.wait(), "shutdown waited for stalled stdout")
            .await
            .unwrap()
            .success()
    );
    slow.abort();
    queued.abort();
    let mut tail = Vec::new();
    bounded(
        stdout.read_to_end(&mut tail),
        "stdout remained open after exit",
    )
    .await
    .unwrap();
    assert!(!tail.is_empty());
    assert!(
        tail.iter().all(|byte| *byte == b'x'),
        "cancelled output was written"
    );
}

#[tokio::test]
async fn execution_timeout_interrupts_stalled_stdout() {
    let dir = TempDir::new().unwrap();
    let mut child = start(
        &dir,
        &format!(
            "call write_stdout with \"{}\"\ncreate file at \"late.txt\"\n",
            "x".repeat(FLOOD_BYTES)
        ),
        "1",
    );
    let mut stdout = child.stdout.take().unwrap();
    bounded(stdout.read_exact(&mut [0]), "program never wrote stdout")
        .await
        .unwrap();
    let status = bounded(child.wait(), "execution timeout waited for stalled stdout")
        .await
        .unwrap();
    assert!(!status.success());
    assert!(
        !dir.path().join("late.txt").exists(),
        "execution continued after timeout"
    );
    let mut stderr = String::new();
    bounded(
        child.stderr.take().unwrap().read_to_string(&mut stderr),
        "stderr stayed open",
    )
    .await
    .unwrap();
    assert!(stderr.contains("Timeout"), "{stderr}");
}
