# 2026-07-28 — Linux builds move into CI (static musl) and artifacts publish to DigitalOcean Spaces

## What changed

Three things, in one change because they only make sense together:

1. **CI now builds Linux.** A new `build-linux` job in `nightly.yml` produces
   `wfl` and `wfl-lsp` for `x86_64-unknown-linux-musl`, statically linked.
2. **Artifacts publish to DigitalOcean Spaces**, which becomes the canonical
   download location. The GitHub Release remains, as a mirror.
3. **Runners were right-sized**, including a fix for the Windows nightly's
   timeout spiral.

## Why musl and not glibc

Until now the project shipped a Windows MSI nightly and no Linux artifact from
CI at all. The Linux tarballs that exist were built by hand on an out-of-band
host (`wflbuild`), on Ubuntu 24.04, against glibc.

That is issue **#616**: a binary linked against glibc 2.39 will not start on
Debian 12 or Ubuntu 22.04. It surfaced when the wfl binary refused to run in an
older sandbox, and again when Scriptorium needed a runtime on an older base
image.

Lowering the floor by building on Ubuntu 22.04 would have moved the problem
rather than removed it — there would still be *a* floor, and the next older
distro would hit it. A statically linked musl binary has no libc floor at all,
so the class of bug is closed rather than deferred.

Two gates enforce this, because a musl *target* does not by itself guarantee a
static *binary* — one dependency that links dynamically silently reintroduces
the floor while CI still reports success:

- **Both** binaries must have no `PT_INTERP` program header, per `readelf`, or
  the job fails: a binary that names no interpreter cannot be asking a loader
  for libc at startup. The first version of this gate grepped `file` output for
  `statically linked` and failed the whole nightly on a perfectly static
  binary, because Rust's musl target emits a static PIE, which `file` describes
  as `static-pie linked`. The ELF fact is the property being claimed; `file`'s
  phrasing is not.
- Both binaries are then executed inside `debian:12-slim` — the exact
  distro where the glibc build failed. This is the real boundary, not a proxy
  for it.

The dependency graph cooperates: TLS is rustls end to end (`ring` via sqlx,
`aws-lc-rs` via reqwest), so there is **no `openssl-sys`** anywhere — the usual
musl blocker does not apply. `aws-lc-sys` does compile C and assembly, which is
why the job installs `musl-tools`, `musl-dev`, `cmake`, and `clang`.

### wfl-lsp is now in the Linux tarball

The MSI has always shipped `wfl` and `wfl-lsp` together; the Linux tarball
carried only `wfl`. That made editor support Windows-only by accident of
packaging rather than by decision. The tarball layout is otherwise unchanged and
remains a strict superset of what is already published, so existing installers
keep working:

```
wfl-<version>-linux-x86_64/
  wfl
  wfl-lsp      <- new
  README.md
  LICENSE
  BUILD_INFO
```

## Publishing

`scripts/publish_spaces.sh` is the only writer to the bucket, so the layout is
defined in one place:

```
releases/wfl-<version>-linux-x86_64-<sha>.tar.gz   immutable
releases/wfl-<version>.msi                         immutable
releases/vscode-wfl-<version>.vsix                 immutable
releases/wfl-latest-linux-x86_64.tar.gz            rolling
releases/wfl-latest-windows-x86_64.msi             rolling
releases/SHA256SUMS                                checksums for this publish
status.json                                        last-publish record
```

Three things this script gets right that are easy to get wrong:

- **Objects are uploaded `public-read`.** The bucket's contents were private —
  the CDN returned `AccessDenied` for every key — so nothing could actually
  install from the published URL. The script ends by fetching two keys back
  *through the CDN* and failing the job on anything but a 200, so an ACL
  regression can never hide behind a green run.
- **Rolling pointers get `max-age=60`, not the CDN's 1-hour default.** Otherwise
  an installer can fetch a stale "latest" for an hour after a successful publish.
- **AWS CLI v2.23+ sends CRC32 integrity headers that Spaces rejects with a 400.**
  `AWS_REQUEST_CHECKSUM_CALCULATION=when_required` is set for this reason. This
  is the single most common cause of "the same command works against S3."

### Failure ordering

Two orderings matter, and the obvious choice for each is the wrong one.

**Within the script**, all immutable objects upload first; the rolling pointers,
`SHA256SUMS`, and `status.json` are written only once every one of them has
succeeded. Writing each rolling pointer next to its immutable counterpart reads
more naturally, but under `set -e` a later failure would then leave
`wfl-latest-linux-x86_64.tar.gz` pointing at the new build while the Windows
pointer, the checksums, and `status.json` still described the previous one: a
mixed release that consumers could observe indefinitely.

