# Deferred: Higher-Order Stdlib Functions

## Status: Not Yet Implemented

The following stdlib functions require callback/closure support and are deferred to a separate effort.

## Deferred Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `filter` | `filter(list, callback)` | Return new list with elements where callback returns true |
| `map` | `map(list, callback)` | Return new list with callback applied to each element |
| `reduce` | `reduce(list, callback, initial)` | Fold list into single value using accumulator |
| `foreach` | `foreach(list, callback)` | Execute callback for each element (no return) |
| `find` (callback) | `find(list, callback)` | Return first element where callback returns true |
| `every` (callback) | `every(list, callback)` | True if callback returns true for all elements |
| `some` (callback) | `some(list, callback)` | True if callback returns true for any element |

**Note:** `find`, `every`, and `some` currently have value-based implementations (comparing elements to a target value). The callback-based versions would be additive — either overloading the existing functions or adding new names like `find_where`, `every_where`, `some_where`.

## Why These Are Deferred

Native functions have the signature:

```rust
fn(Vec<Value>) -> Result<Value, RuntimeError>
```

This is synchronous. User-defined callbacks (`Value::Function`) require async interpreter execution because:

1. WFL functions can contain `await` expressions
2. The interpreter's `call_function()` is async
3. Native functions cannot hold a reference to the interpreter

## Architectural Options

### Option 1: Async Native Functions
Add a new variant that takes an interpreter reference:
```rust
AsyncNativeFunction(&'static str, fn(&mut Interpreter, Vec<Value>) -> BoxFuture<Result<Value, RuntimeError>>)
```
**Pros:** Clean API, reuses existing patterns.
**Cons:** Requires changes to the `Value` enum and function dispatch in the interpreter.

### Option 2: Language-Level Constructs
Implement `map`, `filter`, etc. as special syntax in the interpreter (similar to `for each`).
**Pros:** No changes to `Value` enum. Natural language syntax possible.
**Cons:** Not composable as values; can't pass `map` itself as an argument.

### Option 3: Sync Callback Mechanism
Add a sync callback type that wraps a `Value::Function` with a borrowed interpreter:
```rust
fn call_sync_callback(interpreter: &mut Interpreter, func: &Value, args: Vec<Value>) -> Result<Value, RuntimeError>
```
**Pros:** Minimal changes. Works for non-async callbacks.
**Cons:** Breaks if any callback uses async features.

### Recommended Approach
Option 1 is the cleanest long-term solution. It requires:
1. Adding `AsyncNativeFunction` variant to `Value`
2. Updating the interpreter's function dispatch to handle async natives
3. Implementing each higher-order function as an async native
