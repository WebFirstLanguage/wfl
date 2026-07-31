# WFL Parser Refactor Plan

## Overview

Comprehensive refactor to modernize the WFL parser by:
1. **Cursor-based navigation** - Replace `Peekable<Iter>` with index-based `Cursor`
2. **Pure syntactic parsing** - Remove `known_actions` semantic checks from parser
3. **Span-based diagnostics** - Move `ParseError` out of AST, introduce `Span` types
4. **Explicit statement termination** - Add `Eol` tokens from lexer
5. **Domain-specific modules** - Split 7,755-line `mod.rs` into focused modules

**Implementation order**: Cursor infrastructure first, then module organization.

---

## Phase 1: Cursor Infrastructure (Week 1)

### 1.1 Create Cursor Module (~2 hours)

**File**: `src/parser/cursor.rs` (new)

```rust
pub struct Cursor<'a> {
    toks: &'a [TokenWithPosition],
    i: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(toks: &'a [TokenWithPosition]) -> Self;
    pub fn pos(&self) -> usize;
    pub fn is_eof(&self) -> bool;
    pub fn peek(&self) -> Option<&'a TokenWithPosition>;
    pub fn peek_n(&self, n: usize) -> Option<&'a TokenWithPosition>;
    pub fn peek_next(&self) -> Option<&'a TokenWithPosition>;
    pub fn bump(&mut self) -> Option<&'a TokenWithPosition>;
    pub fn checkpoint(&self) -> usize;
    pub fn rewind(&mut self, cp: usize);
    pub fn remaining(&self) -> usize;
    pub fn peek_kind(&self) -> Option<&Token>;
    pub fn at(&self, expected: Token) -> bool;
    pub fn eat(&mut self, expected: Token) -> bool;
}
```

**Testing**: Unit tests for all cursor methods, checkpoint/rewind.

**Files modified**:
- `src/parser/mod.rs` - Add `mod cursor;` and `use cursor::Cursor;`

---

### 1.2 Update Parser Struct (~1 hour)

**File**: `src/parser/mod.rs` (line 153-166)

**Change**:
```rust
pub struct Parser<'a> {
    cur: Cursor<'a>,                    // NEW: cursor navigation
    errors: Vec<ParseError>,
    // REMOVED: known_actions (being deleted)

    // Keep temporarily for migration
    #[deprecated]
    tokens: Peekable<Iter<'a, TokenWithPosition>>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [TokenWithPosition]) -> Self {
        Parser {
            cur: Cursor::new(tokens),
            tokens: tokens.iter().peekable(),  // Keep during migration
            errors: Vec::with_capacity(4),
        }
    }
}
```

**Testing**: Ensure all existing tests pass (no behavior change yet).

---

### 1.3 Replace Progress Tracking (~1 hour)

**Files**: `src/parser/mod.rs`

**Pattern replacement** (7 instances):
- Lines 206, 211, 366, 4285, 4323, 5496, 5541

**Before**:
```rust
let start_len = self.tokens.clone().count();
// ... parse logic ...
let end_len = self.tokens.clone().count();
assert!(end_len < start_len, "Parser made no progress");
```

**After**:
```rust
let start_pos = self.cur.pos();
// ... parse logic ...
assert!(self.cur.pos() > start_pos, "Parser made no progress");
```

**Testing**: Run full test suite after each replacement.

---

### 1.4 Replace Multi-Token Lookahead (~1 hour)

**Files**: `src/parser/mod.rs`

**Instances**: Lines 1712, 5502, 222-226 (lookahead patterns)

**Before**:
```rust
if let Some(next) = self.tokens.clone().nth(1) {
    if matches!(next.token, Token::Colon) { ... }
}
```

**After**:
```rust
if let Some(next) = self.cur.peek_n(1) {
    if matches!(next.token, Token::Colon) { ... }
}
```

**Testing**: Argument parsing tests, pattern tests.

---

### 1.5 Migrate Token Consumption (~8 hours)

**Files**: `src/parser/mod.rs`

**Strategy**: Incremental migration, one method at a time.

**Order**:
1. Helper methods: `expect_token`, `synchronize` (~1 hour)
2. Leaf expression parsers: literals, variables (~2 hours)
3. Simple statement parsers: variables, display (~2 hours)
4. Complex parsers: control flow, expressions (~2 hours)
5. Main parse loop (~1 hour)

