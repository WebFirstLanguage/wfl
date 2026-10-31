# `random_seed` is an error in any file that does crypto or auth — now actually

**Date:** 2026-10-31

The July diary entry that introduced `ANALYZE-SECURITY` said `random_seed` "is
now an error in any file that also performs cryptographic, authentication, or
session work." Checking that sentence word by word found four ways it was not
true.

## What was wrong

1. **Not anywhere in the file.** The lint's call collector listed the
   statements and expressions it knew about and ended in `_ => {}`. Everything
   else was invisible: container creation (property initializers, constructor
   arguments, property defaults), method-call arguments, `create map` and
   `create list`, `add … to`, file writes, `wait for`, outbound HTTP bodies and
   headers, `expect` assertions, websocket messages, and the *expression* form
   of a database query. A password hash written as
   `create new Account as a: password_hash is hash_password of pw end` was not
   "crypto" to the lint. This is the third time a missing arm opened a hole
   here; transaction blocks and database statements were the first two.
2. **Not under another name.** WFL lets a program store a builtin in a variable
   and call it through that name. `store reseed as random_seed` followed by
   `reseed of 42` seeded the generator without a single call named
   `random_seed`.
3. **Not in every file that runs.** The CLI checks the file it is given before
   running it, but `include from`, `load module from`, and `execute file at`
   only ran the semantic check on the file they loaded. An `auth.wfl` that
   seeded and hashed ran in full when included, and the program exited 0.
4. **Not in the editor.** The language server and the MCP `analyze_wfl` tool
   share a pipeline that ran semantic analysis and type checking only, so they
   reported a file `wfl` refuses to run as clean.

## What changed

- The collector is now `BuiltinUses`, an exhaustive walk with no catch-all arm.
  Every `Statement` and `Expression` variant is matched by name, so adding an
  AST node is a compile error until someone decides how the security walk sees
  it. That turns the recurring bug into a build failure.
- The walk records bare mentions as well as calls. A mention counts unless the
  file binds that name itself, because `store random_seed as 5` is legal WFL
  and after it the name is the program's own number. The same holds for a
  program action that replaces a replaceable builtin such as `session_cookie`
  (Codex review). Container method names are not bindings: they are reached
  as `object.method()`, so a method named `hash_password` does not hide the
  real builtin.
- Each runtime loader runs the same check right after its semantic check. The
  load fails with a runtime error naming the file, line, column, rule, and fix,
  before any statement of that file executes.
- The LSP pipeline runs the check, and LSP diagnostics that carry a line and
  column but no span are placed at that line instead of the top of the file.

What did not change: `sha256` and the WFLHASH family still do not count as
crypto for this rule. That exclusion is deliberate (checksums plus a
reproducible simulation must stay legal) and is pinned by tests, including in
the newly walked constructs.

## Compatibility

A program that seeded the generator and did crypto or auth inside one of the
constructs above, or in a file it included, loaded, or executed, used to run
and now does not. That is the behavior the rule was documented to have since
July, and the same reasoning that justified the original lint applies: the
programs it now stops are the ones it exists to stop. No `.wfl` file in the
repository contains both halves, so no example, test program, or docs example
changed behavior.

## Still open

The check is per file. A main program that seeds and includes a separate file
that does the crypto is not reported, because neither file contains both. The
docs say so plainly. Treating an include chain as one unit is possible but
would need a design decision about `load module` and `execute file`, which run
in separate scopes.

## Tests

`tests/suite/insecure_rng_seeding_coverage_test.rs` hides the only crypto call
or the only seed inside each previously skipped construct, covers both alias
directions with a control for a program variable named `random_seed`, and runs
the real binary through all three loaders with an insecure helper (must fail,
must not run the helper or continue) and a seed-only helper (must still run).
`wfl-lsp/src/core.rs` gained two tests for the editor. Each test was committed
and observed failing for its intended reason before the fix.
