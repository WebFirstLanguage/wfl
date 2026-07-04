# Web Servers

WFL has built-in web server capabilities—no frameworks required! Create HTTP servers using natural language syntax.

## Why Built-in Web Servers?

Traditional languages require frameworks:
- **Node.js:** Requires Express, Koa, or Fastify
- **Python:** Requires Flask, Django, or FastAPI
- **Ruby:** Requires Rails or Sinatra

**WFL:** Web servers are **built-in**. Just use natural language commands.

## The Simplest Web Server

Create a file called `server.wfl`:

```wfl
listen on port 8080 as web_server

wait for request comes in on web_server as req
respond to req with "Hello from WFL!"
```

Run it:
```bash
wfl server.wfl
```

Visit `http://127.0.0.1:8080` in your browser.

**That's it!** A working web server in 3 lines.

## Basic Concepts

### Starting a Server

Use `listen on port` to start listening for HTTP requests:

```wfl
listen on port 8080 as web_server
```

**Syntax:**
```wfl
listen on port <port_number> as <server_variable>
```

This creates a server that listens on the specified port.

### Waiting for Requests

Use `wait for request` to accept incoming HTTP requests:

```wfl
wait for request comes in on web_server as req
```

This can also be written with an optional `that` for readability:
```wfl
wait for request that comes in on web_server as req
```

**Syntax:**
```wfl
wait for request [that] comes in on <server> as <request_variable>
```

This blocks until a request arrives, then stores the request in the variable. Both "comes in" and "that comes in" are supported.

### Responding to Requests

Use `respond to` to send HTTP responses:

```wfl
respond to req with "Hello, World!"
```

**Syntax:**
```wfl
respond to <request> with <content>
```

Sends a 200 OK response with the specified content.


## Configuring Network Binding

By default, WFL web servers bind to **localhost (127.0.0.1)**, which means they only accept connections from the same machine. This is secure by default.

To expose your server on a network, configure the binding address in `.wflcfg`:

### Example: Development (Default - Localhost Only)

Create `.wflcfg` in your project directory:

```ini
# Bind only to localhost (secure)
web_server_bind_address = 127.0.0.1
```

Visit: `http://127.0.0.1:8080`

### Example: Network Deployment

For Docker, Kubernetes, or multi-machine setups:

```ini
# Bind to all interfaces
web_server_bind_address = 0.0.0.0
```

Now accessible from other machines on the network.

### Example: Specific Network Interface

For servers with multiple network cards:

```ini
# Bind to internal network interface
web_server_bind_address = 192.168.1.100
```

### Configuration Precedence

1. **Local project config:** `.wflcfg` in script directory (highest priority)
2. **Global system config:** `/etc/wfl/wfl.cfg` or Windows equivalent
3. **Default:** `127.0.0.1` if not specified

### Security Considerations

| Binding | Use Case | Security |
|---------|----------|----------|
| `127.0.0.1` | Local development | Maximum - local only |
| `0.0.0.0` | Public deployment, Docker | Network-wide - needs firewall |
| Specific IP | Trusted network | Medium - requires network trust |

## Request Properties

The request variable contains information about the HTTP request:

### Path

```wfl
listen on port 8080 as web_server
wait for request comes in on web_server as req

check if path is equal to "/":
    respond to req with "Home page"
otherwise:
    check if path is equal to "/about":
        respond to req with "About page"
    otherwise:
        respond to req with "Not found" and status 404
    end check
end check
```

**Note:** Once a request has arrived, access `path` directly from the request context.

### HTTP Method

```wfl
listen on port 8080 as web_server
wait for request comes in on web_server as req

check if method is equal to "GET":
    respond to req with "GET request"
otherwise:
    check if method is equal to "POST":
        respond to req with "POST request"
    otherwise:
        respond to req with "Method Not Allowed" and status 405
    end check
end check
```

### Headers

Access HTTP headers from requests:

```wfl
listen on port 8080 as web_server
wait for request comes in on web_server as req

store user_agent as header "User-Agent" of req
display "User agent: " with user_agent
respond to req with "ok"
```

### Request Body

The request body is available two ways:

- `body` — the body decoded as text (lossy UTF-8). Use this for form posts,
  JSON, and other text payloads.
- `body_bytes` — the **raw bytes** of the body, preserved exactly. Use this for
  binary uploads (file uploads, images, etc.); it can be written straight to
  disk with `write binary` or echoed back with `respond to`.

