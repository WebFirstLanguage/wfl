# Web Server Implementation Plan for WFL

## Executive Summary

This document outlines a comprehensive plan to implement web server capabilities in WFL (WebFirst Language). The goal is to enable WFL developers to create HTTP servers using natural language syntax consistent with WFL's design philosophy.

## Current State Analysis

### Existing Capabilities
Based on analysis of the WFL codebase, the following relevant features already exist:

1. **HTTP Client Support**
   - `open url at "https://..." and read content as response` syntax for GET requests
   - Async/await support through Tokio runtime (v1.35.1)
   - Reqwest library (v0.11.24) for HTTP client operations

2. **Async Infrastructure**
   - Full Tokio async runtime integration
   - `wait for` (await) keyword for async operations
   - Async action definitions

3. **Main Loop Feature**
   - `main loop:` construct for long-running processes
   - Break conditions for controlled termination
   - Perfect foundation for server event loops

4. **File I/O**
   - Read/write file operations
   - File streaming capabilities
   - Path manipulation

5. **Pattern Matching**
   - Regex-based pattern matching
   - Text parsing capabilities
   - Useful for parsing HTTP headers and URLs

### Missing Components
The following components need to be implemented for web server functionality:

1. **TCP Server Primitives**
   - Socket binding and listening
   - Accept incoming connections
   - Connection management

2. **HTTP Protocol Handling**
   - Request parsing (method, path, headers, body)
   - Response generation (status codes, headers, body)
   - HTTP/1.1 protocol compliance

3. **Request Routing**
   - Path-based routing
   - Method-based routing (GET, POST, etc.)
   - Parameter extraction

4. **Middleware System**
   - Request/response interceptors
   - Authentication/authorization
   - Logging and monitoring

5. **Advanced Features**
   - WebSocket support
   - Server-sent events (SSE)
   - Static file serving
   - Request body parsing (JSON, form data)

## Implementation Phases

### Phase 1: TCP Server Primitives (Foundation)

#### 1.1 New Standard Library Module: `network`
Create `src/stdlib/network.rs` with TCP server capabilities:

```rust
// Key functions to implement:
- listen_on_port(port: u16) -> TcpListener
- accept_connection(listener) -> TcpStream
- read_from_connection(stream) -> String
- write_to_connection(stream, data)
- close_connection(stream)
```

#### 1.2 WFL Syntax Extensions
Add natural language constructs for TCP operations:

```wfl
// Listen on a port
listen on port 8080 as server

// Accept connections
wait for connection on server as client

// Read/write to connections
read request from client as request_data
write response to client
```

#### 1.3 Parser Updates
- Add tokens: `KeywordListen`, `KeywordPort`, `KeywordConnection`
- Add AST nodes: `Expression::Listen`, `Statement::AcceptConnection`
- Update interpreter to handle new operations

### Phase 2: HTTP Request/Response Handling

#### 2.1 HTTP Parser Implementation
Create HTTP request parser that extracts:
- Method (GET, POST, PUT, DELETE, etc.)
- Path and query parameters
- Headers
- Body

#### 2.2 HTTP Response Builder
Implement response construction:
- Status codes (200, 404, 500, etc.)
- Headers (Content-Type, Content-Length, etc.)
- Body encoding

#### 2.3 WFL HTTP Abstractions
Natural language syntax for HTTP concepts:

```wfl
// Parse HTTP request
parse http request from request_data as request

// Access request properties
store method as request's method
store path as request's path
store headers as request's headers
store body as request's body

// Build HTTP response
create http response with status 200 as response
set response's header "Content-Type" to "text/html"
set response's body to "<h1>Hello World</h1>"
```

### Phase 3: Request Routing and Handlers

#### 3.1 Route Definition Syntax
Implement routing with natural language:

```wfl
// Define routes
when request matches "GET /" then:
    perform handle_home with request
end when

when request matches "POST /api/users" then:
    perform handle_create_user with request
end when

when request matches pattern "GET /users/{id}" then:
    store user_id as extract "id" from request
    perform handle_get_user with user_id
end when
```

#### 3.2 Route Matching Engine
- Path pattern matching
- Parameter extraction
- Method filtering
- 404 handling

#### 3.3 Handler Actions
Standard handler pattern:

```wfl
define action handle_home taking request:
    create http response with status 200 as response
    set response's body to read file "static/index.html"
    return response
end action
```

### Phase 4: Complete Web Server Implementation

#### 4.1 Server Container
Implement a reusable server container:

```wfl
container WebServer:
    property port as number
    property routes as list
    
    action start:
        listen on port self's port as server
        display "Server listening on port " with self's port
        
        main loop:
            wait for connection on server as client
            wait for read request from client as request_data
            
            parse http request from request_data as request
            store response as perform route with request
            
            write response to client
            close client
        end loop
    end action
    
    action route taking request:
        // Route matching logic
        for each route in self's routes:
            check if request matches route's pattern:
                return perform route's handler with request
            end check
        end for
        
        // 404 response
        create http response with status 404 as not_found
        set not_found's body to "Page not found"
        return not_found
    end action
end container
```

#### 4.2 Usage Example
Simple web application:

```wfl
// Create server instance
create WebServer with port 3000 as app

// Define routes
add route "GET /" with handler home_page to app's routes
add route "GET /about" with handler about_page to app's routes
add route "POST /api/data" with handler handle_data to app's routes

// Define handlers
define action home_page taking request:
    create http response with status 200 as response
    set response's header "Content-Type" to "text/html"
    set response's body to "<h1>Welcome to WFL Server</h1>"
    return response
end action

define action about_page taking request:
    create http response with status 200 as response
    set response's body to "About our WFL server"
    return response
end action

define action handle_data taking request:
    store data as parse json from request's body
    // Process data...
    create http response with status 201 as response
    set response's body to "Data received"
    return response
end action

// Start server
perform app's start
```

