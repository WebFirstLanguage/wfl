# WFL Architecture Diagram

## System Overview

WFL follows a traditional compiler pipeline architecture with modern enhancements for developer experience and runtime capabilities.

```
                               WFL PROCESSING PIPELINE
                               
┌─────────────┐      ┌─────────────┐      ┌─────────────┐      ┌─────────────┐      ┌─────────────┐
│             │      │             │      │             │      │             │      │             │
│    Lexer    │────> │   Parser    │────> │  Analyzer   │────> │ TypeChecker │────> │ Interpreter │
│             │      │             │      │             │      │             │      │             │
└─────────────┘      └─────────────┘      └─────────────┘      └─────────────┘      └─────────────┘
     │                    │                   │                   │                    │
     ▼                    ▼                   ▼                   ▼                    ▼
┌─────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                   Error Reporting System                                        │
│                              (Unified Diagnostics with Source Context)                         │
└─────────────────────────────────────────────────────────────────────────────────────────────────┘
```

## Detailed Component Flow

```
INPUT: .wfl Source Code
│
├─► LEXER (src/lexer/)
│   ├─ Token Generation (Logos)
│   ├─ Multi-word Identifier Merging
│   ├─ String Interning
│   └─ Position Tracking
│   │
│   ▼ Vec<TokenWithPosition>
│   
├─► PARSER (src/parser/)
│   ├─ Recursive Descent Parsing
│   ├─ AST Construction
│   ├─ Enhanced End Token Handling
│   ├─ Natural Language Syntax Support
│   └─ Error Recovery
│   │
│   ▼ Program AST
│   
├─► ANALYZER (src/analyzer/)
│   ├─ Symbol Table Management
│   ├─ Scope Tracking
│   ├─ Static Analysis
│   │   ├─ Unused Variable Detection
│   │   ├─ Unreachable Code Detection
│   │   ├─ Control Flow Analysis
│   │   └─ Variable Shadowing
│   └─ Container/Interface Validation
│   │
│   ▼ Validated AST + Symbol Table
│   
├─► TYPE CHECKER (src/typechecker/)
│   ├─ Type Inference
│   ├─ Type Compatibility Checking
│   ├─ Container Type Validation
│   └─ Function Signature Validation
│   │
│   ▼ Typed AST
│   
└─► INTERPRETER (src/interpreter/)
    ├─ Direct AST Execution
    ├─ Async/Await Support (Tokio)
    ├─ Environment Management
    ├─ Standard Library Integration
    └─ Exception Handling
    │
    ▼ Program Output
```

## Data Flow Diagram

```
┌─────────────────┐
│   Source Code   │
│    (.wfl)       │
└─────────────────┘
         │
         ▼
┌─────────────────┐    ┌──────────────────┐
│     Tokens      │    │   Position Info  │
│  [Token, ...]   │◄───┤ (Line, Column)   │
└─────────────────┘    └──────────────────┘
         │
         ▼
┌─────────────────┐    ┌──────────────────┐
│   AST Nodes     │    │   Parse Errors   │
│ Statement/Expr  │◄───┤   (Recoverable)  │
└─────────────────┘    └──────────────────┘
         │
         ▼
┌─────────────────┐    ┌──────────────────┐
│ Symbol Tables   │    │ Semantic Errors  │
│ Scope Hierarchy │◄───┤  & Warnings      │
└─────────────────┘    └──────────────────┘
         │
         ▼
┌─────────────────┐    ┌──────────────────┐
│  Type Info      │    │   Type Errors    │
│ Inferred Types  │◄───┤  & Mismatches    │
└─────────────────┘    └──────────────────┘
         │
         ▼
┌─────────────────┐    ┌──────────────────┐
│  Execution      │    │ Runtime Errors   │
│   Results       │◄───┤ & Exceptions     │
└─────────────────┘    └──────────────────┘
```

## Component Interactions

