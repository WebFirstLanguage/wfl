# Nexus Framework

A lightweight, semantic server framework for WFL (WebFirst Language) with a simple Server -> Router architecture.

> **Note**: This is a design/reference implementation that demonstrates the Server -> Router architecture concept. The main.wfl entry point parses correctly and shows the intended usage pattern. The module files (Server.wfl, Router.wfl, handlers.wfl) serve as detailed design documentation for the framework architecture, though they use some WFL reserved keywords (like `status`, `content`, `request`, `response`, `method`, `path`, `body`) that would need to be renamed for full runtime execution.

## Architecture

Nexus follows a strict **Server -> Router** architecture:

```
Request → Server (The "Bouncer")
              ↓
          Router (The "Traffic Cop")
              ↓
          Resolution Algorithm:
              1. Static File Bypass (check /public first)
              2. Dynamic Route Lookup (check registered routes)
              3. 404 Fallback (nothing matched)
              ↓
          Response → Client
```

## Quick Start

```bash
wfl examples/nexus/main.wfl
```

Then visit `http://127.0.0.1:8080` in your browser.

## Project Structure

```
examples/nexus/
├── core/                    # Core framework components
│   ├── Server.wfl          # Connection handling (the "Bouncer")
│   └── Router.wfl          # Request resolution (the "Traffic Cop")
├── public/                  # Static files (the "File Sink")
│   ├── index.html          # Default homepage
│   └── styles.css          # Default styles
├── src/                     # Application logic
│   └── handlers.wfl        # Dynamic route handlers (the "Talent")
├── main.wfl                # Entry point
└── README.md               # This file
```

## Core Concepts

### The Server (The "Bouncer")

The Server module handles connection management only. It does not know about business logic. All incoming requests are immediately passed to `Router.resolve()` for processing.

```wfl
// Create and start a server
store server as create_server with 8080 and "127.0.0.1" and "public"
server.start
server.run with router
```

### The Router (The "Traffic Cop")

The Router implements a "hierarchy of laziness" - it tries to do the easiest thing first:

1. **Static File Bypass**: Check if a file exists at `web_root + path` or `web_root + path + .html`
2. **Dynamic Route Lookup**: Check registered routes for a match
3. **404 Fallback**: Return a 404 response if nothing matched

```wfl
// Create a router with web root directory
store router as create_router with "public"

// Register dynamic routes
router.register with "GET" and "/api/hello" and "api_handler"
router.register with "POST" and "/api/users" and "user_handler"
```

### Handlers (The "Talent")

Handlers are simple containers that implement a `handle` action:

```wfl
create container MyHandler:
    property handler_name: Text
    
    action handle needs request_method: Text, request_path: Text, request_body: Text:
        // Process the request and return a Response
        create new Response as resp
        resp.initialize with "Hello, World!" and 200 and "text/plain"
        give back resp
    end action
end container
```

## Configuration

Configuration is passed when creating the server:

| Parameter | Description | Default |
|-----------|-------------|---------|
| `port_number` | Port to listen on | 8080 |
| `host_address` | Host to bind to | 127.0.0.1 |
| `web_root` | Directory for static files | public |

## Static File Serving

Place files in the `/public` directory. The router will automatically serve them:

- `/` → `public/index.html`
- `/about` → `public/about.html` (pretty URLs)
- `/styles.css` → `public/styles.css`
- `/images/logo.png` → `public/images/logo.png`

Supported MIME types: HTML, CSS, JavaScript, JSON, PNG, JPEG, GIF, SVG, ICO, TXT, XML, WOFF, WOFF2

## Security

The Router includes built-in security features:

- **Path Sanitization**: Blocks directory traversal attempts (`..`)
- **Null Byte Protection**: Blocks null byte injection attacks

## API Endpoints (Example)

The example application includes these endpoints:

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | Static homepage |
| GET | `/api/hello` | Simple API endpoint |
| GET | `/api/users` | List users |
| POST | `/api/users` | Create user |
| GET | `/health` | Health check |
| GET/POST | `/api/echo` | Echo request info |

## Requirements

- WFL version 26.1.19 or later
- WFL features used: Containers, Actions, Lists, Module system, Built-in web server

## License

Part of the WFL project. See main WFL repository for license.
