# Resources

Everything you need to learn WFL and get help when you're stuck.

## Official Documentation

### Core Documentation

**[Complete Documentation Hub →](../README.md)**
The main navigation page for all WFL documentation.

**Quick links:**
- **[Introduction](../01-introduction/index.md)** - What is WFL and why it matters
- **[Getting Started](index.md)** - You are here!
- **[Language Basics](../03-language-basics/index.md)** - Variables, loops, functions
- **[Advanced Features](../04-advanced-features/index.md)** - Web servers, async, OOP
- **[Standard Library](../05-standard-library/index.md)** - 181+ built-in functions
- **[Best Practices](../06-best-practices/index.md)** - Write quality code

### Guides & Tutorials

- **[WFL by Example](../guides/wfl-by-example.md)** - Learn through practical examples
- **[Cookbook](../guides/cookbook.md)** - Recipes for common tasks
- **[Migration from JavaScript](../guides/migration-from-javascript.md)** - JS → WFL guide
- **[Migration from Python](../guides/migration-from-python.md)** - Python → WFL guide
- **[Troubleshooting](../guides/troubleshooting.md)** - Common problems and solutions
- **[FAQ](../guides/faq.md)** - Frequently asked questions

### Language Reference

- **[Language Specification](../reference/language-specification.md)** - Complete formal spec
- **[Syntax Reference](../reference/syntax-reference.md)** - Quick syntax lookup
- **[Keyword Reference](../reference/keyword-reference.md)** - All keywords explained
- **[Operator Reference](../reference/operator-reference.md)** - All operators
- **[Built-in Functions](../reference/builtin-functions-reference.md)** - Complete function list
- **[Error Codes](../reference/error-codes.md)** - Understanding error messages

### Development Resources

- **[Building from Source](../development/building-from-source.md)** - Compile WFL yourself
- **[Contributing Guide](../development/contributing-guide.md)** - Help improve WFL
- **[Architecture Overview](../development/architecture-overview.md)** - How WFL works
- **[LSP Integration](../development/lsp-integration.md)** - Language Server details
- **[MCP Integration](../development/mcp-integration.md)** - AI assistant integration

---

## GitHub Repository

