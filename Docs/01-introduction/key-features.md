# Key Features of WFL

WFL combines natural language syntax with powerful modern features. Here's what makes WFL special:

## 1. Natural Language Syntax

Write code that reads like plain English:

```wfl
store user name as "Alice"
store user age as 28
store is admin as no

check if user age is greater than 18 and is admin is yes:
    display "Full access granted"
otherwise:
    check if user age is greater than 18:
        display "Standard access granted"
    otherwise:
        display "Access denied - must be 18 or older"
    end check
end check
```

No cryptic symbols. No memorizing operator precedence. Just natural phrases.

## 2. Type Safety with Intelligent Inference

WFL's type system catches errors before runtime:

```wfl
store age as 25                    // Inferred as Number
store name as "Alice"              // Inferred as Text
store items as [1, 2, 3]           // Inferred as List

// Type checking prevents errors:
// display age plus name           // ERROR: Cannot add Number and Text
display age plus 5                 // OK: 30
display name with " Smith"         // OK: "Alice Smith"
```

The compiler knows types and prevents mistakes:
- No accidentally adding strings to numbers
- No null pointer errors
- Clear error messages when types don't match

## 3. Modern Async Support

Built-in async/await using Tokio runtime:

```wfl
// Async file operations
wait for file operation completes as result
display "File saved: " with result

// Async web requests
wait for response from "https://api.example.com/data"
display "Data received!"

// Multiple concurrent operations
wait for all operations complete
display "All tasks finished"
```

Non-blocking I/O is natural and easy to use.

## 4. Built-in Web Server

Create HTTP servers without external frameworks:

```wfl
// Start a server on port 8080
listen on port 8080 as web server

display "Server running at http://127.0.0.1:8080"

// Handle incoming requests
wait for request comes in on web server as req

check if path is equal to "/":
    respond to req with "Hello from WFL Web Server!"
check if path is equal to "/about":
    respond to req with "WFL - Programming in Plain English" and content type "text/plain"
otherwise:
    respond to req with "Page not found" and status 404
end check
```

Features include:
- Multiple HTTP methods (GET, POST, PUT, DELETE)
- Static file serving
- JSON support
- Custom headers and status codes
- Middleware support
- Graceful shutdown

## 5. Powerful Pattern Matching

Regex-like pattern engine with natural syntax:

```wfl
create pattern email address:
    one or more letter or digit or symbol from "._-"
    followed by "@"
    followed by one or more letter or digit
    followed by "."
    followed by between 2 and 4 letter
end pattern

check if "user@example.com" matches email address:
    display "Valid email!"
end check
```

Pattern features:
- Natural language quantifiers (`one or more`, `zero or more`, `between X and Y`)
- Character classes (`digit`, `letter`, `whitespace`)
- Lookahead and lookbehind
- Named capture groups
- Unicode support

## 6. Container System (OOP)

Object-oriented programming with natural syntax:

```wfl
create container Person:
    property name as text
    property age as number

    action introduce:
        display "Hello, I'm " with name with " and I'm " with age with " years old."
    end action
end container

create new Person as alice with property name as "Alice" and property age as 28
call alice introduce
// Output: Hello, I'm Alice and I'm 28 years old.
```

Features:
- Properties with type annotations
- Methods (called "actions")
- Inheritance with `extends`
- Interfaces with `implements`
- Events and event handlers

## 7. Comprehensive Standard Library

181+ built-in functions across 11 modules:

### Core Functions
```wfl
display "Hello"                    // Output text
store x as typeof of value         // Get type information
check if isnothing of value        // Check for null/nothing
```

### Math Module
```wfl
store absolute as abs of -5        // 5
store rounded as round of 3.7      // 4
store clamped as clamp of 15 between 0 and 10  // 10
```

### Text Module
```wfl
store upper as touppercase of "hello"           // "HELLO"
store length as length of "WFL"                 // 3
check if contains of "Hello World" and "World"  // yes
store substring as substring of "Hello" from 0 length 2  // "He"
```

### List Module
```wfl
create list numbers: 1, 2, 3, 4, 5 end list
push with numbers and 6
store size as length of numbers                 // 6
check if contains of numbers and 3              // yes
```

### Filesystem Module
```wfl
open file at "data.txt" for reading as file
store content as read content from file
close file

list files in "." as file list
for each file in file list:
    display "Found: " with file
end for
```

### Time Module
```wfl
store now as current time in milliseconds
store today as current date
display "Today is: " with format of today as "YYYY-MM-DD"
```

### Random Module (Cryptographically Secure)
```wfl
store dice as random int between 1 and 6
store coin as random boolean
store choice as random from ["red", "green", "blue"]
```

