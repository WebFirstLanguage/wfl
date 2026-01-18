# Parser Bug Report: Incorrect Operator Precedence in Function Call Arguments

## Summary
Function call arguments parsed with `parse_argument_list()` use `parse_primary_expression()` instead of `parse_expression()`, causing binary operations in arguments to be misparsed. This leads to incorrect grouping and, in recursive functions, infinite recursion.

## Bug Details

### Location
- **File**: `src/parser/mod.rs`
- **Function**: `parse_argument_list()` (line 5449)
- **Problem Line**: Line 5476: `let arg_value = self.parse_primary_expression()?;`

### Current Behavior
When parsing `factorial with n minus 1`, the expression is incorrectly parsed as:
```
(factorial with n) minus 1
```
Which becomes: `factorial(n) - 1`

### Expected Behavior
The expression should be parsed as:
```
factorial with (n minus 1)
```
Which becomes: `factorial(n - 1)`

## Root Cause Analysis

### 1. Parsing Flow for `n times factorial with n minus 1`

#### Step-by-Step Trace:
1. **Parse `n`** (primary) → `left = Variable("n")`
2. **See `times`** → operator with precedence 2
3. **Parse right side** with precedence 3: `parse_binary_expression(3)`
   - **Parse `factorial`** (primary) → `left = Variable("factorial")`
   - **See `with`** → Token::KeywordWith (line 1943)
   - **Check** if "factorial" is in `known_actions` → YES (after our fix)
   - **Create ActionCall**, parse argument list
   - **Argument parsing** calls `parse_primary_expression()`
     - Parses **only** `n` (primary expression)
     - Sees `minus` → NOT part of primary expression → **stops**
   - **Return** `ActionCall("factorial", [n])`
   - Back in binary expression loop, `left = ActionCall("factorial", [n])`
   - **See `minus`** → precedence 1 < 3 → **break** (precedence check fails)
   - **Return** `ActionCall("factorial", [n])`
4. **Back to step 2**: `left = BinaryOp(n, Multiply, ActionCall("factorial", [n]))`
5. **See `minus`** → precedence 1 ≥ 0 → parse right side
6. **Parse `1`**
7. **Final AST**: `BinaryOp(BinaryOp(n, *, factorial(n)), -, 1)`

#### Result:
The expression is parsed as: `(n * factorial(n)) - 1`

This is **completely incorrect**!

### 2. Why This Causes Infinite Recursion

When the factorial function executes:
```wfl
give back n times factorial with n minus 1
```

The misparsed expression `(n * factorial(n)) - 1` causes:
- `factorial(5)` calls `factorial(5)` (same argument!)
- This repeats infinitely → **stack overflow**

### 3. Why Parentheses Fix the Issue

With explicit parentheses:
```wfl
give back n times (factorial with (n minus 1))
```

The parser sees:
1. Parse `n`
2. See `times`
3. Parse right side: `(factorial with (n minus 1))`
   - The parentheses force it to be parsed as a group
   - Inside the group: `factorial with (n minus 1)`
   - The inner parentheses force `(n minus 1)` to be evaluated first
   - Result: `factorial(n - 1)` ✓

## Operator Precedence Table

Current precedence values in `parse_binary_expression()`:

| Operator | Precedence | Token |
|----------|-----------|-------|
| And, Or, Equals | 0 | `and`, `or`, `is equal to`, etc. |
| Plus, Minus | 1 | `plus`, `minus` |
| Times, Divide | 2 | `times`, `divided by` |
| With (ActionCall) | **N/A** | `with` (special handling, no precedence) |
| With (Concatenation) | **N/A** | `with` (calls `parse_expression()` = precedence 0) |

## The Core Problem

### `parse_argument_list()` Design Flaw

```rust
// Line 5476 - CURRENT (WRONG)
let arg_value = self.parse_primary_expression()?;
```

**Primary expressions** include ONLY:
- Literals (numbers, strings, booleans)
- Variables
- Parenthesized expressions
- List/map literals
- Function calls (recursively)

**Primary expressions do NOT include**:
- Binary operations (`+`, `-`, `*`, `/`)
- Comparisons (`is equal to`, `is greater than`)
- Logical operations (`and`, `or`)

This means: `factorial with n minus 1` parses `n` and stops at `minus`.

## Impact Analysis

### Affected Code Patterns

1. **Arithmetic in function arguments**:
   ```wfl
   // BROKEN
   function with a plus b    → function(a) + b
   function with x times 2   → function(x) * 2

   // WORKAROUND
   function with (a plus b)  → function(a + b) ✓
   ```

2. **Recursive functions**:
   ```wfl
   // BROKEN - causes stack overflow
   factorial with n minus 1  → factorial(n) - 1 → infinite recursion!

   // WORKAROUND
   factorial with (n minus 1) → factorial(n - 1) ✓
   ```

