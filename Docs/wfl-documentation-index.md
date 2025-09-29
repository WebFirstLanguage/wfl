# WFL Documentation Index

Welcome to the WebFirst Language documentation! This index provides a comprehensive guide to all available documentation, organized for easy navigation according to the natural-language principles outlined in our [Foundation document](guides/wfl-foundation.md).

## 🤖 AI Assistant Resources

Essential resources for AI agents and automated tools:

- **[WFL Living AI Document](wfl-living-ai.md)** - Constantly-updated cheat sheet for AI agents building WFL apps

## 📚 Core Language Features (WFLDocs)

Implemented language features and syntax documentation:

- **[Language Specification](wfldocs/WFL-spec.md)** - Complete formal specification of WFL syntax and semantics
- **[Variables Guide](wfldocs/WFL-variables.md)** - Creating and using variables in WFL
- **[Control Flow](wfldocs/WFL-control-flow.md)** - Conditionals, loops, and program flow
- **[Actions (Functions)](wfldocs/WFL-actions.md)** - Defining and using actions
- **[Pattern Matching](wfldocs/WFL-patterns.md)** - Comprehensive pattern matching with natural language syntax
- **[Async Programming](wfldocs/WFL-async.md)** - Asynchronous operations and concurrency
- **[Container System](wfldocs/WFL-containers.md)** - Object-oriented programming in WFL
- **[Error Handling](wfldocs/WFL-errors.md)** - Understanding and handling errors
- **[I/O Operations](wfldocs/WFL-io.md)** - File and network input/output
- **[Main Loop](wfldocs/WFL-main-loop.md)** - Event-driven programming
- **[Loop Scoping](wfldocs/WFL-loop-scoping.md)** - Loop variable scoping and iteration behavior

## 🚧 Planned Features (WFLSpecs)

Proposed and experimental features under development:

- **[Web Server Implementation](wflspecs/SPEC-web-server.md)** - Planned web server functionality and API design

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

## 🔌 IDE Integration

Language Server Protocol and editor support:

- **[WFL LSP Guide](guides/wfl-lsp-guide.md)** - Complete guide to using the WFL Language Server Protocol
- **[WFL LSP Quick Reference](guides/wfl-lsp-quick-reference.md)** - Quick reference for LSP features and shortcuts
- **[VS Code Extension Guide](../vscode-extension/README.md)** - Visual Studio Code integration and setup

## 📦 API Reference

Standard library and built-in functionality:

- **[Standard Library Reference](api/wfl-standard-library.md)** - Complete reference for all built-in functions
- **[Core Module](api/core-module.md)** - Core language functions
- **[Math Module](api/math-module.md)** - Mathematical operations
- **[Random Module](api/random-module.md)** - Cryptographically secure random number generation
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
- **[LSP Architecture](technical/wfl-lsp-architecture.md)** - Language Server Protocol implementation details
- **[Parser Limitations](technical/wfl_parser_limitations.md)** - Known parser limitations and workarounds
- **[WFL Hash](technical/wflhash.md)** - Custom cryptographic hash function documentation
- **[Error System](technical/error system.pdf)** - Error handling system design (PDF)

## 🔬 Development Notes

Internal development documentation (not for general users):

- **[TODO List](dev-notes/wfl-todo.md)** - Project task tracking
- **[Bug Reports](dev-notes/wfl-bug-reports.md)** - Historical bug investigations and resolutions
- **[Memory Optimization](dev-notes/wfl-memory-optimization.md)** - Memory management guidelines and best practices
- **[Devin Integration](dev-notes/wfl-devin.md)** - AI assistant integration notes
- **[Gemini Research](dev-notes/wfl-gemini-research.md)** - Research notes
- **[Library Recommendations](dev-notes/wfl-library-recommendations.md)** - External library suggestions
- **[Integration Notes](dev-notes/wfl-int2.md)** - Integration with other systems
- **[Rust LOC Report](dev-notes/wfl-rust-loc-report.md)** - Code metrics
- **[Rust LOC Counter](dev-notes/wfl-rust-loc-counter.md)** - Line counting tool
- **[Pattern Implementation Analysis](dev-notes/pattern-implementation-analysis.md)** - Pattern matching implementation details
- **[Rust LOC Report (Simple)](dev-notes/rust_loc_report_simple.md)** - Simplified code metrics
- **[Rust LOC Report (Detailed)](dev-notes/rust_loc_report.md)** - Detailed code analysis
- **[WFL Rust LOC Report](dev-notes/wfl_rust_loc_report.md)** - WFL-specific code metrics

