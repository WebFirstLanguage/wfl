# Web Examples

Validated WFL web-server programs.

- `html_server.wfl` — serves a styled HTML welcome page on port 8080
  (moved from `Webserver_test/` by the repository hygiene migration).
- `blog_server.wfl` — minimal JSON API server on port 3001 with routed
  endpoints (`/`, `/api/posts`, `/api/posts/1`), a 404 fallback, and a
  request cap so it terminates after 20 requests.

Run one with `wfl examples/web/html_server.wfl` and open the printed URL.
Examples are non-gating; the asserted web-server coverage lives in
`TestPrograms/` and the web test scripts.