### Phase 5: Advanced Features

#### 5.1 Middleware System
Implement middleware chain:

```wfl
// Logging middleware
define action log_requests taking request and next:
    display "Request: " with request's method with " " with request's path
    store response as perform next with request
    display "Response: " with response's status
    return response
end action

// Authentication middleware
define action require_auth taking request and next:
    check if request's header "Authorization" exists:
        return perform next with request
    otherwise:
        create http response with status 401 as unauthorized
        return unauthorized
    end check
end action

// Apply middleware
app's middleware includes log_requests
app's middleware includes require_auth for "/api/*"
```

#### 5.2 WebSocket Support
Enable real-time communication:

```wfl
when request is websocket upgrade to "/ws":
    accept websocket from client as socket
    
    repeat while socket is open:
        wait for message from socket as msg
        
        // Echo server example
        send msg to socket
    end repeat
end when
```

#### 5.3 Static File Serving
Automatic static file handling:

```wfl
// Serve static files from directory
serve static files from "public" at "/static"

// With caching
serve static files from "public" at "/static" with cache for 3600 seconds
```

## Technical Implementation Details

### Interpreter Extensions

1. **New Value Types**
   - `TcpListener` - Server socket
   - `TcpStream` - Client connection
   - `HttpRequest` - Parsed HTTP request
   - `HttpResponse` - HTTP response object

2. **Async Operations**
   All network operations should be async by default:
   ```rust
   // In interpreter/mod.rs
   Expression::Listen { port, .. } => {
       let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
       Ok(Value::TcpListener(listener))
   }
   ```

3. **Error Handling**
   Network-specific errors:
   - `PortInUse` - Port already bound
   - `ConnectionFailed` - Client disconnected
   - `InvalidHttpRequest` - Malformed HTTP

### Parser Modifications

1. **New Tokens** (in `src/lexer/mod.rs`):
   ```rust
   #[token("listen")] KeywordListen,
   #[token("port")] KeywordPort,
   #[token("connection")] KeywordConnection,
   #[token("route")] KeywordRoute,
   #[token("middleware")] KeywordMiddleware,
   ```

2. **New AST Nodes** (in `src/parser/ast.rs`):
   ```rust
   Listen { port: Expression },
   AcceptConnection { listener: Expression },
   ParseHttpRequest { data: Expression },
   CreateHttpResponse { status: Expression },
   ```

### Standard Library Structure

```
src/stdlib/
├── network/
│   ├── mod.rs          # Main network module
│   ├── tcp.rs          # TCP primitives
│   ├── http.rs         # HTTP parsing/building
│   └── websocket.rs    # WebSocket support
```

## Testing Strategy

### Unit Tests
- TCP connection handling
- HTTP request parsing
- Response generation
- Route matching

### Integration Tests
Create test programs in `TestPrograms/`:
- `webserver_simple.wfl` - Basic server
- `webserver_routing.wfl` - Route testing
- `webserver_middleware.wfl` - Middleware chain
- `webserver_static.wfl` - Static file serving

### Performance Tests
- Concurrent connection handling
- Request throughput
- Memory usage under load

## Migration Path

For existing WFL programs that use HTTP client features:
1. No breaking changes to existing `open url at` syntax
2. Server features are additive only
3. Gradual adoption possible

## Example: Complete Web Application

```wfl
// blog_server.wfl - A simple blog server

// Database setup (using existing WFL database support)
open database at "sqlite://blog.db" as db

// Web server setup
create WebServer with port 8080 as blog

// Homepage route
add route "GET /" with action taking request:
    store posts as perform query "SELECT * FROM posts ORDER BY created DESC LIMIT 10" on db
    
    store html as "<h1>My Blog</h1><ul>"
    for each post in posts:
        store html as html with "<li><a href='/post/" with post's id with "'>" 
        store html as html with post's title with "</a></li>"
    end for
    store html as html with "</ul>"
    
    create http response with status 200 as response
    set response's header "Content-Type" to "text/html"
    set response's body to html
    return response
end action to blog's routes

// Individual post route
add route pattern "GET /post/{id}" with action taking request:
    store post_id as extract "id" from request
    store post as perform query "SELECT * FROM posts WHERE id = ?" with post_id on db
    
    check if post exists:
        create http response with status 200 as response
        set response's body to post's content
        return response
    otherwise:
        create http response with status 404 as response
        set response's body to "Post not found"
        return response
    end check
end action to blog's routes

// Start the server
display "Starting blog server on http://localhost:8080"
perform blog's start
```

## Success Metrics

1. **Functionality**
   - Can create basic HTTP server
   - Handles concurrent connections
   - Supports common HTTP methods
   - Routes requests correctly

2. **Performance**
   - Handle 1000+ requests/second
   - Support 100+ concurrent connections
   - Memory usage < 50MB for simple server

3. **Developer Experience**
   - Natural language syntax consistent with WFL
   - Clear error messages
   - Good documentation and examples

## Timeline Estimate

- **Phase 1 (TCP Primitives):** 2-3 weeks
- **Phase 2 (HTTP Handling):** 2-3 weeks
- **Phase 3 (Routing):** 1-2 weeks
- **Phase 4 (Complete Server):** 2-3 weeks
- **Phase 5 (Advanced Features):** 3-4 weeks

**Total:** 10-15 weeks for full implementation

## Conclusion

Implementing web server capabilities in WFL is achievable by building on existing async infrastructure and following WFL's natural language philosophy. The phased approach allows for incremental development and testing, ensuring stability at each step. The resulting server API will be intuitive for WFL developers while providing the power needed for real web applications.