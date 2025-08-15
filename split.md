# Text Split Implementation Proposal for WFL

## Executive Summary

This proposal outlines the implementation of comprehensive text splitting functionality for WFL, addressing the gap between documented features and actual implementation. Currently, WFL documentation describes both simple string splitting and pattern-based splitting, but neither is fully implemented.

## Current State Analysis

### What Exists
- **Pattern Split (Incomplete)**: 
  - AST node: `Expression::PatternSplit`
  - Parser support: `split text on pattern name`
  - Stub implementation: `native_pattern_split` (returns original text)
  - Type checking support
  - **Critical Issue**: Function not registered in stdlib

- **Simple String Split (Missing)**:
  - Documentation examples: `split text by " "`
  - No parser support for `by` syntax in split operations
  - No AST node for simple string split
  - No implementation

### What's Missing
1. Complete pattern split implementation
2. Simple string split functionality
3. Proper stdlib registration
4. Comprehensive test coverage

## Proposed Solution

### 1. Two-Tier Split System

Following WFL's principle of progressive complexity, implement two split approaches:

#### Tier 1: Simple String Split
```wfl
// Basic delimiter splitting
store words as split "hello world test" by " "
store lines as split text by "\n"
store csv_fields as split "a,b,c,d" by ","
```

#### Tier 2: Pattern-Based Split
```wfl
// Advanced pattern splitting
create pattern comma_separator: ","
store fields as split csv_line on pattern comma_separator

create pattern whitespace: one or more space
store tokens as split text on pattern whitespace
```

### 2. Implementation Architecture

#### AST Extensions
```rust
// Add new AST node for simple string split
StringSplit {
    text: Box<Expression>,
    delimiter: Box<Expression>,
    line: usize,
    column: usize,
}

// Keep existing PatternSplit node
PatternSplit {
    text: Box<Expression>,
    pattern: Box<Expression>,
    line: usize,
    column: usize,
}
```

#### Parser Enhancements
```rust
// In parse_binary_expression, add support for:
Token::KeywordSplit => {
    // Handle both "split text by delimiter" and "split text on pattern"
    if next_token_is_by() {
        parse_string_split()
    } else if next_token_is_on() {
        parse_pattern_split()  // existing
    }
}
```

#### Standard Library Functions
```rust
// Simple string split
pub fn native_string_split(args: Vec<Value>) -> Result<Value, RuntimeError> {
    // Split text by string delimiter
    // Return List<Text>
}

// Complete pattern split implementation
pub fn native_pattern_split(args: Vec<Value>) -> Result<Value, RuntimeError> {
    // Split text using compiled pattern
    // Return List<Text>
}
```

### 3. Natural Language Syntax Design

#### Simple Split Syntax
```wfl
// Primary syntax (recommended)
store parts as split text by delimiter

// Alternative syntax (for consistency)
store parts as split text using delimiter
```

#### Pattern Split Syntax
```wfl
// Current syntax (keep existing)
store parts as split text on pattern pattern_name

// Alternative syntax (for clarity)
store parts as split text by pattern pattern_name
```

### 4. Test-Driven Development Plan

Following WFL's mandatory TDD approach:

#### Phase 1: Failing Tests (Commit First)
```wfl
// TestPrograms/text_split_simple.wfl
store sentence as "hello world testing"
store words as split sentence by " "
display "Word count: " with length of words
display "First word: " with words[0]
display "Last word: " with words[2]

// Expected output:
// Word count: 3
// First word: hello
// Last word: testing
```

```wfl
// TestPrograms/text_split_pattern.wfl
store csv_data as "name,age,city"
create pattern comma: ","
store fields as split csv_data on pattern comma
display "Fields: " with length of fields
display "Name: " with fields[0]
display "Age: " with fields[1]
display "City: " with fields[2]
```

```rust
// tests/split_functionality.rs
#[test]
fn test_string_split_basic() {
    let result = run_wfl(r#"
        store text as "a,b,c"
        store parts as split text by ","
        display length of parts
    "#);
    assert_eq!(result.trim(), "3");
}

#[test]
fn test_pattern_split_registered() {
    let result = run_wfl(r#"
        create pattern comma: ","
        store parts as split "x,y,z" on pattern comma
        display length of parts
    "#);
    assert_eq!(result.trim(), "3");
}
```

