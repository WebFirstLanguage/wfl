# 2026-10-09 — Integration-suite already-safe HTTP/web ports (issue #693)

Ninth batch of the single-binary integration pattern: eight HTTP/web
files that already pick OS-assigned ports (`common::free_tcp_port`,
`listen on port 0`, or `TcpListener::bind("127.0.0.1:0")`) move into
`tests/suite/`. Three drop `mod common;` for `crate::common::…`.

`http_request_runtime_test.rs` mentions `127.0.0.1:1` only as a client
URL for an invalid-method error path; it never binds that port.

Fixed-port files stay top-level for a follow-up rewrite to
`free_tcp_port` (`web_server_tls_test`, `request_of_action_body_test`,
`header_access_runtime_test`, `respond_headers_test`,
`web_server_binary_test`, `web_server_bind_address_test`,
`web_server_content_length_test`, `web_server_query_and_request_access_test`,
and Group A names such as `stream_handle_type_test`).

Also updates the stale path comment in `src/parser/tests.rs` that still
pointed at `tests/display_multiple_values_stdout_test.rs` after #770.

Hibiki reproduced a probe-then-rebind race (`free_tcp_port` / local
`:0` probes) against `http_request_runtime_test`'s mock. The four
probing tests now `listen on port 0` and read the published
`WebServer::ip:port` line from the child (the `trusted_proxy_test`
pattern) before connecting. `free_tcp_port` stays for standalone
Group A binaries and documents that it is unsafe in the suite process.

A later 4-process stress also hit a 1s setup timeout in
`http_outbound_budget_test::legacy_post_body_read_observes_cooperative_cancellation`
(request never reached the stalled peer under CPU contention). That
wait is now 10s; the cancellation assertion is unchanged.
