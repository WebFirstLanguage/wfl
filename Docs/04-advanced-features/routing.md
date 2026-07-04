# Routing with `route`

`route` is WFL's natural-language way to **dispatch on a value** — pick one branch
out of many based on what a value is. It reads like the `check if` chains you
already know, but it says the subject once instead of on every line.

Its flagship use is web-server request routing (matching a URL path), but `route`
is general: it is WFL's `match`/`switch`, and works on any value.

## Why `route`?

Dispatching on a value is one of the most common shapes in real programs. Without
`route`, you repeat the subject on every arm:

```wfl
check if path is equal to "/":
    display "home"
otherwise check if path is equal to "/health":
    display "ok"
otherwise check if path is equal to "/pricing":
    display "pricing"
otherwise:
    display "not found"
end check
```

The value — the thing that actually varies — is buried in the middle of each line,
and `path is equal to` is repeated over and over. `route` factors that out:

```wfl
route path:
    when "/":
        display "home"
    when "/health":
        display "ok"
    when "/pricing":
        display "pricing"
    otherwise:
        display "not found"
end route
```

Same behavior, less noise. In fact, `route` **is** the `check if` chain above —
the parser rewrites one into the other — so there is nothing new to learn about how
it runs. Anything you can write in a `check if` body, you can write in a `when` arm.

## Anatomy of a `route` block

```wfl
route <subject>:
    when <pattern>:
        <statements>
    when <pattern>:
        <statements>
    otherwise:
        <statements>
end route
```

- **`route <subject>:`** — `<subject>` is any expression (a variable, a property, a
  function result). It is the value every arm is compared against.
- **`when <pattern>:`** — an arm. If `<pattern>` matches the subject, the arm's
  statements run and the block finishes. Arms are tried top to bottom, and the
  **first match wins** — exactly like `otherwise check if`.
- **`otherwise:`** — the default arm, run when no `when` matched. It is optional and,
  if present, must come last.
- **`end route`** — closes the block, like `end check` and `end action`.

If nothing matches and there is no `otherwise`, the `route` simply does nothing —
it is a no-op, never an error.

## Patterns

A `when` head accepts these pattern forms:

| Pattern                     | Matches when the subject …                    | Example |
|-----------------------------|-----------------------------------------------|---------|
| `when V`                    | is equal to `V`                               | `when "/pricing":` |
| `when V1 or V2`             | is equal to any listed value                  | `when 404 or 410:` |
| `when contains V`           | (text) contains the substring `V`             | `when contains "admin":` |
| `when one of L`             | is a member of the list `L`                   | `when one of asset_files:` |
| `when starts with V`        | (text) starts with the prefix `V`             | `when starts with "/api/":` |
| `when ends with V`          | (text) ends with the suffix `V`               | `when ends with ".css":` |
| `otherwise`                 | no earlier arm matched                        | `otherwise:` |

These are ordinary WFL comparisons under the hood: `when V` is `subject is equal to
V`, `when contains V` is `contains of subject and V`, `when starts with V` is
`starts_with of subject and V`, and so on. You can always fall back to a full
`check if` if you need a condition the pattern table doesn't cover.

### Equality and or-lists

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

### Text tests — `contains`, `starts with`, `ends with`

```wfl
route request_path:
    when starts with "/api/":
        display "API request"
    when ends with ".css":
        display "stylesheet"
    when contains "admin":
        display "restricted area"
    otherwise:
        display "static page"
end route
```

### List membership — `one of`

```wfl
store known_assets as ["/style.css", "/logo.svg", "/app.js"]
route candidate:
    when one of known_assets:
        display "serve known asset"
    otherwise:
        display "404 unknown asset"
end route
```

## First match wins

Arms are checked in order, so put more specific arms first. Here a request to
`/api/theme.css` matches both `starts with "/api/"` and `ends with ".css"`, and the
API arm wins because it is listed first:

```wfl
route path:
    when starts with "/api/":
        display "API"      // this runs for /api/theme.css
    when ends with ".css":
        display "stylesheet"
    otherwise:
        display "page"
end route
```

## `route` is a general match / switch

The subject is any expression, so `route` is not limited to web routing:

```wfl
route count:
    when 1:
        display "first"
    when 2 or 3:
        display "second or third"
    otherwise:
        display "later"
end route
```

## The No-Unlearning path

`route` is designed as a gradient, not a new dialect:

- `when "/pricing":` maps one-to-one onto `check if path is equal to "/pricing":`,
  and `otherwise` is the same word you already use.
- A `when` arm's body is ordinary WFL statements — nothing inside an arm is new.
- A beginner who only knows `check if` can read a `route` block on first sight, and
  can always rewrite it back into a `check if` chain with nothing to unlearn.

## Reserved words

`route` is a reserved keyword and cannot be used as a variable name. `when` is also
reserved (it is shared with `try`/`catch` error handling). If you were using `route`
as an identifier, rename it (for example, `route` → `current_route`).

## See also

- [Control Flow](../03-language-basics/control-flow.md) — the `check if` chain that
  `route` builds on.
- [Web Servers](web-servers.md) — `route`'s flagship use: request dispatch.
- [Keyword Reference](../reference/keyword-reference.md) — the full keyword list.
