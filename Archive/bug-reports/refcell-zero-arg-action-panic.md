# Bug Report: RefCell Double Borrow Panic in Zero-Arg Action Auto-Call

## Summary

Zero-argument user-defined actions that modify parent-scope variables cause a `RefCell already borrowed` panic when auto-called from certain expression contexts. The root cause is a temporary `Ref<Environment>` held across an `.await` boundary in `_evaluate_expression` (and `_execute_statement`), preventing the called function from acquiring a mutable borrow on the same environment.

## Severity

**Runtime panic** — crashes the interpreter with `thread 'main' panicked at src\interpreter\environment.rs:186:40: RefCell already borrowed`.

## Reproduction

Minimal WFL program:

```wfl
store counter as 0

define action called bump:
    change counter to counter plus 1
    give back counter
end action

store my_list as [bump]
display counter
```

Run: `wfl repro.wfl`

Result:
```
thread 'main' panicked at src\interpreter\environment.rs:186:40:
RefCell already borrowed
```

Also reproduces with a bare expression statement (no list needed):

```wfl
store counter as 0

define action called bump:
    change counter to counter plus 1
end action

bump
```

## Root Cause

Two locations in `src/interpreter/mod.rs` hold an immutable `Ref<Environment>` borrow across an async `call_function().await` call. When the called function's body tries to mutate a variable in the same environment (via parent-scope traversal), `borrow_mut()` panics because the immutable borrow is still active.

### Location 1: `_evaluate_expression` — Variable branch (line 6308)

```rust
// src/interpreter/mod.rs:6308
if let Some(value) = env.borrow().get(name) {
    match &value {
        // ...
        Value::Function(func) => {
            if func.params.is_empty() {
                // env.borrow() Ref is STILL ALIVE here
                self.call_function(func, vec![], *line, *column).await  // line 6329
            }
            // ...
        }
    }
}
```

### Location 2: `_execute_statement` — ExpressionStatement branch (line 2181)

```rust
// src/interpreter/mod.rs:2181
if let Some(Value::Function(func)) = env.borrow().get(name) {
    // env.borrow() Ref is STILL ALIVE here
    return self.call_function(&func, vec![], *var_line, *var_column)
        .await  // line 2186-2187
        .map(|value| (value, ControlFlow::None));
}
```

### Why the Ref stays alive

In Rust (all editions including 2024), the temporary `Ref<Environment>` created by `env.borrow()` in an `if let` scrutinee lives for the duration of the **then-block**. The Rust 2024 edition change ([if let temporary scope](https://doc.rust-lang.org/edition-guide/rust-2024/temporary-if-let-scope.html)) only shortens the lifetime so temporaries are dropped *before the else block* — but they still live through the entire then-block.

### The borrow conflict chain

1. `env.borrow()` on line 6308 (or 2181) creates an immutable `Ref` on the global environment
2. `.get(name)` returns an owned `Value::Function(...)`, but the `Ref` temporary persists through the then-block
3. Inside the then-block, `call_function()` is called:
   - `func.env.upgrade()` obtains an `Rc` to the **same global env** (the function captured it at definition time, line 2143: `env: Rc::downgrade(&env)`)
   - A child env (`call_env`) is created as a child of the global env
4. The function body executes `change counter to counter plus 1`
5. The assignment statement calls `call_env.borrow_mut().assign("counter", ...)` (line 2061)
6. `assign()` doesn't find `counter` in the local scope, so it walks parent scopes
7. `parent_rc.borrow_mut()` (environment.rs:186) attempts a **mutable borrow** on the global env
8. **Panic**: the global env is already immutably borrowed from step 1

### Why it works when actions DON'T modify parent scope

If the action only uses `display` or `give back` (no `change` to parent-scope variables), no parent-scope `borrow_mut()` is attempted, so the immutable borrow from step 1 doesn't conflict.

## Affected Code Paths

| Trigger | Location | Line |
|---------|----------|------|
| Variable in expression (e.g., list element, RHS of assignment) | `_evaluate_expression` Variable branch | 6308 |
| Bare expression statement (e.g., `bump` alone on a line) | `_execute_statement` ExpressionStatement branch | 2181 |

## Suggested Fix

Extract the `env.borrow().get(name)` result into a `let` binding before the `if let`, so the `Ref` is dropped before entering the block:

### Location 1 (line 6308):

```rust
// Before (buggy):
if let Some(value) = env.borrow().get(name) {
    // Ref alive here — borrow_mut() in call_function will panic
}

// After (fixed):
let lookup = env.borrow().get(name);
if let Some(value) = lookup {
    // Ref already dropped — safe to borrow_mut() in call_function
}
```

### Location 2 (line 2181):

```rust
// Before (buggy):
if let Some(Value::Function(func)) = env.borrow().get(name) {
    return self.call_function(&func, vec![], *var_line, *var_column).await...
}

// After (fixed):
let lookup = env.borrow().get(name);
if let Some(Value::Function(func)) = lookup {
    return self.call_function(&func, vec![], *var_line, *var_column).await...
}
```

This is a one-line change at each location. The owned `Value` returned by `get()` doesn't need the `Ref` to stay alive, so extracting it releases the borrow immediately.

## Scope of Impact

Any zero-arg user-defined action that modifies a parent-scope variable will panic when auto-called from:
- A list literal: `[bump]`
- A bare expression statement: `bump` (on its own line)
- Assignment RHS: `store x as bump`
- Any expression context that goes through `_evaluate_expression`'s Variable branch

Actions that only read parent-scope variables (or don't access parent scope at all) are unaffected, because read-only access uses `borrow()` which is compatible with the existing `borrow()`.

## Related

- **Commit e44195b**: Fixed double execution of side effects in list literals. That fix (the `requires_async_evaluation` pre-scan) works correctly and prevents double execution, but the underlying borrow conflict still causes a panic for actions that mutate parent scope.
- **`try_evaluate_variable_sync`** (line 6013): Has the same `env.borrow().get(name)` pattern but calls `handle_variable_auto_call` which is synchronous and returns `Ok(None)` for user functions, so it never reaches `call_function` — not affected.

## Test Coverage

The integration tests in `tests/list_double_execution_test.rs` were designed to work around this bug by using `display` for side-effect tracking instead of counter mutation. Once this bug is fixed, tests using `change counter to counter plus 1` inside list-embedded actions should also pass.
