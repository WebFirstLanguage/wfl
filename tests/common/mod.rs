//! Shared helpers for integration tests.
#![allow(dead_code)]

use std::net::TcpListener;

/// Ask the OS for a currently-free TCP port on loopback, then release it so the
/// caller can bind it via WFL's `listen on port <N>`.
///
/// WFL takes a *literal* port in `listen on port <N>`, so the port must be chosen
/// before the program source is built — we cannot bind an ephemeral `:0` and read
/// the assigned port back the way the mock upstreams do. Picking a free port from
/// the OS (instead of a hardcoded constant) avoids collisions under parallel test
/// runs and on busy runners. A small TOCTOU window remains between releasing the
/// probe socket and WFL re-binding it, but it is far less flaky than a fixed port.
pub fn free_tcp_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind an ephemeral TCP port")
        .local_addr()
        .expect("read the ephemeral local address")
        .port()
}
