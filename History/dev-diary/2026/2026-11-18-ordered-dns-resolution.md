# 2026-11-18 — Ordered DNS resolution, and DNS servers in `.wflcfg`

The Linux release binary is statically linked against musl (it has to be: the
old glibc build would not run on Debian 12 or Ubuntu 22.04, wfl#616). musl's
resolver sends every DNS question to **all** nameservers in `/etc/resolv.conf`
at once and takes the first reply. On a machine whose list is `10.3.0.100`,
`10.3.0.101`, `8.8.8.8`, Google's "no such name" for an internal host often
arrived first. Internal names failed at random, and every internal name was
sent to Google on every lookup.

PRD and threat model:
`Engineering/designs/2026-11-18-ordered-dns-resolution-design.md`.

## What changed

- **New module `src/interpreter/dns.rs`.** WFL asks DNS servers one at a time,
  in order, with glibc's rules, and hands the addresses to reqwest through
  `ClientBuilder::dns_resolver`. A final answer (addresses, "no such name", or
  "no addresses") ends the search. WFL moves to the next server only after no
  reply, "server failure", "refused", "not implemented", "format error", or an
  unreadable reply.
- **On by default on Linux** (the musl tarball, the Docker image built from it,
  and glibc builds from source). Windows and macOS keep the OS resolver unless
  `dns_servers` is set.
- **Four new `.wflcfg` keys:** `dns_servers`, `dns_servers_mode` (`only`,
  `before_system`, `after_system`), `dns_timeout_seconds`, and `dns_resolver`
  (`wfl` / `system`, the off switch).
- **Clear errors.** A failed lookup lists every server asked, in order, with
  the file it came from and what it said, plus a hint. It never repeats the
  URL's path or query.
- `wfl --dump-env` gained a `=== DNS ===` section; `wfl --configCheck`
  validates the new keys with the loader's own rules.
- Docs: `Docs/reference/configuration-reference.md` (DNS settings), the HTTP
  client section of `Docs/04-advanced-features/interoperability.md`,
  `Docs/reference/supported-platforms.md`, and the Linux tarball notes in
  `Docs/02-getting-started/installation.md`.

## Why it is built this way

- **Not reqwest's `hickory-dns` feature.** The original idea was to turn on
  that feature. Reading the source of `reqwest 0.13.4` and `hickory-resolver
  0.26.3` showed it would not help: it asks two servers at once, reorders them
  by measured speed, stops at a server that says "server failure", and
  silently falls back to Google DNS when `/etc/resolv.conf` cannot be read. A
  test now fails if that feature is ever enabled.
- **One single-server hickory resolver per server.** hickory still does the
  wire work (random query IDs and source ports, reply matching, TCP after a
  truncated reply), but each instance knows exactly one server, so it cannot
  race or reorder them. WFL's loop makes every ordering decision.
- **Fail closed.** An invalid `dns_servers`, mode, or resolver value makes every
  lookup fail with an explanation. Falling back to the system's servers would
  bring back exactly the leak the setting exists to prevent. An invalid
  `dns_timeout_seconds` only warns, because a wrong timeout cannot leak a
  question to an unchosen server.
- **No servers means an error, not `127.0.0.1`.** musl quietly tries
  `127.0.0.1` when `resolv.conf` has no servers; WFL says what it looked at
  instead. `dns_servers = 127.0.0.1` restores the old behavior explicitly.
- **WFL parses `resolv.conf` itself** (about 60 lines, glibc's rules: the first
  three `nameserver` lines, the last `search`/`domain` line, capped `ndots`,
  `timeout`, `attempts`) instead of adding the `resolv-conf` crate. That is one
  dependency fewer for a parser of a trusted local file.
- **A and AAAA are combined per server.** If one family fails while the other
  comes back empty, that server could not help and the next one is asked. This
  is slightly more forgiving than glibc, and it keeps IPv4-only internal hosts
  working when a server is flaky for A records (raised in PR review).

## How it was tested

Two Red → Green cycles, each with a test-only Red commit first:

1. **Through `.wflcfg`, the interpreter, and the real binary** (Red `2798c8d`,
   Green `b0ac379`): order, failover rules, A/AAAA rules, timeouts, error
   messages, fail-closed config, `--dump-env`, `--configCheck`, the off switch,
   `localhost`.
2. **Through the resolver's own API with a temporary `resolv.conf` and hosts
   file** (refactor `9ee7089` added `SystemSources`; Red `cbd8284`): the Linux
   default, `before_system`/`after_system`, search domains and `ndots`, the
   hosts file and its re-reading, the answer cache and its 1,024-name bound,
   concurrency, and the "no servers" error.

The DNS servers in the tests are real UDP and TCP servers on loopback
(`tests/suite/dns_fake_server.rs`) with scripted replies: delays, silence,
garbage, truncation, wrong query IDs, and every failure code. The main race test
gives the second server an instant "no such name" and the first a 100 ms delay;
WFL uses the first server's answer in 100 out of 100 runs, and the second server
is never asked.

## Surprises

- **hickory re-sends UDP questions inside one try.** With a silent server and a
  one-second timeout, hickory sends the same question again about every 333 ms
  (1.2× the measured round-trip time, at least ~333 ms, up to 3 re-sends). Its
  options do not expose this. It only ever goes to the server currently being
  asked, so ordering is unaffected, and it helps with lost packets. One cycle-2
  test had counted packets to check `attempts`; it now measures rounds by time
  instead, and the PRD wording was corrected.
- **The fuzz workspace resolved `async-trait` to a newer release** that pulls in
  `syn 3`. Its new lock entries were pinned to the root lockfile's versions, so
  no existing dependency moved.

## Not done (on purpose)

- Database connections (`sqlx`) still use the OS resolver.
- No DNS-over-HTTPS/TLS and no DNSSEC.
- No protection against DNS rebinding (a public name answered with a private
  address). Documented as a known risk. Under the PRD's moderate risk
  tolerance, a follow-up issue for optional rebinding protection must be filed
  before this merges.
- `before_system` / `after_system` are Linux-only for now.