```
                    ┌─────────────────────────────────────────┐
                    │              WFL RUNTIME                │
                    └─────────────────────────────────────────┘
                                        │
        ┌───────────────────────────────┼───────────────────────────────┐
        │                               │                               │
        ▼                               ▼                               ▼
┌─────────────┐                ┌─────────────┐                ┌─────────────┐
│   TOKIO     │                │  STANDARD   │                │   ERROR     │
│  RUNTIME    │                │  LIBRARY    │                │ REPORTING   │
│             │                │             │                │             │
│ ┌─────────┐ │                │ ┌─────────┐ │                │ ┌─────────┐ │
│ │ Async   │ │                │ │  Core   │ │                │ │ Diag.   │ │
│ │ Executor│ │                │ │  Math   │ │                │ │ System  │ │
│ └─────────┘ │                │ │  Text   │ │                │ └─────────┘ │
│ ┌─────────┐ │                │ │  List   │ │                │ ┌─────────┐ │
│ │ HTTP    │ │                │ │  I/O    │ │                │ │ Source  │ │
│ │ Client  │ │                │ │  Time   │ │                │ │ Context │ │
│ └─────────┘ │                │ └─────────┘ │                │ └─────────┘ │
│ ┌─────────┐ │                │ ┌─────────┐ │                │ ┌─────────┐ │
│ │Database │ │                │ │Pattern  │ │                │ │Suggest. │ │
│ │ Access  │ │                │ │Matching │ │                │ │ Engine  │ │
│ └─────────┘ │                │ └─────────┘ │                │ └─────────┘ │
└─────────────┘                └─────────────┘                └─────────────┘
```

## Memory Management

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           MEMORY ARCHITECTURE                                │
└─────────────────────────────────────────────────────────────────────────────┘

STACK                          HEAP                           STATIC
┌─────────────┐               ┌─────────────────┐            ┌─────────────┐
│ Parser      │               │ AST Nodes       │            │ String Pool │
│ State       │               │ (Boxed)         │            │ (Interned)  │
├─────────────┤               ├─────────────────┤            ├─────────────┤
│ Local       │               │ Symbol Tables   │            │ Standard    │
│ Variables   │               │ (HashMap)       │            │ Library     │
├─────────────┤               ├─────────────────┤            ├─────────────┤
│ Function    │               │ Error Messages  │            │ Built-in    │
│ Frames      │               │ (Vec)           │            │ Functions   │
├─────────────┤               ├─────────────────┤            └─────────────┘
│ Loop        │               │ Runtime Values  │
│ Contexts    │               │ (Environment)   │
└─────────────┘               └─────────────────┘
```

## Error Flow

```
                          ERROR HANDLING PIPELINE
                          
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Lexer     │    │   Parser    │    │  Analyzer   │    │    Type     │
│   Errors    │    │   Errors    │    │   Errors    │    │   Errors    │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
     │                   │                   │                   │
     └───────────────────┼───────────────────┼───────────────────┘
                         │                   │
                         ▼                   ▼
            ┌─────────────────────────────────────────────┐
            │         DIAGNOSTIC COLLECTOR                │
            │                                             │
            │  ┌─────────────┐  ┌─────────────────────┐   │
            │  │ Error Code  │  │   Source Context    │   │
            │  │   & Span    │  │  (Line Highlight)   │   │
            │  └─────────────┘  └─────────────────────┘   │
            │                                             │
            │  ┌─────────────────────────────────────────┐ │
            │  │        Suggestion Engine            │ │
            │  │   (Fix Recommendations)            │ │
            │  └─────────────────────────────────────────┘ │
            └─────────────────────────────────────────────┘
                         │
                         ▼
            ┌─────────────────────────────────────────────┐
            │         FORMATTED OUTPUT                    │
            │                                             │
            │   Error: Undefined variable 'usr_name'     │
            │   --> program.wfl:5:12                     │
            │   |                                         │
            │ 5 | display usr_name                        │
            │   |         ^^^^^^^^                        │
            │   |                                         │
            │   = help: did you mean 'user_name'?         │
            └─────────────────────────────────────────────┘
