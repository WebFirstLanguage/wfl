# Weave Web Framework for WFL

**Status**: Initial Development (v0.1.0-alpha)

A simple web framework for WFL that provides routing, middleware, and security features for building web applications.

## Project Overview

This implementation provides a foundation for the Weave web framework. During development, we discovered important constraints in the current WFL implementation that affect the framework design.

## Current Implementation Status

### ✅ Completed
- Directory structure created
- Core framework concepts designed
- Router module created (`router.wfl`)
- Response helper functions (`response.wfl`)
- Basic web server functionality validated
- Working hello world example (in progress)

### 🔧 In Progress
- Container-based WeaveApp implementation
- Property access syntax for containers
- Action calling conventions

### ⏳ Planned
- MIME type detection module
- Static file serving
- Middleware system (CORS, security headers, rate limiting)
- Advanced routing (parameters, wildcards)
- Complete documentation

## Key Discoveries

### WFL Web Server Capabilities

**What Works:**
- `listen on port <number> as <server_name>` - Start web server
- `wait for request comes in on <server> as <request>` - Receive requests
- `respond to <request> with <content>` - Send responses
- Request variables auto-defined as **globals** after `wait for request`:
  - `method` - HTTP method (GET, POST, etc.)
  - `path` - Request path
  - `client_ip` - Client IP address
  - `body` - Request body
  - `headers` - Request headers

**Example:**
```wfl
listen on port 3000 as web_server
wait for request comes in on web_server as req

// These are now available as global variables:
display "Method: " with method
display "Path: " with path
display "Client IP: " with client_ip

respond to req with "Hello, World!"
```

### Reserved Keywords

Many web-related keywords are reserved in WFL and cannot be used as variable/property names:
- `port`, `server`, `request`, `response`, `status`
- `method`, `path`, `content`, `header`
- Use alternatives: `server_port`, `req_path`, `http_method`, etc.

See `Docs/reference/keyword-reference.md` for the complete list of 178 reserved keywords.

### Action Syntax

```wfl
// Define action with parameters
define action called my_action with parameters param1 and param2:
    display param1 with " " with param2
end action

// Call action
call my_action with "Hello" and "World"
```

### Container Syntax

```wfl
// Define container
create container MyContainer:
    property my_property: Text
end

// Create instance
create new MyContainer as instance:
    my_property is "value"
end

// Access property (syntax needs verification)
store value as my_property of instance
```

## Architecture

### Simplified Architecture (Current)

```
HTTP Request
    ↓
WFL listen on port
    ↓
wait for request (auto-defines method, path, client_ip, body, headers)
    ↓
Route Matching (manual conditionals)
    ↓
Handler Execution
    ↓
respond to request
```

### Planned Full Architecture

```
HTTP Request
    ↓
WeaveApp
    ↓
Middleware Pipeline
    ├─→ Request Logging
    ├─→ Rate Limiting
    ├─→ CORS Headers
    ├─→ Security Headers
    └─→ Request Validation
    ↓
Router
    ├─→ Static File Handler
    └─→ Route Matcher
    ↓
Route Handler
    ↓
Response
```

## Files Created

### Core Framework
- `weave_lib/weave.wfl` - Main WeaveApp container (needs container property access fix)
- `weave_lib/router.wfl` - Route matching and parameter extraction
- `weave_lib/response.wfl` - Response helper functions (HTML, JSON, 404 pages)

### Examples
- `weave_lib/examples/01_hello_world.wfl` - Container-based example
- `weave_lib/examples/hello_world_working.wfl` - Procedural working example (in progress)

### Tests
- `weave_lib/test_basic_server.wfl` - Container and web server test
- `weave_lib/test_simple_server.wfl` - Web server only test
- `weave_lib/test_request_vars.wfl` - Request variable test
- `weave_lib/test_request_simple.wfl` - **✅ WORKING** Simple request test

## Working Test Example

The file `test_request_simple.wfl` demonstrates a fully working WFL web server:

```wfl
listen on port 3004 as test_server

wait for request comes in on test_server as req

// Access auto-defined globals
display "Method: " with method
display "Path: " with path
display "Client IP: " with client_ip

// Send response
respond to req with "Hello from WFL!"
```

**Test result**: ✅ Successfully serves HTTP requests

## Next Steps

### Immediate (To Complete MVP)
1. Fix container property access syntax
2. Complete working hello world example with routing
3. Test action calling with multiple parameters
4. Validate container instantiation and property mutation

### Short Term
1. Implement MIME type detection
2. Create static file serving module
3. Build middleware system
4. Add security features (CORS, headers, rate limiting)

### Documentation
1. Complete API reference
2. Write tutorial guide
3. Create example applications
4. Add to main WFL documentation

## Design Philosophy

1. **Simplicity First** - Easy to understand, minimal boilerplate
2. **Natural Language** - Follow WFL's readable syntax
3. **Security by Default** - CORS and security headers enabled automatically
4. **Pure WFL** - No Rust interpreter changes required
5. **Progressive Enhancement** - Start simple, add features as needed

## Known Limitations

### Current WFL Constraints
- Container property access syntax unclear/not working
- No custom response headers support in `respond to` statement
- Property mutation in containers may have issues
- Action parameter syntax requires `and` separators, not commas

### Future Features
- Query parameter parsing
- Cookie handling
- Session management
- Request body parsing (JSON, form data)
- File upload handling
- WebSocket support

## Example Usage (Target API)

```wfl
// Load framework
load module from "weave_lib/weave.wfl"

// Create app
create new WeaveApp as app:
    server_port is 3000
    enable_cors is yes
    enable_security_headers is yes
end

// Register routes
app.get("/", "Hello, World!")
app.get("/about", "About page")
app.get("/api/status", "{\"status\": \"running\"}")

// Start server
app.start()
```

## Contributing

This is an active development project. Key areas needing attention:

1. **Container Property Access** - Need to determine correct WFL syntax
2. **Testing** - More test cases for actions, containers, and web features
3. **Documentation** - Complete API docs and tutorials
4. **Examples** - Real-world application examples

## Resources

- **WFL Documentation**: `Docs/` directory
- **Keyword Reference**: `Docs/reference/keyword-reference.md`
- **Web Server Docs**: `Docs/04-advanced-features/web-servers.md`
- **Test Programs**: `TestPrograms/` directory

## License

Part of the WFL project. See main LICENSE file.

## Version History

- **0.1.0-alpha** (2026-01-16) - Initial implementation
  - Core framework structure
  - Router and response modules
  - Working web server tests
  - Documentation foundation

---

**Next Update**: Complete working hello world with routing
