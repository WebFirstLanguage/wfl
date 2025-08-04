# WFL Architecture Documentation

## System Architecture Overview

WFL is implemented as a traditional language processing pipeline with modern enhancements for developer experience and runtime capabilities.

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│    Lexer    │────>│   Parser    │────>│  Analyzer   │────>│ TypeChecker │────>│ Interpreter │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
     │                    │                   │                   │                    │
     ▼                    ▼                   ▼                   ▼                    ▼
┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│                                   Error Reporting System                                    │
└─────────────────────────────────────────────────────────────────────────────────────────────┘
```

## Core Design Philosophy

A fundamental architectural principle of WFL is **backward compatibility by design**. The system is built to adapt to existing WFL code rather than requiring users to modify their code as the language evolves. This principle influences every component of the architecture:

1. **Robust Error Recovery**: The parser and analyzer include sophisticated error recovery mechanisms to handle unexpected or non-standard syntax
2. **Flexible Interpretation**: The interpreter adapts to varying coding styles and patterns
3. **Adaptive Analysis**: Static analysis tools detect issues without demanding code changes
4. **Progressive Enhancement**: New features build upon existing syntax rather than replacing it

## Core Components

### 1. Lexer (`src/lexer/`)
- **Purpose**: Converts source code text into tokens
- **Implementation**: Based on Logos for efficient tokenization
- **Key Features**:
  - Full support for natural language constructs
  - Error recovery with context-aware diagnostics
  - Position tracking for accurate error reporting

### 2. Parser (`src/parser/`)
- **Purpose**: Transforms token stream into Abstract Syntax Tree (AST)
- **Key Features**:
  - Recursive descent parsing with error recovery
  - Enhanced end token handling (critical stability fix, May 2025)
  - Support for natural language function calls
  - Comprehensive token lookahead with proper borrow checking
  - Advanced error recovery for backward compatibility with existing code

### 3. Semantic Analyzer (`src/analyzer/`)
- **Purpose**: Analyzes AST for semantic correctness
- **Key Features**:
  - Unused variable detection
  - Unreachable code detection
  - Variable shadowing analysis
  - Inconsistent return path detection
  - Control flow graph generation and analysis
  - Adaptive variable usage detection in various contexts (I/O statements, action calls)

### 4. Type Checker (`src/typechecker/`)
- **Purpose**: Performs static type analysis
- **Key Features**:
  - Type inference
  - Type compatibility checking
  - Error reporting with suggestions
  - Flexible type handling to maintain backward compatibility

### 5. Interpreter (`src/interpreter/`)
- **Purpose**: Executes the AST
- **Implementation**: Direct AST interpretation with Tokio integration
- **Key Features**:
  - Full async/await support
  - HTTP requests via Reqwest
  - Database access via SQLx
  - Try/when/otherwise exception handling
  - Environment management with proper scoping
  - Adaptability to different coding styles and syntax variations

### 6. Standard Library (`src/stdlib/`)
- **Modules**:
  - Core: Basic operations and utilities
  - Math: Mathematical operations
  - Text: String manipulation
  - List: Collection operations
  - Pattern: Regular expression and pattern matching
  - I/O: File operations and network access

### 7. Error Reporting System (`src/diagnostics/`)
- **Purpose**: Comprehensive error reporting
- **Implementation**: Based on codespan-reporting
- **Key Features**:
  - Source context with highlighting
  - Actionable suggestions
  - Unified error formatting
  - Helpful diagnostics that don't demand code changes

### 8. AI Integration System
- **Purpose**: Enhance development and research capabilities
- **Components**:
  - Claude Code integration for code review and assistance
  - Gemini API for deep technical research
  - Memory Bank system for context preservation
- **Key Features**:
  - GitHub Actions integration for CI/CD
  - Structured knowledge base in `.kilocode/rules/memory-bank/`
  - Context-aware assistance for development tasks

## Development Tools

### 1. Linter & Code Fixer (`src/linter/`, `src/fixer/`)
- **Purpose**: Code quality tools
- **Key Features**:
  - Style checking
  - Best practice enforcement
  - Automatic code fixes
  - Suggestions that respect backward compatibility

### 2. Logging System (`src/logging/`)
- **Purpose**: Debug output and tracing
- **Key Features**:
  - Standardized exec_trace! macro
  - Clean separation of debug and program output
  - Memory optimization

### 3. REPL (`src/repl/`)
- **Purpose**: Interactive development
- **Key Features**:
  - Command history
  - Multi-line editing
  - Immediate feedback

### 4. LSP Server (`wfl-lsp/`)
- **Purpose**: IDE integration
- **Key Features**:
  - Real-time diagnostics
  - Auto-completion
  - Go-to definition
  - Hover information

## File Organization

```
wfl/
├── src/                # Source code
│   ├── lexer/          # Lexical analyzer
│   ├── parser/         # Parser and AST definition
│   ├── analyzer/       # Semantic analyzer
│   ├── typechecker/    # Type checker
│   ├── interpreter/    # Runtime interpreter
│   ├── stdlib/         # Standard library
│   ├── diagnostics/    # Error reporting
│   ├── linter/         # Code quality tools
│   ├── fixer/          # Automatic code fixes
│   ├── logging/        # Logging system
│   └── repl/           # Interactive shell
├── Docs/               # Documentation
├── Test Programs/      # Example programs and tests
├── wfl-lsp/            # Language Server Protocol implementation
├── vscode-wfl/         # VSCode extension
├── .kilocode/          # AI assistant rules and memory bank
│   └── rules/          # Rules for AI assistants
│       └── memory-bank/# Structured knowledge base
└── Tools/              # Utility scripts
```

## Key Design Patterns

1. **Visitor Pattern**: Used in the analyzer and interpreter to traverse the AST
2. **Builder Pattern**: Used in AST construction
3. **Command Pattern**: Used in the REPL for command history
4. **Observer Pattern**: Used in error reporting and logging
5. **Factory Pattern**: Used in standard library function registration
6. **Adapter Pattern**: Used to accommodate different syntax forms for backward compatibility

## Critical Implementation Paths

1. **Execution Pipeline**:
   - Source → Lexer → Parser → Analyzer → Type Checker → Interpreter
   - All runs are type-checked and semantically analyzed by default

2. **Error Recovery**:
   - Parser synchronization points
   - Context-aware error reporting
   - Graceful degradation in analysis
   - Smart error recovery to handle syntax variations without breaking

3. **Async Execution**:
   - Tokio runtime initialization
   - Task spawning and management
   - Future resolution and handling

4. **Memory Management**:
   - Environment hierarchies with weak references
   - Efficient string and value representation
   - Memory leak prevention in closures

5. **Backward Compatibility**:
   - Parser designed to handle syntax variations
   - Enhanced end token handling for improved resilience
   - Variable usage detection in various contexts (I/O, action calls)
   - Flexible type checking with adaptable rules

6. **AI Integration**:
   - GitHub Actions workflow for Claude Code
   - Memory Bank system for context preservation
   - Structured knowledge organization for AI assistants

## Future Architecture Plans

1. **Bytecode Compiler**:
   - Convert AST to bytecode instructions
   - Optimization passes
   - Constant folding and dead code elimination

2. **Virtual Machine**:
   - Register-based VM
   - JIT compilation support
   - Performance optimizations

3. **Enhanced AI Integration**:
   - Deeper integration with development workflow
   - Automated code review and suggestions
   - Context-aware assistance for complex tasks