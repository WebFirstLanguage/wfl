# WFL Documentation Index

Welcome to the WebFirst Language documentation! This index provides a comprehensive guide to all available documentation, organized for easy navigation according to the natural-language principles outlined in our [Foundation document](guides/wfl-foundation.md).

## 📚 Language Reference

Core language documentation for learning and using WFL:

- **[Language Specification](language-reference/wfl-spec.md)** - Complete formal specification of WFL syntax and semantics
- **[Variables Guide](language-reference/wfl-variables.md)** - Creating and using variables in WFL
- **[Control Flow](language-reference/wfl-control-flow.md)** - Conditionals, loops, and program flow
- **[Actions (Functions)](language-reference/wfl-actions.md)** - Defining and using actions
- **[Pattern Matching](language-reference/wfl-patterns.md)** - Comprehensive pattern matching with natural language syntax
- **[Async Programming](language-reference/wfl-async.md)** - Asynchronous operations and concurrency
- **[Container System](language-reference/wfl-containers.md)** - Object-oriented programming in WFL
- **[Error Handling](language-reference/wfl-errors.md)** - Understanding and handling errors
- **[I/O Operations](language-reference/wfl-io.md)** - File and network input/output
- **[Main Loop](language-reference/wfl-main-loop.md)** - Event-driven programming

## 📖 Guides and Tutorials

Best practices and learning resources:

- **[WFL Foundation](guides/wfl-foundation.md)** - Core principles and design philosophy
- **[Getting Started](guides/wfl-getting-started.md)** - Installation and first steps
- **[WFL by Example](guides/wfl-by-example.md)** - Learn through practical examples
- **[WFL Cookbook](guides/wfl-cookbook.md)** - Recipes for common tasks
- **[Building WFL](guides/building.md)** - Building from source
- **[Deployment Guide](guides/wfl-deployment.md)** - Deploying WFL applications
- **[Pattern Migration Guide](guides/pattern-migration-guide.md)** - Migrating from regex to WFL patterns
- **[General Migration Guide](guides/wfl-migration-guide.md)** - Migrating from other languages
- **[Documentation Policy](guides/wfl-documentation-policy.md)** - Guidelines for writing documentation

## 📦 API Reference

Standard library and built-in functionality:

- **[Standard Library Reference](api/wfl-standard-library.md)** - Complete reference for all built-in functions
- **[Core Module](api/core-module.md)** - Core language functions
- **[Math Module](api/math-module.md)** - Mathematical operations
- **[Text Module](api/text-module.md)** - String manipulation
- **[List Module](api/list-module.md)** - List operations
- **[Pattern Module](api/pattern-module.md)** - Pattern matching API (legacy)
- **[Time Module](api/time-module.md)** - Date and time operations
- **[Filesystem Module](api/filesystem-module.md)** - File system operations
- **[Container System](api/container-system.md)** - Container/class API
- **[Async Patterns](api/async-patterns.md)** - Asynchronous programming patterns

## 🔧 Technical Documentation

Internal technical documentation for contributors and advanced users:

### Core Components
- **[Lexer Implementation](technical/wfl-lexer.md)** - Tokenization and lexical analysis
- **[Lexer Fix Details](technical/wfl-lexer-fix-1.md)** - Documentation of lexer improvements
- **[Parser Design](technical/wfl-parser.md)** - Syntax analysis and AST generation
- **[Analyzer](technical/wfl-analyzer.md)** - Semantic analysis
- **[Type Checker](technical/wfl-static-type-checker.md)** - Static type analysis system
- **[Interpreter Design](technical/wfl-interpreter.md)** - AST execution and runtime
- **[Bytecode System](technical/wfl-bytecode.md)** - Bytecode compilation and VM

### Development Tools
- **[CLI Arguments](technical/wfl-args.md)** - Command-line argument handling
- **[Linter System](technical/wfl-lint.md)** - Code style and quality checks
- **[Logging System](technical/wfl-logging.md)** - Structured logging
- **[Step Debugging](technical/wfl-step.md)** - Step-by-step execution
- **[Version Management](technical/wfl-version.md)** - Version numbering and releases
- **[Memory Profiling](technical/memory-profiling.md)** - Performance analysis
- **[OOP Design](technical/wfl-oop-design.md)** - Object-oriented programming architecture

### Architecture
- **[Architecture Diagram](technical/wfl-architecture-diagram.md)** - System architecture overview

## 🔬 Development Notes

Internal development documentation (not for general users):

- **[TODO List](dev-notes/wfl-todo.md)** - Project task tracking
- **[Devin Integration](dev-notes/wfl-devin.md)** - AI assistant integration notes
- **[Gemini Research](dev-notes/wfl-gemini-research.md)** - Research notes
- **[Library Recommendations](dev-notes/wfl-library-recommendations.md)** - External library suggestions
- **[Integration Notes](dev-notes/wfl-int2.md)** - Integration with other systems
- **[Rust LOC Report](dev-notes/wfl-rust-loc-report.md)** - Code metrics
- **[Rust LOC Counter](dev-notes/wfl-rust-loc-counter.md)** - Line counting tool

## 🚀 Quick Links

- [README](../README.md) - Project overview and quick start
- [CLAUDE.md](../CLAUDE.md) - AI assistant instructions
- [TestPrograms](../TestPrograms/) - Example WFL programs
- [Dev Diary](../Dev%20diary/) - Development history

## 📝 Contributing to Documentation

When adding new documentation:

1. **Choose the right location:**
   - `language-reference/` - User-facing language documentation
   - `guides/` - Tutorials, how-tos, and best practices
   - `api/` - API and library reference
   - `technical/` - Internal technical documentation
   - `dev-notes/` - Development notes and temporary docs

2. **Follow naming conventions:**
   - Use clear, descriptive filenames
   - Prefix with `wfl-` for consistency
   - Use lowercase with hyphens

3. **Update this index** with a link to your new document

4. **Follow the documentation policy** outlined in [guides/wfl-documentation-policy.md](guides/wfl-documentation-policy.md)

5. **Include examples** and cross-references where appropriate

6. **Align with WFL principles** from [guides/wfl-foundation.md](guides/wfl-foundation.md):
   - Use natural language descriptions
   - Prioritize clarity over brevity
   - Make documentation accessible to beginners
   - Provide clear, actionable information

## 📊 Documentation Statistics

- **Language Reference:** 10 comprehensive guides
- **User Guides:** 9 tutorials and how-tos
- **API Documentation:** 10 module references
- **Technical Docs:** 15 internal documents  
- **Dev Notes:** 7 development documents
- **Total Documentation:** 51 organized documents

*Last updated: August 2025*

---

*This documentation is organized according to the principles in [WFL Foundation](guides/wfl-foundation.md), emphasizing natural language, clarity, and accessibility for all developers.*