# Architecture Overview

How WFL works internally. Understanding the compiler pipeline and key components.

## Compiler Pipeline

```
Source Code → Lexer → Parser → Analyzer → Type Checker → Interpreter
              ↓       ↓         ↓           ↓              ↓
            Tokens   AST    Validated   Type Info      Execution
```

## Components

### Lexer (`src/lexer/`)

**Purpose:** Tokenize source code

**Technology:** Logos crate (high-performance lexer generator)

**Input:** Raw WFL source code
**Output:** Token stream

**Tokens:**
- Keywords (store, check, display)
- Identifiers (variable names)
- Literals (numbers, strings)
- Operators (plus, is, with)

**Features:**
- Contextual keyword handling
- Error recovery
- Fast (optimized state machine)

---

### Parser (`src/parser/`)

**Purpose:** Build Abstract Syntax Tree (AST)

**Algorithm:** Recursive descent with error recovery

**Input:** Token stream
**Output:** AST (syntax tree)

**Features:**
- Natural language construct parsing
- Container/pattern specialized parsers
- Helpful error messages with location
- Error recovery for multiple errors

**Key files:**
- `mod.rs` - Main parser
- `container_parser.rs` - Container syntax
- `ast.rs` - AST definitions

---

### Analyzer (`src/analyzer/`)

**Purpose:** Semantic validation and static analysis

**Input:** AST
**Output:** Validated AST with semantic information

**Checks:**
- Undefined variables
- Undefined actions
- Scope validation
- Dead code detection
- Unused variables

---

### Type Checker (`src/typechecker/`)

**Purpose:** Static type analysis

**Input:** Validated AST
**Output:** Type-annotated AST

**Features:**
- Type inference
- Type mismatch detection
- Function arity checking
- Built-in function validation

---

### Interpreter (`src/interpreter/`)

**Purpose:** Execute WFL programs

**Technology:** Direct AST interpretation with Tokio runtime

**Features:**
- Async/await support (Tokio)
- Web server support (Warp framework)
- Subprocess execution
- Environment/scope management
- Error handling

**Key files:**
- `mod.rs` - Main interpreter
- `environment.rs` - Variable scope
- `value.rs` - Runtime values
- `error.rs` - Runtime errors

---

### Pattern Module (`src/pattern/`)

**Purpose:** Pattern matching engine

**Architecture:** Bytecode VM

**Components:**
- Pattern compiler (patterns to bytecode)
- VM executor (runs bytecode)
- Unicode support
- Capture groups

**Similar to:** Regex engines, but with natural language syntax

---

### Standard Library (`src/stdlib/`)

**Purpose:** Built-in functions (181+)

**Modules:**
- core.rs (3 functions)
- math.rs (5 functions)
- text.rs (8 functions)
- list.rs (5 functions)
- filesystem.rs (20+ functions)
- time.rs (14 functions)
- random.rs (6 functions)
- crypto.rs (4 functions)
- pattern.rs (3 functions)

**Registration:** `mod.rs` registers all modules

---

### LSP Server (`wfl-lsp/`)

**Purpose:** Language Server Protocol for IDE integration

**Technology:** tower-lsp crate

**Features:**
- Real-time diagnostics
- Auto-completion
- Go-to definition
- Hover documentation
- MCP server mode (--mcp flag)

**MCP Tools:**
- parse_wfl
- analyze_wfl
- typecheck_wfl
- lint_wfl
- get_completions
- get_symbol_info

---

## Data Flow

### Compilation

```
source.wfl
    → Lexer: Creates tokens
    → Parser: Builds AST
    → Analyzer: Validates semantics
    → Type Checker: Verifies types
```

### Execution

```
AST
    → Interpreter: Executes statements
    → Standard Library: Calls built-in functions
    → Async Runtime (Tokio): Handles I/O
    → Output: Results displayed/returned
```

## Key Dependencies

- **logos** - Lexer generation
- **tokio** - Async runtime
- **warp** - Web server
- **tower-lsp** - LSP server
- **chrono** - Time/date
- **rand** - Random numbers
- **zeroize, subtle** - Crypto security

## Memory Model

**Values:**
- Numbers: 64-bit float
- Text: Rc<String> (reference counted)
- Lists: Rc<RefCell<Vec<Value>>> (shared mutable)
- Containers: Rc<RefCell<HashMap>> (shared state)

**Rust safety:** Memory-safe, no garbage collector needed

---

**Previous:** [← Contributing Guide](contributing-guide.md) | **Next:** [LSP Integration →](lsp-integration.md)
