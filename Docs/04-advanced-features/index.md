# Advanced Features

WFL includes powerful features for real-world applications: web servers, file I/O, async operations, pattern matching, object-oriented programming, and more.

## What You'll Learn

This section covers WFL's advanced capabilities:

1. **[Async Programming](async-programming.md)** - Non-blocking operations with `wait for`
2. **[Web Servers](web-servers.md)** - Built-in HTTP servers without frameworks
3. **[File I/O](file-io.md)** - Reading, writing, and managing files
4. **[Pattern Matching](pattern-matching.md)** - Regex-like patterns with natural syntax
5. **[Containers (OOP)](containers-oop.md)** - Object-oriented programming
6. **[Subprocess Execution](subprocess-execution.md)** - Running external commands
7. **[Interoperability](interoperability.md)** - Working with other technologies

## Prerequisites

Before diving into Advanced Features, you should:

✅ **Complete Language Basics** - [Language Basics Section](../03-language-basics/index.md)
✅ **Understand** variables, conditionals, loops, actions, lists, error handling
✅ **Have WFL installed** - [Installation Guide](../02-getting-started/installation.md)

If you've completed those, you're ready for advanced features!

## What Makes These "Advanced"?

These features are "advanced" not because they're complicated, but because they're **powerful**:

- **Web Servers** - Build HTTP APIs and web applications
- **Async** - Handle multiple operations concurrently
- **File I/O** - Persist data and process files
- **Pattern Matching** - Validate and extract data
- **Containers** - Organize code with object-oriented programming
- **Subprocess** - Integrate with external tools

WFL's natural language syntax makes these advanced features approachable!

## Quick Preview

### Web Server (3 Lines!)

```wfl
listen on port 8080 as server
wait for request comes in on server as req
respond to req with "Hello, Web!"
```

That's a working web server!

### File Operations

```wfl
open file at "data.txt" for reading as file
store content as read content from file
close file
display content
```

Simple and clear.

### Pattern Matching

```wfl
create pattern email:
    one or more letter or digit
    followed by "@"
    followed by one or more letter or digit
    followed by "."
    followed by between 2 and 4 letter
end pattern

check if "user@example.com" matches email:
    display "Valid email!"
end check
```

Natural language patterns instead of cryptic regex.

### Container (Class)

```wfl
create container Person:
    property name as text
    property age as number

    action introduce:
        display "I'm " with name
    end action
end container

create new Person as alice with property name as "Alice" and property age as 28
call alice introduce
```

Object-oriented programming with readable syntax.

## Learning Approach

### For Web Development

**Focus on:**
1. [Web Servers](web-servers.md) - Build HTTP APIs
2. [File I/O](file-io.md) - Store and retrieve data
3. [Async Programming](async-programming.md) - Handle requests efficiently
4. [Error Handling](../03-language-basics/error-handling.md) - Robust applications

### For Automation/Scripts

**Focus on:**
1. [File I/O](file-io.md) - Process files
2. [Subprocess Execution](subprocess-execution.md) - Run commands
3. [Pattern Matching](pattern-matching.md) - Validate and extract data
4. [Error Handling](../03-language-basics/error-handling.md) - Reliable scripts

### For Application Development

**Focus on:**
1. [Containers (OOP)](containers-oop.md) - Code organization
2. [Web Servers](web-servers.md) - Backend services
3. [Async Programming](async-programming.md) - Concurrent operations
4. [File I/O](file-io.md) - Persistence

## Real-World Examples

Each advanced topic includes real-world examples:

- **Web Servers:** Complete HTTP server with routing, static files, middleware
- **File I/O:** File processing, directory operations, CRUD operations
- **Patterns:** Email validation, phone numbers, data extraction
- **Containers:** User management, shopping cart, task manager
- **Subprocess:** Git integration, build automation, system administration

All examples are **validated and tested** to ensure they work!

## From Basics to Advanced

Here's how the basics connect to advanced features:

**Variables** → Used everywhere in advanced features
**Conditionals** → Route handling, validation, business logic
**Loops** → Processing requests, iterating files, batch operations
**Actions** → Web handlers, file processors, validators
**Lists** → Collections of users, products, requests
**Error Handling** → Robust file operations, web servers, subprocess management

Everything builds on the foundation you learned in Language Basics.

## Tips for Learning Advanced Features

### 1. **Read in Order (First Time)**

If this is your first time:
1. Start with Async Programming (foundation for web/file operations)
2. Then Web Servers or File I/O (whichever interests you)
3. Explore Pattern Matching for validation
4. Learn Containers for code organization

### 2. **Jump Around (Reference Use)**

Already familiar? Jump directly to what you need:
- Building API? → [Web Servers](web-servers.md)
- Processing files? → [File I/O](file-io.md)
- Validating input? → [Pattern Matching](pattern-matching.md)

### 3. **Use TestPrograms**

The repository includes comprehensive examples:

- `comprehensive_web_server_demo.wfl` - Complete web server
- `file_io_comprehensive.wfl` - All file operations
- `patterns_comprehensive.wfl` - Pattern matching reference
- `containers_comprehensive.wfl` - OOP examples
- `subprocess_comprehensive.wfl` - Process management

**These are validated, working programs you can learn from!**

### 4. **Try in the REPL**

Some features work in the REPL:
- ✅ Pattern creation and matching
- ✅ Action definitions
- ⚠️ File operations (creates real files!)
- ❌ Web servers (need separate terminal)

### 5. **Combine Features**

Real applications use multiple advanced features:

**Example: File Processing API**
- Web Server (HTTP endpoint)
- File I/O (read uploaded files)
- Pattern Matching (validate file format)
- Async (handle multiple uploads)
- Error Handling (graceful failures)

You'll see combined examples throughout this section.

## What You'll Build

By the end of Advanced Features, you'll be able to build:

✅ **HTTP APIs** - RESTful web services
✅ **File Processors** - Batch file operations
✅ **Data Validators** - Pattern-based validation
✅ **Web Applications** - Complete server-side apps
✅ **Automation Tools** - Scripts that integrate with external tools
✅ **Structured Applications** - Well-organized code with containers

## Features Comparison

| Feature | Basics | Advanced |
|---------|--------|----------|
| Variables | `store x as 5` | Web request data, file handles |
| Loops | `count from 1 to 10` | Processing web requests, file lists |
| Actions | Simple functions | Web handlers, file processors |
| Error Handling | Try-catch | Network errors, file errors, validation |
| Data | Lists and text | JSON, file content, HTTP requests |

Advanced features let you build **real applications**, not just practice programs.

## Prerequisites Checklist

Before starting Advanced Features, make sure you can:

- ✅ Create and change variables
- ✅ Use conditionals (`check if`, `otherwise`)
- ✅ Write loops (`count`, `for each`)
- ✅ Define and call actions
- ✅ Work with lists
- ✅ Handle errors with try-catch
- ✅ Read documentation and understand examples

**Not confident?** Review [Language Basics](../03-language-basics/index.md) first!

## Ready to Start?

Choose where to begin:

### **New to Advanced Features?**
**[Start with Async Programming →](async-programming.md)**
Learn the foundation for non-blocking operations.

### **Want to Build a Web Server?**
**[Jump to Web Servers →](web-servers.md)**
Build HTTP APIs in minutes.

### **Need File Operations?**
**[Start with File I/O →](file-io.md)**
Read, write, and manage files.

### **Validating User Input?**
**[Learn Pattern Matching →](pattern-matching.md)**
Regex-like patterns with natural syntax.

### **Building Large Applications?**
**[Explore Containers →](containers-oop.md)**
Object-oriented code organization.

---

**Previous:** [← Comments and Documentation](../03-language-basics/comments-and-documentation.md) | **Next:** [Async Programming →](async-programming.md)
