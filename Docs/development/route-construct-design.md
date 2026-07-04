# Design: The `route` Construct

**Status:** Proposed (design/spec — not yet implemented)
**Author:** WFL design discussion
**Applies to:** Language + parser; flagship use is web-server request routing.

## Motivation

Dispatching on a value is one of the most common shapes in real WFL programs,
and today the only tool for it is a long `otherwise check if` chain. A typical
web server's request handler looks like this:

```wfl
check if path is equal to "/checkout/start":
    store api_out as call handle_checkout_start with body and secret and req_now and site_db
    respond to req with api_out and content_type "text/html"
otherwise check if path is equal to "/checkout/complete":
    store api_out as call handle_checkout_complete with body and secret and req_now and site_db
    respond to req with api_out and content_type "text/html"
otherwise check if path is equal to "/pricing":
    store body_html as call pricing_page
    respond to req with body_html and content_type "text/html"
otherwise check if path is equal to "/style.css":
    store asset_data as call read_public with "style.css"
    respond to req with asset_data and content_type "text/css"
... (a dozen more) ...
otherwise:
    store body_html as call not_found_page
    respond to req with body_html and status 404 and content_type "text/html"
end check
```

Three problems:

1. **The subject repeats on every arm.** `path is equal to` appears N times; the
   thing that varies — the value — is buried in the middle of each line.
2. **Mechanical arms drown out the contract.** Asset routes (`/style.css`, SVGs)
   are pure boilerplate: the filename is the path minus its slash and the content
   type follows the extension. They add vertical noise around the routes that
   actually carry logic.
3. **There is no natural "dispatch on a value" form.** WFL has no `switch`/`match`;
   the `otherwise check if` chain is the workaround, and it reads as a workaround.

## Non-goals

