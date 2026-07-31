# 2026-07-31 — Four gaps a config subsystem found (#664, #665, #666, #667)

## What changed

Four issues filed the same morning by one person who had just built a real
configuration subsystem in WFL. Read together they describe a language that
could mint a secret but not seal it, write a config file but not protect it,
read JSON but not the format most config files use, and write several rows but
not atomically. All four are now closed.

| Issue | Before | After |
| --- | --- | --- |
| #664 | `BEGIN`/`ROLLBACK` through `execute` silently did nothing | `in transaction on db: … end transaction`, and raw transaction SQL is refused |
| #665 | `secure_random_bytes of 32` minted a key with nothing to use it for | `seal` / `unseal` (XChaCha20-Poly1305) |
| #666 | No way to set or read a file's mode | `file_mode`, `set_file_mode` |
| #667 | No TOML support at all | `parse_toml`, `stringify_toml`, `stringify_toml_pretty` |

## #664 was the real bug

The other three are missing features. This one was a silent data-integrity
failure, which is worse, because it looked like it worked.

`open database` maintains a pool of five connections, and every `query`/`execute`
takes whichever one is free. WFL had no transaction construct, so the natural
workaround was to send transaction control as SQL:

```wfl
store t1  as execute db with "BEGIN"
store ins as execute db with "INSERT INTO projects (slug) VALUES ('should-vanish')"
store t2  as execute db with "ROLLBACK"
```

Those three statements land on three *different* connections. The `BEGIN` opens
a transaction on a connection that then goes back to the pool untouched; the
`INSERT` commits on its own; the `ROLLBACK` rolls back an empty transaction
somewhere else. Nothing errors. The reporter measured
`rows surviving rollback: 1` where the code plainly says 0.

The part that makes this properly nasty is the test story. In-memory SQLite is
special-cased to a single connection, because an in-memory database only exists
for the connection that made it. So the workaround *works* under
`sqlite::memory:` — which is what most tests use — and fails against a real
file-backed or networked database. A program could have a green suite and lose
data in production.

That shaped the tests before it shaped the fix: every atomicity assertion in
`tests/database_transaction_test.rs` and
`TestPrograms/database_transaction_test.wfl` runs against a temp *file*, with a
comment saying why, so nobody later "simplifies" them to in-memory and quietly
removes the coverage.

### The fix, both halves

`Pool::begin()` hands back an owned `Transaction<'static, DB>`, so a transaction
can be parked in a map keyed by database handle, right alongside the pool
itself. While an entry is present, `query`/`execute` on that handle route to the
transaction's pinned connection instead of taking a fresh one. That is the whole
mechanism; `run_query` and `run_execute` grew a `DbTarget` parameter that is
either a pool or a transaction, and the per-backend bodies became a macro so the
six combinations stayed one line each rather than six copies.

The block commits on a normal exit and rolls back on an error. `break`,
`continue` and `return` commit — they are ordinary ways to leave a block, and
the work inside genuinely finished. Only a failure rolls back. Nesting on one
handle and closing a database mid-transaction are refused with an explanation
rather than doing something surprising.

The second half matters as much: `BEGIN`, `COMMIT`, `ROLLBACK`,
`START TRANSACTION`, `SAVEPOINT` and `RELEASE` through `query`/`execute` now
raise an error that names the block syntax. Only the first real token is
inspected — leading comments are skipped first, so `-- go\nBEGIN` is caught —
and a column named `begin_at` or a value of `'rollback plan'` still runs.

I first wrote that this broke nothing, "because the old behavior did not work."
Review pushed back, and review was right. In-memory SQLite is capped at one
connection, so under `sqlite::memory:` the hand-written pattern *did* work; I
confirmed it by building `main` and watching a rollback correctly discard its
row. So this does remove working behaviour, on precisely the configuration most
WFL tests and examples use.

It ships as a hard error anyway, deliberately. The pattern is not a feature that
happens to be unsupported elsewhere — it is a program that passes in development
and silently loses writes the moment it meets a real database. Keeping it alive
for a deprecation window would mean keeping the corruption alive too, and the
only programs it can still "work" for are the ones most likely to be moved to a
pooled backend later. The error names the replacement, and the rewrite is
mechanical. Recorded in the changelog under Removed rather than buried in Fixed.

### `transaction` is not a keyword

