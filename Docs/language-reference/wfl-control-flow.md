# Control Flow Handling in the WFL Interpreter

The WFL interpreter implements structured control flow handling for loops and functions using a dedicated `ControlFlow` enum. This allows for proper handling of `break`, `continue`, `exit`, and `return` statements within nested structures.

## Control Flow Enum

The `ControlFlow` enum is defined in `src/interpreter/control_flow.rs` with the following variants:

```rust
pub enum ControlFlow {
    None,       // Normal execution, no control flow change
    Break,      // Break out of the current loop
    Continue,   // Skip to the next iteration of the current loop
    Exit,       // Exit from the outer loop (used by WFL's `exit loop`)
    Return(Value), // Return a value from an action/function
}
```

## Control Flow Propagation

All statement execution functions return a tuple of `(Value, ControlFlow)` where:
- `Value` is the result value of the statement
- `ControlFlow` indicates any special control flow signal

Most statements return `ControlFlow::None`, indicating normal execution flow. Special statements return specific control flow signals:

- `BreakStatement` returns `ControlFlow::Break`
- `ContinueStatement` returns `ControlFlow::Continue`
- `ExitStatement` returns `ControlFlow::Exit`
- `ReturnStatement` returns `ControlFlow::Return(value)`

## Loop Handling

Each loop implementation (`ForeverLoop`, `CountLoop`, `RepeatWhileLoop`, `RepeatUntilLoop`, `ForEachLoop`) handles control flow signals as follows:

- On `ControlFlow::Break` → Break out of the current loop only
- On `ControlFlow::Continue` → Skip to the next iteration of the current loop
- On `ControlFlow::Exit` → Propagate upward to allow breaking out of nested loops
- On `ControlFlow::Return` → Propagate upward to the function caller

## Function/Action Handling

When a function encounters a control flow signal:
- `Return` signals are consumed by the function, which returns the specified value
- Other control flow signals (`Break`, `Continue`, `Exit`) are propagated upward

## Nested Structure Handling

Control flow signals bubble up through nested structures:
- A `break` in a nested loop only breaks out of the innermost loop
- An `exit loop` breaks out of the outer loop, regardless of nesting depth
- A `return` in a nested loop or conditional exits the entire function

This structured approach ensures that control flow statements work correctly in all contexts, including complex nested structures.