**Within the release job**, the Spaces publish runs *before* the nightly tag and
GitHub Release are created. Publishing last looks safer, but the tag and release
*are* the success marker `check-for-changes` reads: it resolves the newest
`nightly-*` tag to a commit and sets `should_build=false` when that matches HEAD.
A publish failure behind an already-pushed tag therefore would not be retried on
the next scheduled run, and the rolling downloads would stay stale until an
unrelated code commit happened to land. Failing before the tag is written keeps
the run retryable, and the canonical location being written before its mirror is
the right order anyway.

## Runner sizing

Cost on Blacksmith is strictly linear in vCPU, so a larger runner only saves
money if wall-clock time drops proportionally. Each job was classified by what
dominates its runtime rather than given a blanket size.

| Job | Before | After | Reasoning |
|---|---|---|---|
| nightly `build` (Windows) | `4vcpu-windows-2025` | `8vcpu-windows-2025` | See below |
| nightly `build-linux` | *(new)* | `8vcpu-ubuntu-2404` | Release codegen genuinely scales with cores |
| `fmt` | `4vcpu-ubuntu-2404` | `2vcpu-ubuntu-2404-arm` | `cargo fmt --check` never saturates one core |
| `fuzz-check` | `4vcpu-ubuntu-2404` | `4vcpu-ubuntu-2404-arm` | `cargo check` only, no shipped artifact |
| `bump-version` (both) | `4vcpu-ubuntu-2404` | `4vcpu-ubuntu-2404-arm` | Shells out to a locked `cargo check` on fuzz |
| `config-lint` | `4vcpu-ubuntu-2404` | `4vcpu-ubuntu-2404-arm` | Full `cargo build` — a compile, not a lint |
| `auto-fmt`, `update-security-doc` | `4vcpu-ubuntu-2404` | `2vcpu-ubuntu-2404-arm` | No compile at all |

**`clippy-and-test`, `integration-tests`, `database-tests`, and
`run-wfl-programs` deliberately stay on x64** even though nothing in them
requires it. They are the gate on the x86_64 binary the project ships; moving
them to ARM would mean testing an architecture that is not released. The ~38%
saving is not worth that gap.

### The Windows timeout spiral

The Windows nightly had died on its own timeout more than once (run
`30190411454`, and again after #641/#643 added 26 test files). The failure was
self-perpetuating: a cancelled job never runs its cache-save post step, so the
enlarged test binaries never landed in the `rust-cache` entry, and the next
nightly restarted from the same cold state and timed out identically. Raising
the timeout treated the symptom.

`cargo test --release` here compiles 112 integration test binaries, which
parallelizes well, so doubling the cores should more than halve wall-clock:
roughly 70 min at $0.016/min ($1.12) becomes roughly 30 min at $0.032/min
($0.96). Cheaper *and* it breaks the loop.

## Other CI hygiene in this change

- **`concurrency: cancel-in-progress`** on `ci.yml`. Without it, every push to a
  branch leaves the previous run compiling a commit whose result nobody will
  read — up to a full 30-minute `clippy-and-test` budget, per superseded push.
- **`CARGO_INCREMENTAL: 0`** in `ci.yml`, matching `nightly.yml`. The target dir
  is restored from cache and never reused across edits, so incremental artifacts
  only bloat what gets saved back.
- **`retention-days: 7`** on the artifact uploads. The 90-day default was
  retaining ~90 nightlies of MSIs and VSIXs on artifact storage, which GitHub
  bills separately from Blacksmith compute — so it shows up on a different
  invoice than people expect.

## Consequences

- `wflbuild.starnet` no longer has a job. Its decommission is deliberately *not*
  part of this change; it should happen only after several nightlies have proven
  the CI pipeline green, and it is reversible until then.
- **#616 should be verifiable as closed** once the first nightly publishes: the
  Debian 12 gate proves the property the issue is about.
- Docs ship with the change. `Docs/02-getting-started/installation.md` gains a
  *Linux x86_64 Tarball* section pointing at
  `https://wfl.nyc3.cdn.digitaloceanspaces.com/releases/wfl-latest-linux-x86_64.tar.gz`,
  which is now always current and no longer requires a glibc check first, and
  `Docs/reference/supported-platforms.md` no longer claims static-musl Linux has
  "no CI lane" and is unverified. Static musl stays **Tier 2**: the new lane is
  post-merge only and does not run the integration or `TestPrograms` suites, and
  the promotion policy in that document requires both.