```wfl
wait for request comes in on web_server as req

open file at "uploads/received.bin" for writing binary as out_file
write binary body_bytes into out_file
close file out_file

respond to req with "Uploaded"
```

## Response Options

### With Status Code

```wfl
respond to req with "Created!" and status 201
respond to req with "Not found" and status 404
respond to req with "Server error" and status 500
```

**Syntax:**
```wfl
respond to <request> with <content> and status <code>
```

**Common status codes:**
- 200 - OK (default)
- 201 - Created
- 204 - No Content
- 400 - Bad Request
- 404 - Not Found
- 500 - Internal Server Error

### With Content Type

```wfl
respond to req with "Hello!" and content_type "text/plain"
respond to req with html_content and content_type "text/html"
respond to req with json_data and content_type "application/json"
```

**Syntax:**
```wfl
respond to <request> with <content> and content_type <type>
```

**Common content types:**
- `text/plain` - Plain text
- `text/html` - HTML pages
- `application/json` - JSON data
- `text/css` - CSS stylesheets
- `application/javascript` - JavaScript files

### Serving Binary Files (Fonts, Images, etc.)

`respond to` carries **binary** content losslessly, so you can serve fonts,
images, favicons, PDFs, and other non-text assets straight from disk. Read the
file with `read binary` (see [File I/O](file-io.md#binary-files)) and respond
with the resulting bytes:

```wfl
open file at "public/fonts/Alegreya-Regular.ttf" for reading binary as f
store font_bytes as read binary from f
close file f

respond to req with font_bytes and content_type "font/ttf"
```

If you omit `content_type` for binary content, it defaults to
`application/octet-stream` (rather than `text/plain`).

To pick the content type automatically from a file name, use the `mime_type`
helper — handy for a static-file route:

```wfl
store asset_path as "public/fonts/Alegreya-Regular.ttf"
open file at asset_path for reading binary as f
store asset_bytes as read binary from f
close file f

respond to req with asset_bytes and content_type (mime_type of asset_path)
```

`mime_type of <name>` maps a file name or path to a content type by its
extension (`.ttf`→`font/ttf`, `.woff2`→`font/woff2`, `.png`→`image/png`,
`.svg`→`image/svg+xml`, `.ico`→`image/x-icon`, `.css`, `.js`, `.json`, …),
falling back to `application/octet-stream` for unknown extensions.

> **Note:** `data` is a reserved keyword — name the variable holding the bytes
> something else (e.g. `font_bytes`, `payload`).

### With Custom Headers

Set extra response headers by passing a map to `and headers`. This mirrors the
outbound client's `with headers` clause — the same "headers are a map" idea,
nothing new to learn.

```wfl
create map extra_headers:
    "Cache-Control" is "no-store"
    "X-Request-Id" is "abc-123"
end map

respond to req with json_data and content_type "application/json" and headers extra_headers
```

**Syntax:**
```wfl
respond to <request> with <content> and headers <map>
```

**Notes:**
- The map keys are header names; values are text (numbers and booleans are
  accepted and converted to text).
- The `content_type` clause remains authoritative for `Content-Type`. Keys the
  response pipeline computes itself — `Content-Type`, `Content-Length`, and
  `Transfer-Encoding` — are ignored if present in the map, so the response never
  carries duplicate or conflicting copies of them.

### Combined

```wfl
respond to req with "Created!" and status 201 and content_type "application/json" and headers extra_headers
```

All optional clauses (`status`, `content_type`, `headers`) can appear in any
order after the content.

## The QUERY Method (RFC 10008)

WFL supports [RFC 10008](https://www.rfc-editor.org/info/rfc10008/), the HTTP
`QUERY` method. `QUERY` is a *safe* and *idempotent* request that carries a
body — it fills the gap between `GET` (safe/idempotent but no body) and `POST`
(has a body but is neither), which is ideal for complex read-only lookups whose
parameters are too large or sensitive for the URL.

Because WFL treats HTTP methods as plain text, no special syntax is required.

**Sending a QUERY (client):**

Per RFC 10008 a `QUERY` request must include a `Content-Type` describing the
query body, so set it in the request headers map:

```wfl
create map request_headers:
    "Content-Type" is "application/jsonpath"
end map

open url at "https://api.example.com/search" with method "QUERY" and headers request_headers and body "$.items[*]" and read response as result
```

**Handling a QUERY (server):**

A `QUERY` request arrives like any other; dispatch on the method and use the
`headers` clause to advertise `Accept-Query` (which query formats the endpoint
accepts) and, optionally, `Content-Location` to point at a resource holding the
results.

```wfl
create map query_headers:
    "Accept-Query" is "application/jsonpath, application/sql"
    "Content-Location" is "/data/results/latest"
end map

listen on port 8080 as web_server
wait for request comes in on web_server as req

store results_json as "{\"items\": [1, 2, 3]}"

check if method is equal to "QUERY":
    respond to req with results_json and content_type "application/json" and headers query_headers
otherwise:
    respond to req with "Method Not Allowed" and status 405
end check
```

See `TestPrograms/rfc10008_query_server.wfl` for a complete example.

## Routing

Handle different paths with conditionals:

```wfl
listen on port 8080 as web_server

wait for request comes in on web_server as req

check if path is equal to "/":
    respond to req with "Home Page"
otherwise:
    check if path is equal to "/hello":
        respond to req with "Hello, World!"
    otherwise:
        check if path is equal to "/about":
            respond to req with "About WFL Server"
        otherwise:
            respond to req with "404 - Page Not Found" and status 404
        end check
    end check
end check
```

### Nested Routing

```wfl
listen on port 8080 as web_server

main loop:
    wait for request comes in on web_server as req
    store req_path as path

    check if req_path is equal to "/":
        respond to req with "Home"
    otherwise:
        check if req_path starts with "/api/":
            check if req_path is equal to "/api/status":
                respond to req with "Status: OK"
            otherwise:
                check if req_path is equal to "/api/time":
                    store now_ms as current time in milliseconds
                    respond to req with "Time: " with now_ms
                otherwise:
                    respond to req with "API endpoint not found" and status 404
                end check
            end check
        otherwise:
            respond to req with "Page not found" and status 404
        end check
    end check
end loop
```

### Route Parameters

Extract values from path segments with `path_params`. The template marks
captured segments with `:name`; the result is an object of captures, or
`nothing` when the path does not match:

```wfl
store params as path_params of path and "/users/:id"
check if params is nothing:
    respond to req with "Not Found" and status 404
otherwise:
    store user_id as params["id"]
    store user_text as "User " with user_id
    respond to req with user_text
end check
```

Template rules:

- `:name` captures exactly one path segment (`/users/:id` matches `/users/42`
  but not `/users` or `/users/42/extra`).
- A trailing `*name` captures the rest of the path
  (`/static/*filepath` matches `/static/css/main.css` with
  `filepath` = `"css/main.css"`).
- Other segments must match literally.
- Captures are percent-decoded (`/users/John%20Doe` captures `"John Doe"`),
  and any query string on the path is ignored.

Multiple parameters work as expected:

```wfl
store params as path_params of path and "/users/:user_id/posts/:post_id"
check if params is not nothing:
    store post_text as "Post " with params["post_id"] with " by user " with params["user_id"]
    respond to req with post_text
end check
```

Use `path_matches` when you only need a yes/no answer:

```wfl
check if path_matches of path and "/users/:id":
    respond to req with "looks like a user page"
end check
```

**Security note:** captured values come straight from the request (after
percent-decoding) and can contain anything, including `..` or path separators.
If you use a capture to build a filesystem path — common with `*filepath`
templates — validate it first: reject values containing `..` and confirm the
final path stays inside the directory you intend to serve.

## Serving Static Files

Serve files from a directory:

```wfl
listen on port 8080 as web_server

main loop:
    wait for request comes in on web_server as req
    store req_path as path

    check if req_path starts with "/static/":
        // Validate the request path BEFORE building a filesystem path, so a
        // crafted request like /static/../secret cannot escape the public dir.
        check if req_path contains "..":
            respond to req with "Forbidden" and status 403
        otherwise:
            check if req_path contains "\\":
                respond to req with "Forbidden" and status 403
            otherwise:
                // Safe to build the path: extract /static/file.html -> file.html
                store filename as substring of req_path from 8 length 100
                store filepath as "public/" with filename

                check if file exists at filepath:
                    try:
                        open file at filepath for reading as static_file
                        store file_body as read content from static_file
                        close file static_file

                        // Determine content type
                        check if filepath ends with ".html":
                            respond to req with file_body and content_type "text/html"
                        otherwise:
                            check if filepath ends with ".css":
                                respond to req with file_body and content_type "text/css"
                            otherwise:
                                check if filepath ends with ".js":
                                    respond to req with file_body and content_type "application/javascript"
                                otherwise:
                                    respond to req with file_body and content_type "text/plain"
                                end check
                            end check
                        end check
                    catch:
                        respond to req with "Error reading file" and status 500
                    end try
                otherwise:
                    respond to req with "File not found" and status 404
                end check
            end check
        end check
    otherwise:
        respond to req with "Home page"
    end check
end loop
```

## Serving Dynamic WFL Pages

Execute another WFL file on each request and send its output to the browser —
like PHP, but in WFL. The `execute file` statement runs a `.wfl` file
in-process, passes it the current request, and captures everything the file
displays:

```wfl
listen on port 8080 as web_server

main loop:
    wait for request comes in on web_server as req

    try:
        execute wfl file at "pages/home.wfl" with req and read output as page_output
        respond to req with page_output and content_type "text/html"
    when file not found:
        respond to req with "Page not found" and status 404
    when error:
        respond to req with "Server error" and status 500
    end try
end loop
```

The page file `pages/home.wfl` is a normal WFL program. Because the request
was passed along with `with req`, the page sees the same request variables a
server sees: `method`, `path`, `client_ip`, `body` and `headers`:

```wfl
display "<h1>Welcome!</h1>"
display "<p>You asked for " with path with " using " with method with "</p>"
```

Everything the page displays is captured into `page_output` instead of being
printed, ready to send to the browser.

### The execute file statement

```wfl
execute [wfl] file at <path> [with <request>] [and read output as <variable>]
```

- The word `wfl` is optional — `execute file at "page.wfl"` works the same.
- `with <request>` is optional. Without it the page runs with no request
  context.
- `and read output as <variable>` is optional. Without it the page's output is
  displayed normally instead of captured.
- The path is resolved relative to the directory of the WFL file doing the
  executing, just like `load module`.
- The page runs in its own fresh environment with the full standard library —
  it cannot see or change the server's variables.
- Errors inside the page (a missing file, parse errors, runtime errors) become
  catchable errors in the server, so one broken page cannot crash the server.
- Pages can execute other pages (for layouts or partials), up to 4 levels deep.
- A `with` directly after the path always passes request context. To execute a
  dynamically chosen page, build the path into a variable first:

```wfl
store page_path as "pages/" with page_name with ".wfl"
execute wfl file at page_path with req and read output as page_output
```

## JSON Responses

Build JSON responses for APIs:

```wfl
listen on port 8080 as api_server

wait for request comes in on api_server as req

check if path is equal to "/api/status":
    store status_json as "{
    \"status\": \"running\",
    \"server\": \"WFL Web Server\",
    \"version\": \"1.0.0\"
}"
    respond to req with status_json and content_type "application/json"
end check
```

**With variables:**
```wfl
store request_count as 42
store uptime as 3600000  // milliseconds

store json_response as "{
    \"requests\": " with request_count with ",
    \"uptime\": " with uptime with "
}"

respond to req with json_response and content_type "application/json"
```

## Error Handling

Always handle errors in web servers:

```wfl
listen on port 8080 as web_server

wait for request comes in on web_server as req

try:
    // Process request
    check if path is equal to "/data":
        open file at "data.txt" for reading as data_file
        store file_body as read content from data_file
        close file data_file
        respond to req with file_body
    otherwise:
        respond to req with "Home"
    end check
catch:
    respond to req with "Internal server error" and status 500
end try
```

## Request Logging

Log each request for debugging:

```wfl
store request_number as 0

listen on port 8080 as web_server

wait for request comes in on web_server as req

add 1 to request_number
display "Request #" with request_number with ": " with method with " " with path

respond to req with "Request logged"
```

## Graceful Shutdown

Handle shutdown signals (if supported):

```wfl
listen on port 8080 as web_server

register signal handler for SIGINT as shutdown_handler

// Request loop
wait for request comes in on web_server as req
respond to req with "OK"

// Shutdown handler
define action called shutdown_handler:
    display "Shutting down gracefully..."
    stop accepting connections on web_server
    close server web_server
end action
```

## Complete Example: Multi-Route Server

```wfl
display "=== WFL Web Server ==="
display "Starting server on port 8080..."

listen on port 8080 as web_server

display "Server running at http://127.0.0.1:8080"
display "Press Ctrl+C to stop"
display ""

store request_count as 0

// Main request loop
wait for request comes in on web_server as req

add 1 to request_count
display "Request #" with request_count with ": " with method with " " with path

// Routing
check if path is equal to "/":
    store home_html as "<!DOCTYPE html>
<html>
<head><title>WFL Server</title></head>
<body>
    <h1>Welcome to WFL Web Server!</h1>
    <p>A web server written in natural language.</p>
    <ul>
        <li><a href=\"/hello\">Hello</a></li>
        <li><a href=\"/api/status\">Status API</a></li>
        <li><a href=\"/api/time\">Time API</a></li>
    </ul>
</body>
</html>"
    respond to req with home_html and content_type "text/html"

otherwise:
    check if path is equal to "/hello":
        respond to req with "Hello from WFL Web Server!" and content_type "text/plain"

    otherwise:
        check if path is equal to "/api/status":
            store status_json as "{
    \"status\": \"running\",
    \"requests_handled\": " with request_count with "
}"
            respond to req with status_json and content_type "application/json"

        otherwise:
            check if path is equal to "/api/time":
                store now_ms as current time in milliseconds
                store time_json as "{\"timestamp\": " with now_ms with "}"
                respond to req with time_json and content_type "application/json"

            otherwise:
                respond to req with "404 - Not Found" and status 404
            end check
        end check
    end check
end check
```

## Testing Your Server

### With a Browser

1. Start your server: `wfl server.wfl`
2. Open browser: `http://127.0.0.1:8080`
3. Try different paths: `/hello`, `/api/status`

### With curl

```bash
# GET request
curl http://127.0.0.1:8080/

# GET with specific path
curl http://127.0.0.1:8080/api/status

# POST request
curl -X POST -d "data" http://127.0.0.1:8080/api/echo
```

### Programmatically

```wfl
// In another WFL program
open url at "http://127.0.0.1:8080/" and read content as api_response
display "Response: " with api_response
```

## HTTPS / TLS

WFL's web server can terminate TLS itself — no reverse proxy required. Add `secured` to a `listen` statement:

```wfl
listen on port 8443 secured with certificate "cert.pem" and key "key.pem" as secure_server
```

The certificate and private key are PEM files. Both values are ordinary expressions, so variables work too:

```wfl
store cert_file as "/etc/wfl/tls/cert.pem"
store key_file as "/etc/wfl/tls/key.pem"
listen on port 8443 secured with certificate cert_file and key key_file as secure_server
```

Everything else — `wait for request`, `respond to` — works exactly as for plain HTTP.

### Certificate paths from configuration

For deployments where certificate locations differ per machine, leave the paths out of the code and use the bare `secured` form:

```wfl
listen on port 8443 secured as secure_server
```

Then set the defaults in `.wflcfg`:

```ini
web_server_tls_cert_file = /etc/wfl/tls/cert.pem
web_server_tls_key_file = /etc/wfl/tls/key.pem
```

Paths written in the `listen` statement always win over the configuration file. A plain `listen` (without `secured`) always serves HTTP, no matter what the configuration contains — adding certificate paths to `.wflcfg` never silently converts an HTTP server to HTTPS.

If a server is marked `secured` but no certificate can be found — in the statement or the configuration — the program stops at startup with an error explaining both options.

### Redirecting HTTP to HTTPS

To send visitors who arrive over plain HTTP to your secure server, start a redirect server:

```wfl
listen on port 8443 secured with certificate "cert.pem" and key "key.pem" as secure_server
listen on port 8080 redirecting to port 8443 as redirect_server
```

The redirect server answers **every** request natively with `301 Moved Permanently`, pointing at the same host, path, and query string on the HTTPS port (the port is omitted from the `Location` URL when the target is 443). Requests to a redirect server never reach your `wait for request` loop — you don't write any handler code for it. `close server redirect_server` shuts it down like any other server.

### Serving HTTP and HTTPS side by side

If you'd rather serve different content on HTTP (instead of redirecting), just start two servers — each `listen` creates an independent server:

```wfl
listen on port 8080 as http_server
listen on port 8443 secured with certificate "cert.pem" and key "key.pem" as secure_server

wait for request comes in on http_server as plain_request
respond to plain_request with "This is the plain HTTP page"

wait for request comes in on secure_server as tls_request
respond to tls_request with "This is the secure page"
```

Note that `wait for request` listens to one named server at a time, and responses are handled sequentially (see Limitations below). TLS handshakes themselves are concurrent.

### Certificates for local development

Create a self-signed certificate with openssl:

```bash
openssl req -x509 -newkey rsa:2048 -nodes -keyout key.pem -out cert.pem -days 365 -subj "/CN=localhost"
```

Or, for a certificate your browser trusts without warnings, use [mkcert](https://github.com/FiloSottile/mkcert):

```bash
mkcert -install
mkcert localhost 127.0.0.1
```

Test with curl (`-k` skips certificate validation for self-signed certs):

```bash
curl -k https://localhost:8443/
```

⚠️ Self-signed certificates are for development only. In production, use a certificate from a real authority (e.g. Let's Encrypt via certbot).

### Production notes

- The private key file must be readable by the WFL process — protect it with file permissions (`chmod 600 key.pem`).
- Certificate and key files are validated at `listen` time; a missing or malformed file is reported with the offending path before the server starts.
- Terminating TLS at a reverse proxy (Caddy, nginx) or CDN in front of a plain-HTTP WFL server remains a perfectly good deployment option — use whichever fits your infrastructure.
- Remember to set `web_server_bind_address = 0.0.0.0` in `.wflcfg` if the server must be reachable from other machines.

## Common Patterns

### API Endpoint

```wfl
check if path is equal to "/api/users":
    store users_json as "{\"users\": [\"Alice\", \"Bob\", \"Carol\"]}"
    respond to req with users_json and content_type "application/json"
end check
```

### Health Check

```wfl
check if path is equal to "/health":
    respond to req with "OK" and content_type "text/plain"
end check
```

### Redirect

```wfl
create map redirect_headers:
    "Location" is "/new-page"
end map

check if path is equal to "/old-page":
    respond to req with "" and status 301 and headers redirect_headers
end check
```

## Best Practices

✅ **Always handle 404s** - Provide a default "not found" response

✅ **Use proper content types** - Helps browsers render correctly

✅ **Log requests** - Makes debugging easier

✅ **Handle errors** - Use try-catch for file operations

✅ **Set appropriate status codes** - 200, 404, 500, etc.

✅ **Validate input** - Check paths and data before processing

❌ **Don't expose sensitive data** - Validate paths to prevent directory traversal

❌ **Don't forget error handling** - Servers should never crash

❌ **Don't serve without validation** - Check file exists before reading

## Limitations & Notes

### Current Limitations

- **Single request handling:** Each `wait for request` handles one request
- **Blocking:** Server handles requests sequentially (TLS handshakes are concurrent, but your responses are serialized)
- **No middleware system** (yet) - Implement manually
- **No built-in session management** - Implement yourself

### Workarounds

**For multiple requests:** Use loops (requires signal handling for shutdown)

```wfl
listen on port 8080 as web_server

main loop:
    wait for request comes in on web_server as req
    // Handle request
    respond to req with "OK"
end loop
```

## Security Considerations

⚠️ **Important:** Web servers expose your application to the internet. Always:

1. **Validate paths** - Prevent directory traversal
2. **Sanitize input** - Validate all user data
3. **Use proper status codes** - Don't leak error details
4. **Limit file access** - Only serve from specific directories
5. **Set timeouts** - Prevent slow clients from hanging
6. **Log securely** - Don't log sensitive data

**[See Security Guidelines →](../06-best-practices/security-guidelines.md)** *(coming soon)*

## What You've Learned

In this section, you learned:

✅ **Starting servers** - `listen on port`
✅ **HTTPS** - `listen on port ... secured with certificate ... and key ...`
✅ **HTTP→HTTPS redirects** - `listen on port ... redirecting to port ...`
✅ **Accepting requests** - `wait for request`
✅ **Sending responses** - `respond to`
✅ **Routing** - Using conditionals to handle different paths
✅ **Status codes** - 200, 404, 500, etc.
✅ **Content types** - text/plain, text/html, application/json
✅ **Static files** - Serving files from disk
✅ **Error handling** - Try-catch for robust servers
✅ **Request logging** - Tracking requests

## Next Steps

Expand your web development skills:

**[File I/O →](file-io.md)**
Learn to read and write files for data persistence.

**[Async Programming →](async-programming.md)**
Handle multiple operations concurrently.

**[Pattern Matching →](pattern-matching.md)**
Validate request data and extract parameters.

**[Error Handling →](../03-language-basics/error-handling.md)**
Review error handling for robust servers.

---

**Previous:** [← Advanced Features](index.md) | **Next:** [File I/O →](file-io.md)
