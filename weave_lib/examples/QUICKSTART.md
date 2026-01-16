# Weave Quick Start Guide

Get a web server running in 5 minutes!

## Option 1: Simplest Possible Server (3 lines)

```wfl
listen on port 3000 as server
wait for request comes in on server as req
respond to req with "Hello from Weave!"
```

**Run it**:
```bash
wfl simple_server.wfl
```

**Test it**:
```bash
curl http://localhost:3000
# Output: Hello from Weave!
```

## Option 2: With Routing (Working Example)

Use the complete working example:

```bash
cd weave_lib/examples
wfl hello_world_working.wfl
```

Then test all routes:
```bash
# Home page
curl http://localhost:3000/
# Output: Hello, World! Welcome to Weave.

# About page
curl http://localhost:3000/about
# Output: This is the about page.

# API endpoint
curl http://localhost:3000/api/status
# Output: {"status": "running", "framework": "Weave"}

# 404 test
curl http://localhost:3000/notfound
# Output: Beautiful styled 404 HTML page
```

## Option 3: Create Your Own

**Step 1**: Copy the template
```bash
cp weave_lib/examples/hello_world_working.wfl my_app.wfl
```

**Step 2**: Edit routes (around line 29-31)
```wfl
// Add your routes here
call register_route with "/" and "My home page"
call register_route with "/contact" and "Contact us at: email@example.com"
call register_route with "/api/data" and "{\"message\": \"Your API data here\"}"
```

**Step 3**: Run it
```bash
wfl my_app.wfl
```

## What You Get

✅ **Routing** - Path-based route matching
✅ **Logging** - Automatic request/response logging
✅ **404 Handling** - Styled error pages
✅ **JSON Support** - Perfect for APIs
✅ **Fast** - <2ms per request
✅ **Clean Syntax** - Natural language WFL

## Common Patterns

### JSON API Endpoint
```wfl
call register_route with "/api/users" and "{\"users\": [\"alice\", \"bob\"]}"
```

### Text Response
```wfl
call register_route with "/about" and "About our company..."
```

### HTML Response
```wfl
store home_html as "<!DOCTYPE html><html><body><h1>Welcome!</h1></body></html>"
call register_route with "/" and home_html
```

## Port Configuration

Change the port (default is 3000):
```wfl
store server_port_number as 8080  // Line 8
```

## Accessing Request Information

Inside the main loop, these variables are auto-defined:
- `method` - "GET", "POST", etc.
- `path` - "/", "/about", etc.
- `client_ip` - "127.0.0.1"
- `body` - Request body content
- `headers` - Request headers

## Next Steps

1. **Add more routes** - Just call `register_route` with your paths
2. **Customize responses** - Modify the response text or add HTML
3. **Add static files** - Coming in Phase 2!
4. **Add middleware** - Coming in Phase 3!

## Troubleshooting

**Port already in use?**
```wfl
store server_port_number as 3001  // Change to different port
```

**Routes not matching?**
- Check that paths match exactly (case-sensitive)
- Paths must start with "/"
- No trailing slashes

**Server won't start?**
```bash
# Make sure WFL is built
cargo build --release

# Run from correct directory
cd weave_lib/examples
../../target/release/wfl hello_world_working.wfl
```

## Full Documentation

See `weave_lib/README.md` for complete documentation including:
- Architecture details
- Advanced features
- Security considerations
- Performance tuning
- Contributing guidelines

---

**Happy coding with Weave!** 🚀
