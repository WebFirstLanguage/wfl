# WFL Documentation Index

Welcome to the WebFirst Language documentation! This index provides a comprehensive guide to all available documentation.

## 📚 Language Reference

Core language documentation for learning and using WFL:

- **[Language Specification](language-reference/wfl-spec.md)** - Complete formal specification of WFL syntax and semantics
- **[Variables Guide](language-reference/wfl-variables.md)** - Creating and using variables in WFL
- **[Control Flow](language-reference/wfl-control-flow.md)** - Conditionals, loops, and program flow
- **[Actions (Functions)](language-reference/wfl-actions.md)** - Defining and using actions
- **[Async Programming](language-reference/wfl-async.md)** - Asynchronous operations and concurrency
- **[Container System](language-reference/wfl-containers.md)** - Object-oriented programming in WFL
- **[Error Handling](language-reference/wfl-errors.md)** - Understanding and handling errors

## 🔧 Technical Documentation

Internal technical documentation for contributors and advanced users:

- **[Lexer Implementation](technical/wfl-lexer.md)** - Tokenization and lexical analysis
- **[Lexer Fix Details](technical/lexer_fix_1.md)** - Documentation of lexer improvements
- **[Interpreter Design](technical/wfl-interpreter.md)** - AST execution and runtime
- **[Type Checker](technical/wfl-staticTypeChecker.md)** - Static type analysis system

## 📦 API Reference

Standard library and built-in functionality:

- **[Standard Library Reference](api/wfl-standard-library.md)** - Complete reference for all built-in functions
- **[Pattern Module](api/pattern-module.md)** - Regular expression and pattern matching

## 📖 Guides and Policies

Best practices and development guidelines:

- **[WFL Foundation](guides/wfl-foundation.md)** - Core principles and design philosophy
- **[Documentation Policy](guides/wfl-documentation-policy.md)** - Guidelines for writing documentation

## 🔍 Additional Resources

Other documentation and resources:

### Development Tools
- **[Building WFL](BUILDING.md)** - Instructions for building from source
- **[Deployment Guide](wfl-deployment.md)** - Deploying WFL applications
- **[Version Management](wfl-version.md)** - Version numbering and releases

### Language Features
- **[I/O Operations](wfl-IO.md)** - File and network I/O
- **[Pattern Matching](patterns.md)** - Natural language pattern matching system
- **[Logging System](wfl-logging.md)** - Structured logging
- **[Linting](wfl-lint.md)** - Code style and quality checks
- **[OOP Design](wfl-oop-design.md)** - Object-oriented programming concepts

### Implementation Details
- **[Arguments Handling](wfl-args.md)** - Command-line arguments
- **[Integration Notes](wfl-int2.md)** - Integration with other systems
- **[Step Execution](wfl-step.md)** - Step-by-step execution details

### Historical and Research
- **[Devin Integration](wfl-devin.md)** - AI assistant integration notes
- **[Gemini Research](Gemini Reserch.md)** - Research notes
- **[Library Recommendations](lib recs.md)** - External library suggestions
- **[Memory Profiling](memory_profiling.md)** - Performance analysis
- **[Rust LOC Report](rust_loc_report.md)** - Code metrics
- **[Rust LOC Counter](rust_loc_counter.md)** - Line counting tool
- **[TODO List](wfl-todo.md)** - Project task tracking

## 🚀 Quick Links

- [README](../README.md) - Project overview and quick start
- [CLAUDE.md](../CLAUDE.md) - AI assistant instructions
- [TestPrograms](../TestPrograms/) - Example WFL programs
- [Dev Diary](../Dev%20diary/) - Development history

## 📝 Contributing to Documentation

When adding new documentation:

1. Place files in the appropriate subdirectory:
   - `language-reference/` - User-facing language documentation
   - `technical/` - Internal technical documentation
   - `api/` - API and library reference
   - `guides/` - Best practices and guidelines

2. Update this index with a link to your new document

3. Follow the documentation policy guidelines

4. Use clear, descriptive filenames with the `wfl-` prefix

5. Include examples and cross-references where appropriate