### Crypto Module (WFLHASH)
```wfl
store hash as wflhash256 of "sensitive data"
store mac as wflmac256 of "message" with "secret key"
```

**Note:** WFLHASH is a custom hash function, NOT externally audited. Use SHA-256/SHA-3/BLAKE3 for production security.

## 8. Developer-Friendly Tooling

### Language Server Protocol (LSP)
- Real-time error checking
- Auto-completion
- Go-to definition
- Hover documentation
- Works in VS Code, Vim, Emacs, and more

### VS Code Extension
- Syntax highlighting
- Integrated diagnostics
- Code snippets
- One-command installation

### Model Context Protocol (MCP)
- AI assistant integration (Claude Desktop)
- Code analysis and understanding
- Documentation assistance
- 6 tools: parse, analyze, typecheck, lint, completions, symbol info

### Code Quality Tools
```bash
wfl --lint your_program.wfl       # Check style
wfl --analyze your_program.wfl    # Static analysis
wfl --fix your_program.wfl        # Auto-fix issues
```

## 9. Security Features

### Automatic Output Escaping
WFL escapes output automatically to prevent XSS attacks.

### Secure Subprocess Execution
```wfl
execute command "ls" with arguments ["-la"] as result
// Input sanitization built-in
```

### Secure Random Number Generation
All random functions use cryptographically secure RNG.

### Memory Safety
Built on Rust, inheriting memory safety guarantees.

## 10. Clear Error Messages

Inspired by Elm, WFL provides helpful, actionable error messages:

```
❌ Type Error at line 5, column 8:

    Expected: Number
    Found:    Text ("hello")

The expression:
    age plus "hello"

Cannot add Number and Text.

💡 Suggestion: Convert both values to the same type:
    age plus 5
    or
    string of age with "hello"
```

Errors include:
- Clear descriptions
- Exact location (line and column)
- Helpful suggestions
- Context (what you were trying to do)

## 11. File I/O Made Simple

Comprehensive file operations:

```wfl
// Reading
open file at "data.txt" for reading as file
store content as read content from file
close file

// Writing
open file at "output.txt" for writing as file
write content "Hello, WFL!" into file
close file

// Appending
open file at "log.txt" for appending as file
write content "New log entry\n" into file
close file

// Path operations
check if path exists at "file.txt"
store size as file size at "file.txt"
store extension as path extension of "document.pdf"  // "pdf"
```

## 12. Subprocess Execution

Run external commands safely:

```wfl
execute command "git" with arguments ["status"] as result
display "Git status: " with result

spawn command "python" with arguments ["script.py"] as process
wait for process completes
```

Features:
- Synchronous and asynchronous execution
- Output capture
- Error handling
- Security sanitization

## 13. REPL for Experimentation

Interactive Read-Eval-Print Loop:

```bash
$ wfl
WFL REPL v26.1.17
> store name as "Alice"
> display "Hello, " with name
Hello, Alice
> count from 1 to 3:
...     display "Number: " with the current count
... end count
Number: 1
Number: 2
Number: 3
```

Perfect for:
- Learning WFL
- Testing snippets
- Debugging expressions
- Quick calculations

## 14. Cross-Platform Support

WFL runs on:
- ✅ Windows (with MSI installer)
- ✅ Linux
- ✅ macOS

Built on Rust for portability and performance.

## 15. Backward Compatibility Guarantee

**Sacred Promise:** Code you write today will work with all future WFL versions.

- No surprise breaking changes
- 1+ year deprecation notice if absolutely necessary
- Your learning investment is protected
- Long-term project stability

## Performance Characteristics

- **Startup:** Fast (milliseconds)
- **Execution:** Interpreted with optimizations
- **Memory:** Rust-based memory safety
- **Async:** Tokio runtime for efficient I/O
- **Future:** Bytecode VM planned for better performance

## Limitations (Current Alpha)

What WFL **doesn't** (yet) have:
- ❌ Production stability (alpha software)
- ❌ Package manager
- ❌ Module system (planned)
- ❌ Debugger (basic debugging available)
- ❌ Comprehensive documentation (you're helping build it!)
- ❌ Large ecosystem of libraries

What WFL **isn't** designed for:
- Mobile app development
- Desktop GUI applications
- Low-level systems programming
- Game development
- Real-time embedded systems

## Next Steps

Explore more about WFL:

- **[Natural Language Philosophy](natural-language-philosophy.md)** - The 19 principles behind WFL's design
- **[First Look](first-look.md)** - More code examples
- **[Why WFL?](why-wfl.md)** - Why you should use WFL
- **[Getting Started](../02-getting-started/index.md)** - Install WFL and start coding

---

**Previous:** [← What is WFL?](what-is-wfl.md) | **Next:** [Natural Language Philosophy →](natural-language-philosophy.md)
