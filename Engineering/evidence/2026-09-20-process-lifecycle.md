# Subprocess lifecycle acceptance evidence

Risk: R3, process ownership, cancellation, public syntax and CLI exit behavior.
Provider: WFL. Consumer: Scriptorium's required WFL-only test runner and HTTP
integration driver. No application or Python test-driver workaround is used.

## Behavioral baseline before implementation

Base: `cb1dadaad96939a4450a6eb2b3a6a51678035b7f`.
Runtime: official Windows nightly `26.9.12`, with its bin directory first on
`PATH` so the parent and all child runtimes match. Date: 2026-09-20.

Command: `wfl --test TestPrograms/process/lifecycle.test.wfl`.
Observed: 3 tests, 0 passed, 3 failed, exit 1. All programs parsed and executed.

1. Absolute fixture source path still used the repository working directory;
   assertion expected the isolated fixture directory.
2. Numeric wait obtained child exit 0, but subsequent output retrieval raised
   `Invalid process ID`; final-output assertion found empty text.
3. Foreground execution demonstrated a real `Division by zero` stderr
   diagnostic. Background execution returned only its stdout marker; assertion
   for that diagnostic failed even though child exit 1 was observed.

These establish the behavioral requirements. The implementation will add an
explicit working-directory clause and full-result completion, preserving the
existing numeric wait's release behavior. Green tests will use the additive
API: retaining every legacy completion indefinitely would break bounded
ownership, while silently dropping retained output would hide failures.

Planned contract: direct argv, optional per-launch cwd, finite explicit wait
timeout, joined bounded stdout/stderr result and exit status, atomic release,
idempotent close, reliable kill/reap on errors, and clean explicit program exit
codes. Existing subprocess opt-in policy remains authoritative. Tests, fixture
generation and assertions are WFL; existing Rust harnesses may discover them.

Green, compatibility, resource, platform, review and final CI evidence remain
pending. This document is not completion evidence.