Making it a lexer token would have broken any program using `transaction` as a
variable name, and backward compatibility is not negotiable here. It is
recognized positionally instead — an identifier directly after a leading `in`,
and after the `end` that closes the block. A statement beginning with `in` was
always a parse error before, so the syntax claims nothing that used to be valid.
It joins `secured`, `certificate`, `key`, `redirecting` and `content_type` in
the "marker words that are not keywords" list, and the reserved-keyword count
stays at 181.

One parser detail worth recording: the header uses `parse_primary_expression`,
not `parse_expression`. The general expression parser runs straight past the `:`
that ends the header and reports "expected Colon, found Eol" from the *next*
line — the same choice `open database at <url>` already makes.

## #665 — the key stops being decorative

The crypto stdlib was entirely one-way: hashes, MACs, password KDFs, CSPRNG,
constant-time compare. Every one of those verifies something you were handed. A
stored API token has to be *sent* to the service later, so a one-way function
cannot stand in — the reporter's design ended up storing a key and encrypting
nothing.

XChaCha20-Poly1305, from RustCrypto. The choice over AES-GCM is about the nonce:
192 bits is wide enough that a freshly generated random nonce per message is
safe with no counter and no caller-visible state. That lets the implementation
own nonce handling completely, and nonce reuse is the one mistake that breaks
this kind of encryption outright. A natural-language surface is the last place
to expose it.

```wfl
store project_key as secure_random_bytes of 32
store sealed as seal of api_token and project_key
store token  as unseal of sealed and project_key
```

The optional third argument is associated data, binding a ciphertext to its
context so a value lifted out of one record cannot be replayed into another. Two
arguments and three are the same call, so the beginner form is a strict subset
of the expert one — no unlearning.

Sealed values are `wflseal1:` plus hex of nonce ‖ ciphertext ‖ tag:
self-describing, versioned so the algorithm can change later without ambiguity
about what an old stored value holds, and hex to match `secure_random_bytes`,
which is where the key comes from. `unseal` reports every failure identically —
wrong key, flipped byte, truncated blob, mismatched context — so it cannot be
used as an oracle by someone holding the ciphertext.

## #666 — 0600 moves into the program

The reporter's mitigation was `UMask=0077` in a systemd unit. It works, and it
is invisible to the program and unverifiable from inside it. The half they cared
about more was the reading: even with a correct umask, a WFL program could not
implement "refuse to start if this config is group- or world-readable," because
it could not read the mode at all.

```wfl
set_file_mode of "settings.toml" and "0600"
store mode as file_mode of "settings.toml"       // "0600"
```

Mode parsing is strict on purpose. `"rw-------"`, `"0999"` and `"0o600"` are
errors, not values to be masked into *some* mode — landing on something more
permissive than the author wrote is exactly the failure this exists to prevent.

Windows was the interesting call. Modes do not map onto ACLs, so `set_file_mode`
raises an explicit unsupported error there rather than quietly doing nothing;
`file_mode` still returns a documented approximation from the read-only
attribute, so a cross-platform program can call it unconditionally. The
integration gate runs on Windows as well as Linux, so
`TestPrograms/file_mode_test.wfl` probes for support once and then asserts
whichever contract applies — the per-platform detail lives in the cfg-gated
Rust tests.

## #667 — the JSON deviation goes away

The reporter's specification mandated TOML 1.0. Their options were to hand-roll
a TOML subset in WFL, which gets progressively more wrong as it meets real TOML,
or to change the format; they changed the format to JSON and wrote down the
deviation.

`src/stdlib/toml.rs` is a close mirror of `src/stdlib/json.rs`, deliberately.
The issue sketched `parse_toml` / `to_toml`, but the existing JSON surface is
`parse_json` / `stringify_json` / `stringify_json_pretty` — matching the sketch
would have introduced a new inconsistency, so the naming mirrors what is already
there.

Two places TOML genuinely is not JSON, both handled explicitly rather than
fudged:

- **A TOML document is always a table.** There is no valid TOML file whose top
  level is an array. `stringify_toml` accepts only an object and says so, rather
  than emitting something that will not parse back.
- **TOML has no null.** Absence is a missing key, so a `nothing` value is
  omitted when writing a table. Inside an *array* there is no way to leave a
  hole, so that is an error — silently dropping an element would change the
  array's length.

Whole numbers write as TOML integers, so a round-tripped config reads
`listen_port = 8080` and not `listen_port = 8080.0`.

The module file is named `toml.rs` for symmetry with `json.rs`, and refers to
the crate as `::toml::` throughout so the module never shadows it.

## Testing