**Pattern**: Replace `self.tokens.next()` with `self.cur.bump()`, `self.tokens.peek()` with `self.cur.peek()`.

**Testing**: Run tests after each method migration.

---

### 1.6 Eliminate Peek-and-Clone (~6 hours)

**Files**: `src/parser/mod.rs`

**Instances**: 90+ uses of `.peek().cloned()`

**Before**:
```rust
while let Some(token_pos) = self.tokens.peek().cloned() {
    let token = token_pos.token.clone();
    // ...
}
```

**After**:
```rust
while let Some(token_pos) = self.cur.peek() {
    let token = &token_pos.token;
    // ...
}
```

**Strategy**: Batch migration (10-15 instances), test, repeat. Handle borrow checker issues.

**Testing**: Full test suite after each batch.

---

### 1.7 Remove Deprecated Iterator Field (~1 hour)

**Files**: `src/parser/mod.rs`

**Actions**:
1. Remove `tokens: Peekable<Iter<'a, TokenWithPosition>>` field
2. Remove `#[deprecated]` attributes
3. Remove any sync checking code

**Testing**: Full test suite + all TestPrograms.

---

## Phase 2: Lexer Enhancement (Week 1)

### 2.1 Add Eol Token to Lexer (~2 hours)

**File**: `src/lexer/token.rs`

**Add**:
```rust
pub enum Token {
    // ... existing tokens ...
    Eol,  // Explicit newline/statement terminator
}
```

**File**: `src/lexer/mod.rs`

**Modify** `lex_wfl_with_positions`:
- Emit `Token::Eol` at end of each line (when appropriate)
- Don't emit within string literals or certain contexts
- Emit before significant newlines (not all whitespace)

**Testing**: Lexer tests for Eol emission, ensure multi-line expressions still work.

---

### 2.2 Update Parser to Consume Eol (~3 hours)

**File**: `src/parser/mod.rs`

**Changes**:
1. Statement dispatcher consumes trailing `Eol` tokens
2. Expression parser stops on `Eol` instead of line comparison
3. Block parsers consume `Eol` between statements

**Remove**: All `line > left_line` comparisons (lines 214, 358, 1759)

**Before**:
```rust
if line > left_line || Parser::is_statement_starter(&token) {
    break;
}
```

**After**:
```rust
if matches!(token, Token::Eol) || Parser::is_statement_starter(token) {
    break;
}
```

**Testing**: All TestPrograms, especially multi-line expressions.

---

## Phase 3: Diagnostics Refactor (Week 1-2)

### 3.1 Create Diagnostic Module (~2 hours)

