# Compiler Internals

Deep dive into WFL's implementation. For contributors and those curious about how WFL works.

## Pipeline Overview

```
Source Code
    ↓
Lexer (logos crate)
    ↓
Token Stream
    ↓
Parser (recursive descent)
    ↓
AST (Abstract Syntax Tree)
    ↓
Analyzer (semantic validation)
    ↓
Type Checker (static types)
    ↓
Interpreter (Tokio async)
    ↓
Execution / Output
```

## Lexer Implementation

**File:** `src/lexer/mod.rs`

**Technology:** Logos crate (proc macro)

**How it works:**
1. Logos generates optimized state machine from token definitions
2. Processes source byte-by-byte
3. Produces Token enum variants
4. Tracks line/column positions

**Contextual keywords:** Words that are keywords in some contexts, identifiers in others.

**Performance:** Very fast (compiled state machine)

## Parser Design

**File:** `src/parser/mod.rs`

**Algorithm:** Recursive descent

**How it works:**
1. Consumes tokens from lexer
2. Builds AST nodes recursively
3. Natural language construct handling
4. Error recovery (continues after errors)

**AST:** Defined in `src/parser/ast.rs`

**Special parsers:**
- Container parser for OOP constructs
- Pattern parser for pattern expressions

## Analyzer

**File:** `src/analyzer/mod.rs`

**Semantic checks:**
- Variable defined before use
- Action defined before call
- Scope validation
- Dead code detection
- Unused variable warnings

**Traverses AST** to build symbol tables and validate semantics.

## Type Checker

**File:** `src/typechecker/mod.rs`

**Type inference:**
- Literal types from values
- Variable types from assignments
- Expression types from operands
- Action return types from returns

**Type checking:**
- Operation type compatibility
- Function call type matching
- Assignment type consistency

**Built-in types:** Registered in `src/builtins.rs` with arity checking.

## Interpreter

**File:** `src/interpreter/mod.rs`

**Execution model:** Direct AST interpretation

**Runtime:** Tokio async runtime

**Key components:**
- Environment (scope/variables)
- Value enum (runtime values)
- Error handling
- Async operation support

**How execution works:**
1. Traverse AST
2. Evaluate expressions
3. Execute statements
4. Manage scope (push/pop environments)
5. Handle async operations

## Pattern Engine

**File:** `src/pattern/`

**Architecture:** Bytecode VM

**Components:**
- Compiler: Pattern AST → bytecode
- VM: Execute pattern bytecode
- Match engine: Track state during matching

**Why bytecode:** Performance and complexity management

## Standard Library

**File:** `src/stdlib/mod.rs`

**Organization:** One module per category (math, text, list, etc.)

**Registration:** `register_stdlib()` adds all functions to environment

**Native functions:** Rust implementations called from WFL

## Memory Management

**Strategy:** Rust ownership + Rc<RefCell<T>> for shared mutable state

**No GC:** Rust's ownership prevents leaks

**Shared data:**
- Lists: `Rc<RefCell<Vec<Value>>>`
- Text: `Rc<String>`
- Containers: Reference counted

**Security:** Crypto module uses `zeroize` for sensitive data

## Error Recovery

**Lexer:** Skips invalid input, continues tokenizing
**Parser:** Records error, attempts recovery, continues parsing
**Result:** Multiple errors reported in one pass

## Performance

**Current:** Interpreted (no bytecode yet)
**Optimization:** Short-circuit evaluation automatic
**Future:** Bytecode VM planned

---

**Previous:** [← MCP Integration](mcp-integration.md) | **Next:** [Contributing Guide →](contributing-guide.md)
