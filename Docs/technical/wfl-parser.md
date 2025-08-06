# WFL Parser Documentation

## Overview

The WFL parser is a recursive descent parser that transforms a stream of tokens from the lexer into an Abstract Syntax Tree (AST). It implements sophisticated error recovery mechanisms and supports WFL's natural language syntax while maintaining backward compatibility with existing code.

## Architecture

### Core Components

1. **Parser Structure** (`src/parser/mod.rs`)
   - Main parser implementation using recursive descent
   - Error collection and reporting
   - Action tracking for function resolution

2. **AST Definitions** (`src/parser/ast.rs`)
   - Complete AST node types
   - Container and OOP support structures
   - Position information for error reporting

3. **Container Parser** (`src/parser/container_parser.rs`)
   - Specialized parsing for container definitions
   - Interface parsing
   - Property and method parsing

### Key Design Principles

1. **Backward Compatibility**: The parser is designed to handle various syntax forms without breaking existing code
2. **Error Recovery**: Sophisticated mechanisms to continue parsing after errors
3. **Natural Language Support**: Handles multi-word identifiers and English-like syntax
4. **Position Tracking**: Maintains line and column information for accurate error reporting

## Implementation Details

### Parser Initialization

```rust
pub struct Parser<'a> {
    tokens: Peekable<Iter<'a, TokenWithPosition>>,
    errors: Vec<ParseError>,
    known_actions: std::collections::HashSet<String>,
}
```

The parser maintains:
- A peekable iterator over tokens for lookahead
- An error collection for reporting multiple issues
- A set of known actions for function resolution

### Enhanced End Token Handling (May 2025 Fix)

One of the critical stability improvements is the enhanced end token handling that prevents infinite loops:

```rust
// Comprehensive handling of "end" tokens that might be left unconsumed
let mut tokens_clone = self.tokens.clone();
if let Some(first_token) = tokens_clone.next() {
    if first_token.token == Token::KeywordEnd {
        if let Some(second_token) = tokens_clone.next() {
            match &second_token.token {
                Token::KeywordAction => {
                    exec_trace!("Consuming orphaned 'end action' at line {}", first_token.line);
                    self.tokens.next(); // Consume "end"
                    self.tokens.next(); // Consume "action"
                    continue;
                }
                // ... similar handling for other block endings
            }
        }
    }
}
```

This mechanism:
- Detects orphaned "end" tokens during parsing
- Consumes them appropriately based on context
- Prevents parser stalls on malformed input
- Maintains parsing flow for better error recovery

### Statement Parsing

The parser uses a dispatch mechanism to parse different statement types:

```rust
fn parse_statement(&mut self) -> Result<Statement, ParseError> {
    match token {
        Token::KeywordStore | Token::KeywordCreate => self.parse_variable_declaration(),
        Token::KeywordDisplay => self.parse_display_statement(),
        Token::KeywordCheck | Token::KeywordIf => self.parse_if_statement(),
        Token::KeywordCount => self.parse_count_loop(),
        Token::KeywordFor => self.parse_for_loop(),
        Token::KeywordDefine => self.parse_action_definition(),
        // ... more statement types
    }
}
```

### Natural Language Parsing

The parser supports WFL's natural language constructs:

1. **Multi-word Identifiers**: Already tokenized by the lexer, the parser treats them as single units
2. **Function Calls with "of" Syntax**: `length of mylist` is parsed as a function call
3. **Comparison Operators**: `is greater than`, `is equal to`, etc.
4. **Assignment Variations**: Both `store X as Y` and `change X to Y`

### Expression Parsing

Expression parsing follows standard precedence rules while supporting natural language:

```rust
fn parse_expression(&mut self) -> Result<Expression, ParseError> {
    self.parse_logical_or()
}

fn parse_logical_or(&mut self) -> Result<Expression, ParseError> {
    let mut left = self.parse_logical_and()?;
    while self.peek_token() == Some(&Token::KeywordOr) {
        self.tokens.next();
        let right = self.parse_logical_and()?;
        left = Expression::LogicalOp {
            left: Box::new(left),
            op: LogicalOperator::Or,
            right: Box::new(right),
            // ... position info
        };
    }
    Ok(left)
}
```

### Error Recovery

The parser implements several error recovery strategies:

1. **Synchronization Points**: After an error, skip tokens until a statement starter
2. **Orphaned Token Handling**: Consume stray "end" tokens and other orphaned constructs
3. **Line-based Recovery**: Use line boundaries as natural synchronization points
4. **Progress Assertion**: Ensures the parser always makes progress to prevent infinite loops

```rust
assert!(
    end_len < start_len,
    "Parser made no progress - token {:?} caused infinite loop",
    self.tokens.peek()
);
```

## AST Structure

### Core Node Types

