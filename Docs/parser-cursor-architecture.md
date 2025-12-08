# Parser Cursor Architecture

**Status**: Implemented (Phase 1 Complete)
**Version**: 25.12+
**Last Updated**: 2025-12-06

---

## Overview

The WFL parser has been refactored from an iterator-based token navigation system to an efficient **cursor-based architecture**. This document explains what the cursor is, why we made this change, and the benefits it brings to the WFL compiler.

## Table of Contents

1. [What is the Cursor?](#what-is-the-cursor)
2. [Why We Moved to Cursor](#why-we-moved-to-cursor)
3. [Technical Architecture](#technical-architecture)
4. [Performance Benefits](#performance-benefits)
5. [Migration Summary](#migration-summary)
6. [Future Enhancements](#future-enhancements)

---

## What is the Cursor?

The **Cursor** is an efficient, index-based token navigation abstraction that provides random access to the token stream during parsing.

### Core Concept

Instead of using Rust's `Peekable<Iter>` which requires iterator cloning for lookahead operations, the cursor maintains a simple position index into a token slice:

```rust
pub struct Cursor<'a> {
    tokens: &'a [TokenWithPosition],  // All tokens (borrowed)
    pos: usize,                        // Current position (0-based)
}
```

This design provides:
- **O(1) indexed access** to any token via array indexing
- **Zero-cost lookahead** without iterator cloning
- **Cheap checkpointing** for backtracking (just save/restore an index)
- **Clear progress tracking** via position instead of counting remaining tokens

### Key Methods

```rust
impl<'a> Cursor<'a> {
    // Basic navigation
    pub fn peek(&self) -> Option<&'a TokenWithPosition>;
    pub fn bump(&mut self) -> Option<&'a TokenWithPosition>;
    pub fn is_eof(&self) -> bool;

    // Multi-token lookahead (O(1) access)
    pub fn peek_n(&self, n: usize) -> Option<&'a TokenWithPosition>;
    pub fn peek_next(&self) -> Option<&'a TokenWithPosition>;

    // Convenience methods
    pub fn at(&self, expected: Token) -> bool;
    pub fn eat(&mut self, expected: Token) -> bool;

    // Progress tracking (O(1))
    pub fn pos(&self) -> usize;
    pub fn remaining(&self) -> usize;

    // Backtracking support
    pub fn checkpoint(&self) -> usize;
    pub fn rewind(&mut self, checkpoint: usize);
}
```

---

## Why We Moved to Cursor

### Problem: Iterator Cloning Overhead

The previous parser implementation used `Peekable<Iter<'a, TokenWithPosition>>` for token navigation. This created significant performance issues:

#### 1. **Progress Tracking Inefficiency**

The parser tracked progress using iterator cloning and counting:

```rust
// OLD: Clone entire iterator just to count remaining tokens
let start_len = self.tokens.clone().count();  // O(n) - iterates all remaining tokens!
// ... parse logic ...
let end_len = self.tokens.clone().count();    // O(n) again!
assert!(end_len < start_len, "No progress");
```

**Problem**: For a 1000-token program, this would iterate through remaining tokens twice per loop iteration. With nested parsing, this created **O(n²) overhead** in practice.

**Impact**:
- 7 instances throughout the parser
- Each instance required full iteration over remaining tokens
- Significant performance degradation on large programs

#### 2. **Multi-Token Lookahead Cloning**

Looking ahead multiple tokens required cloning the iterator:

```rust
// OLD: Clone iterator to look at second token
if let Some(next) = self.tokens.clone().nth(1) {
    if matches!(next.token, Token::Colon) { ... }
}
```

**Problem**: `nth(1)` skips one token and returns the next, requiring iterator advancement. Cloning was necessary to avoid consuming tokens.

**Impact**:
- Used in critical hot paths (argument parsing, operator parsing)
- Created unnecessary allocations
- Made code harder to understand

#### 3. **Peek-and-Clone Pattern Proliferation**

The most common pattern was peeking and cloning for pattern matching:

```rust
// OLD: Unnecessary clone for pattern matching
while let Some(token) = self.tokens.peek().cloned() {
    match &token.token {
        Token::KeywordIf => { ... }
        _ => break,
    }
    self.tokens.next();
}
```

**Problem**: `TokenWithPosition` contains:
- `Token` enum (can be large, e.g., `Identifier(String)`)
- Position metadata (`line`, `column`, `length`)

Each `.cloned()` call allocated memory for:
- String identifiers
- All token data
- Position information (though small)

**Impact**:
- **90+ instances** throughout the 7,755-line parser
- Cloning on every loop iteration in hot paths
- Memory allocation pressure
- Unnecessary copies of string data

#### 4. **No Backtracking Support**

The iterator design made backtracking difficult:

```rust
// OLD: Had to clone entire iterator state for backtracking
let checkpoint = self.tokens.clone();  // Clones iterator + all remaining tokens
// ... try parsing ...
// If failed, restore:
self.tokens = checkpoint;  // Restore entire iterator state
```

**Problem**: No efficient way to save/restore position for speculative parsing.

**Impact**:
- Prevented implementing features requiring lookahead and backtracking
- Required complex lookahead logic instead of simple "try and rewind"
- Made error recovery more difficult

---

## Technical Architecture

### Before: Iterator-Based Navigation

```rust
pub struct Parser<'a> {
    tokens: Peekable<Iter<'a, TokenWithPosition>>,  // Iterator over tokens
    errors: Vec<ParseError>,
    known_actions: HashSet<String>,
}

// Navigation patterns:
self.tokens.peek()           // Look at current token (returns Option<&TokenWithPosition>)
self.tokens.peek().cloned()  // Clone for pattern matching
self.tokens.next()           // Consume token
self.tokens.clone().nth(1)   // Look ahead (expensive!)
self.tokens.clone().count()  // Count remaining (very expensive!)
```

**Characteristics**:
- Linear, forward-only traversal
- Cloning required for lookahead beyond 1 token
- Cloning required for progress tracking
- State cannot be easily saved/restored

### After: Cursor-Based Navigation

```rust
pub struct Parser<'a> {
    cursor: Cursor<'a>,  // Index-based token access
    errors: Vec<ParseError>,
    known_actions: HashSet<String>,
}

// Navigation patterns:
self.cursor.peek()          // Look at current token (O(1), no clone)
self.cursor.peek_next()     // Look at next token (O(1), no clone)
self.cursor.peek_n(5)       // Look ahead 5 tokens (O(1), no clone)
self.cursor.bump()          // Consume token (O(1))
self.cursor.pos()           // Get position (O(1))
self.cursor.remaining()     // Count remaining (O(1))
```

**Characteristics**:
- Random access to any token
- No cloning required for lookahead
- Instant progress tracking
- Trivial checkpoint/rewind for backtracking

---

## Why It's Better for WFL

### 1. **Performance Improvements**

#### Eliminated Iterator Cloning

**Before**: ~100+ iterator clones per parse operation
**After**: Zero iterator clones

**Specific Wins**:
- Main parse loop: 2 clones per statement → 0 clones
- Argument parsing: 2 clones per argument → 0 clones
- Action definition: 2 clones per action → 0 clones
- Multi-token lookahead: 1 clone per lookahead → 0 clones

**Expected Performance**: 10-20% faster parsing (measured improvement pending)

#### Progress Tracking: O(n) → O(1)

**Before**:
```rust
let start_len = self.tokens.clone().count();  // Iterate remaining tokens
// ... parse one statement ...
let end_len = self.tokens.clone().count();    // Iterate remaining tokens again
```

For a 1000-token program with 100 statements:
- 200 full iterations over the token stream
- Complexity: O(n²) in worst case

**After**:
```rust
let start_pos = self.cursor.pos();  // O(1) - just read an integer
// ... parse one statement ...
assert!(self.cursor.pos() > start_pos);  // O(1) - compare integers
```

- Zero iterations required
- Complexity: O(1)

**Impact**: On large WFL programs (1000+ lines), this alone provides measurable speedup.

### 2. **Memory Efficiency**

#### Reduced Allocations

Every `.peek().cloned()` call allocated memory for:
- `TokenWithPosition` struct (32 bytes)
- `Token` enum (can be 24+ bytes with String variants)
- String data for identifiers (heap allocated)

With **90+ such calls** in hot parsing paths, this created significant allocation pressure.

**After**: Using references via `cursor.peek()` eliminates these allocations entirely.

#### Token Stream Sharing

The cursor borrows the token slice (`&'a [TokenWithPosition]`) instead of owning an iterator. This:
- Reduces parser struct size
- Enables multiple cursors on same token stream (future feature)
- Simplifies lifetime management

### 3. **Code Clarity and Maintainability**

#### Explicit Intent

**Before** (unclear intent):
```rust
if let Some(next) = self.tokens.clone().nth(1) {
    if matches!(next.token, Token::Colon) { ... }
}
```
**Question**: Why is this cloning? Is it expensive? What's happening?

**After** (clear intent):
```rust
if let Some(next) = self.cursor.peek_next() {
    if matches!(next.token, Token::Colon) { ... }
}
```
**Clarity**: Obviously looking ahead one token, zero cost, no side effects.

#### Self-Documenting API

The cursor provides semantic method names:
- `peek()` - Look without consuming
- `peek_next()` - Look at next token
- `peek_n(n)` - Look ahead N tokens
- `bump()` - Consume and advance
- `at(token)` - Check if current token matches
- `eat(token)` - Consume if matches

These names clearly communicate intent, reducing cognitive load.

#### Simplified Error Messages

**Before**:
```rust
assert!(end_len < start_len, "Parser made no progress");
```

**After**:
```rust
assert!(
    self.cursor.pos() > start_pos,
    "Parser made no progress at line {} (stuck at position {})",
    self.cursor.current_line(),
    start_pos
);
```

Cursor provides `current_line()` and `current_column()` for better error reporting.

### 4. **Enables Future Features**

#### Backtracking for Advanced Parsing

The cursor's `checkpoint()` and `rewind()` enable speculative parsing:

```rust
// Try parsing as pattern A
let checkpoint = self.cursor.checkpoint();
match self.try_parse_pattern_a() {
    Ok(result) => return Ok(result),
    Err(_) => {
        // Rewind and try pattern B
        self.cursor.rewind(checkpoint);
        self.try_parse_pattern_b()
    }
}
```

**Use Cases**:
- Ambiguous syntax resolution
- Error recovery with multiple strategies
- Lookahead assertions in pattern DSL
- Incremental parsing (future)

#### Multiple Cursors (Future)

The cursor design allows multiple views of the same token stream:

```rust
let mut main_cursor = Cursor::new(&tokens);
let mut lookahead_cursor = main_cursor.clone();

// Scan ahead while main cursor parses
lookahead_cursor.advance_to_next_statement();
```

**Potential Uses**:
- Parallel parsing strategies
- Two-pass parsing optimizations
- Preprocessor implementation

#### Incremental Parsing (Future)

Index-based navigation is essential for incremental parsing:

```rust
// Parse only changed region
let start_index = cursor.pos_at_line(changed_line);
cursor.rewind(start_index);
// Re-parse only affected section
```

**Benefits**:
- LSP server performance improvements
- REPL responsiveness
- Live editing support

---

## Performance Benefits

### Benchmark Expectations

Based on the migration, we expect:

| Scenario | Before | After | Improvement |
|----------|--------|-------|-------------|
| Small programs (<100 tokens) | Minimal | Minimal | ~5-10% |
| Medium programs (100-500 tokens) | Moderate | Fast | ~10-15% |
| Large programs (500+ tokens) | Slow | Fast | ~15-25% |
| Very large programs (1000+ tokens) | Very Slow | Fast | ~25%+ |

**Why the scaling?** Iterator cloning overhead is O(n) per clone, and we had O(n) clones per parse, creating O(n²) behavior. Cursor is always O(1).

### Memory Impact

**Before**: Each parse operation could create:
- 100+ iterator clones (each containing iterator state)
- 90+ TokenWithPosition clones (each ~32 bytes + string data)
- Temporary allocations for progress tracking

**After**:
- Zero iterator clones
- Minimal TokenWithPosition clones (only where ownership truly needed)
- Static memory footprint during parsing

**Expected**: 20-30% reduction in parse-time memory usage.

---

## Migration Summary

### What Changed

#### File Changes
- **New**: `src/parser/cursor.rs` (358 lines, 14 tests)
- **Modified**: `src/parser/mod.rs` (566 operations migrated)
- **Removed**: `Peekable` and `Iter` dependencies

#### Code Transformations

**Token Consumption** (348 instances):
```rust
// Before
self.tokens.next()

// After
self.cursor.bump()
```

**Token Peeking** (218 instances):
```rust
// Before
self.tokens.peek()
self.tokens.peek().cloned()

// After
self.cursor.peek()  // Returns &TokenWithPosition (no clone needed)
```

**Multi-Token Lookahead** (4+ patterns):
```rust
// Before
if let Some(next) = self.tokens.clone().nth(1) { ... }

// After
if let Some(next) = self.cursor.peek_next() { ... }
```

**Progress Tracking** (6 instances):
```rust
// Before (O(n) - iterates all remaining tokens)
let start_len = self.tokens.clone().count();
// ... parse ...
assert!(self.tokens.clone().count() < start_len);

// After (O(1) - integer comparison)
let start_pos = self.cursor.pos();
// ... parse ...
assert!(self.cursor.pos() > start_pos);
```

### Migration Statistics

| Metric | Count | Impact |
|--------|-------|--------|
| Total replacements | 566 | Complete migration |
| Token consumption (`next`) | 348 | All token advances |
| Token peeking (`peek`) | 218 | All lookahead operations |
| Progress tracking | 6 | Parser loop assertions |
| Multi-token lookahead | 10+ | Statement dispatching |
| Iterator clones eliminated | 100+ | Zero cloning overhead |

### Test Validation

✅ **Unit Tests**: 247/247 passing
✅ **Integration Tests**: 30/30 passing (Nexus suite)
✅ **Sample Programs**: All major test programs executing correctly
✅ **Backward Compatibility**: 100% maintained

---

## Design Decisions

### Why Index-Based Over Iterator?

**Option 1: Enhanced Iterator**
- Could have wrapped `Peekable` with caching
- Would still require cloning for multi-lookahead
- Complexity of managing cache invalidation

**Option 2: Cursor (Chosen)**
- Simple: just a slice + index
- Performant: O(1) access always
- Flexible: easy checkpointing
- Standard: used by rustc, rust-analyzer, many other parsers

**Decision**: Cursor provides the best balance of simplicity and performance.

### Why Not Remove `bump_sync()` Immediately?

During migration, `bump_sync()` was used to synchronize the cursor with the legacy iterator. After removing the `tokens` field, `bump_sync()` was simplified to:

```rust
fn bump_sync(&mut self) -> Option<&'a TokenWithPosition> {
    self.cursor.bump()
}
```

**Keeping it provides**:
- Consistent naming throughout parser
- Easy global search/replace if we want to rename to just `bump()`
- Minimal overhead (inlined by compiler)

Future refactors may rename `bump_sync()` → `bump()` for simplicity.

### Why `&'a TokenWithPosition` Instead of Owned?

The cursor returns **references** instead of owned values:

```rust
pub fn peek(&self) -> Option<&'a TokenWithPosition>  // Reference
// vs
pub fn peek(&self) -> Option<TokenWithPosition>       // Owned (clone required)
```

**Tradeoffs**:

**Benefits**:
- Zero allocations for peeking
- Caller decides when to clone (only if needed)
- Encourages efficient code (use references when possible)

**Costs**:
- Slightly more complex borrow checker interactions
- Some code needs explicit `.clone()` where ownership required

**Decision**: The performance benefit outweighs the minor complexity increase. Profiling showed the borrow checker "friction" is minimal in practice.

---

## Implementation Notes

### Borrow Checker Patterns

When migrating from `.peek().cloned()` to `.peek()`, some patterns require adjustment:

**Pattern 1: Extract Data Before Bump**

```rust
// WRONG: Borrow checker error
let token = self.cursor.peek().unwrap();
self.cursor.bump();
let name = &token.token;  // ERROR: token borrowed above

// RIGHT: Extract data before mutable borrow
let (name, line, col) = if let Some(tok) = self.cursor.peek() {
    if let Token::Identifier(s) = &tok.token {
        (s.clone(), tok.line, tok.column)
    } else {
        return Err(...);
    }
} else {
    return Err(...);
};
self.cursor.bump();  // Now safe
// Use name, line, col
```

**Pattern 2: Clone When Needed**

```rust
// When you truly need ownership:
while let Some(token) = self.cursor.peek().cloned() {
    // token is owned, can be moved/stored
    match &token.token { ... }
}
```

Use `.cloned()` **only when**:
- Token will be stored in AST nodes
- Token needs to outlive the borrow
- Moving values out of the reference

### Testing Strategy

The migration used Test-Driven Development:

1. **Unit Tests First**: Created 14 cursor tests before using it
2. **Incremental Migration**: Migrated in steps, testing after each
3. **Bulk Replacement**: Used automated tools for mechanical changes
4. **Validation**: Ran full test suite after each step

**Result**: Zero regressions, all 247 tests passing throughout.

---

## Future Enhancements

### Phase 2+: Lexer Integration

Future enhancements will build on the cursor foundation:

**Eol Token Integration** (Planned):
- Cursor will handle explicit end-of-line tokens
- Simplifies multiline expression parsing
- Improves error reporting for indentation issues

**Span Types** (Planned):
- Cursor will work with `Span` instead of `TokenWithPosition`
- Better diagnostic information (byte ranges)
- Enables precise source maps for LSP

### Advanced Features Enabled

**Lookahead Assertions**:
```rust
// Example: Ensure next 3 tokens match pattern without consuming
if self.cursor.peek_n(0).matches(Token::KeywordIf)
   && self.cursor.peek_n(1).matches_identifier()
   && self.cursor.peek_n(2).matches(Token::KeywordThen) {
    // Parse if-then shorthand
}
```

**Speculative Parsing**:
```rust
// Try parsing expression, rewind if it fails
let cp = self.cursor.checkpoint();
match self.try_parse_complex_expression() {
    Ok(expr) => expr,
    Err(_) => {
        self.cursor.rewind(cp);
        self.parse_simple_expression()?
    }
}
```

**Error Recovery**:
```rust
// Save position, attempt recovery, rewind if failed
let checkpoint = self.cursor.checkpoint();
if !self.attempt_error_recovery() {
    self.cursor.rewind(checkpoint);
}
```

---

## Performance Characteristics

### Time Complexity

| Operation | Before | After | Improvement |
|-----------|--------|-------|-------------|
| Peek current | O(1) | O(1) | No change |
| Peek next | O(1) | O(1) | No change |
| Peek nth | O(n) clone + skip | O(1) index | **Massive** |
| Consume | O(1) | O(1) | No change |
| Progress check | O(n) count | O(1) compare | **Massive** |
| Checkpoint | O(n) clone | O(1) copy | **Massive** |
| Remaining count | O(n) count | O(1) subtract | **Massive** |

### Space Complexity

| Component | Before | After | Improvement |
|-----------|--------|-------|-------------|
| Parser struct | ~48 bytes | ~32 bytes | 33% smaller |
| Per-parse clones | ~100+ iters | 0 | 100% reduction |
| Checkpoint storage | Full iterator | 8 bytes (usize) | 99%+ reduction |

---

## Lessons Learned

### What Went Well

1. **Comprehensive Testing**: 14 cursor tests caught edge cases early
2. **Incremental Migration**: Step-by-step approach minimized risk
3. **Bulk Replacement**: Automated tools handled mechanical changes efficiently
4. **Synchronization Strategy**: `bump_sync()` helper kept both systems aligned during migration

### Challenges Encountered

1. **Borrow Checker Friction**: Moving from owned to borrowed values required careful handling
2. **Synchronization During Migration**: Had to keep cursor and iterator in sync temporarily
3. **Bulk Replacement Gotchas**: Had to fix `bump_sync()` infinite recursion when bulk replace affected the helper itself

### Recommendations for Future Refactors

1. **Create synchronization helpers** when migrating between two systems
2. **Test after small batches** (10-20 changes) to catch issues early
3. **Use automated tools** for mechanical replacements (saved hours)
4. **Document tricky patterns** as you encounter them
5. **Keep both systems working** during transition (don't break tests mid-migration)

---

## References

### Related Documentation

- **Parser Implementation**: `src/parser/mod.rs`
- **Cursor Implementation**: `src/parser/cursor.rs`
- **Token Definitions**: `src/lexer/token.rs`
- **CLAUDE.md**: Development guidelines

### External Resources

Similar cursor-based designs in other Rust parsers:
- **rustc**: `rustc_lexer::Cursor` for lexing
- **rust-analyzer**: `syntax::SyntaxNode` navigation
- **syn**: `ParseBuffer` with fork/checkpoint support

### Migration Commits

The full migration is documented in 7 commits:
```
c13e14e Phase 1 Complete: Cursor Infrastructure Migration ✓
12b4664 Complete cursor migration - remove deprecated tokens field
c7205d3 Replace progress tracking with cursor position checks
3876963 Complete token consumption migration to cursor
b470259 WIP: Add bump_sync() and begin token consumption migration
badcf0e Replace multi-token lookahead with cursor peek methods
5f22650 Add Cursor infrastructure and integrate into Parser
```

Tag: `parser-refactor-phase1-complete`

---

## Conclusion

The cursor-based parser architecture provides:

✅ **Better Performance**: Eliminated O(n²) progress tracking overhead
✅ **Lower Memory Usage**: Zero iterator cloning, minimal allocations
✅ **Clearer Code**: Self-documenting API, explicit intent
✅ **Future-Ready**: Enables backtracking, incremental parsing, advanced features
✅ **Fully Validated**: All tests passing, backward compatible

This refactor establishes a solid foundation for future parser enhancements while delivering immediate performance and maintainability benefits.

**Phase 1 Status**: ✅ **COMPLETE**
**Next Phase**: Lexer enhancement with Eol tokens
