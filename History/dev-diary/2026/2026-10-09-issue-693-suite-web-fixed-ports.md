# 2026-10-09 — Integration-suite fixed-port web files (issue #693)

Tenth batch: the eight remaining HTTP/web files that interpolated a
hardcoded `listen on port N` move into `tests/suite/` after a rewrite
to `listen on port 0` plus the shared `published_web_server_addr`
parser from #771. None of them use `free_tcp_port`.

In-process servers write `PREFIXWebServer::ip:port` to a unique ready
file after listen, then the harness reads that address before any
client connects. Single-request fixtures do not send a separate HTTP
probe (it would consume the only `wait for request`); the test request
itself asserts the expected body or headers. Multi-request TLS cases
confirm the published server with the body the fixture is supposed to
serve.

`request_of_action_body_test` never binds — `listen on port 8080` was
only source text for analyzer/typecheck/`--analyze`. Those snippets now
say `listen on port 0`.

`test_tls_listener_reports_occupied_port_without_panicking` still
targets a specific port, but it *holds* a `TcpListener::bind(:0)` for
the whole attempt and interpolates that live port. That is not a
probe-then-rebind.

`test_redirect_server_returns_301_with_location` listens on port 0 and
uses 8443 only as the Location target (nothing binds it).

The parser now accepts IPv6 handles (`WebServer:::1:port`) so
`web_server_bind_address_test` can publish `::1`.