R3 across the board — concurrency and lifecycle and backward compatibility
(#664), crypto and secrets (#665), secrets and untrusted input (#666), untrusted
input (#667). Red first: a tests-only commit
(`test: failing coverage for transactions, AEAD, file modes and TOML`) with all
four suites failing for the intended reason, recorded in
`Engineering/evidence/red-664-667-transactions-aead-modes-toml.txt`:

```text
database_transaction_test  parse error on `in transaction on db:`
crypto_seal_test           Undefined variable 'seal'
filesystem_mode_test       Undefined variable 'file_mode' / 'set_file_mode'
toml_test                  Undefined variable 'parse_toml'
```

Rust suites in `tests/`, WFL end-to-end suites in `TestPrograms/` using
`describe`/`test` (the integration runner detects `describe` and adds `--test`,
so a failed assertion exits nonzero). The negative paths carry the weight: for
`unseal`, a wrong key, a flipped nonce byte, a flipped ciphertext byte, a
flipped tag byte, a truncated blob, an unknown version prefix and a mismatched
context; for transactions, rollback on error, atomicity across a partially
successful block, read-your-own-writes inside the block, nesting, close-during,
and the raw-SQL rejection including the case it must *not* fire on.

## Gotchas hit along the way

- **Two builtin catalogs and a static-contract table.** `builtins.rs` holds both
  a reserved-name list and a runtime inventory, and `stdlib/typechecker.rs`
  holds the static contracts; a lib test asserts the runtime inventory matches
  the registrations exactly. Six new builtins meant touching all of them, plus
  the arity tables. The test caught it immediately, which is the point of it.
- **`second` is a builtin.** A TestProgram variable named `second` failed with a
  confusing module-scope error. `port`, `server` and `text` are keywords too —
  worth remembering when naming test variables.
- **`expect` does not count as a use.** The analyzer reports variables that are
  only read by an `expect` assertion as unused. Pre-existing, noisy in test
  programs, not addressed here.
- **Disk.** A full debug+release build with `debug = true` on release exceeds
  this environment's allowance. Building the test profile with
  `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0` keeps `target/` small
  enough to hold both without touching the committed profile.

## What review caught

Three automated reviewers went over the PR. Most of what they raised was real,
and two findings were more serious than anything I had found myself.

**`hex_to_bytes` panicked on non-ASCII input.** It checked that the byte length
was even and then sliced the `str` at fixed two-byte offsets, which lands inside
a multi-byte character and panics on the boundary check. Keys and sealed values
are exactly the untrusted text this module promises to fail closed on — they
come from config files, database rows and HTTP requests — so a stored blob
containing one accented character could take down a running server. Every other
malformed input returned `None` and became a clean error; this one path escaped.
Now decoded over bytes, with tests for a non-ASCII key, a non-ASCII sealed value,
and a multi-byte character inside an otherwise well-formed key.

**A transaction was owned by a database handle, not by the handler that opened
it.** Under `main loop concurrently:` all handlers share one `IoClient` and can
name the same global handle, so an unrelated request running an ordinary
`execute` was silently enrolled in someone else's transaction — and its write got
rolled back by that other request. Transactions are now keyed by
`(scope, handle)`, where the scope is handler-local state swapped per poll by
`InstalledRunState`, exactly like the existing count-loop and call-stack
isolation. `tokio::task_local!` would not have worked: handlers are futures in a
`FuturesUnordered` on one thread, not separate tasks. The test drives two real
requests over a socket and asserts the bystander's row survives; with the scope
key removed it fails, and both rows disappear.

Also fixed: the transaction map's lock was held across the SQL `await`, so one
slow statement serialized every database operation in the program — transactions
now live behind their own per-handle lock, and the map's lock is held only long
enough to clone an `Arc`. `begin` had a check-then-insert race that let a second
concurrent begin replace and drop a live transaction; the handle is now reserved
under one atomic step before the round-trip. A leading SQL comment defeated the
transaction-control guard entirely (`-- go\nBEGIN` read as an empty first word).
Two of the analyzer's three AST walkers had never been taught about the new
statement, so a `random_seed` call inside a transaction block escaped the
insecure-RNG security lint. And `exit` inside a block committed its partial work,
which contradicted the rule that an abrupt stop discards it — it now rolls back.

Two documentation claims did not survive checking either. TOML dates round-trip
as *strings*, not dates, and integers above 2⁵³ are rounded by WFL's f64 numbers;
I had written that the round trip was exact. Both are now stated plainly and
pinned with tests.

The pattern worth remembering: the findings I would not have reached on my own
were all about what happens when two things run at once, or when input is
hostile. Those are the parts where reading the code carefully is not the same as
proving it.
