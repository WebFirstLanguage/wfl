# 2026-09-23 — Proposed container-based package system

## Scope and outcome

Added [the proposed package-system PRD](../../../Engineering/designs/wfl-package-system-prd.md).
This is a design-only R0 change, not a restored package manager or a registry
implementation. No runtime, CLI, parser, policy, archive record, or existing
test was changed. Adoption and registry trust-root decisions remain with
Maintainers.

The PRD selects a real container definition as the package root, preserves
ordinary instance creation, proposes static literal-map metadata and explicit
package binding, and defines source/resource membership, deterministic archives,
dependency resolution, locks, offline operation, and execution boundaries. It
reconciles the archived proposals, maps all 19 principles, and assigns 26 stable
requirements to future acceptance tests. It distinguishes declared permissions
from a runtime sandbox and exact-byte integrity from publisher trust.

## Evidence basis

Baseline: `77ac72a02290433563957ff81ad453fffaffda9c`, WFL `26.9.18`.
Read canonical instructions, foundation/documentation policies, root governance
and testing/hygiene/security policies, container/module parser and runtime,
analyzer/typechecker contracts, current documentation/tests, the six requested
archived designs, archive README, and package-system removal record/tests.

The actual implementation matters: exports do not enforce selective exposure;
includes share definitions while loads isolate them; scalar/default and static
state behavior is not equivalent to inert package inspection. New behavior is
explicitly proposed rather than attributed to today's runtime.

## Validation

Future package acceptance tests in the PRD are not claimed to exist or pass.
No artificial Red step is required for this prose-only change under
`testing.md` §6.3.

| Check | Result |
|---|---|
| `cargo test --locked --test package_protocol_removed_test --test static_container_member_test --test typechecker_container_contract_test --test container_parsing_fixes --test load_module_undefined_hint_test` | 57 passed; no failures or ignored tests |
| `cargo test --locked --test include_statement_test --test include_diamond_test --test interface_contract_test` | 40 passed; no failures; 3 pre-existing ignored tests in `include_statement_test` were not counted as passes |
| `cargo build --locked --bin wfl`; `cargo build --locked -p wfl-lsp` | Both passed, refreshing the local debug binaries used for direct verification |
| Actual `wfl-lsp --mcp` calls: `parse_wfl`, `analyze_wfl`, `typecheck_wfl`, `lint_wfl` | All three current WFL snippets checked; 12 successful responses, no diagnostics. The source-only MCP interface received the consumer with its included definition expanded |
| Actual two-file consumer, debug WFL | Exit 0; stdout exactly `Hello, Ada`. Non-fatal top-level type warnings about the included `Greeting` are documented in PRD §§2 and 9 |
| `scripts/validate_docs_examples.py`'s `validate_example` on the three extracted snippets | All passed through its five reported layers using the existing release binary; separate real MCP type checks above avoid treating the script's combined analysis layer as independent type-check evidence |
| `python -X utf8 scripts/validate_docs_examples.py --force --ci` | 37 passed, 0 failed at their configured layers. Existing manifest metadata warnings and configured static-only examples remain visible; this is not 37 runtime executions |
| Existing-map spelling of the three proposed metadata examples | Actual MCP parser accepted them as WFL map syntax. This does not validate or implement the proposed package schema |
| `python -X utf8 scripts/check_repo_hygiene.py --mode static` | Passed, including archive checksum verification |
| Hygiene functions applied to tracked files plus both untracked deliverables | Passed placement, forbidden-file, size, personal-path and formatting checks without staging changes |
| Local document links, requirement/acceptance mapping, whitespace | All PRD local links resolve; 26 requirements and 26 matching acceptance rows; new-file checks passed; `git diff --check` clean |

Python was not on the shell PATH, so the available bundled Python interpreter
was used. The first temporary verification harness stopped while printing
Unicode diagnostic text through the Windows default encoding; rerunning it
with Python's UTF-8 mode completed the harness. This was a reporting failure,
not a failed WFL test retried to obtain a pass.

Temporary verification source and outputs stayed under `target/`; no proposed
syntax was added to the shipped example manifest. The existing docs runner
limits single-file selection to that manifest, so its actual validation function
was called for the draft's extracted examples. The full runner was also run
unchanged. Local MCP calls were real JSON-RPC requests, not the docs script's
placeholder MCP helper.

Full workspace/release-platform gates were not run for this R0 document change.
This record is local Windows evidence, not a Linux/macOS/CI or package-feature
conformance claim. Existing ignored tests and current include-analysis warnings
remain limitations; no assertions, policies, or test filters were relaxed.