## 🚀 Quick Links

### 📋 Project Overview
- **[README](../README.md)** - Project overview and quick start
- **[CHANGELOG](../CHANGELOG.md)** - Version history and changes
- **[SECURITY](../SECURITY.md)** - Security policy and vulnerability reporting
- **[LICENSE](../LICENSE)** - Apache 2.0 license details

### 🛠️ Development Resources
- **[Development Guide](../.augment/rules/DEVELOPMENT.md)** - Comprehensive guide for AI assistants and contributors
- **[TestPrograms](../TestPrograms/)** - Example WFL programs and integration tests
- **[Dev Diary](../Dev%20diary/)** - Development history and progress logs
- **[Tools](../Tools/)** - Development tools and utilities

### 🏗️ Project Structure
- **[Source Code](../src/)** - Main WFL interpreter source code
- **[LSP Server](../wfl-lsp/)** - Language Server Protocol implementation
- **[VSCode Extension](../vscode-extension/)** - Editor integration
- **[Tests](../tests/)** - Unit and integration tests

## 📝 Contributing to Documentation

When adding new documentation:

1. **Choose the right location:**
   - `wfldocs/` - Core language features (implemented) with `WFL-` prefix
   - `wflspecs/` - Planned/experimental features with `SPEC-` prefix
   - `guides/` - Tutorials, how-tos, and best practices
   - `api/` - API and library reference
   - `technical/` - Internal technical documentation
   - `dev-notes/` - Development notes and temporary docs

2. **Follow naming conventions:**
   - Use `WFL-` prefix for core language features in `wfldocs/`
   - Use `SPEC-` prefix for planned features in `wflspecs/`
   - Use lowercase, hyphen-separated names
   - Avoid cryptic abbreviations

3. **Update this index** with a link to your new document

4. **Follow the documentation policy** outlined in [guides/wfl-documentation-policy.md](guides/wfl-documentation-policy.md)

5. **Include examples** and cross-references where appropriate

6. **Align with WFL principles** from [guides/wfl-foundation.md](guides/wfl-foundation.md):
   - Use natural language descriptions
   - Prioritize clarity over brevity
   - Make documentation accessible to beginners
   - Provide clear, actionable information

## 📊 Documentation Statistics

- **Core Language Features (WFLDocs):** 11 comprehensive guides
- **Planned Features (WFLSpecs):** 1 specification document
- **User Guides:** 14 tutorials and how-tos
- **IDE Integration:** 3 LSP and editor guides
- **API Documentation:** 11 module references
- **Technical Docs:** 19 internal documents (including moved files)
- **Dev Notes:** 13 development documents (including moved LOC reports)
- **AI Resources:** 1 living AI document
- **Total Documentation:** 73 organized documents

*Last updated: September 2025 - Reorganized according to WFL documentation standards*

## 🔗 External Resources

### 🌐 Online Presence
- **[GitHub Repository](https://github.com/WebFirstLanguage/wfl)** - Source code and issue tracking
- **[GitHub Issues](https://github.com/WebFirstLanguage/wfl/issues)** - Bug reports and feature requests
- **[GitHub Discussions](https://github.com/WebFirstLanguage/wfl/discussions)** - Community discussions

### 📚 Learning Resources
- **[TestPrograms](../TestPrograms/)** - Comprehensive example programs demonstrating WFL features
- **[Examples](../examples/)** - Additional code examples and demonstrations
- **[Syntax Test](../syntax_test/)** - Syntax validation and testing files

### 🔧 Development Tools
- **[Scripts](../scripts/)** - Build and development automation scripts
- **[Tools](../Tools/)** - Development utilities and helpers
- **[Benchmarks](../benches/)** - Performance benchmarking suite

---

*This documentation is organized according to the principles in [WFL Foundation](guides/wfl-foundation.md), emphasizing natural language, clarity, and accessibility for all developers.*