3. **Multiple arguments with expressions**:
   ```wfl
   // BROKEN
   sum with a plus 1 and b minus 1  → ???

   // WORKAROUND
   sum with (a plus 1) and (b minus 1) ✓
   ```

## Proposed Fixes

### Option 1: Parse Full Expressions in Arguments (RECOMMENDED)

```rust
// Line 5476 - PROPOSED FIX
let arg_value = self.parse_expression()?;  // Instead of parse_primary_expression()
```

**Pros**:
- Allows natural syntax: `function with a plus b`
- Consistent with how expressions work elsewhere
- Matches user expectations

**Cons**:
- May need careful handling of `and` keyword (argument separator vs. logical operator)
- Could require precedence-based argument parsing

### Option 2: Parse with Precedence Limit

```rust
// Line 5476 - ALTERNATIVE FIX
let arg_value = self.parse_binary_expression(1)?;  // Allow precedence >= 1
```

**Pros**:
- Allows arithmetic but not comparisons in arguments
- Prevents ambiguity with logical operators

**Cons**:
- Still requires parentheses for comparisons: `function with (a is equal to b)`
- May not be intuitive

### Option 3: Document Current Behavior

Keep the current implementation but clearly document that:
- Function arguments must be primary expressions
- Use parentheses for any operations: `function with (a + b)`

**Pros**:
- No code changes needed
- Explicit syntax

**Cons**:
- Counterintuitive
- Error-prone
- Already caused bugs

## Recommendation

**Implement Option 1** with special handling for the `and` keyword:
1. When parsing arguments, track context (in argument list vs. in expression)
2. In argument list context, `and` at precedence 0 terminates argument
3. In expression context, `and` is logical operator
4. Use precedence-based parsing: `parse_binary_expression(1)` to allow arithmetic but special-case `and`

### Proposed Implementation

```rust
fn parse_argument_list(&mut self) -> Result<Vec<Argument>, ParseError> {
    let mut arguments = Vec::with_capacity(4);
    let before_count = self.tokens.clone().count();

    loop {
        // Check for named arguments (name: value)
        let arg_name = if let Some(name_token) = self.tokens.peek().cloned() {
            if let Token::Identifier(id) = &name_token.token {
                if let Some(next) = self.tokens.clone().nth(1) {
                    if matches!(next.token, Token::Colon) {
                        self.tokens.next(); // Consume name
                        self.tokens.next(); // Consume ":"
                        Some(id.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // FIX: Parse full expression instead of just primary
        // But stop at 'and' keyword when used as argument separator
        let arg_value = self.parse_argument_expression()?;

        arguments.push(Argument {
            name: arg_name,
            value: arg_value,
        });

        if let Some(token) = self.tokens.peek().cloned() {
            if matches!(token.token, Token::KeywordAnd) {
                self.tokens.next(); // Consume "and"
                continue; // Continue parsing next argument
            } else {
                break;
            }
        } else {
            break;
        }
    }

    let after_count = self.tokens.clone().count();
    assert!(
        after_count < before_count,
        "Parser made no progress while parsing argument list"
    );

    Ok(arguments)
}

fn parse_argument_expression(&mut self) -> Result<Expression, ParseError> {
    // Parse expression but stop before 'and' if it would be an argument separator
    // This is a simplified version - full implementation needs careful precedence handling
    self.parse_binary_expression_until_and()
}
```

## Test Cases

### Test 1: Simple Arithmetic
```wfl
define action called add needs a and b:
    give back a plus b
end action

// Should parse as: add(x + 1, y - 1)
store result as add with x plus 1 and y minus 1
```

### Test 2: Recursive Functions
```wfl
define action called factorial needs n:
    check if n is equal to 0:
        give back 1
    otherwise:
        give back n times factorial with n minus 1  // Should work without parentheses
    end check
end action
```

### Test 3: Complex Expressions
```wfl
// Should parse as: calculate(a * 2, b / 3, c + d)
store result as calculate with a times 2 and b divided by 3 and c plus d
```

### Test 4: Comparisons (may still need parentheses)
```wfl
// This might be ambiguous - needs careful design
store result as check with (a is greater than b) and (c is less than d)
```

## Related Issues

1. **Type inference errors**: The current bug also causes type inference failures because the misparsed expressions have incorrect types
2. **Error messages**: Users get confusing errors when expressions don't parse as expected
3. **Documentation**: Current docs don't mention this limitation

## Priority

**CRITICAL** - This bug:
- Causes stack overflow in recursive functions
- Makes common patterns unusable without workarounds
- Violates principle of least surprise
- Already affected user code (nexus.wfl)

## Workaround for Users

Until this is fixed, **always use parentheses for any operation in function arguments**:

```wfl
// WRONG (causes bugs)
factorial with n minus 1
add with a plus b and c times 2

// CORRECT (use parentheses)
factorial with (n minus 1)
add with (a plus b) and (c times 2)
```

---

**Report Generated**: 2025-12-05
**Affected Version**: WFL 25.12.3
**Reporter**: Claude Code Investigation
