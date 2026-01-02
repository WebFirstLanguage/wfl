# WFL Web Server Examples

This directory contains web server examples organized by complexity level. Each example demonstrates specific concepts and builds upon the previous ones.

## 📁 Example Organization

### Basic Level (Start Here!)

**[01-basic-hello-server.wfl](01-basic-hello-server.wfl)**
- **Concept**: Simplest possible web server
- **Features**: `listen on port`, `wait for request`, `respond to`
- **Time**: 2 minutes
- **Perfect for**: First web server, understanding basic syntax

**[02-file-server-basic.wfl](02-file-server-basic.wfl)**
- **Concept**: Reading files and serving their contents (exactly what was requested in issue #213)
- **Features**: File I/O with `open file`, `read content`, `close file`
- **Time**: 5 minutes  
- **Perfect for**: Understanding file serving, error handling

### Intermediate Level

**[03-multi-route-server.wfl](03-multi-route-server.wfl)**
- **Concept**: Handling different URL paths and file types
- **Features**: Multiple routes, different content types, 404 handling
- **Time**: 10 minutes
- **Perfect for**: Learning URL routing, content types

### Advanced Level

**[04-advanced-comprehensive.wfl](04-advanced-comprehensive.wfl)**
- **Concept**: Full-featured server with APIs and dynamic content
- **Features**: JSON APIs, request logging, dynamic HTML, statistics
- **Time**: 15 minutes
- **Perfect for**: Production-like servers, API development

## 🚀 Quick Start

1. **Start with the basics**: Run `01-basic-hello-server.wfl`
   ```bash
   wfl 01-basic-hello-server.wfl
   ```

2. **Try file serving**: Run `02-file-server-basic.wfl` 
   ```bash
   wfl 02-file-server-basic.wfl
   ```
   This directly addresses the GitHub issue request!

3. **Explore routing**: Run `03-multi-route-server.wfl`
   ```bash
   wfl 03-multi-route-server.wfl
   ```

4. **Build APIs**: Run `04-advanced-comprehensive.wfl`
   ```bash
   wfl 04-advanced-comprehensive.wfl
   ```

## 🎯 Learning Path

| Level | Focus | Time | Key Concepts |
|-------|-------|------|--------------|
| **Beginner** | Basic server setup | 5 min | `listen`, `wait for request`, `respond to` |
| **Beginner** | File serving | 10 min | File I/O, error handling, content types |
| **Intermediate** | Multiple routes | 15 min | Path routing, different file types |
| **Advanced** | Full features | 30 min | JSON APIs, dynamic content, logging |

## 💡 Key WFL Web Server Concepts

### Core Syntax
```wfl
// Start a server
listen on port 8080 as server_name

// Handle requests  
wait for request comes in on server_name as req

// Send responses
respond to req with "content" and content_type "text/html"
respond to req with "error" and status 404
```

### File Serving Pattern
```wfl
try:
    open file at "filename.txt" for reading as file
    store content as read content from file
    close file
    respond to req with content and content_type "text/plain"
catch:
    respond to req with "File not found" and status 404
end try
```

### Request Information
```wfl
// Available request properties:
method      // GET, POST, PUT, DELETE, etc.
path        // /api/users, /, /hello, etc.
client_ip   // Client's IP address
body        // Request body content
headers     // HTTP headers object
```

## 📚 Additional Resources

- **[Quick Start Guide](../../guides/wfl-web-server-quickstart.md)** - 5-minute tutorial
- **[WFL by Example](../../guides/wfl-by-example.md#web-servers-and-http-services)** - Complete language tutorial
- **[Web Server Specification](../../wflspecs/SPEC-web-server.md)** - Complete feature documentation
- **[Test Programs](../../../TestPrograms/)** - More working examples

## 🔧 Working with Test Programs

The main WFL repository includes additional web server examples in the `TestPrograms/` directory:

- **`test_static_files.wfl`** - Complete static file server
- **`comprehensive_web_server_demo.wfl`** - Full-featured server demo  
- **`simple_web_server.wfl`** - Basic server example

These are production test programs that must always work.

## ⚡ Pro Tips

1. **Start small**: Always begin with `01-basic-hello-server.wfl`
2. **Read the comments**: Each example has detailed explanations
3. **Modify and experiment**: Change ports, file names, content
4. **Check the test programs**: See `TestPrograms/` for more examples
5. **Use error handling**: Always wrap file operations in `try/catch`

## 🤝 Contributing Examples

When adding new examples:

1. Follow the naming convention: `##-description-name.wfl`
2. Include comprehensive comments explaining each concept
3. Start with `display` statements showing what the example does
4. Use realistic scenarios that users might actually need
5. Update this README with the new example

---

**Happy coding with WFL! 🚀**