```

## Container System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         CONTAINER SYSTEM                                     │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────┐       ┌─────────────┐       ┌─────────────┐
│ Container   │       │ Interface   │       │   Event     │
│ Definition  │──────>│ Definition  │──────>│ Definition  │
└─────────────┘       └─────────────┘       └─────────────┘
     │                       │                       │
     ▼                       ▼                       ▼
┌─────────────┐       ┌─────────────┐       ┌─────────────┐
│ Properties  │       │ Required    │       │ Parameters  │
│ & Methods   │       │ Actions     │       │ & Handlers  │
└─────────────┘       └─────────────┘       └─────────────┘
     │                       │                       │
     └───────────────────────┼───────────────────────┘
                             │
                             ▼
                   ┌─────────────────┐
                   │   Inheritance   │
                   │   Hierarchy     │
                   │                 │
                   │  ┌───────────┐  │
                   │  │   Base    │  │
                   │  └───────────┘  │
                   │       │         │
                   │  ┌───────────┐  │
                   │  │ Derived   │  │
                   │  └───────────┘  │
                   └─────────────────┘
```

## Development Tools Integration

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           DEVELOPMENT ECOSYSTEM                              │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│    REPL     │     │    Linter   │     │    Fixer    │     │     LSP     │
│ Interactive │     │   (Code     │     │ (Auto Fix)  │     │   Server    │
│   Shell     │     │  Quality)   │     │             │     │             │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
     │                   │                   │                   │
     └───────────────────┼───────────────────┼───────────────────┘
                         │                   │
                         ▼                   ▼
            ┌─────────────────────────────────────────────┐
            │           WFL CORE ENGINE                   │
            │                                             │
            │  ┌─────────────────────────────────────────┐ │
            │  │     Lexer → Parser → Analyzer         │ │
            │  └─────────────────────────────────────────┘ │
            │                                             │
            │  ┌─────────────────────────────────────────┐ │
            │  │      Type Checker → Interpreter       │ │
            │  └─────────────────────────────────────────┘ │
            └─────────────────────────────────────────────┘
                         │
                         ▼
            ┌─────────────────────────────────────────────┐
            │            IDE INTEGRATION                  │
            │                                             │
            │  ┌─────────────┐  ┌─────────────────────┐   │
            │  │  VSCode     │  │   Real-time Error   │   │
            │  │ Extension   │  │     Checking        │   │
            │  └─────────────┘  └─────────────────────┘   │
            │                                             │
            │  ┌─────────────────────────────────────────┐ │
            │  │       Auto-completion & Hover       │ │
            │  └─────────────────────────────────────────┘ │
            └─────────────────────────────────────────────┘
```

## Performance Characteristics

```
COMPONENT PERFORMANCE PROFILE

Lexer:        O(n) - Linear scan with string interning
Parser:       O(n) - Single pass recursive descent
Analyzer:     O(n²) - Symbol resolution with scoping
TypeChecker:  O(n) - Single pass type inference
Interpreter:  O(n) - Direct AST execution

MEMORY USAGE:
┌─────────────────────────────────────────────────────────┐
│ Component     │ Memory Usage    │ Optimization          │
├─────────────────────────────────────────────────────────┤
│ Lexer         │ O(n)           │ String interning       │
│ Parser        │ O(n)           │ Boxed AST nodes        │
│ Analyzer      │ O(n)           │ Scope hierarchy        │
│ TypeChecker   │ O(n)           │ Type caching           │
│ Interpreter   │ O(n)           │ Environment cleanup    │
└─────────────────────────────────────────────────────────┘
```

## Future Architecture (Planned)

```
                        FUTURE BYTECODE ARCHITECTURE
                        
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│             │    │             │    │             │    │             │
│   Parser    │───>│  Bytecode   │───>│   Bytecode  │───>│     VM      │
│    AST      │    │  Compiler   │    │ Instructions│    │  Executor   │
│             │    │             │    │             │    │             │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
                         │                   │                  │
                         ▼                   ▼                  ▼
                   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
                   │Optimization │    │   Constant  │    │ Just-In-Time│
                   │   Passes    │    │   Folding   │    │  Compiler   │
                   └─────────────┘    └─────────────┘    └─────────────┘
```

This architecture diagram provides a comprehensive view of WFL's processing pipeline, component interactions, and data flow patterns. Each component is designed for maintainability, performance, and extensibility while preserving backward compatibility.