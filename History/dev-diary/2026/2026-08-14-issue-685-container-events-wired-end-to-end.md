# 2026-08-14 — Container events stop being a no-op

## The problem

Container events were half-wired (issue #685). `event on_click` parsed and was
stored on the container definition; `trigger on_click` parsed and executed. What
did *not* work was everything in between:

1. **The handler statement could not be written.** `parse_event_handler` called
   `parse_expression()` for the event source. Since the lexer merges adjacent
   bare words into a single identifier, `on btn on_click` lexed as
   `KeywordOn` + `Identifier("btn on_click")` — the expression swallowed both
   names and the parser died with `Expected identifier for event name, found Eol`.
2. **The handler body was thrown away.** `parse_event_handler` was a stub that
   returned `handler_body: Vec::new()`.
3. **`trigger` never parsed arguments.** `parse_event_trigger` returned
   `arguments: Vec::new()`, so an event declared with `needs` could never
   receive values; the runtime bound every parameter to `Null`.
4. **Registration and triggering did not share state.** The `Statement::EventHandler`
   arm cloned the event, appended the handler, and defined the result as a plain
   environment binding. A `trigger` inside a method instead read events freshly
   injected from `ContainerDefinitionValue.events`, whose `handlers` list is
   always empty. Even with 1–3 fixed, a registered handler would never be seen.

The result was the worst of both worlds: a program could `trigger` all day and
look successful while running nothing. `TestPrograms/containers_comprehensive.wfl`
exercised exactly this much — its triggers "passed" because running zero handlers
is a no-op.

## The grammar

```wfl
create container Button:
    property label: Text
    event on_click
    action click:
        trigger on_click
    end
end

create new Button as save_button:
    label is "Save"
end

on on_click of save_button:
    display "Saving your work..."
end on

save_button.click()
```

The event name comes **before** the instance, separated by `of`. This is forced
by the lexer's word-merging: with the source first there is no token boundary
between the two names. `of` is the separator that keeps them apart, and it reads
naturally — "on on_click of save_button". The block closes with `end on`, matching
the existing `on websocket … end on` handler form.

Two more pieces of grammar:

- `trigger <event> with <a> and <b>` supplies positional values for the
  parameters the event declared with `needs`. Handlers refer to them by the
  declared names.
- `on <event>:` without `of` attaches to an event of that name already in scope.
  That makes the top-level `event`/`trigger` statements — which existed and were
  equally inert — usable on their own.

## Where handlers live

Handlers are stored **per instance**, not per container definition. Registering
on one `Button` must not fire for another one.

- `ContainerEventValue.handlers` became `Rc<RefCell<Vec<EventHandler>>>`, so
  registration and dispatch reach the same list no matter which path found the
  event.
- `ContainerInstanceValue` gained an `events` map. Each instance gets its own
  copy of the definition's events (`ContainerEventValue::fresh_instance`) with
  its own handler list; inherited events are shared with the parent instance's
  `Rc`, so a handler registered on the child is seen by a `trigger` running in an
  inherited action.
- The method-call path now injects the *instance's* events into the method
  environment instead of the definition's empty templates.
- A deep clone of an instance gets independent event registrations — the handlers
  registered so far carry over, later ones stay with their own copy.

A handler runs in a child of the environment it was **registered** in, so its
body sees the variables that were visible there rather than the trigger site's.

## Two problems the wiring surfaced

**Re-entrancy.** A handler may register another handler for the event being
dispatched. The dispatch loop snapshots the handler list before running anything,
so the newly added handler runs on the *next* trigger and the shared `RefCell` is
never borrowed across an `await`.

**Runaway recursion.** A handler that triggers its own event used to overflow the
native stack and abort the process, because dispatch went straight to
`execute_block` and bypassed the ceiling `call_function` enforces. Event dispatch
now checks `budget.check_call_depth` and takes a `CallDepthGuard`, so an event
loop ends with `Maximum call depth (1000) exceeded` like any other runaway
recursion.

## Static analysis caught up

- The analyzer and type checker now resolve events through the `extends` chain
  (`Analyzer::resolve_container_event`), so an inherited event no longer reports
  `Event 'started' not found in container 'Derived'`.
- The unused-variable pass walks handler bodies and trigger arguments. Without
  that, a variable read only by a handler body was falsely reported unused —
  a new false positive created by the bodies finally being non-empty.
- The fixer (`wfl --lint --fix`) reprints `event`/`trigger`/`on` properly instead
  of dumping Rust debug output. Its existing `event` arm also printed
  `event name with p as Type`, which never parsed back; the grammar is
  `event name needs p: Type`.

## TDD evidence

Red commit `test: cover container events end to end (red for #685)` is an
ancestor of the implementation commit. All 18 tests in
`tests/container_events_test.rs` and all 11 in
`TestPrograms/containers/events.wfl` failed against the pre-change tree and pass
after. Every test asserts an observable effect of a handler having *run* —
output, or a list the handler pushes to — never merely that the program did not
crash, which is exactly the trap the old tests fell into.

`TestPrograms/containers_comprehensive.wfl` now registers handlers in its events
section, so its triggers prove dispatch rather than proving nothing.

## Risk class

R3 — lifecycle and resource limits (re-entrant registration during dispatch,
recursion ceiling) plus backward compatibility. Nothing that previously parsed
changed meaning: the `on <source> <event>` form the AST implied never parsed
successfully, so the new grammar claims no previously valid syntax.
