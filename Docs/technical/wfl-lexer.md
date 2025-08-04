# WFL Lexer Documentation

## Overview

The WFL lexer is responsible for tokenizing source code into a stream of tokens for the parser. It's implemented using the Logos crate for efficient lexical analysis and supports WFL's natural language syntax with multi-word identifiers and minimal symbol usage.

## Architecture

### Core Components

1. **Token Module** (`src/lexer/token.rs`)
   - Token enum definition using Logos
   - Token patterns and matching rules
   - TokenWithPosition for position tracking

2. **Lexer Module** (`src/lexer/mod.rs`)
   - Main lexing functions
   - Multi-word identifier handling
   - String interning for memory efficiency
   - Position tracking

### Key Features

1. **Natural Language Support**: Handles English-like syntax with minimal symbols
2. **Multi-word Identifiers**: Combines consecutive word tokens into single identifiers
3. **Case Sensitivity**: Keywords are case-sensitive (lowercase)
4. **Comment Support**: Skips `//` and `#` style comments
5. **String Interning**: Optimizes memory usage for repeated strings
6. **Position Tracking**: Maintains line and column information for error reporting

## Implementation Details

### Token Definition

The lexer uses the Logos crate with derive macros to define tokens:

```rust
#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\f\r]+|//.*|#.*")] // Skip whitespace and comments
pub enum Token {
    #[token("\n")]
    Newline,
    #[token("store")]
    KeywordStore,
    #[token("create")]
    KeywordCreate,
    // ... more keywords
    
    // Literals
    #[regex(r#""([^"\\]|\\.)*""#, |lex| parse_string(lex))]
    StringLiteral(String),
    #[regex("[0-9]+\\.[0-9]+", |lex| lex.slice().parse::<f64>())]
    FloatLiteral(f64),
    #[regex("[0-9]+", |lex| lex.slice().parse::<i64>())]
    IntLiteral(i64),
    
    // Identifiers
    #[regex("[A-Za-z_][A-Za-z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),
    
    #[error]
    Error,
}
```

### Lexing Process

1. **Tokenization**: Logos generates a state machine for efficient token matching
2. **Multi-word Handling**: Consecutive identifier tokens are merged
3. **Position Tracking**: Each token records its line, column, and length
4. **String Interning**: Repeated strings share memory

### Multi-word Identifier Handling

The lexer implements a two-phase approach:

```rust
pub fn lex_wfl(input: &str) -> Vec<Token> {
    let mut lexer = Token::lexer(&input);
    let mut tokens = Vec::new();
    let mut current_id: Option<String> = None;
    
    while let Some(token_result) = lexer.next() {
        match token_result {
            Ok(Token::Identifier(word)) => {
                // Accumulate multi-word identifier
                if let Some(ref mut id) = current_id {
                    id.push(' ');
                    id.push_str(&word);
                } else {
                    current_id = Some(word);
                }
            }
            Ok(other) => {
                // Flush accumulated identifier
                if let Some(id) = current_id.take() {
                    tokens.push(Token::Identifier(id));
                }
                tokens.push(other);
            }
            // ... error handling
        }
    }
}
```

### Position Tracking

The lexer provides position information for error reporting:

```rust
pub struct TokenWithPosition {
    pub token: Token,
    pub line: usize,
    pub column: usize,
    pub length: usize,
}

pub fn lex_wfl_with_positions(input: &str) -> Vec<TokenWithPosition> {
    // Build line start positions for efficient lookup
    let mut line_starts = vec![0];
    for (i, c) in input.char_indices() {
        if c == '\n' {
            line_starts.push(i + 1);
        }
    }
    
    // Convert byte offset to line/column
    let position = |offset: usize| -> (usize, usize) {
        let line_idx = line_starts.binary_search(&offset)
            .unwrap_or_else(|i| i - 1);
        let line = line_idx + 1;
        let column = offset - line_starts[line_idx] + 1;
        (line, column)
    };
    // ... tokenization with position tracking
}
```

### String Interning

The lexer uses string interning to optimize memory usage:

```rust
static STRING_POOL: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn intern_string(s: String) -> String {
    let pool = STRING_POOL.get_or_init(|| Mutex::new(HashMap::new()));
    let mut pool_guard = pool.lock().unwrap();
    
    if let Some(interned) = pool_guard.get(&s) {
        interned.clone()
    } else {
        pool_guard.insert(s.clone(), s.clone());
        s
    }
}
```