- We do **not** hide the list of routes behind a registry or reflection. The list
  of URLs an app answers is its contract and must stay readable and greppable
  (Foundation principles #3 Readability, #11 Balanced Simplicity & Power). `route`
  makes the list *cleaner*, never *invisible*.
- We do not change `check if` semantics. `route` is additive.

## Foundation alignment — the No-Unlearning Invariant

Per `Docs/wfl-foundation.md`, the governing test is:

> For every feature, the beginner form and the expert form must be the same form,
> or connected by a smooth path with nothing to unlearn.

`route` is designed as a **gradient**, not a new dialect:

- The head `when "/pricing":` maps one-to-one onto the beginner's existing mental
  model of `check if path is equal to "/pricing":`. `otherwise` is the *same word*
  they already know.
- The block body of a `when` arm is ordinary WFL statements — the exact same
  statements the beginner already writes. Nothing inside an arm is new.
- The terser expert forms (declarative arms, below) are **optional** and reachable
  by growth. A beginner can always fall back to a full statement body, and an
  expert form never invalidates the simple form.

A beginner who only knows `check if` can read a `route` block on first sight, and
an expert can compress without the beginner having to unlearn anything. The
invariant holds.

## Syntax

### Level 1 — imperative arms (beginner; zero prerequisites)

```wfl
route path:
    when "/":
        store body_html as call home_page
        respond to req with body_html and content_type "text/html"
    when "/health":
        respond to req with "OK" and content_type "text/plain"
    otherwise:
        store body_html as call not_found_page
        respond to req with body_html and status 404 and content_type "text/html"
end route
```

- `route <subject>:` — `<subject>` is any expression (here the variable `path`).
- `when <pattern>:` — a block-bodied arm; runs if `<pattern>` matches the subject.
- `otherwise:` — the default arm (optional; if omitted and nothing matches, the
  `route` is a no-op, matching `check if` semantics).
- `end route` — closes the block, consistent with `end check` / `end action`.

This is **pure syntactic sugar** over the existing `check if` chain (see
Desugaring). It needs no analyzer, type-checker, or interpreter changes.

### Patterns

| Pattern form                     | Matches when …                                  | Desugars to |
|----------------------------------|-------------------------------------------------|-------------|
| `when "/pricing":`               | subject `is equal to` the value                 | `is equal to` |
| `when "/a" or "/b":`             | subject equals any listed value                 | `or`-chain of equalities |
| `when starts with "/api/":`      | subject (text) starts with the prefix           | `starts with` |
| `when ends with ".css":`         | subject (text) ends with the suffix             | `ends with` |
| `when contains "admin":`         | subject (text) contains the substring           | `contains` |
| `when one of asset_files:`       | subject is a member of the list                 | list `contains` |
| `otherwise:`                     | no earlier arm matched                          | trailing `otherwise` |

All pattern forms reuse operators WFL already has, so nothing new must be learned
to read them.

### Level 2 — declarative arms (expert; optional response shorthands)

For the extremely common "match a path, produce a response" shape, `route` offers
one-line arms:

```wfl
route path:
    when "/pricing" show page pricing_page
    when "/license" show page license_page
    when "/style.css" serve asset "style.css"
    when starts with "/api/" call api_router with req
    otherwise show page not_found_page with status 404
end route
```

- `show page <action>` → calls `<action>` and responds with `content_type "text/html"`.
- `serve asset <name>` → reads `<name>` from the public directory and responds with
  the content type inferred from its extension (safe: name is not a caller-supplied
  path; see Security).
- `call <action> [with <args>]` → calls `<action>` and responds with its result.
- Any arm may add `with status <n>` and/or `with content_type <ct>`.

These shorthands desugar to the same statements you would have written by hand.
They read as English to a beginner and are droppable back to full block bodies at
any time, satisfying the invariant.

**Feasibility note:** `show page pricing_page` names an action as a value. WFL
already represents actions as first-class `Value::Function` and the interpreter
already calls a function value through `Expression::FunctionCall`
(`src/interpreter/mod.rs`). So Level 2 does **not** require a new dispatch engine —
only surface syntax that references an action by name and emits a `FunctionCall`.

## Desugaring (implementation strategy)

The lowest-risk, most backward-compatible implementation lowers `route` **in the
parser** into the AST WFL already executes. No new runtime behavior is introduced,
which keeps the change small and keeps every existing `TestPrograms/` program
untouched.

```
route <subject>:
    when P1: B1
    when P2: B2
    otherwise: Bd
end route
```

lowers to:

```
check if <subject> <matches P1>:
    B1
otherwise check if <subject> <matches P2>:
    B2
otherwise:
    Bd
end check
```

where `<matches Pn>` expands per the pattern table above. Level 2 arm bodies lower
to the equivalent `store … as call …` + `respond …` statements.

Two viable lowerings:

1. **Parser desugaring (recommended for Level 1):** `parse_route` builds the
   existing `Statement::IfStatement` chain directly. Analyzer, type checker, and
   interpreter need no changes. Fastest path to a correct, safe feature.
2. **Dedicated AST node (`Statement::Route { subject, arms, default }`):** cleaner
   for LSP/tooling (hover, folding, "list all routes") and better error messages,
   at the cost of touching analyzer + interpreter. Recommended as a follow-up once
   Level 1 sugar is proven, so tooling can special-case routes.

Start with (1); migrate to (2) if tooling wants first-class route awareness.

## Security

`serve asset <name>` must only ever serve from the configured public directory and
must reject `..`/absolute paths — the `name` is author-provided in the source, but
the desugaring helper should still sandbox to prevent a future refactor from
piping a request path straight through. This preserves Foundation principle #8
(secure by default) and mirrors the existing `read_public`/allowlist pattern in
`comprehensive_web_server_demo.wfl`.

## Other uses beyond web routing

Because the subject is any expression and patterns cover equality, membership, and
text tests, `route` is really WFL's natural-language `match`/`switch`:

```wfl
route status_code:
    when 200:
        display "OK"
    when 404 or 410:
        display "Gone"
    otherwise:
        display "Unexpected status"
end route
```

Web routing is the flagship use and the reason for the keyword's name, but the
construct is general.

## Reserved-keyword impact

`route` becomes a structural keyword; `when` becomes contextual (it already reads
naturally and does not collide with common identifiers). Per the two-tiered keyword
policy, both `Docs/reference/keyword-reference.md` and
`Docs/reference/reserved-keywords.md` must be updated when this lands, and the
total keyword count adjusted. Follow the No-Unlearning Invariant here too: prefer
`when` remaining usable as an identifier outside a `route` head so beginners are
never told "you can't name it that."

## Phased implementation plan (TDD)

Per `CLAUDE.md`, write failing tests first at each phase.

1. **Lexer:** add `route`/`when` tokens (keep `when` contextual). Tests:
   `src/lexer` token tests.
2. **Parser (Level 1):** `parse_route` desugars to the `IfStatement` chain.
   Tests: parser unit tests asserting the lowered AST equals the hand-written
   `check if` chain; `--parse` snapshot.
3. **End-to-end (Level 1):** `TestPrograms/route_comprehensive.wfl` covering every
   pattern form, run under the release build; add to the docs-examples manifest.
4. **Patterns:** add `starts with` / `ends with` / `contains` / `one of` heads.
5. **Level 2 shorthands:** `show page` / `serve asset` / `call` arm forms +
   `with status` / `with content_type` modifiers.
6. **User docs:** once validated, promote examples into
   `Docs/04-advanced-features/routing.md` and cross-link from
   `Docs/03-language-basics/control-flow.md`; update keyword references.
7. **LSP / dedicated AST (optional follow-up):** `Statement::Route` node for route
   awareness in tooling.

## Interim: what to do today (no language change)

Until `route` lands, the same block can be tightened with existing features by
collapsing the mechanical arms and factoring the shared response tail. The asset
cluster reduces from one arm per file to a single allowlisted arm plus a
content-type helper:

```wfl
store asset_files as ["style.css", "logbie-wordmark.svg",
    "logbie-wordmark-on-forest.svg", "bie.svg", "bie-waiting.svg"]

define action called content_type_for with name:
    check if name ends with ".css":
        return "text/css"
    otherwise check if name ends with ".svg":
        return "image/svg+xml"
    otherwise check if name ends with ".json":
        return "application/json"
    otherwise:
        return "text/html"
    end check
end action
```

```wfl
otherwise check if contains of asset_files and substring of path from 1:
    store filename as substring of path from 1
    store asset_data as call read_public with filename
    store ctype as call content_type_for with filename
    respond to req with asset_data and content_type ctype
```

This is the drop-in for the six asset arms; the API and page arms stay explicit
because they *are* the route contract. See
`TestPrograms/route_interim_pattern.wfl` for a validated demonstration of the
reusable pieces (`content_type_for` + allowlist dispatch).