#### Phase 2: Minimal Implementation
1. Implement `native_string_split` with basic functionality
2. Complete `native_pattern_split` implementation
3. Register both functions in stdlib
4. Add parser support for `split by` syntax

#### Phase 3: Edge Cases and Robustness
```wfl
// Edge case tests
store empty as split "" by ","           // Should return empty list
store no_delim as split "hello" by ","   // Should return ["hello"]
store adjacent as split "a,,b" by ","    // Should return ["a", "", "b"]
store trailing as split "a,b," by ","    // Should return ["a", "b", ""]
```

### 5. Error Handling Strategy

#### Type Safety
```rust
// Ensure proper types
if !matches!(text_value, Value::Text(_)) {
    return Err(RuntimeError::new(
        "Split requires text as first argument".to_string(),
        line, column
    ));
}
```

#### Clear Error Messages
```wfl
// Error cases with helpful messages:
split 123 by ","          // "Cannot split number - expected text"
split text by 456         // "Delimiter must be text - got number"
split text by ""          // "Empty delimiter not allowed"
```

### 6. Implementation Timeline

#### Sprint 1: Foundation (TDD)
- [ ] Write comprehensive failing tests
- [ ] Commit failing tests
- [ ] Add `StringSplit` AST node
- [ ] Implement basic parser support for `split by`

#### Sprint 2: Core Functions (TDD)
- [ ] Implement `native_string_split`
- [ ] Complete `native_pattern_split`
- [ ] Register functions in stdlib
- [ ] Verify tests pass

#### Sprint 3: Polish (TDD)
- [ ] Add edge case tests
- [ ] Optimize performance
- [ ] Update documentation
- [ ] Add examples to cookbook

### 7. Performance Considerations

#### String Split Optimization
```rust
// Use efficient string splitting
let parts: Vec<&str> = text.split(delimiter).collect();
let values: Vec<Value> = parts.iter()
    .map(|s| Value::Text(Rc::from(*s)))
    .collect();
```

#### Pattern Split Integration
```rust
// Leverage existing pattern VM for splitting
// Use pattern matching to find split points
// Build list of text segments between matches
```

### 8. Backward Compatibility

#### No Breaking Changes
- All existing syntax continues to work
- New functionality is additive only
- Pattern split behavior remains consistent

#### Migration Path
- Existing pattern split usage (if any) unchanged
- New simple split provides easier entry point
- Documentation updated with examples

### 9. Documentation Updates Required

#### Files to Update
- `Docs/api/text-module.md` - Add string split functions
- `Docs/api/pattern-module.md` - Complete pattern split documentation
- `Docs/guides/wfl-cookbook.md` - Update examples to use working syntax
- `Docs/language-reference/wfl-syntax.md` - Document split syntax variations

#### Examples to Add
```wfl
// Text processing examples
define action called count_words:
    parameter text as Text
    store words as split text by " "
    return length of words
end action

// CSV parsing example
define action called parse_csv_line:
    parameter line as Text
    store fields as split line by ","
    return fields
end action
```

### 10. Quality Assurance

#### Test Coverage Requirements
- [ ] Unit tests for both split functions
- [ ] Integration tests in TestPrograms/
- [ ] Edge case coverage (empty strings, no delimiters, etc.)
- [ ] Error condition testing
- [ ] Performance benchmarks

#### Code Quality
- [ ] Follow existing WFL patterns
- [ ] Proper error handling with codespan integration
- [ ] Memory efficiency (use Rc<str> appropriately)
- [ ] Clear function documentation

## Conclusion

This proposal addresses the critical gap in WFL's text processing capabilities by implementing both simple string splitting and completing the pattern-based split functionality. The approach follows WFL's TDD principles, maintains backward compatibility, and provides a natural progression from simple to advanced text processing operations.

The implementation will make WFL significantly more capable for text processing tasks while maintaining its beginner-friendly natural language syntax.

## Next Steps

1. **Approval**: Review and approve this implementation plan
2. **TDD Phase**: Write comprehensive failing tests first
3. **Implementation**: Follow the outlined sprint plan
4. **Testing**: Ensure all TestPrograms continue to pass
5. **Documentation**: Update all relevant documentation

---

*This proposal follows WFL's test-driven development methodology and backward compatibility commitments.*