**Main Repository:**
**[https://github.com/WebFirstLanguage/wfl](https://github.com/WebFirstLanguage/wfl)**

What's there:
- ✅ Complete source code
- ✅ Latest releases and installers
- ✅ Issue tracker (report bugs)
- ✅ Pull requests (contribute code)
- ✅ TestPrograms directory (90+ working examples)
- ✅ Dev Diary (development history)

### Key Directories

- **`src/`** - Core compiler/runtime source
- **`TestPrograms/`** - 90+ working WFL examples
- **`Docs/`** - This documentation
- **`wfl-lsp/`** - Language Server
- **`vscode-extension/`** - VS Code extension
- **`scripts/`** - Utility scripts

---

## Community & Support

### Get Help

**GitHub Issues:**
[github.com/WebFirstLanguage/wfl/issues](https://github.com/WebFirstLanguage/wfl/issues)

- Report bugs
- Request features
- Ask questions
- Search existing issues

**GitHub Discussions:**
[github.com/WebFirstLanguage/wfl/discussions](https://github.com/WebFirstLanguage/wfl/discussions)

- General questions
- Share projects
- Discuss ideas
- Community help

**Email:**
info@logbie.com

- Direct support
- Business inquiries
- Security reports

### Contributing

Want to help improve WFL?

**[Contributing Guide →](../development/contributing-guide.md)**

Ways to contribute:
- 🐛 Report bugs
- 💡 Suggest features
- 📝 Improve documentation
- 🧪 Add test cases
- 💻 Submit code
- 🎨 Create examples

**All contributions welcome!** Even fixing typos helps.

---

## Example Programs

### TestPrograms Directory

The `TestPrograms/` directory contains 90+ validated WFL programs:

**Beginner Examples:**
- `basic_syntax_comprehensive.wfl` - Language basics
- `simple_web_server.wfl` - Minimal web server
- `hello.wfl` - Hello World variations

**Intermediate Examples:**
- `file_io_comprehensive.wfl` - Complete file operations
- `error_handling_comprehensive.wfl` - Error patterns
- `stdlib_comprehensive.wfl` - All standard library functions

**Advanced Examples:**
- `comprehensive_web_server_demo.wfl` - Full web server
- `containers_comprehensive.wfl` - OOP examples
- `patterns_comprehensive.wfl` - Pattern matching

**Browse them:**
[github.com/WebFirstLanguage/wfl/tree/main/TestPrograms](https://github.com/WebFirstLanguage/wfl/tree/main/TestPrograms)

---

## Learning Paths

Choose a path based on your experience:

### Path 1: Complete Beginner (New to Programming)

**Week 1:**
1. [What is WFL?](../01-introduction/what-is-wfl.md)
2. [Installation](installation.md)
3. [Hello World](hello-world.md)
4. [Your First Program](your-first-program.md)
5. [REPL Guide](repl-guide.md)

**Week 2:**
1. [Variables and Types](../03-language-basics/variables-and-types.md)
2. [Control Flow](../03-language-basics/control-flow.md)
3. [Loops](../03-language-basics/loops-and-iteration.md)

**Week 3:**
1. [Functions](../03-language-basics/actions-functions.md)
2. [Lists](../03-language-basics/lists-and-collections.md)
3. [Error Handling](../03-language-basics/error-handling.md)

**Week 4:** Build small projects!

### Path 2: Experienced Developer (Know Another Language)

**Day 1:**
1. [Key Features](../01-introduction/key-features.md) - See what's different
2. [First Look](../01-introduction/first-look.md) - Code comparisons
3. [Installation](installation.md)
4. [Your First Program](your-first-program.md)

**Day 2:**
1. [Language Basics](../03-language-basics/index.md) - Skim familiar concepts
2. [Advanced Features](../04-advanced-features/index.md) - Focus here
3. [Standard Library](../05-standard-library/index.md) - Reference

**Day 3:**
1. [Migration Guide](../guides/migration-from-javascript.md) (or Python)
2. Build something real
3. Refer to docs as needed

### Path 3: Quick Reference (Just Need Syntax)

Use these as lookups:
- [Syntax Reference](../reference/syntax-reference.md)
- [Keyword Reference](../reference/keyword-reference.md)
- [Operator Reference](../reference/operator-reference.md)
- [Built-in Functions](../reference/builtin-functions-reference.md)
- [Cookbook](../guides/cookbook.md)

---

## Cheat Sheets

### Quick Syntax Reference

```wfl
// Variables
store name as "value"
change name to "new value"

// Output
display "Hello"
display "Value: " with name

// Conditionals
store score as 75
check if score is greater than 50:
    display "high score"
otherwise:
    display "low score"
end check

// Loops
count from 1 to 10:
    display count
end count

for each item in ["first", "second"]:
    display item
end for

// Functions
define action called add_numbers with parameters param1 and param2:
    return param1 plus param2
end action

display add_numbers of 2 and 3

// Lists
create list items:
    add "first"
    add "second"
end list

// Error Handling
try:
    store result as 10 divided by 2
    display result
when error:
    display "handle error"
end try
```

### Common Built-in Functions

```wfl
store value as 42
store my_list as ["a", "b", "c"]
store item as "a"

// Core
display "text"
typeof of value
isnothing of value

// Math
abs of -5          // 5
round of 3.7       // 4
floor of 3.7       // 3
ceil of 3.2        // 4

// Text
touppercase of "hello"                    // "HELLO"
tolowercase of "HELLO"                    // "hello"
contains of "hello world" and "world"     // yes
substring of "hello" and 0 and 2          // "he"
length of "hello"                         // 5

// Lists
length of my_list                         // count
push with my_list and item
pop of my_list
contains of my_list and item
```

### File Operations

```wfl
// Write file
open file at "output.txt" for writing as out_file
write content "data" into out_file
close file out_file

// Read file
open file at "output.txt" for reading as in_file
store file_content as read content from in_file
close file in_file
display file_content

// List files
store dir_files as list files in "."
for each entry in dir_files:
    display entry
end for
```

---

## Tools & Utilities

### WFL CLI Commands

```bash
# Run a program
wfl program.wfl

# Start REPL
wfl

# Check syntax
wfl --parse program.wfl

# Analyze code
wfl --analyze program.wfl

# Lint code
wfl --lint program.wfl

# Auto-fix issues
wfl --fix program.wfl --in-place

# Show version
wfl --version

# Show help
wfl --help
```

### Editor Tools

- **VS Code Extension** - Syntax highlighting, LSP, error checking
- **LSP Server** - Language Server for any editor
- **MCP Server** - AI assistant integration (Claude Desktop)

**Install VS Code extension:**
```powershell
.\scripts\install_vscode_extension.ps1
```

---

## External Resources

### Learning Programming (If New to Coding)

If WFL is your first language:
- **[MDN Web Docs](https://developer.mozilla.org/)** - General web concepts
- **[freeCodeCamp](https://www.freecodecamp.org/)** - Free programming courses
- **[Codecademy](https://www.codecademy.com/)** - Interactive learning

(These teach JavaScript/Python, but concepts transfer to WFL)

### Related Languages

WFL draws inspiration from:
- **[Inform 7](http://inform7.com/)** - Natural language programming
- **[Elm](https://elm-lang.org/)** - Friendly error messages
- **[Rust](https://www.rust-lang.org/)** - WFL is built in Rust

### Web Development

Since WFL is web-first:
- **[HTTP Basics](https://developer.mozilla.org/en-US/docs/Web/HTTP)** - Understanding HTTP
- **[REST APIs](https://restfulapi.net/)** - API design
- **[Web Servers](https://developer.mozilla.org/en-US/docs/Learn/Common_questions/What_is_a_web_server)** - Server concepts

---

## Stay Updated

### Releases

Follow releases on GitHub:
**[github.com/WebFirstLanguage/wfl/releases](https://github.com/WebFirstLanguage/wfl/releases)**

WFL uses calendar versioning: **YY.MM.BUILD**
- Example: `26.1.17` = January 2026, build 17
- Monthly release cycles

### Changelog

Check what's new:
**[github.com/WebFirstLanguage/wfl/blob/main/CHANGELOG.md](https://github.com/WebFirstLanguage/wfl/blob/main/CHANGELOG.md)**

### Dev Diary

Follow development progress:
**`Dev diary/` directory** in the repository

---

## Quick Links Summary

**Get Started:**
- [Installation](installation.md)
- [Hello World](hello-world.md)
- [Your First Program](your-first-program.md)

**Learn More:**
- [Language Basics](../03-language-basics/index.md)
- [Standard Library](../05-standard-library/index.md)
- [WFL by Example](../guides/wfl-by-example.md)

**Get Help:**
- [GitHub Issues](https://github.com/WebFirstLanguage/wfl/issues)
- [GitHub Discussions](https://github.com/WebFirstLanguage/wfl/discussions)
- [Troubleshooting Guide](../guides/troubleshooting.md)
- Email: info@logbie.com

**Contribute:**
- [Contributing Guide](../development/contributing-guide.md)
- [GitHub Repository](https://github.com/WebFirstLanguage/wfl)

---

## What's Next?

You've completed the Getting Started section! Here's where to go:

### Continue Learning

**[Language Basics →](../03-language-basics/index.md)**
Deep dive into WFL syntax and features.

### Build Something

**[Cookbook →](../guides/cookbook.md)**
Recipes for common tasks.

### Explore Examples

**[TestPrograms →](https://github.com/WebFirstLanguage/wfl/tree/main/TestPrograms)**
90+ working WFL programs.

### Get Advanced

**[Advanced Features →](../04-advanced-features/index.md)**
Web servers, async, pattern matching, OOP.

---

**Remember:** The WFL community is here to help. Don't hesitate to ask questions!

---

**Previous:** [← Editor Setup](editor-setup.md) | **Next:** [Language Basics →](../03-language-basics/index.md)
