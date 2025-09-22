# WFL Web Server Implementation Plan

## Overview

This document outlines the comprehensive web server implementation for the WebFirst Language (WFL). The implementation follows Test-Driven Development (TDD) principles and demonstrates WFL's natural language syntax for web development.

## Current Status

### ✅ Completed (TDD Phase 1)

1. **Parser Bug Fix**: Fixed critical bug in `parse_respond_statement()` where `and content_type` was not parsed correctly due to expression parser consuming the `and` token as a binary operator.

2. **Comprehensive Test Suite**: Created failing tests that demonstrate all required functionality:
   - `TestPrograms/web_server_request_response_test.wfl` - Basic request/response handling
   - `TestPrograms/web_server_graceful_shutdown_test.wfl` - Graceful shutdown with signal handling
   - `TestPrograms/web_server_comprehensive_test.wfl` - Advanced HTTP features
   - `TestPrograms/web_server_middleware_test.wfl` - Middleware and logging
   - `TestPrograms/comprehensive_web_server_demo.wfl` - Complete demonstration

3. **Parser Enhancements**: 
   - Fixed `respond` statement parsing to handle `and content_type` and `and status` correctly
   - Used `parse_primary_expression()` instead of `parse_expression()` to avoid consuming keywords

### 🔄 In Progress (TDD Phase 2)

4. **Interpreter Implementation**: Currently returning placeholder errors. Need to implement:
   - `WaitForRequestStatement` - Async request handling
   - `RespondStatement` - HTTP response sending
   - Request object properties (method, path, client_ip, body, headers)
   - Error handling with `error_message` variable

## Required Features

### Core HTTP Server Functionality

1. **HTTP Server Setup**
   - ✅ `listen on port X as server_name` - Basic server creation
   - ❌ Proper warp-based HTTP server with request routing
   - ❌ Async request handling with Tokio runtime

2. **Request Handling**
   - ❌ `wait for request comes in on server as request_name`
   - ❌ Request object with properties:
     - `method of request` - HTTP method (GET, POST, PUT, DELETE)
     - `path of request` - URL path
     - `client_ip of request` - Client IP address
     - `body of request` - Request body content
     - `headers of request` - HTTP headers

3. **Response Handling**
   - ❌ `respond to request with content and content_type "type"`
   - ❌ `respond to request with content and status 404`
   - ❌ `respond to request with content and content_type "type" and status 201`
   - ❌ Proper HTTP status codes and headers

### Advanced Features

4. **Multiple HTTP Methods**
   - ❌ GET request handling
   - ❌ POST request handling with body parsing
   - ❌ PUT request handling
   - ❌ DELETE request handling
   - ❌ Method validation and 405 responses

5. **Static File Serving**
   - ❌ File existence checking
   - ❌ MIME type detection based on file extension
   - ❌ Proper file reading and serving
   - ❌ 404 handling for missing files

6. **JSON Support**
   - ❌ JSON request body parsing
   - ❌ JSON response generation
   - ❌ Proper Content-Type headers

### Graceful Shutdown Features

7. **Signal Handling**
   - ❌ SIGINT (Ctrl+C) handling
   - ❌ SIGTERM handling
   - ❌ `register signal handler for SIGINT as handler_name`

8. **Connection Management**
   - ❌ Active connection tracking
   - ❌ `stop accepting connections on server`
   - ❌ Connection draining with timeout
   - ❌ `close server server_name`

9. **Resource Cleanup**
   - ❌ File handle cleanup
   - ❌ Memory cleanup
   - ❌ Logging of shutdown process

### Middleware and Logging

10. **Request Logging**
    - ❌ Timestamp generation
    - ❌ Request method, path, IP logging
    - ❌ Response time measurement
    - ❌ Access log file writing

11. **Error Handling**
    - ❌ `error_message` variable in catch blocks
    - ❌ Proper error response generation
    - ❌ 500 Internal Server Error handling

## Implementation Architecture

### Current Warp Integration

The current `ListenStatement` implementation uses warp but only creates a basic "Hello World" server:

```rust
let routes = warp::path::end().map(|| "Hello from WFL Web Server!");
let server_task = warp::serve(routes).try_bind_ephemeral(([127, 0, 0, 1], port_num));
```

### Required Architecture Changes

1. **Request Queue System**: Need async channel for request/response communication
2. **Server State Management**: Track active connections and server state
3. **Request Object Creation**: Create WFL Value objects with request properties
4. **Response Channel**: Async response sending mechanism

### Proposed Implementation

```rust
// Pseudo-code for new implementation
struct WflWebServer {
    request_sender: mpsc::Sender<WflRequest>,
    response_receivers: HashMap<RequestId, oneshot::Receiver<WflResponse>>,
    active_connections: Arc<AtomicUsize>,
    shutdown_signal: Arc<AtomicBool>,
}

struct WflRequest {
    id: RequestId,
    method: String,
    path: String,
    client_ip: String,
    body: String,
    headers: HashMap<String, String>,
    response_sender: oneshot::Sender<WflResponse>,
}

struct WflResponse {
    content: String,
    status: u16,
    content_type: String,
    headers: HashMap<String, String>,
}
```

## Natural Language Syntax Examples

The implementation showcases WFL's natural language approach:

```wfl
// Server setup
listen on port 8080 as web_server

// Request handling
wait for request comes in on web_server as incoming_request

// Request properties
store method as method of incoming_request
store path as path of incoming_request
store client_ip as client_ip of incoming_request

// Response sending
respond to incoming_request with "Hello World" and content_type "text/plain"
respond to incoming_request with json_data and content_type "application/json" and status 201

// Graceful shutdown
register signal handler for SIGINT as shutdown_handler
stop accepting connections on web_server
close server web_server
```

## Testing Strategy

### TDD Approach

1. **Phase 1**: ✅ Write comprehensive failing tests
2. **Phase 2**: 🔄 Implement minimal functionality to pass tests
3. **Phase 3**: ❌ Refactor and optimize
4. **Phase 4**: ❌ Add advanced features

### Test Coverage

- ✅ Basic server startup and shutdown
- ✅ Request/response handling
- ✅ Multiple HTTP methods
- ✅ Static file serving
- ✅ JSON request/response
- ✅ Error handling
- ✅ Graceful shutdown
- ✅ Middleware functionality

## Next Steps

1. **Implement Basic Request/Response**: Start with simple request queue and response mechanism
2. **Add Request Properties**: Implement method, path, client_ip, body extraction
3. **Implement Response Sending**: Add proper HTTP response generation
4. **Add Static File Serving**: Implement file reading and MIME type detection
5. **Add Graceful Shutdown**: Implement signal handling and connection management
6. **Add Middleware Features**: Implement logging and error handling
7. **Performance Optimization**: Optimize for concurrent requests
8. **Documentation**: Update user documentation with examples

## Success Criteria

The implementation will be considered complete when:

1. All test programs in `TestPrograms/web_server_*.wfl` pass
2. The comprehensive demo (`comprehensive_web_server_demo.wfl`) runs successfully
3. All HTTP methods are supported
4. Static file serving works with proper MIME types
5. Graceful shutdown works with signal handling
6. Request logging and middleware functionality is operational
7. Error handling provides meaningful error messages
8. Performance is acceptable for typical web server workloads

This implementation will serve as a flagship example of WFL's capabilities in web development, demonstrating how natural language programming can be applied to complex, real-world scenarios while maintaining professional-grade functionality.
