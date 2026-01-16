# Nexus Framework

A lightweight demonstration of the Server -> Router architecture pattern for WFL (WebFirst Language).

## Architecture

Nexus demonstrates a strict **Server -> Router** architecture with a "hierarchy of laziness" resolution algorithm:

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

This runs a demonstration of the Router's resolution algorithm, showing how different request paths are handled.

## Project Structure

```
examples/nexus/
├── public/                  # Static files (the "File Sink")
│   ├── index.html          # Default homepage
│   └── styles.css          # Default styles
├── main.wfl                # Entry point and demo
└── README.md               # This file
```

## Core Concepts

### The Server (The "Bouncer")

The Server handles connection management only. It does not know about business logic. All incoming requests are immediately passed to the Router for processing.

### The Router (The "Traffic Cop")

The Router implements a "hierarchy of laziness" - it tries to do the easiest thing first:

1. **Static File Bypass**: Check if a file exists at `web_root + path` or `web_root + path + .html`
2. **Dynamic Route Lookup**: Check registered routes for a match
3. **404 Fallback**: Return a 404 response if nothing matched

### Resolution Algorithm Demo

The main.wfl file demonstrates how the Router resolves different types of requests:

- **Static file requests** (e.g., `/index.html`) are served directly from the `/public` directory
- **Dynamic API requests** (e.g., `/api/hello`) are matched against registered routes
- **Unknown paths** (e.g., `/unknown`) fall through to the 404 handler

## Example Output

```
==============================================
       Nexus Framework - WFL Server          
==============================================

Configuration:
  Port: 8080
  Host: 127.0.0.1
  Web Root: examples/nexus/public

Router Resolution Algorithm (Hierarchy of Laziness):

STEP 1: Static File Bypass
  - Check if file exists at web_root + path
  - Check if file exists at web_root + path + .html
  - If found, serve immediately (no further processing)

STEP 2: Dynamic Route Lookup
  - Check registered routes for path match
  - If found, execute handler and return response

STEP 3: 404 Fallback
  - If nothing matched, return 404 Not Found

Example 1: Static File Request
  Request: GET /index.html
  -> STEP 1: Static file found
  -> Response: 200 OK (text/html)

Example 2: Dynamic API Request
  Request: GET /api/hello
  -> STEP 1: No static file
  -> STEP 2: Dynamic route matched
  -> Response: 200 OK (application/json)
  -> Body: {"message":"Hello from Nexus!"}

Example 3: Unknown Path Request
  Request: GET /unknown
  -> STEP 1: No static file
  -> STEP 2: No dynamic route
  -> STEP 3: 404 Fallback
  -> Response: 404 Not Found
```

## Available Endpoints (Conceptual)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | Static homepage |
| GET | `/styles.css` | Static stylesheet |
| GET | `/api/hello` | Simple API endpoint |
| GET | `/api/users` | List users |
| GET | `/health` | Health check |
| GET/POST | `/api/echo` | Echo request info |

## Full Web Server Implementation

For a complete, runnable web server implementation, see `TestPrograms/comprehensive_web_server_demo.wfl` in the main WFL repository. The Nexus framework demonstrates the architectural pattern that can be applied to build full web applications.

## Requirements

- WFL version 26.1.19 or later

## License

Part of the WFL project. See main WFL repository for license.
