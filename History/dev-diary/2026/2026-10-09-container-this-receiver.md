# Container actions call sibling actions through `this`

Issue #701 reported that a container action could not call another action of the same container. Every spelling failed before the program ran: `emit("x")` and `call emit with "x"` reported an undefined name, and `this.emit("x")` reported `Variable 'this' is not defined` (exit 3). Containers could hold shared state, but their actions could not cooperate, so a class-shaped port such as a minifier had to be rewritten around top-level actions.

The Maintainer chose the `this` receiver as the one supported form. Inside an instance action, `this` means the object the action was called on, so `this.emit("x")` inside a container mirrors `m.emit("x")` outside it. Bare sibling calls stay errors, but the analyzer now names the fix: `'emit' is an action of container 'M'. Inside the container's actions, call it on the current object: this.emit(...)`.

## `this` is additive

`this` is not a reserved word, and the keyword count stays at 181. The analyzer and type checker resolve `this` through the container context, the same way they already resolve property names. That means `this` means the object only where nothing else named `this` is in scope, which is exactly where the name used to be undefined. A program's own variable named `this` keeps its meaning. That covers a global, `store this as 5` inside an action, and `for each this in items`. This follows GOVERNANCE.md 3.1, which allows no break without a deprecation path. The analyzer makes this decision where the container is defined. The interpreter makes the same decision at the same point: when a container definition runs, it records whether the program's own `this` is visible there. An enclosing action's frame binding of `this` does not count. Each action frame then binds `this` to the object or leaves the program's variable in charge. So a global `this` created after the container does not take over, and a container defined inside another container's action gets its own `this`. `this` in a static action and `change this to ...` are rejected, each with a message that says why.

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

Three older behaviors change. All three were silent errors that contradicted the analyzer's model of the program:

- A nested call on another object runs on that object.
- An action reads its own property, not a caller's variable with the same name. The old runtime printed `label=caller-param`, `label=loop-var` and finally `after: loop-var`. The same applies when a parameter of the calling action shares a property's name: the old runtime printed `buf=p`, the new one prints `buf=pq`.
- A called action cannot change its caller's locals. Before, a sibling or ordinary-action caller saw `tmp=99`; now it sees `tmp=1`, and `fib(10)` is 55.

Removing the caller-scope leak also changes two cases that are not wrong results:

- An instance action can no longer read a caller's local variable. That was only possible where the analyzer merely warns about an undefined name, inside `try` or in a file that uses `include from`. The read now raises an undefined-variable error, as it does in an ordinary action.
- An instance action called from a `describe` or `test` block now reads and changes the program's globals directly, as ordinary actions do. Before, it went through the test block's isolated copy: a `push` onto a global list had no effect, and `change` of a global failed.

CHANGELOG.md lists all five under its compatibility note. All gated `TestPrograms/` programs pass unchanged.

## Review follow-up

An independent review of the first version returned REVISE. Its blocking finding was that caller and callee locals shared one scope, the third behavior above. The first version also broke previously valid programs that used `this` as a variable inside actions. The first version had bound `this` as a fixed symbol. The second version makes `this` additive and makes action frames lexical. The review also found that `call helper with this` lost the helper's write (`az` instead of `aHz`). The `caller` links fix that. The remaining review notes were corrected in the docs: the `include from` case and the concurrency caveat.

A second independent review of the revised version found no blocking issues and confirmed the earlier findings fixed. It raised one new defect. A container declared inside an instance action was treated as if the enclosing action's `this` were a program variable, so its `this.note()` ran on the enclosing object. The same review also raised a related mismatch: the analyzer accepted a global `this` created after the container as meaning the object, but the runtime used the global. Deciding `this` at definition time, as described above, fixes both. It also asked for a complete compatibility note, which now lists the two cases above. Finally, it asked for the `parent` caveat in the user docs, for tests of every call form that records a `caller` link, and for the static-action message on `change this to ...`.

Codex then reviewed the ready-for-review PR and raised two findings:

- **A container may declare `property this` (P2).** Fixed. There the name was never undefined, so `this` keeps meaning the property. The analyzer now checks for a property before rejecting `change this to ...`. The runtime no longer binds the object over a property named `this`. Before #701 it did: reads returned the object, and the write-back stored the object into the property.
- **`initialize` reached through the constructor path does not bind `this` (P1).** Fixed after a human review requested it. That path runs only for an instantiation with constructor arguments. The parser never produces any (`parse_instantiation_body` always returns an empty list), so no program can reach it today. But the analyzer accepts `this` in `initialize`, and the path ran the action with no object at all: no receiver, no `this`, no properties. Both `obj.action(...)` and the constructor path now go through one shared `invoke_instance_action`, so `initialize` sees its object exactly as an ordinary call does. A test builds the instantiation with constructor arguments directly in the AST.

## Evidence

- The first red test-only ancestor is commit ccf2bda. It adds `tests/container_this_receiver_test.rs` and `TestPrograms/containers/sibling_actions.wfl`. On `dev` at b7fa839, 12 of its 13 Rust tests failed, and the program exited 3 with `Variable 'this' is not defined`. The 13th passed by design: it guards `this` as an ordinary name outside containers.
- The broaden step added four regression tests: a caller's parameter, a caller's local, a same-named global, and a sibling error caught by its caller. Their old-runtime results were captured by rebuilding without the interpreter change.
- The review red commit is ba5a3c7. On 688c5e7, four of its 24 tests failed, for these cases: recursion with locals, callee locals, `this` passed to an ordinary action, and existing variables named `this`. The TestProgram failed with `Expected 5 to equal 55`.
- The second review's red commit is 83e30d6. On 869c666, three of its 29 tests failed: the nested container, the late global `this`, and the static-action message.
- The Codex P2 red commit is aefb59e. On 71b1513, one of its 31 tests failed: the property named `this`.
- The constructor-path red commit is 31d98cf. On 48bb2ef, its test failed with `Undefined variable 'v'`: `initialize` ran without the object's properties.
- The validated docs examples are `TestPrograms/docs_examples/containers/sibling_actions_01.wfl` and `bare_sibling_call_01.wfl`.

## Residual risk and known limits

- Under `main loop concurrently:`, two handlers running actions on the same object at the same time each work on their own copies, and the copy written back last wins. One handler's property changes can therefore be lost. This predates #701; the containers guide now states it. Fixing it means changing how actions hold property state, so it is left for a follow-up.
- Static actions still run inside their caller's scope, so a static action's `store x as ...` can still overwrite its caller's `x`. This predates #701, and the containers guide says so.
- `parent action_name` runs the parent's action in the scope where the parent container was defined, without the object's properties or `this`. A parent action that reads a property, or calls `this.other()`, therefore fails at run time when it is reached through `parent`. This predates #701 and is not changed here.
- A bare call with empty parentheses, such as `emit()`, is a parse error before analysis. It gets the parser's message, not the `this.emit(...)` suggestion. In a file that uses `include from`, a bare sibling call only draws an analyzer warning, and it fails at run time with `Undefined action`.
- Inside an action, a global with the same name as a property still hides the property, and when the action returns the global's value is still written into the property. Both behaviors predate this change.
