# Container actions call sibling actions through `this`

Issue #701 reported that a container action could not call another action of the same container. Every spelling failed before the program ran: `emit("x")` and `call emit with "x"` reported an undefined name, and `this.emit("x")` reported `Variable 'this' is not defined` (exit 3). Containers could hold shared state, but their actions could not cooperate, so a class-shaped port such as a minifier had to be rewritten around top-level actions.

The Maintainer chose the `this` receiver as the one supported form. Inside an instance action, `this` means the object the action was called on, so `this.emit("x")` inside a container mirrors `m.emit("x")` outside it. Bare sibling calls stay errors, but the analyzer now names the fix: `'emit' is an action of container 'M'. Inside the container's actions, call it on the current object: this.emit(...)`. `this` is not a reserved word. Outside container actions it remains an ordinary name, and the keyword count stays at 181.

## Static side

The analyzer and type checker bind `this` as a read-only name typed as an object of the container in every instance action body. `this.sibling(...)` is therefore checked like any other method call, including argument count and inherited actions. The analyzer now registers all of a container's instance actions before it analyzes any body, so an action may call a sibling declared after it. `this` in a static action, `change this to ...`, and `store this as ...` each get a message that explains why they are rejected.

## Runtime side

The interpreter already bound `this` when it dispatched a method, but the binding used `Environment::define`, which silently refuses a name that any enclosing scope already holds. Because an action frame's parent is the caller's scope, a nested method call did not get its own `this` or properties. Instead it reused the caller's bindings. Same-object calls worked by accident, and calls on another object did not. In the reproduction, `b.bump()` called from inside `a.poke(b)` ran on `a`: it printed "bump ran on a" and left `a.n=1 b.n=1` instead of `a.n=0 b.n=101`. That program passed analysis with only a type warning, so the corruption was silent.

Each action frame now owns its receiver. The frame binds `this` unconditionally and records which object it serves. Properties are bound over any binding that is visible only because the frame's parent is the caller's scope: another frame's working copy, or a caller's local or parameter. A same-named binding that is visible where the container was defined, such as a global, keeps its historical precedence. That matches how the analyzer resolves the name, and changing it would be a separate language decision.

An action works on copies of its object's properties and writes them back when it returns. That is why a nested call on the same object needs coordination. Before the call, the caller's working copies are pushed into the object. After the callee writes back, the caller's copies are refreshed. Frames are matched by object identity, so the same holds when the object is reached under another name. Arguments are now evaluated before the callee's frame is built, so an argument that itself calls an action on the object cannot be overwritten by a stale copy. Reading `obj.prop`, including `this.prop`, returns the running action's working copy, so `this.buf` right after `store buf as "new"` reads `new` instead of the stale value.

## Behavior changes to note

Two older behaviors change. Both were silent data corruption that contradicted the analyzer's model of the program:

- A nested call on another object now runs on that object, as described above.
- An action reads its own property even when the caller has a local, parameter, or loop variable with the same name. Before, the action read the caller's variable and then wrote that value into the object. The old runtime printed `label=caller-param`, `label=loop-var`, and finally `after: loop-var`. Similarly, a parameter of the calling action that shares a property's name no longer captures a sibling's write. The old runtime printed `buf=p`, and the new one prints `buf=pq`.

All 166 gated `TestPrograms/` programs pass unchanged.

## Evidence

The red test-only ancestor is commit ccf2bda. It adds `tests/container_this_receiver_test.rs` and `TestPrograms/containers/sibling_actions.wfl`. On `dev` at b7fa839, 12 of its 13 Rust tests failed and the program exited 3 with `Variable 'this' is not defined`. The 13th, which guards `this` as an ordinary name outside containers, passed by design. The broaden step added four regression tests: a caller's parameter, a caller's local, a same-named global, and a sibling error caught by its caller. Their old-runtime results were captured by rebuilding without the interpreter change, and are quoted above. The validated docs examples are `TestPrograms/docs_examples/containers/sibling_actions_01.wfl` and `bare_sibling_call_01.wfl`.

## Known limits

- `parent action_name` runs the parent's action in the scope where the parent container was defined, without the object's properties or `this`. A parent action that reads a property, or calls `this.other()`, therefore fails at run time when it is reached through `parent`. This predates this change and was not changed here.
- A bare call with empty parentheses, such as `emit()`, is a parse error before analysis. It therefore gets the parser's message, not the `this.emit(...)` suggestion.
- Inside an action, a global with the same name as a property still hides the property. When the action returns, the global's value is still written into the property. Both behaviors predate this change.
