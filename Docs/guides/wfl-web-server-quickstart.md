# WFL Web Server Quick Start Guide

*Get your first WFL web server running in 5 minutes*

WFL makes creating web servers natural and readable with its built-in web capabilities. This guide shows you how to create a simple web server that reads files and serves them as HTTP responses.

## Table of Contents

1. [Your First Web Server](#your-first-web-server)
2. [Serving Static Files](#serving-static-files)
3. [Handling Different HTTP Methods](#handling-different-http-methods)
4. [Error Handling](#error-handling)
5. [Next Steps](#next-steps)

---

## Your First Web Server

Let's start with the simplest possible web server:

```wfl
// Start a web server on port 8080
listen on port 8080 as web_server

display "Server started on port 8080!"
display "Visit http://127.0.0.1:8080 to test it"

// Wait for a request and respond
wait for request comes in on web_server as req
respond to req with "Hello from WFL Web Server!"

display "Server responded to request"
```

**What's happening:**
- `listen on port 8080 as web_server` - Creates a web server listening on port 8080
- `wait for request comes in` - Waits for an HTTP request to arrive
- `respond to req with` - Sends an HTTP response back to the client

Save this as `hello-server.wfl` and run it with:
```bash
wfl hello-server.wfl
```

Visit `http://127.0.0.1:8080` in your browser to see "Hello from WFL Web Server!"

---

## Serving Static Files

Now let's create a server that reads and serves file contents - exactly what was requested in the GitHub issue:

```wfl
// Simple file server that reads and displays file contents
display "=== File Server Demo ==="
display "Starting web server on port 8080..."

// Start the web server
listen on port 8080 as file_server

display "✓ Server started successfully!"
display "Testing file serving..."

// Wait for a request
wait for request comes in on file_server as request

display "Request: " with method with " " with path

// Check the requested path and serve appropriate files
check if path is equal to "/":
    // Serve index.html for the root path
    try:
        open file at "index.html" for reading as html_file
        store html_content as read content from html_file
        close file html_file
        respond to request with html_content and content_type "text/html"
        display "✓ Served index.html"
    catch:
        respond to request with "File not found: index.html" and status 404
        display "❌ index.html not found"
    end try

check if path is equal to "/data.txt":
    // Serve a text file
    try:
        open file at "data.txt" for reading as text_file
        store text_content as read content from text_file
        close file text_file
        respond to request with text_content and content_type "text/plain"
        display "✓ Served data.txt"
    catch:
        respond to request with "File not found: data.txt" and status 404
        display "❌ data.txt not found"
    end try

otherwise:
    // 404 for unknown paths
    store not_found_html as "<!DOCTYPE html>
<html>
<head><title>404 Not Found</title></head>
<body>
    <h1>404 - File Not Found</h1>
    <p>The requested file <code>" with path with "</code> was not found.</p>
    <p><a href=\"/\">← Return to home</a></p>
</body>
</html>"
    respond to request with not_found_html and status 404 and content_type "text/html"
    display "❌ 404 Not Found: " with path
end check

display "Server finished handling request"
```

**Key concepts:**
- **Reading files:** `open file at "filename" for reading` → `read content from file` → `close file`
- **HTTP responses:** `respond to request with content and content_type "text/html"`
- **Path routing:** `check if path is equal to "/"` to handle different URLs
- **Error handling:** `try/catch` blocks for file operations, HTTP status codes

**To test this server:**

1. Create an `index.html` file:
   ```html
   <!DOCTYPE html>
   <html>
   <head><title>WFL File Server</title></head>
   <body>
       <h1>Welcome to WFL File Server!</h1>
       <p>This file was served from disk by WFL.</p>
       <p><a href="/data.txt">View data.txt</a></p>
   </body>
   </html>
   ```

2. Create a `data.txt` file:
   ```
   Hello from WFL!
   This text file was read from disk and served by the web server.
   WFL makes file serving simple and readable.
   ```

3. Run the server: `wfl file-server.wfl`

4. Test the URLs:
   - `http://127.0.0.1:8080/` - Serves the HTML file
   - `http://127.0.0.1:8080/data.txt` - Serves the text file
   - `http://127.0.0.1:8080/missing` - Shows 404 error

---

## Handling Different HTTP Methods

WFL automatically provides access to HTTP request details:

```wfl
listen on port 8080 as api_server

wait for request comes in on api_server as req

display "Received " with method with " request to " with path

check if method is equal to "GET":
    respond to req with "GET request received" and content_type "text/plain"
check if method is equal to "POST":
    respond to req with "POST request received" and content_type "text/plain"
otherwise:
    respond to req with "Method not supported" and status 405
end check
```

**Request properties available:**
- `method` - HTTP method (GET, POST, PUT, DELETE, etc.)
- `path` - URL path (e.g., "/api/users")
- `headers` - HTTP headers
- `body` - Request body (for POST/PUT requests)
- `client_ip` - Client IP address

---

## Error Handling

WFL provides natural error handling for web servers:

```wfl
listen on port 8080 as secure_server

try:
    wait for request comes in on secure_server as req
    
    // Read configuration file
    try:
        open file at "config.json" for reading as config_file
        store config as read content from config_file
        close file config_file
        respond to req with config and content_type "application/json"
    catch:
        respond to req with "Configuration error" and status 500
    end try
    
when server error:
    display "Server encountered an error"
when port unavailable:
    display "Port 8080 is already in use"
otherwise:
    display "Unexpected error: " with error message
end try
```

**Common error scenarios:**
- File not found → HTTP 404
- Permission denied → HTTP 403
- Server errors → HTTP 500
- Port already in use → Server startup failure

---

## Next Steps

Congratulations! You now know how to:
- ✅ Start a basic WFL web server
- ✅ Read files and serve their contents as HTTP responses
- ✅ Handle different URL paths
- ✅ Manage HTTP methods and status codes
- ✅ Implement error handling

### Explore More Advanced Features

- **Static file serving with MIME detection** - See [`TestPrograms/test_static_files.wfl`](../../TestPrograms/test_static_files.wfl)
- **Comprehensive web server** - See [`TestPrograms/comprehensive_web_server_demo.wfl`](../../TestPrograms/comprehensive_web_server_demo.wfl)
- **JSON APIs and POST handling** - Check out examples in [`TestPrograms/`](../../TestPrograms/)
- **Complete web server specification** - Read [SPEC-web-server.md](../wflspecs/SPEC-web-server.md) for all planned features and implementation status

### Learn More About WFL

- **[WFL by Example](wfl-by-example.md)** - Complete language tutorial
- **[WFL Getting Started](wfl-getting-started.md)** - Installation and first steps
- **[File I/O Guide](../wfldocs/WFL-io.md)** - Working with files and I/O
- **[Error Handling](../wfldocs/WFL-errors.md)** - Complete error handling guide

### Community and Support

- **Test Programs** - All examples in [`TestPrograms/`](../../TestPrograms/) are guaranteed to work
- **Documentation** - Complete documentation in [`Docs/`](../../Docs/)
- **Issues** - Report problems or request features on GitHub

---

**Pro tip:** WFL web servers are built on the robust Warp framework with Tokio async runtime, giving you production-ready performance with natural language syntax!