1. **Program**: Root node containing a vector of statements
2. **Statement**: Enum of all possible statement types
3. **Expression**: Enum of all expression types
4. **Type**: Type annotations and inference information

### Statement Types

```rust
pub enum Statement {
    VariableDeclaration { name, value, line, column },
    Assignment { name, value, line, column },
    IfStatement { condition, then_body, else_body, line, column },
    CountLoop { variable, start, end, step, body, line, column },
    ForEachLoop { variable, collection, body, line, column },
    WhileLoop { condition, body, line, column },
    RepeatLoop { condition, body, is_until, line, column },
    ActionDefinition { name, parameters, body, line, column },
    ActionCall { name, arguments, line, column },
    Display { expressions, newline, line, column },
    ContainerDefinition { /* container fields */ },
    // ... more statement types
}
```

### Expression Types

```rust
pub enum Expression {
    Identifier { name, line, column },
    StringLiteral { value, line, column },
    NumberLiteral { value, line, column },
    BooleanLiteral { value, line, column },
    NothingLiteral { line, column },
    BinaryOp { left, op, right, line, column },
    UnaryOp { op, operand, line, column },
    FunctionCall { name, args, line, column },
    MemberAccess { object, property, line, column },
    ListLiteral { elements, line, column },
    // ... more expression types
}
```

## Container System Support

The parser fully supports WFL's object-oriented features:

### Container Definition Parsing

```rust
fn parse_container_definition(&mut self) -> Result<Statement, ParseError> {
    // Parse: create container Name [extends Parent] [implements Interface1, Interface2]:
    let name = self.parse_identifier()?;
    let (extends, implements) = self.parse_inheritance()?;
    self.expect_token(Token::Colon)?;
    let (properties, methods, events, static_properties, static_methods) = 
        self.parse_container_body()?;
    // ... create ContainerDefinition statement
}
```

### Property Definitions

Properties support:
- Type annotations
- Default values
- Validation rules
- Visibility modifiers (public/private)
- Static properties

### Method Definitions

Methods are parsed as actions within container scope:
- Constructor special handling
- Parameter lists
- Return types
- Static methods

## Integration with Other Components

### Lexer Integration

The parser receives a vector of `TokenWithPosition` from the lexer:
- Maintains position information throughout parsing
- Uses token positions for error reporting
- Handles multi-word identifiers as single tokens

### Analyzer Integration

The parser produces an AST that the analyzer validates:
- All nodes include position information
- Structure supports semantic analysis
- Type information slots for type checker

### Error Reporting

Parse errors include:
- Precise position (line and column)
- Descriptive error messages
- Context for better debugging

## Common Patterns

### Parsing Optional Constructs

```rust
// Parse optional "with" clause
let with_value = if self.peek_token() == Some(&Token::KeywordWith) {
    self.tokens.next(); // Consume "with"
    Some(self.parse_expression()?)
} else {
    None
};
```

### Parsing Lists

```rust
// Parse comma-separated list
let mut items = vec![self.parse_item()?];
while self.peek_token() == Some(&Token::Comma) {
    self.tokens.next(); // Consume comma
    items.push(self.parse_item()?);
}
```

### Expecting Specific Tokens

```rust
fn expect_token(&mut self, expected: Token, message: &str) -> Result<(), ParseError> {
    if let Some(token) = self.tokens.peek() {
        if token.token == expected {
            self.tokens.next();
            Ok(())
        } else {
            Err(ParseError::new(message.to_string(), token.line, token.column))
        }
    } else {
        Err(ParseError::new("Unexpected end of input".to_string(), 0, 0))
    }
}
```

## Future Enhancements

1. **Incremental Parsing**: Parse only changed portions for better IDE performance
2. **Parser Combinators**: Consider migration to parser combinator library
3. **Better Error Messages**: Include suggestions for common mistakes
4. **Parallel Parsing**: Parse independent sections concurrently
5. **AST Optimization**: Fold constants and simplify during parsing

## Testing

The parser includes comprehensive tests in `src/parser/tests.rs`:
- Unit tests for each parsing function
- Integration tests for complete programs
- Error recovery tests
- Edge case handling
- Performance benchmarks

## Performance Considerations

1. **Memory Usage**: AST nodes are boxed to reduce stack usage
2. **Lookahead**: Limited to one token for O(n) parsing
3. **Error Collection**: Bounded error vector to prevent memory issues
4. **String Handling**: Careful string allocation and cloning

## Debugging

Enable parser tracing with the `exec_trace!` macro:
```bash
cargo run -- --debug program.wfl > debug.txt 2>&1
```

This provides detailed parsing decisions and token consumption.

## Pattern Parsing System (Phase 1 - August 2025)

### Overview

The WFL parser includes a comprehensive pattern matching system that parses natural language pattern definitions into structured ASTs. This system is part of Phase 1 of the pattern matching implementation and provides the foundation for WFL's readable pattern syntax.

