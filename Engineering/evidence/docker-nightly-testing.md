# Docker nightly test runtime — validation record

Date: 2026-09-12. Risk class: **R3** (credentials, destructive owned-tag
cleanup, concurrency, and release publication). Base: `55f0aba9`.

## Acceptance criteria

1. Other projects can mount their WFL scripts and execute `--test` using the
   image; failing assertions return a failing process exit status.
2. The nightly packages its tested static Linux binary on Blacksmith.
3. Only a changed, newer canonical WFL product version builds and publishes;
   unchanged or stale queued versions cannot replace the current image.
4. A verified replacement precedes removal of old managed version tags;
   unrelated tags and the current version survive cleanup.
5. Registry errors, failed tests, failed publication, and unexpected metadata
   fail closed. Credentials never enter image layers or test scripts.

## Red evidence

Commit `9a62a0c7` introduces the container acceptance harness and four workflow
policy tests before implementation. The command below failed with three
assertion failures and one missing-workflow error because Docker publication,
version gating, and credential-free image validation did not yet exist:

```text
python -m unittest discover -s tests/tooling -p test_docker_workflow.py -v
```

The WFL fixtures were separately checked against the existing local runtime:
mounted includes/file I/O/SQLite passed 3/3; the intentional failed assertion
returned exit 1; the stdin fixture passed 1/1. This is fixture validation,
not evidence that the Linux container boundary has passed.

## Green and release evidence

The first PR Docker workflow run, [34700711591](https://github.com/WebFirstLanguage/wfl/actions/runs/34700711591),
failed at startup because the organization Actions allowlist disallows the two
Blacksmith Docker actions. No jobs or tests executed. Repository policy cannot
override that inherited allowlist (HTTP 409); no policy settings changed.
Red commit `e719d3b1` captures the supported build path. The workflows now use
the installed Docker CLI on the same Blacksmith runners, with no new action
permission or downloaded replacement action. Optional remote Docker layer cache
is not enabled.

- `fc5e534a`: 31 registry/version/publication tests failed before the publisher
  existed. They exercise a real loopback HTTP transport and separately executed
  Docker process double; they do not claim live Docker Hub interoperability.
- `59eb5ab9`: negative transport, timeout, interrupted-publication, and full-CI
  release-gate cases. The malformed HTTP status exposed an unredacted exception;
  recovery exposed reliance on reproducible fresh image bytes.
- `478386b5`: expired bearer renewal failed before monotonic refresh was added.
- `db401250`: the locked re-plan lacked a required full-CI result assertion.
- `f5d306bb`: version-check container timeout lacked explicit daemon cleanup.

Local Green evidence:

- All 40 final publisher tests passed; the prior 39-case suite also passed an
  independent reviewer rerun before the final lifecycle regression was added.
- Five workflow policy tests passed after implementation.
- The complete Python tooling suite passed 77/77 before final lifecycle review.
- `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets
  --all-features -- -D warnings`, `cargo build --workspace --release --locked`,
  and `cargo test --workspace --locked` passed on Windows Rust 1.98.1. Existing
  ignored doctests remain unchanged; no new skip/retry was introduced.
- Documentation validation passed 36/36 with `--ci --force`; manifest-designated
  server examples receive their existing static validation layers.
- Windows integration runner passed its Rust suite and 144 WFL programs, with
  24 existing declared program exclusions. Windows web runner passed both HTTP
  scenarios; TLS was unavailable locally because OpenSSL is not installed and
  remains required in Linux CI.
- Static hygiene passed after all new paths were staged. Runtime home uses
  `/var/lib/wfl`, consistent with repository path-hygiene rules.
- Actionlint 1.7.12 passed the three changed workflows. Blacksmith labels are
  project-specific; optional shellcheck/pyflakes were unavailable locally.
- Independent R3 review found and verified fixes for immutable-artifact recovery,
  release gating, token expiry, and malformed-HTTP error redaction.

Release publication is gated on the full reusable CI workflow at the same SHA,
Windows/Linux build results, and real candidate-container acceptance. The
credential-free Docker validation workflow supplies PR container evidence.

Final [remote presubmit](https://github.com/WebFirstLanguage/wfl/actions/runs/34700985572)
and [Blacksmith container acceptance](https://github.com/WebFirstLanguage/wfl/actions/runs/34700985579)
passed, including Linux TLS and Windows integration, before PR #728 merged.
The first live nightly subsequently uploaded the tested versioned image but
stopped before rolling promotion on a Docker Hub tag-count mismatch. The
[provider repair record](docker-hub-tag-count-repair.md) preserves that failure,
the image digest, regression evidence, and the remaining live verification.
Local logs are ephemeral under `target/reports/docker-nightly/`; rolling
publication and live provider cleanup are not yet claimed here.
