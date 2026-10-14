//! End-to-end regression test for issue #466.
//!
//! Before the fix, `src/main.rs` built the interpreter with
//! `Interpreter::with_timeout(config.timeout_seconds)`, which copied *only* the
//! timeout out of the loaded `.wflcfg` and reset every other field to its
//! default. The most visible symptom was that `web_server_bind_address` was
//! silently ignored: `listen on port N` always bound `127.0.0.1` regardless of
//! the config file, making WFL web servers loopback-only in any deployment that
//! relied on `.wflcfg` to change the bind address.
//!
//! These tests drive the *actual compiled binary* against a real `.wflcfg` and
//! assert that the configured bind address reaches the listening socket. They
//! would have failed before the switch to `Interpreter::with_config(...)`.
//!
//! Socket inspection uses `/proc/net/tcp`, so the assertions run on Linux only;
//! on other platforms the harness still boots the binary but skips the
//! address check (the interpreter-level behavior is covered portably in
//! `web_server_bind_address_test.rs`).

use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

/// Write a `.wflcfg` with the given bind address plus a tiny server program
/// into a fresh temp directory, then launch the compiled `wfl` binary on it.
/// The child listens on port 0 and publishes the bound address before any
/// client connects.
fn spawn_server(dir: &std::path::Path, bind_address: &str) -> Child {
    std::fs::write(
        dir.join(".wflcfg"),
        format!("web_server_bind_address = {bind_address}\n"),
    )
    .expect("write .wflcfg");

    let program = "listen on port 0 as s\n\
         display \"BIND_CLI_READY \" with s\n\
         wait for request comes in on s as r with timeout 4000\n\
         respond to r with \"hi\"\n\
         close server s\n";
    let script = dir.join("server.wfl");
    std::fs::write(&script, program).expect("write server.wfl");
    let log = std::fs::File::create(dir.join("process.log")).expect("create process log");

    Command::new(env!("CARGO_BIN_EXE_wfl"))
        .arg(&script)
        .current_dir(dir)
        .stdout(Stdio::from(log.try_clone().expect("clone log")))
        .stderr(Stdio::from(log))
        .spawn()
        .expect("spawn wfl binary")
}

fn wait_for_published_addr(dir: &std::path::Path, child: &mut Child) -> std::net::SocketAddr {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait().expect("inspect server process") {
            panic!(
                "bind-address server exited before publishing its port ({status}): {}",
                std::fs::read_to_string(dir.join("process.log")).unwrap_or_default()
            );
        }
        let log = std::fs::read_to_string(dir.join("process.log")).unwrap_or_default();
        if let Some(address) = crate::common::published_web_server_addr(&log, "BIND_CLI_READY ") {
            assert_ne!(address.port(), 0, "server must report its assigned port");
            return address;
        }
        assert!(
            Instant::now() < deadline,
            "server did not publish BIND_CLI_READY: {log}"
        );
        sleep(Duration::from_millis(50));
    }
}

/// Create a unique temp directory without pulling in extra dev-dependencies.
fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wfl_bind_cli_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

/// Read the hex local-address of the listening (state 0A) socket bound to
/// `port` from `/proc/net/tcp`. Returns the `HHHHHHHH` local-address field
/// (little-endian IPv4), e.g. `00000000` for 0.0.0.0 or `0100007F` for
/// 127.0.0.1.
#[cfg(target_os = "linux")]
fn listening_local_addr(port: u16) -> Option<String> {
    let hex_port = format!("{port:04X}");
    let contents = std::fs::read_to_string("/proc/net/tcp").ok()?;
    for line in contents.lines().skip(1) {
        let mut cols = line.split_whitespace();
        let _sl = cols.next()?; // leading slot index, e.g. "3:"
        let local = cols.next()?; // e.g. "0100007F:20A9"
        let _remote = cols.next()?;
        let state = cols.next()?; // "0A" == LISTEN
        if state == "0A"
            && let Some((addr, prt)) = local.split_once(':')
            && prt.eq_ignore_ascii_case(&hex_port)
        {
            return Some(addr.to_uppercase());
        }
    }
    None
}

fn kill(mut child: Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Poll `/proc/net/tcp` until the server is listening on `port` (or we give up).
/// Startup latency varies a lot under a parallel `cargo test` run, so a fixed
/// sleep is unreliable; poll for up to ~4s instead.
#[cfg(target_os = "linux")]
fn wait_for_listen(port: u16) -> Option<String> {
    for _ in 0..40 {
        if let Some(addr) = listening_local_addr(port) {
            return Some(addr);
        }
        sleep(Duration::from_millis(100));
    }
    None
}

#[test]
#[cfg(target_os = "linux")]
fn wflcfg_bind_address_reaches_listening_socket() {
    // Loopback default: `.wflcfg` says 127.0.0.1 -> socket bound to 127.0.0.1.
    {
        let dir = temp_dir("loopback");
        let mut child = spawn_server(&dir, "127.0.0.1");
        let published = wait_for_published_addr(&dir, &mut child);
        let addr = wait_for_listen(published.port());
        kill(child);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(
            addr.as_deref(),
            Some("0100007F"),
            "server with `web_server_bind_address = 127.0.0.1` should bind loopback"
        );
    }

    // The regression itself: `.wflcfg` says 0.0.0.0 -> socket must bind all
    // interfaces (00000000). Before issue #466 was fixed this bound 0100007F
    // because the config never reached the interpreter.
    {
        let dir = temp_dir("allifaces");
        let mut child = spawn_server(&dir, "0.0.0.0");
        let published = wait_for_published_addr(&dir, &mut child);
        let addr = wait_for_listen(published.port());
        kill(child);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(
            addr.as_deref(),
            Some("00000000"),
            "server with `web_server_bind_address = 0.0.0.0` must bind all interfaces \
             (regression for issue #466: loaded .wflcfg was being discarded)"
        );
    }
}

/// Portable smoke test: the binary honors a `.wflcfg` and comes up listening on
/// loopback so a client can reach it. Runs everywhere; the address-level
/// distinction above is Linux-only.
#[test]
fn wflcfg_server_is_reachable() {
    let dir = temp_dir("reachable");
    let mut child = spawn_server(&dir, "0.0.0.0");
    let published = wait_for_published_addr(&dir, &mut child);
    let target = std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, published.port()));

    // The child already owns `published`. Read the body so a colliding mock
    // cannot satisfy readiness with a bare TCP accept.
    let mut ok = false;
    for _ in 0..40 {
        if let Ok(mut stream) =
            std::net::TcpStream::connect_timeout(&target, Duration::from_millis(300))
        {
            let _ = stream.write_all(b"GET / HTTP/1.0\r\n\r\n");
            let mut buf = [0u8; 128];
            if let Ok(n) = stream.read(&mut buf)
                && std::str::from_utf8(&buf[..n]).is_ok_and(|body| body.contains("hi"))
            {
                ok = true;
                break;
            }
        }
        sleep(Duration::from_millis(100));
    }

    kill(child);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        ok,
        "server launched via the compiled binary with a .wflcfg should be reachable on loopback"
    );
}
