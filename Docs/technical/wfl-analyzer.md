# WFL Analyzer Documentation

## Overview

The WFL Analyzer performs semantic analysis on the Abstract Syntax Tree (AST) produced by the parser. It validates program semantics, manages symbol tables, performs static analysis, and prepares the AST for type checking and interpretation.

## Architecture

### Core Components

1. **Semantic Analyzer** (`src/analyzer/mod.rs`)
   - Symbol table management
   - Scope tracking
   - Variable resolution
   - Container and interface tracking

2. **Static Analyzer** (`src/analyzer/static_analyzer.rs`)
   - Control flow analysis
   - Unused variable detection
   - Unreachable code detection
   - Variable shadowing checks
   - Return path consistency

### Key Data Structures

#### Symbol Table

```rust
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub symbol_type: Option<Type>,
    pub line: usize,
    pub column: usize,
}

pub enum SymbolKind {
    Variable { mutable: bool },
    Function {
        parameters: Vec<Parameter>,
        return_type: Option<Type>,
    },
}
```

#### Scope Management

```rust
pub struct Scope {
    pub symbols: HashMap<String, Symbol>,
    pub parent: Option<Box<Scope>>,
}
```

Scopes form a hierarchical structure:
- Global scope contains built-in symbols
- Function scopes inherit from their parent
- Block scopes (if/loop bodies) create nested scopes

#### Container Information

```rust
pub struct ContainerInfo {
    pub name: String,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub properties: HashMap<String, PropertyInfo>,
    pub methods: HashMap<String, MethodInfo>,
    pub static_properties: HashMap<String, PropertyInfo>,
    pub static_methods: HashMap<String, MethodInfo>,
}
```

## Semantic Analysis Features

### 1. Symbol Resolution

The analyzer maintains a symbol table for:
- Variable declarations
- Function definitions
- Container and interface definitions
- Built-in symbols (yes, no, nothing, etc.)

```rust
impl Analyzer {
    pub fn new() -> Self {
        let mut global_scope = Scope::new();
        
        // Define built-in symbols
        let yes_symbol = Symbol {
            name: "yes".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Boolean),
            line: 0,
            column: 0,
        };
        global_scope.define(yes_symbol);
        // ... more built-ins
    }
}
```

### 2. Scope Management

The analyzer tracks lexical scopes:
- Enters new scope for functions, loops, conditions
- Resolves variables through scope chain
- Detects variable shadowing
- Manages scope lifecycle

### 3. Container Analysis

For object-oriented features:
- Validates container definitions
- Checks inheritance relationships
- Verifies interface implementations
- Tracks property and method visibility

### 4. Expression Analysis

Validates expressions for:
- Variable existence
- Function call validity
- Member access correctness
- Operator usage

## Static Analysis Features

### 1. Control Flow Graph (CFG)

The analyzer builds a control flow graph to understand program flow:

```rust
enum CFGNode {
    Entry,
    Exit,
    Statement { stmt_idx, line, column },
    Branch { condition_idx, then_branch, else_branch, line, column },
}

struct ControlFlowGraph {
    nodes: Vec<CFGNode>,
    edges: HashMap<usize, Vec<usize>>,
    reachable: HashSet<usize>,
}
```

### 2. Unused Variable Detection

Tracks variable usage throughout the program:
- Marks variables when declared
- Records usage in expressions
- Reports unused variables with precise locations
- Handles special cases (action parameters, loop counters)

### 3. Unreachable Code Detection

Uses CFG to identify unreachable code:
- Code after unconditional returns
- Code in impossible branches
- Dead loops
- Orphaned statements

### 4. Variable Shadowing

Detects when variables shadow outer scope variables:
```wfl
store x as 10
if x > 5:
    store x as 20  // Warning: shadows outer 'x'
end if
```

### 5. Return Path Analysis

Ensures consistent return behavior:
- All paths through a function return values
- Return types are consistent
- No missing returns in branches

## Variable Usage Tracking

The analyzer tracks variable usage in various contexts:

### Standard Usage
- Expressions: `display x`
- Assignments: `change x to 10`
- Conditions: `if x > 5`

### I/O Operations
- File operations: `append content message_text into file`
- Web operations: `wait for web.get(url)`
- Database queries: `wait for database.query(sql)`

### Action Calls
- Parameters: `calculate sum of x and y`
- Natural language calls: `length of mylist`

## Error Reporting

The analyzer produces semantic errors with:
- Precise location (line and column)
- Descriptive messages
- Error categories for filtering

```rust
pub struct SemanticError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}
```

## Integration with Other Components

### Parser Integration

The analyzer receives:
- Complete AST from parser
- Position information for all nodes
- Container and interface definitions

### Type Checker Integration

The analyzer provides:
- Symbol table with type information
- Container hierarchy
- Resolved function signatures

### Diagnostic System

Produces diagnostics with severity levels:
- Error: Semantic violations
- Warning: Potential issues (unused variables)
- Info: Suggestions for improvement

## Analysis Passes

The analyzer performs multiple passes:

### Pass 1: Symbol Collection
- Collect all declarations
- Build container hierarchy
- Register function signatures

### Pass 2: Resolution and Validation
- Resolve all variable references
- Validate function calls
- Check container relationships

### Pass 3: Static Analysis
- Build control flow graph
- Detect unused variables
- Find unreachable code
- Check return paths

## Special Handling

### Action Parameters

The analyzer handles space-separated parameter names:
```wfl
define action called validate needs label expected actual:
    // 'label', 'expected', and 'actual' are all parameters
end action
```

### Built-in Variables

Certain variables are always defined:
- `yes`, `no` (boolean literals)
- `nothing`, `missing`, `undefined` (null values)
- Loop counters (`count`, `loopcounter`)

### Dynamic Features

The analyzer adapts to:
- Runtime-defined variables
- Dynamic function calls
- Flexible typing

## Performance Considerations

1. **Incremental Analysis**: Future support for analyzing only changed portions
2. **Caching**: Symbol table caching for repeated analyses
3. **Parallel Analysis**: Independent modules can be analyzed concurrently
4. **Memory Efficiency**: Careful management of scope lifetimes

## Testing

The analyzer includes tests for:
- Symbol resolution edge cases
- Scope management scenarios
- Static analysis accuracy
- Error reporting precision
- Performance benchmarks

## Future Enhancements

1. **Data Flow Analysis**: Track value flow through programs
2. **Type Inference**: Improve type deduction
3. **Security Analysis**: Detect potential vulnerabilities
4. **Optimization Hints**: Suggest performance improvements
5. **Cross-Module Analysis**: Analyze dependencies between files

## Debugging

Enable analyzer tracing:
```bash
cargo run -- --analyze program.wfl
```

This provides detailed analysis results including:
- Symbol table contents
- Scope hierarchy
- Static analysis warnings
- Control flow visualization