This ensures that identical strings (common in identifiers and keywords) share memory.

## Token Categories

### Keywords

The lexer recognizes all WFL keywords:

1. **Variable Operations**: `store`, `create`, `change`
2. **Control Flow**: `if`, `check`, `otherwise`, `then`, `end`
3. **Loops**: `count`, `for`, `each`, `repeat`, `while`, `until`, `forever`
4. **Loop Control**: `break`, `continue`, `skip`, `exit loop`
5. **Functions**: `define`, `action`, `called`, `needs`, `give back`, `return`
6. **I/O Operations**: `open`, `close`, `read`, `write`, `append`
7. **Resources**: `file`, `directory`, `url`, `database`
8. **Operators**: `plus`, `minus`, `times`, `divided by`, `greater than`, `less than`
9. **Logical**: `and`, `or`, `not`
10. **Async**: `wait`, `for`
11. **Containers**: `container`, `interface`, `new`, `extends`, `implements`

### Literals

1. **String Literals**: Double-quoted text with escape sequences
   - Pattern: `"([^"\\]|\\.)*"`
   - Supports: `\"`, `\\`, `\n`, `\t`, etc.

2. **Number Literals**:
   - Integers: `[0-9]+`
   - Floats: `[0-9]+\.[0-9]+`
   - Future: Word forms like "1 million"

3. **Boolean Literals**:
   - `yes` / `no` (preferred)
   - `true` / `false` (synonyms)

4. **Nothing Literal**:
   - `nothing` (preferred)
   - `missing`, `undefined` (synonyms)

### Identifiers

- Pattern: `[A-Za-z_][A-Za-z0-9_]*` for single words
- Multi-word identifiers are formed by merging consecutive identifier tokens
- Cannot contain reserved keywords
- Examples: `user`, `user_name`, `user name`, `is active`

## Special Handling

### Comments

Two comment styles are supported:
- `//` - C-style line comments
- `#` - Shell-style line comments

Both are skipped during tokenization.

### Newlines

Newlines are tokenized but filtered out during the main lexing process. They're used for:
- Multi-word identifier boundary detection
- Line-based error recovery in the parser
- Position tracking

### Error Handling

The lexer handles errors gracefully:
- Unrecognized characters produce error messages with position
- Lexing continues after errors (doesn't halt)
- Position information helps with debugging

## Integration with Parser

The lexer provides two main functions:

1. **`lex_wfl(input: &str) -> Vec<Token>`**
   - Simple tokenization without position information
   - Used for quick lexing or testing

2. **`lex_wfl_with_positions(input: &str) -> Vec<TokenWithPosition>`**
   - Full tokenization with position tracking
   - Used by the parser for error reporting
   - Provides line, column, and length for each token

## Performance Optimizations

1. **String Interning**: Reduces memory usage for repeated identifiers
2. **Binary Search**: Efficient line/column lookup from byte offsets
3. **Pre-allocated Vectors**: Reduces allocations during tokenization
4. **Logos State Machine**: Efficient DFA-based token matching

## Testing

The lexer includes comprehensive tests (`src/lexer/tests.rs`):
- Multi-word identifier handling
- All token types
- Position tracking accuracy
- Error handling
- Edge cases (empty input, only comments, etc.)

## Common Usage Examples

### Basic Variable Declaration
```wfl
store user name as "Alice"
```
Tokens: `[KeywordStore, Identifier("user name"), KeywordAs, StringLiteral("Alice")]`

### Multi-word Identifiers
```wfl
create is active as yes
```
Tokens: `[KeywordCreate, Identifier("is active"), KeywordAs, BooleanLiteral(true)]`

### Natural Language Function Calls
```wfl
display length of my list
```
Tokens: `[KeywordDisplay, Identifier("length"), KeywordOf, Identifier("my list")]`

## Future Enhancements

1. **Word-form Numbers**: Support "1 million", "twenty-five", etc.
2. **Unicode Support**: Full Unicode identifier support
3. **Error Recovery**: Continue lexing with better error recovery
4. **Incremental Lexing**: Re-lex only changed portions
5. **Custom Keywords**: User-defined keywords for DSLs

## Debugging

Enable lexer debugging by using the debug build:
```bash
cargo run -- --lex program.wfl
```

This outputs the token stream for inspection.