**File**: `src/parser/diagnostic.rs` (new)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,  // Byte offset
    pub end: usize,    // Byte offset
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
    pub line: usize,    // Keep for display
    pub column: usize,  // Keep for display
}
```

**File**: `src/parser/ast.rs`

**Remove**: `ParseError` definition (lines 766-791)

**Testing**: Update all ParseError construction sites, ensure tests pass.

---

### 3.2 Introduce Span in AST (~4 hours)

**File**: `src/parser/ast.rs`

**Strategy**: Gradual migration, add `span: Span` field alongside `line, column` (don't remove old fields yet).

**Priority AST nodes**:
- `Statement` variants (start with simple ones)
- `Expression` variants (complex due to volume)

**Example**:
```rust
VariableDeclaration {
    name: String,
    value: Box<Expression>,
    is_constant: bool,
    span: Span,        // NEW
    line: usize,       // Keep for now
    column: usize,     // Keep for now
}
```

**Testing**: Incremental, run tests after each batch of AST updates.

---

## Phase 4: Remove known_actions (~3 hours)

### 4.1 Pure Syntactic Call Parsing (~2 hours)

**File**: `src/parser/mod.rs`

**Current** (lines 1946-1958): Uses `known_actions.contains(name)` to distinguish action calls from variables.

**New approach**: Parse all `identifier with args` as calls syntactically:

```rust
// In parse_primary_expression
Token::Identifier(name) => {
    let name = name.clone();
    let line = token_pos.line;
    let column = token_pos.column;
    self.cur.bump(); // Consume identifier

    // Lookahead for call patterns
    if let Some(next) = self.cur.peek() {
        match &next.token {
            Token::KeywordWith => {
                // Always parse as call if 'with' follows
                self.cur.bump(); // Consume "with"
                let arguments = self.parse_argument_list()?;
                return Ok(Expression::ActionCall {
                    name,
                    arguments,
                    line,
                    column,
                });
            }
            Token::LeftParen => {
                // Function call
                self.cur.bump(); // Consume "("
                let arguments = self.parse_argument_list()?;
                self.expect_token(Token::RightParen, "Expected ')'")?;
                return Ok(Expression::FunctionCall {
                    function: name,
                    arguments,
                    line,
                    column,
                });
            }
            _ => {
                // Just a variable reference
                return Ok(Expression::Variable(name, line, column));
            }
        }
    }
}
```

**Remove**:
- `known_actions` field from Parser struct
- `self.known_actions.insert()` call in `parse_action_definition` (line 4270)
- `self.known_actions.contains()` check (line 1947)

**Semantic resolution**: Defer to analyzer/type checker to verify action exists.

**Testing**: Action call tests, recursive action tests, forward reference tests.

---

### 4.2 Update Analyzer for Action Resolution (~1 hour)

**File**: `src/analyzer/mod.rs`

**Add**: Validation that action names in `ActionCall` expressions exist and have correct signatures.

**Testing**: Error tests for undefined actions, analyzer tests.

---

## Phase 5: Module Organization (Week 2-3)

### 5.1 Foundation Modules (~2 hours)

**Create**:
- `src/parser/helpers.rs` - Extract `expect_token`, `synchronize`, `get_token_text`, `is_statement_starter`

**File**: `src/parser/mod.rs`

**Changes**:
- Move helper functions to `helpers.rs`
- Keep as `impl Parser` methods
- Update visibility to `pub(crate)` or `pub(super)`

**Testing**: Ensure all tests pass.

---

### 5.2 Expression Module (~5 hours)

**Create**:
- `src/parser/expr/mod.rs` - `ExprParser` trait
- `src/parser/expr/primary.rs` - `PrimaryExprParser` trait with `parse_primary_expression`
- `src/parser/expr/binary.rs` - `BinaryExprParser` trait with `parse_binary_expression`

**Structure**:
```rust
// expr/mod.rs
pub(crate) trait ExprParser<'a> {
    fn parse_expression(&mut self) -> Result<Expression, ParseError>;
}

impl<'a> ExprParser<'a> for Parser<'a> {
    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_binary_expression(0)
    }
}
```

**Testing**: Expression-heavy TestPrograms.

---

### 5.3 Statement Modules (~12 hours)

**Create**:
- `src/parser/stmt/mod.rs` - `StmtParser` trait with `parse_statement` dispatcher
- `src/parser/stmt/variables.rs` - `VariableParser` trait
- `src/parser/stmt/control_flow.rs` - `ControlFlowParser` trait
- `src/parser/stmt/actions.rs` - `ActionParser` trait
- `src/parser/stmt/containers.rs` - `ContainerParser` trait
- `src/parser/stmt/io.rs` - `IoParser` trait
- `src/parser/stmt/processes.rs` - `ProcessParser` trait
- `src/parser/stmt/web.rs` - `WebParser` trait
- `src/parser/stmt/errors.rs` - `ErrorHandlingParser` trait
- `src/parser/stmt/collections.rs` - `CollectionParser` trait
- `src/parser/stmt/patterns.rs` - `PatternParser` trait

**Migration order** (risk-based):
1. Variables (~2 hours) - Low risk
2. Collections (~2 hours) - Low risk
3. Actions (~2 hours) - Medium risk
4. Control flow (~2 hours) - Medium-high risk
5. File I/O (~1 hour) - Medium risk
6. Processes, Web, Errors (~1 hour each) - Medium risk
7. Containers (~2 hours) - High risk
8. Patterns (~2 hours) - High risk

**Testing**: Run relevant TestPrograms after each module extraction.

---

### 5.4 Final Integration (~2 hours)

**File**: `src/parser/mod.rs`

**Final structure**:
```rust
pub mod ast;
mod cursor;
mod diagnostic;
mod helpers;
mod expr;
mod stmt;

pub use ast::*;
pub use diagnostic::{ParseError, Span};
use expr::*;
use stmt::*;

