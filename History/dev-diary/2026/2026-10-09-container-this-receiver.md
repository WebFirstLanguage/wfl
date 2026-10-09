# Container actions call sibling actions through `this`

Issue #701 reported that a container action could not call another action of the same container. Every spelling failed before the program ran: `emit("x")` and `call emit with "x"` reported an undefined name, and `this.emit("x")` reported `Variable 'this' is not defined` (exit 3). Containers could hold shared state, but their actions could not cooperate, so a class-shaped port such as a minifier had to be rewritten around top-level actions.

The Maintainer chose the `this` receiver as the one supported form. Inside an instance action, `this` means the object the action was called on, so `this.emit("x")` inside a container mirrors `m.emit("x")` outside it. Bare sibling calls stay errors, but the analyzer now names the fix: `'emit' is an action of container 'M'. Inside the container's actions, call it on the current object: this.emit(...)`.

## `this` is additive

`this` is not a reserved word, and the keyword count stays at 181. The analyzer and type checker resolve `this` through the container context, the same way they already resolve property names. That means `this` means the object only where nothing else named `this` is in scope, which is exactly where the name used to be undefined. A program's own variable named `this` keeps its meaning. That covers a global, `store this as 5` inside an action, and `for each this in items`. This follows GOVERNANCE.md 3.1, which allows no break without a deprecation path. The interpreter binds `this` in each action frame by the same rule it uses for properties, so a same-named global still wins there too. `this` in a static action and `change this to ...` are rejected, each with a message that says why.

The analyzer now registers all of a container's instance actions before it analyzes any body, so an action may call a sibling declared after it. The type checker types `this` as an object of the container, so `this.sibling(...)` gets the usual arity, argument-type and not-found checks, including for inherited actions.

## Runtime: lexical frames and caller links

Container method dispatch used to create each action's scope as a child of its caller's scope, and it bound `this` and the properties with `Environment::define`. That call silently refuses any name an enclosing scope already holds, which caused three silent errors:

- A nested call did not get its own `this` or properties. `b.bump()` called from inside `a.poke(b)` ran on `a`: it printed "bump ran on a" and left `a.n=1 b.n=1`.
- An action read a caller's same-named local, parameter or loop variable instead of its own property, and then wrote that value into the object.
- A called action's `store x as ...` assigned the caller's own `x`, so a recursive `fib(10)` through `this` returned 5.

An action's scope now has the scope where its container was defined as its parent, which is how ordinary actions already work. Name lookup no longer reaches the caller. The scope the call came from is kept only as a separate `caller` link, on action frames and on the call scopes of ordinary actions called from user code. Each frame also records the object it serves.

An action works on copies of its object's properties and writes them back when it returns. For that reason, a nested call on the same object needs coordination:

- Before the call, the nearest running frame of that object on the call chain pushes its working copies into the object. The frame is found by following `caller` links, so the search crosses actions on other objects and ordinary actions.
- After the callee writes back, that frame's copies are refreshed.
- Arguments are evaluated before the callee's frame is built, so a write made by an argument is not overwritten by a stale copy.
- Reading `obj.prop` returns the running frame's working copy. A helper action that receives `this` therefore sees the caller's latest write.

## Behavior changes

Three older behaviors change. All three were silent errors that contradicted the analyzer's model of the program, and all are listed in CHANGELOG.md:

- A nested call on another object runs on that object.
- An action reads its own property, not a caller's variable with the same name. The old runtime printed `label=caller-param`, `label=loop-var` and finally `after: loop-var`. The same applies when a parameter of the calling action shares a property's name: the old runtime printed `buf=p`, the new one prints `buf=pq`.
- A called action cannot change its caller's locals. Before, a sibling or ordinary-action caller saw `tmp=99`; now it sees `tmp=1`, and `fib(10)` is 55.

All gated `TestPrograms/` programs pass unchanged.

## Review follow-up

An independent review of the first version returned REVISE. Its blocking finding was that caller and callee locals shared one scope, the third behavior above. The first version also broke previously valid programs that used `this` as a variable inside actions. The first version had bound `this` as a fixed symbol. The second version makes `this` additive and makes action frames lexical. The review also found that `call helper with this` lost the helper's write (`az` instead of `aHz`). The `caller` links fix that. The remaining review notes were corrected in the docs: the `include from` case and the concurrency caveat.

## Evidence

- The first red test-only ancestor is commit ccf2bda. It adds `tests/container_this_receiver_test.rs` and `TestPrograms/containers/sibling_actions.wfl`. On `dev` at b7fa839, 12 of its 13 Rust tests failed, and the program exited 3 with `Variable 'this' is not defined`. The 13th passed by design: it guards `this` as an ordinary name outside containers.
- The broaden step added four regression tests: a caller's parameter, a caller's local, a same-named global, and a sibling error caught by its caller. Their old-runtime results were captured by rebuilding without the interpreter change.
- The review red commit is ba5a3c7. On 688c5e7, four of its 24 tests failed, for these cases: recursion with locals, callee locals, `this` passed to an ordinary action, and existing variables named `this`. The TestProgram failed with `Expected 5 to equal 55`.
- The validated docs examples are `TestPrograms/docs_examples/containers/sibling_actions_01.wfl` and `bare_sibling_call_01.wfl`.

## Residual risk and known limits

- Under `main loop concurrently:`, two handlers running actions on the same object at the same time each work on their own copies, and the copy written back last wins. One handler's property changes can therefore be lost. This predates #701; the containers guide now states it. Fixing it means changing how actions hold property state, so it is left for a follow-up.
- `parent action_name` runs the parent's action in the scope where the parent container was defined, without the object's properties or `this`. A parent action that reads a property, or calls `this.other()`, therefore fails at run time when it is reached through `parent`. This predates #701 and is not changed here.
- A bare call with empty parentheses, such as `emit()`, is a parse error before analysis. It gets the parser's message, not the `this.emit(...)` suggestion. In a file that uses `include from`, a bare sibling call only draws an analyzer warning, and it fails at run time with `Undefined action`.
- Inside an action, a global with the same name as a property still hides the property, and when the action returns the global's value is still written into the property. Both behaviors predate this change.
