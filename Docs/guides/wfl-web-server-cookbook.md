# WFL Web Server Cookbook

*Common patterns and solutions for building web servers with WFL*

This cookbook provides ready-to-use patterns for common web server tasks. Each recipe includes working code that you can copy and adapt for your own projects.

## Table of Contents

- [Basic Patterns](#basic-patterns)
- [File Serving Patterns](#file-serving-patterns)
- [API Development Patterns](#api-development-patterns)
- [Error Handling Patterns](#error-handling-patterns)
- [Security and Validation](#security-and-validation)
- [Performance Tips](#performance-tips)

---

## Basic Patterns

### Simple HTTP Server
*Start a basic web server and respond to requests*

```wfl
listen on port 8080 as server

wait for request comes in on server as req
respond to req with "Hello, World!"
```

### Multi-Route Server
*Handle different URL paths*

```wfl
listen on port 8080 as server

wait for request comes in on server as req

check if path is equal to "/":
    respond to req with "Home page"
check if path is equal to "/about":
    respond to req with "About page"
check if path is equal to "/contact":
    respond to req with "Contact page"
otherwise:
    respond to req with "Page not found" and status 404
end check
```

### Method-Based Routing
*Handle different HTTP methods*

```wfl
listen on port 8080 as server

wait for request comes in on server as req

check if method is equal to "GET" and path is equal to "/users":
    respond to req with "List of users"
check if method is equal to "POST" and path is equal to "/users":
    respond to req with "Create new user"
check if method is equal to "PUT" and path is equal to "/users":
    respond to req with "Update user"
check if method is equal to "DELETE" and path is equal to "/users":
    respond to req with "Delete user"
otherwise:
    respond to req with "Method not allowed" and status 405
end check
```

---

## File Serving Patterns

### Basic File Server
*Read a file and serve its contents*

```wfl
listen on port 8080 as server

wait for request comes in on server as req

try:
    open file at "content.txt" for reading as file
    store content as read content from file
    close file
    respond to req with content and content_type "text/plain"
catch:
    respond to req with "File not found" and status 404
end try
```

### Static File Server with MIME Types
*Serve different file types with correct content types*

```wfl
listen on port 8080 as server

wait for request comes in on server as req

// Remove leading slash for file path
store file_path as substring of path from 1
check if file_path is equal to "":
    store file_path as "index.html"
end check

try:
    open file at file_path for reading as file
    store content as read content from file
    close file
    
    // Set content type based on file extension
    check if file_path ends with ".html":
        respond to req with content and content_type "text/html"
    check if file_path ends with ".css":
        respond to req with content and content_type "text/css"
    check if file_path ends with ".js":
        respond to req with content and content_type "text/javascript"
    check if file_path ends with ".json":
        respond to req with content and content_type "application/json"
    otherwise:
        respond to req with content and content_type "text/plain"
    end check
    
catch:
    respond to req with "File not found" and status 404
end try
```

### Directory-Based File Serving
*Serve files from a specific directory*

```wfl
listen on port 8080 as server
store public_dir as "public"

wait for request comes in on server as req

// Build safe file path
store requested_file as substring of path from 1
check if requested_file is equal to "":
    store requested_file as "index.html"
end check

store full_path as public_dir with "/" with requested_file

try:
    open file at full_path for reading as file
    store content as read content from file
    close file
    respond to req with content and content_type "text/html"
catch:
    respond to req with "File not found" and status 404
end try
```

---

## API Development Patterns

### JSON API Response
*Return structured JSON data*

```wfl
listen on port 8080 as server

wait for request comes in on server as req

check if path is equal to "/api/status":
    store response_json as "{
  \"status\": \"OK\",
  \"timestamp\": \"" with current time with "\",
  \"server\": \"WFL Web Server\"
}"
    respond to req with response_json and content_type "application/json"
end check
```

### POST Data Handler
*Process incoming POST data*

```wfl
listen on port 8080 as server

wait for request comes in on server as req

check if method is equal to "POST" and path is equal to "/api/data":
    // Process the POST body
    store received_data as body
    
    // Log the received data (in real app, you might save to file/database)
    display "Received POST data: " with received_data
    
    // Send confirmation response
    store response as "{\"message\": \"Data received successfully\"}"
    respond to req with response and content_type "application/json"
    
otherwise:
    respond to req with "{\"error\": \"Not found\"}" and status 404
end check
```

### REST API Pattern
*Complete RESTful API for a resource*

```wfl
listen on port 8080 as api_server

wait for request comes in on api_server as req

// Users API endpoints
check if path starts with "/api/users":
    check if method is equal to "GET" and path is equal to "/api/users":
        // GET /api/users - List all users
        store users_json as "[{\"id\": 1, \"name\": \"Alice\"}, {\"id\": 2, \"name\": \"Bob\"}]"
        respond to req with users_json and content_type "application/json"
        
    check if method is equal to "POST" and path is equal to "/api/users":
        // POST /api/users - Create new user
        store new_user as "{\"id\": 3, \"name\": \"New User\", \"created\": \"" with current time with "\"}"
        respond to req with new_user and content_type "application/json"
        
    check if method is equal to "GET" and path starts with "/api/users/":
        // GET /api/users/:id - Get specific user
        store user_id as substring of path from 11  // Extract ID from path
        store user_json as "{\"id\": " with user_id with ", \"name\": \"User " with user_id with "\"}"
        respond to req with user_json and content_type "application/json"
        
    otherwise:
        respond to req with "{\"error\": \"Method not allowed\"}" and status 405
    end check
    
otherwise:
    respond to req with "{\"error\": \"API endpoint not found\"}" and status 404
end check
```

---

## Error Handling Patterns

### Graceful Error Handling
*Handle errors without crashing the server*

```wfl
listen on port 8080 as server

wait for request comes in on server as req

try:
    // Attempt to serve a file
    store file_name as substring of path from 1
    check if file_name is equal to "":
        store file_name as "index.html"
    end check
    
    open file at file_name for reading as file
    store content as read content from file
    close file
    respond to req with content and content_type "text/html"
    
when file not found:
    respond to req with "The requested file was not found" and status 404
when permission denied:
    respond to req with "Access to file is denied" and status 403
when error:
    respond to req with "Internal server error occurred" and status 500
    display "Unexpected error: " with error message
end try
```

### Custom Error Pages
*Serve custom HTML error pages*

```wfl
define action called send error page:
    parameter request as Request
    parameter status_code as Number
    parameter error_title as Text
    parameter error_message as Text
    
    store error_html as "<!DOCTYPE html>
<html>
<head>
    <title>" with error_title with "</title>
    <style>
        body { font-family: Arial; margin: 40px; text-align: center; }
        .error { color: #d32f2f; }
        .message { color: #666; margin: 20px 0; }
    </style>
</head>
<body>
    <h1 class=\"error\">" with status_code with " " with error_title with "</h1>
    <p class=\"message\">" with error_message with "</p>
    <p><a href=\"/\">← Return to home page</a></p>
</body>
</html>"
    
    respond to request with error_html and status status_code and content_type "text/html"
end action

// Usage:
listen on port 8080 as server

wait for request comes in on server as req

check if path is equal to "/missing":
    send error page with req and 404 and "Not Found" and "The page you requested does not exist."
otherwise:
    respond to req with "Welcome to the home page!"
end check
```

---

## Security and Validation

### Input Validation
*Validate request data before processing*

```wfl
listen on port 8080 as server

wait for request comes in on server as req

check if method is equal to "POST" and path is equal to "/api/validate":
    // Validate that body is not empty
    check if length of body is equal to 0:
        respond to req with "{\"error\": \"Request body is required\"}" and status 400
        return
    end check
    
    // Validate maximum body size (example: 1MB limit)
    check if length of body is greater than 1048576:
        respond to req with "{\"error\": \"Request body too large\"}" and status 413
        return
    end check
    
    // Process valid request
    respond to req with "{\"message\": \"Data is valid\"}" and content_type "application/json"
end check
```

### Path Security
*Prevent directory traversal attacks*

```wfl
define action called is safe path:
    parameter requested_path as Text
    
    // Check for directory traversal attempts
    check if requested_path contains "..":
        return no
    end check
    
    check if requested_path contains "~":
        return no
    end check
    
    // Only allow alphanumeric, hyphens, underscores, and slashes
    // (In a real implementation, you'd use pattern matching)
    return yes
end action

listen on port 8080 as server

wait for request comes in on server as req

store requested_file as substring of path from 1
check if requested_file is equal to "":
    store requested_file as "index.html"
end check

check if is safe path with requested_file:
    // Safe to serve file
    try:
        open file at "public/" with requested_file for reading as file
        store content as read content from file
        close file
        respond to req with content
    catch:
        respond to req with "File not found" and status 404
    end try
else:
    respond to req with "Invalid file path" and status 400
end check
```

---

## Performance Tips

### Request Logging
*Log requests for monitoring and debugging*

```wfl
listen on port 8080 as server
store request_count as 0

wait for request comes in on server as req

// Increment request counter
store request_count as request_count plus 1

// Log request details
store log_entry as "[" with current time with "] Request #" with request_count with ": " with method with " " with path with " from " with client_ip
display log_entry

// You could also write to a log file:
try:
    append log_entry with "\n" to file "access.log"
catch:
    display "Warning: Could not write to log file"
end try

// Process request normally
respond to req with "Request processed successfully"
```

### Response Caching Headers
*Set appropriate cache headers for static content*

```wfl
listen on port 8080 as server

wait for request comes in on server as req

check if path ends with ".css" or path ends with ".js":
    // Static assets - cache for 1 hour
    try:
        store file_path as substring of path from 1
        open file at file_path for reading as file
        store content as read content from file
        close file
        
        respond to req with content and content_type "text/css" and header "Cache-Control" as "public, max-age=3600"
    catch:
        respond to req with "File not found" and status 404
    end try
else:
    // Dynamic content - no cache
    respond to req with "Dynamic content" and header "Cache-Control" as "no-cache"
end check
```

---

## Complete Example: Blog Server

Here's a complete example that combines many patterns:

```wfl
// Blog server demonstrating multiple patterns
display "=== WFL Blog Server ==="
display "Starting blog server with multiple features..."

listen on port 8080 as blog_server
store post_count as 0

display "✓ Blog server started on port 8080"
display "Available endpoints:"
display "  GET  / - Blog home page"
display "  GET  /api/posts - List all posts (JSON)"
display "  POST /api/posts - Create new post"
display "  GET  /static/* - Static files"

wait for request comes in on blog_server as req

// Log request
display "[" with current time with "] " with method with " " with path with " from " with client_ip

// Route handling
check if path is equal to "/":
    // Serve blog home page
    store home_html as "<!DOCTYPE html>
<html>
<head>
    <title>WFL Blog</title>
    <style>
        body { font-family: Arial; margin: 40px; }
        .post { border: 1px solid #ddd; padding: 20px; margin: 20px 0; }
        .header { color: #333; border-bottom: 2px solid #007acc; }
    </style>
</head>
<body>
    <h1 class=\"header\">WFL Blog Server</h1>
    <p>Welcome to the blog server built with WebFirst Language!</p>
    <div class=\"post\">
        <h2>Sample Post</h2>
        <p>This is an example blog post served by WFL.</p>
        <p><em>Posted on " with current time with "</em></p>
    </div>
    <p><a href=\"/api/posts\">View posts as JSON</a></p>
</body>
</html>"
    respond to req with home_html and content_type "text/html"

check if path is equal to "/api/posts" and method is equal to "GET":
    // List posts as JSON
    store posts_json as "[
  {\"id\": 1, \"title\": \"Welcome to WFL\", \"content\": \"First post!\"},
  {\"id\": 2, \"title\": \"Web Servers in WFL\", \"content\": \"Easy to create!\"}
]"
    respond to req with posts_json and content_type "application/json"

check if path is equal to "/api/posts" and method is equal to "POST":
    // Create new post
    store post_count as post_count plus 1
    store new_post as "{
  \"id\": " with post_count with ",
  \"title\": \"New Post\",
  \"content\": \"" with body with "\",
  \"created\": \"" with current time with "\"
}"
    respond to req with new_post and content_type "application/json"

check if path starts with "/static/":
    // Serve static files
    store file_name as substring of path from 8  // Remove "/static/"
    try:
        open file at "static/" with file_name for reading as static_file
        store file_content as read content from static_file
        close file static_file
        respond to req with file_content
    catch:
        respond to req with "Static file not found" and status 404
    end try

otherwise:
    // 404 error with custom page
    store error_html as "<!DOCTYPE html>
<html>
<head><title>404 - Not Found</title></head>
<body>
    <h1>Page Not Found</h1>
    <p>The page <code>" with path with "</code> was not found.</p>
    <p><a href=\"/\">← Return to blog home</a></p>
</body>
</html>"
    respond to req with error_html and status 404 and content_type "text/html"
end check

display "Blog server request processed successfully"
```

---

## Next Steps

- **[Web Server Quick Start](wfl-web-server-quickstart.md)** - 5-minute tutorial
- **[Web Server Examples](../examples/web-servers/)** - Organized examples by complexity  
- **[WFL by Example](wfl-by-example.md)** - Complete language tutorial
- **[SPEC-web-server.md](../wflspecs/SPEC-web-server.md)** - Complete feature documentation

Happy web development with WFL! 🌐