pub struct Parser<'a> {
    cur: Cursor<'a>,
    errors: Vec<ParseError>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [TokenWithPosition]) -> Self { ... }
    pub fn parse(&mut self) -> Result<Program, Vec<ParseError>> { ... }
}
```

**Goal**: `mod.rs` reduced from 7,755 lines to ~300 lines.

**Testing**: Full test suite, all TestPrograms, integration tests.

---

## Testing Strategy

### Per Phase

**Phase 1 (Cursor)**: After each step, run:
```bash
cargo test --lib
.\scripts\run_integration_tests.ps1
```

**Phase 2 (Lexer)**: Lexer tests + parser tests for statement termination.

**Phase 3 (Diagnostics)**: Error reporting tests, diagnostic tests.

**Phase 4 (known_actions)**: Action call tests, analyzer tests.

**Phase 5 (Modules)**: Module-specific tests after each extraction.

### Final Validation

**Before merging**:
- [ ] All unit tests pass: `cargo test --lib`
- [ ] All integration tests pass: `.\scripts\run_integration_tests.ps1`
- [ ] All 50+ TestPrograms execute correctly
- [ ] No clippy warnings: `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Code formatted: `cargo fmt --all`
- [ ] Performance benchmarks acceptable (within 5% of baseline)

---

## Critical Files

### To Modify
- **`src/parser/mod.rs`** (7,755 lines) - Main refactor target
- **`src/parser/ast.rs`** (824 lines) - Remove ParseError, add Span
- **`src/lexer/mod.rs`** - Add Eol token emission
- **`src/lexer/token.rs`** - Add Eol token variant
- **`src/analyzer/mod.rs`** - Add action resolution validation

### To Create
- **`src/parser/cursor.rs`** - Cursor implementation
- **`src/parser/diagnostic.rs`** - ParseError + Span
- **`src/parser/helpers.rs`** - Helper functions
- **`src/parser/expr/mod.rs`** - Expression parsing trait
- **`src/parser/expr/primary.rs`** - Primary expression parser
- **`src/parser/expr/binary.rs`** - Binary expression parser
- **`src/parser/stmt/mod.rs`** - Statement dispatcher
- **`src/parser/stmt/*.rs`** - 11 statement domain modules

### Test Programs (Must Pass)
- `TestPrograms/basic_syntax_comprehensive.wfl`
- `TestPrograms/containers_comprehensive.wfl`
- `TestPrograms/file_io_comprehensive.wfl`
- `TestPrograms/error_handling_comprehensive.wfl`
- `TestPrograms/comparison_operators_test.wfl`
- All other 45+ test programs

---

## Risk Mitigation

### High-Risk Areas

1. **Operator precedence** (Phase 1.6 + 5.2)
   - Test extensively with complex expressions
   - Compare AST output before/after

2. **Container inheritance** (Phase 5.3)
   - Dedicated container tests
   - OOP feature validation

3. **Pattern DSL** (Phase 5.3)
   - Pattern matching tests
   - Regex-like pattern tests

4. **known_actions removal** (Phase 4)
   - Recursive action calls
   - Forward references
   - Semantic validation in analyzer

### Rollback Strategy

- Git commit after each successful phase
- Tag commits: `parser-refactor-cursor-complete`, `parser-refactor-modules-complete`
- Can abandon Phase 5 if needed - Phases 1-4 standalone valuable

---

## Success Criteria

1. **Zero test regressions** - All existing tests pass
2. **Backward compatibility** - All WFL programs work identically
3. **Code organization** - `mod.rs` < 500 lines (from 7,755)
4. **Performance** - Within 5% of baseline (likely faster)
5. **Code quality** - No clippy warnings, formatted
6. **Architecture** - Clean separation: cursor, diagnostics, expressions, statements

---

## Estimated Timeline

- **Phase 1 (Cursor)**: 20 hours (~3 days)
- **Phase 2 (Lexer)**: 5 hours (~1 day)
- **Phase 3 (Diagnostics)**: 6 hours (~1 day)
- **Phase 4 (known_actions)**: 3 hours (~0.5 day)
- **Phase 5 (Modules)**: 21 hours (~3 days)

**Total**: ~55 hours (7-8 work days, or 2-3 weeks part-time)

**With testing buffer**: ~70 hours (9 work days, or 3 weeks part-time)
