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

**Syntax:**
```wfl
wait for request comes in on <server> as <request_variable>
```

This blocks until a request arrives, then stores the request in the variable.

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

## Request Properties

The request variable contains information about the HTTP request:

### Path

```wfl
wait for request comes in on server as req

check if path is equal to "/":
    respond to req with "Home page"
check if path is equal to "/about":
    respond to req with "About page"
otherwise:
    respond to req with "Not found" and status 404
end check
```

**Note:** Access `path` directly from the request context.

### HTTP Method

```wfl
check if method is "GET":
    respond to req with "GET request"
check if method is "POST":
    respond to req with "POST request"
end check
```

### Headers

Access HTTP headers from requests:

```wfl
store user_agent as header "User-Agent" from req
display "User agent: " with user_agent
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
respond to req with "Hello!" and content type "text/plain"
respond to req with html_content and content type "text/html"
respond to req with json_data and content type "application/json"
```

**Syntax:**
```wfl
respond to <request> with <content> and content type <type>
```

**Common content types:**
- `text/plain` - Plain text
- `text/html` - HTML pages
- `application/json` - JSON data
- `text/css` - CSS stylesheets
- `application/javascript` - JavaScript files

### Combined

```wfl
respond to req with "Created!" and status 201 and content type "application/json"
```

## Routing

Handle different paths with conditionals:

```wfl
listen on port 8080 as server

wait for request comes in on server as req

check if path is equal to "/":
    respond to req with "Home Page"
check if path is equal to "/hello":
    respond to req with "Hello, World!"
check if path is equal to "/about":
    respond to req with "About WFL Server"
otherwise:
    respond to req with "404 - Page Not Found" and status 404
end check
```

### Nested Routing

```wfl
check if path is equal to "/":
    respond to req with "Home"
otherwise:
    check if path starts with "/api/":
        check if path is equal to "/api/status":
            respond to req with "Status: OK"
        check if path is equal to "/api/time":
            store current as current time in milliseconds
            respond to req with "Time: " with current
        otherwise:
            respond to req with "API endpoint not found" and status 404
        end check
    otherwise:
        respond to req with "Page not found" and status 404
    end check
end check
```

## Serving Static Files

Serve files from a directory:

```wfl
listen on port 8080 as server

wait for request comes in on server as req

check if path starts with "/static/":
    // Extract filename: /static/file.html -> file.html
    store filename as substring of path from 8 length 100
    store filepath as "public/" with filename

    check if file exists at filepath:
        try:
            open file at filepath for reading as static_file
            store content as read content from static_file
            close file static_file

            // Determine content type
            check if filepath ends with ".html":
                respond to req with content and content type "text/html"
            check if filepath ends with ".css":
                respond to req with content and content type "text/css"
            check if filepath ends with ".js":
                respond to req with content and content type "application/javascript"
            otherwise:
                respond to req with content and content type "text/plain"
            end check
        catch:
            respond to req with "Error reading file" and status 500
        end try
    otherwise:
        respond to req with "File not found" and status 404
    end check
otherwise:
    respond to req with "Home page"
end check
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
    respond to req with status_json and content type "application/json"
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

respond to req with json_response and content type "application/json"
```

## Error Handling

Always handle errors in web servers:

```wfl
listen on port 8080 as server

wait for request comes in on server as req

try:
    // Process request
    check if path is equal to "/data":
        open file at "data.txt" for reading as file
        store content as read content from file
        close file
        respond to req with content
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

listen on port 8080 as server

wait for request comes in on server as req

add 1 to request_number
display "Request #" with request_number with ": " with method with " " with path

respond to req with "Request logged"
```

## Graceful Shutdown

Handle shutdown signals (if supported):

```wfl
listen on port 8080 as server

register signal handler for "SIGINT" as shutdown_handler

// Request loop
wait for request comes in on server as req
respond to req with "OK"

// Shutdown handler
action shutdown_handler:
    display "Shutting down gracefully..."
    stop accepting connections on server
    close server server
end action
```

## Complete Example: Multi-Route Server

```wfl
display "=== WFL Web Server ==="
display "Starting server on port 8080..."

listen on port 8080 as server

display "Server running at http://127.0.0.1:8080"
display "Press Ctrl+C to stop"
display ""

store request_count as 0

// Main request loop
wait for request comes in on server as req

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
    respond to req with home_html and content type "text/html"

check if path is equal to "/hello":
    respond to req with "Hello from WFL Web Server!" and content type "text/plain"

check if path is equal to "/api/status":
    store status_json as "{
    \"status\": \"running\",
    \"requests_handled\": " with request_count with "
}"
    respond to req with status_json and content type "application/json"

check if path is equal to "/api/time":
    store current as current time in milliseconds
    store time_json as "{\"timestamp\": " with current with "}"
    respond to req with time_json and content type "application/json"

otherwise:
    respond to req with "404 - Not Found" and status 404
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
open url at "http://127.0.0.1:8080/" and read content as response
display "Response: " with response
```

## Common Patterns

### API Endpoint

```wfl
check if path is equal to "/api/users":
    store users_json as "{\"users\": [\"Alice\", \"Bob\", \"Carol\"]}"
    respond to req with users_json and content type "application/json"
end check
```

### Health Check

```wfl
check if path is equal to "/health":
    respond to req with "OK" and content type "text/plain"
end check
```

### Redirect

```wfl
check if path is equal to "/old-page":
    respond to req with "" and status 301 and header "Location: /new-page"
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
- **Blocking:** Server handles requests sequentially
- **No middleware system** (yet) - Implement manually
- **No built-in session management** - Implement yourself
- **No HTTPS** (yet) - HTTP only for now

### Workarounds

**For multiple requests:** Use loops (requires signal handling for shutdown)

```wfl
listen on port 8080 as server

repeat while server is running:
    wait for request comes in on server as req
    // Handle request
    respond to req with "OK"
end repeat
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