### Pattern AST Structure

The pattern system introduces several new AST nodes in `src/parser/ast.rs`:

#### Core Pattern Types

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum PatternExpression {
    /// Literal text to match exactly
    Literal(String),
    /// Character class (digit, letter, whitespace)
    CharacterClass(CharClass),
    /// A quantified pattern (e.g., "one or more digit")
    Quantified {
        pattern: Box<PatternExpression>,
        quantifier: Quantifier,
    },
    /// A sequence of patterns (e.g., "digit '-' digit")
    Sequence(Vec<PatternExpression>),
    /// Alternative patterns (e.g., "letter or digit")
    Alternative(Vec<PatternExpression>),
    /// Named capture group
    Capture {
        name: String,
        pattern: Box<PatternExpression>,
    },
    /// Anchor pattern (start/end of text)
    Anchor(Anchor),
}
```

#### Supporting Types

- **`CharClass`**: Digit, Letter, Whitespace
- **`Quantifier`**: Optional, ZeroOrMore, OneOrMore, Exactly(u32), Between(u32, u32)
- **`Anchor`**: StartOfText, EndOfText

#### Pattern Definition Statement

```rust
PatternDefinition {
    name: String,
    pattern: PatternExpression,
    line: usize,
    column: usize,
}
```

### Parsing Implementation

#### Entry Point

Pattern parsing is integrated into the main statement parser through the `create pattern` syntax:

```rust
Token::KeywordCreate => {
    // Check if it's "create pattern"
    if next_token == Token::KeywordPattern {
        self.parse_create_pattern_statement()
    }
    // ... other create variants
}
```

#### Core Parsing Functions

1. **`parse_pattern_tokens`**: Main entry point that converts token streams to PatternExpression
2. **`parse_pattern_sequence`**: Handles alternation with "or" operators
3. **`parse_pattern_concatenation`**: Handles sequences of pattern elements
4. **`parse_pattern_element`**: Parses individual pattern components
5. **`parse_quantifier`**: Handles post-element quantifiers (exactly, between)

#### Natural Language Quantifier Handling

The parser handles multi-word quantifiers by looking ahead for complete phrases:

```rust
Token::KeywordOne => {
    if tokens[i+1] == Token::KeywordOr && tokens[i+2] == Token::KeywordMore {
        // Parse "one or more" as a quantifier
        let base_element = Self::parse_pattern_element(tokens, i)?;
        PatternExpression::Quantified {
            pattern: Box::new(base_element),
            quantifier: Quantifier::OneOrMore,
        }
    }
}
```

### Supported Syntax

#### Basic Patterns

```wfl
create pattern greeting:
    "hello"
end pattern
```

#### Character Classes

```wfl
create pattern phone:
    digit digit digit
end pattern

create pattern word:
    any letter
end pattern
```

#### Quantifiers

```wfl
create pattern flexible:
    one or more digit
    optional letter
    zero or more whitespace
end pattern
```

#### Alternatives

```wfl
create pattern greeting:
    "hello" or "hi" or "hey"
end pattern
```

#### Sequences

```wfl
create pattern email:
    one or more letter
    "@"
    one or more letter
    "."
    letter letter letter
end pattern
```

#### Captures (Basic Implementation)

```wfl
create pattern name:
    capture {
        one or more letter
    } as first_name
end pattern
```

### Error Handling

The pattern parser provides detailed error messages for common mistakes:

- **Missing closing tokens**: "Expected 'end pattern' to close pattern definition"
- **Invalid quantifier usage**: "Unexpected 'one' in pattern (did you mean 'one or more'?)"
- **Incomplete character classes**: "Expected 'letter', 'digit', or 'whitespace' after 'any'"
- **Unclosed capture groups**: "Unclosed capture group"

### Integration with Main Parser

Pattern definitions are fully integrated into the main parsing loop:

1. Recognized by the statement parser
2. Error recovery mechanisms apply
3. Position tracking for accurate error reporting
4. Supports nested pattern definitions (via depth tracking)

### Testing

Comprehensive unit tests cover:

- Simple literal patterns
- Character class parsing
- Quantifier handling
- Alternative parsing
- Error conditions
- Integration with lexer tokens

Test files: `src/parser/tests.rs` contains `test_parse_*_pattern` functions.

### Future Extensions

This Phase 1 implementation provides the foundation for:

- Pattern compilation to executable bytecode (Phase 2)
- Runtime pattern matching engine (Phase 2)
- Advanced features like lookarounds and backreferences (Phase 3)
- Performance optimizations and caching (Phase 5)

### Backward Compatibility

The new pattern system maintains full backward compatibility:

- Existing code continues to work unchanged
- Old pattern syntax is still supported (marked as legacy)
- No breaking changes to existing APIs