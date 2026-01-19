# WFL Web Server Examples

This directory contains examples of WFL web servers with different levels of complexity.

## Files

### server.wfl
The original simple web server that serves a single HTML page for all requests. Good starting point for understanding basic WFL web server concepts.

**Run it:**
```bash
wfl Webserver_test/server.wfl
```

### minimal_routing_demo.wfl ⭐ **Recommended**
A complete working example demonstrating **module-based routing** - the core concept of loading different `.wfl` files per route. This is the practical, production-ready approach for WFL routing.

**Features:**
- Routes GET / to `routes/home.wfl`
- Routes GET /api/status to `routes/api_status.wfl`
- Routes POST /api/echo to `routes/api_echo.wfl`
- 404 handling for unknown routes
- 405 handling for wrong HTTP methods
- REQUEST_* context variables passed to route modules

**Run it:**
```bash
wfl Webserver_test/minimal_routing_demo.wfl
```

**Test it:**
```bash
# Home page
curl http://localhost:8080/

# API status (JSON)
curl http://localhost:8080/api/status

# Echo endpoint
curl -X POST http://localhost:8080/api/echo -d "Hello WFL"

# 404 test
curl http://localhost:8080/nonexistent
```

### routes/
Directory containing route modules loaded by `minimal_routing_demo.wfl`:

- **home.wfl** - Home page handler with HTML response
- **api_status.wfl** - JSON API status endpoint
- **api_echo.wfl** - Echo endpoint that returns POST body

Each route module has access to these variables:
- `REQUEST_METHOD` - HTTP method (GET, POST, etc.)
- `REQUEST_PATH` - Request path
- `REQUEST_BODY` - Request body content
- `REQUEST_HEADERS` - Request headers
- `REQUEST_CLIENT_IP` - Client IP address
- `REQUEST_OBJECT` - WFL request object for responding

### ROUTING_GUIDE.md
Comprehensive guide explaining:
- Core routing concepts
- How to implement module-based routing
- Route module patterns
- Advanced patterns (dynamic routes, static files)
- Best practices and examples
- Complete project structure recommendations

## Quick Start

1. **Run the minimal routing demo:**
   ```bash
   cd G:\Logbie\wfl
   wfl Webserver_test/minimal_routing_demo.wfl
   ```

2. **Access the routes:**
   - http://localhost:8080/ - Home page
   - http://localhost:8080/api/status - Server status (JSON)
   - POST to http://localhost:8080/api/echo - Echo endpoint

3. **Study the routing pattern:**
   - Read `ROUTING_GUIDE.md` for comprehensive documentation
   - Examine `minimal_routing_demo.wfl` for routing logic
   - Look at `routes/*.wfl` for module examples

## Creating Your Own Routed Server

1. Create main server file with routing logic (see `minimal_routing_demo.wfl`)
2. Create `routes/` directory
3. Add route module files (e.g., `routes/my_route.wfl`)
4. In each route module:
   - Access REQUEST_* variables
   - Process the request
   - Respond using `respond to REQUEST_OBJECT`

## Why Module-Based Routing?

**Benefits:**
- **Clean separation** - Each route in its own file
- **Easy to maintain** - Find route handlers quickly
- **Modular** - Add/remove routes without touching main server
- **Scalable** - Organize routes by feature or API version
- **Testable** - Test route modules independently

**Example structure:**
```
my_server/
├── server.wfl           # Main routing logic
└── routes/
    ├── home.wfl         # GET /
    ├── about.wfl        # GET /about
    ├── api_status.wfl   # GET /api/status
    ├── api_users.wfl    # GET /api/users
    └── api_echo.wfl     # POST /api/echo
```

## Next Steps

- Read `ROUTING_GUIDE.md` for detailed patterns
- Modify `routes/home.wfl` to customize the home page
- Add new routes by creating new modules and updating the routing logic
- Implement static file serving (see ROUTING_GUIDE.md)
- Add parameter extraction for dynamic routes (see ROUTING_GUIDE.md)

## Notes

The minimal routing demo demonstrates WFL's module loading capability for clean, organized web server routing. While WFL doesn't have a built-in routing framework, this pattern provides a practical, maintainable approach for real applications.
