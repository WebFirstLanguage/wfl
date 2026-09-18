# Dependabot 70: js-yaml merge-work limit

## Scope and fix

[Dependabot alert 70](https://github.com/WebFirstLanguage/wfl/security/dependabot/70)
reports [CVE-2026-84375 / GHSA-2883-xcg3-v3hh](https://github.com/advisories/GHSA-2883-xcg3-v3hh).
The extension's explicit npm override pinned `js-yaml` to vulnerable `4.3.1`,
preventing Dependabot from resolving the patched `4.3.2` release.

Updated the override and regenerated `vscode-extension/package-lock.json`.
Only the js-yaml package version, tarball URL, and integrity changed in the
lockfile. ESLint, its configuration loader, and Mocha through `@vscode/test-cli`
all resolve to the same patched version. This is a development dependency;
the extension runtime and both Rust dependency graphs are unchanged.

Risk class: **R3**, because this fixes an untrusted-input resource limit. The
affected boundary is YAML parsing in extension development/test tools. The
regression invokes the real installed parser with tiny inputs and explicit
budgets, avoiding timing assertions and expensive denial-of-service payloads.
It is invoked by `npm run test:security`, `pretest`, and `vscode:prepublish`.
The existing nightly packaging invokes prepublish; existing PR CI does not
run this Node suite.

## Acceptance criteria and Red-to-Green evidence

Base: `ec3ae773036c1ff416b3fa3fdd57552286ec5121`.
Test-only Red commit: `a388c54202a94e5ad327d7501a704e4c19f3f83a`.
The dependency fix follows that commit on `codex/fix-dependabot-70-js-yaml`.

| Acceptance criterion | Automated evidence |
| --- | --- |
| Ordinary YAML merges and explicit overrides remain compatible | Positive parser regression; passes on 4.3.1 and 4.3.2 |
| Exactly four empty-source merges fit a budget of four | Boundary regression; passes on both versions |
| Repeated empty sources exhaust a document budget of three | Fails on 4.3.1 with missing expected `YAMLException`; passes on 4.3.2 |
| A single empty source consumes a work unit | Zero-budget regression fails on 4.3.1; passes on 4.3.2 |
| No vulnerable js-yaml remains in the installed graph | Fresh `npm ci`, `npm ls js-yaml --all`, and npm advisory audit |
| Extension still builds and test tooling reads normal YAML | TypeScript compile and 10 existing extension-structure tests through Mocha using a YAML configuration, before and after the patch |

Validation used isolated copies under `target/test-artifacts/dependabot-70/`
to keep installed packages, compiled output, and audit reports out of the
source tree. The final staged manifest, lockfile, and test matched the source
files. The security tests passed **4/4** after the patch; **2/4** failed for the
intended reason before it. An independent review agent reproduced both results
and found no actionable issue in the diff.

Commands used during validation, from the relevant staged extension directory
(Windows, Node 24.19.0, npm 12.0.2; the initial baseline compile used the host
Node 22.23.2):

```text
npm ci --no-audit --no-fund
node --test ../tests/tooling/js_yaml_security.test.cjs
npm run compile
npm run lint
npm run vscode:prepublish
npm ls js-yaml --all
npm audit --json
node node_modules/mocha/bin/mocha.js --config ../mocha.yaml out/test/extension-structure.test.js
npm test
```

The baseline install used `--ignore-scripts`; the patched clean install used
normal lifecycle processing. The temporary Mocha YAML config contains only
`reporter: spec` and `timeout: 30000`.

Repository checks also passed: `cargo fmt --all -- --check`,
`cargo clippy --all-targets --all-features -- -D warnings`,
`cargo test --all --locked` (2,286 passed, 0 failed, 27 existing ignored),
`cargo check --locked --manifest-path fuzz/Cargo.toml`,
`python -m unittest discover -s tests/tooling -v` (89 tests), and
`python scripts/check_repo_hygiene.py --mode static`.

## Limitations and remaining findings

- `npm audit` no longer reports js-yaml or any High/Critical vulnerability.
  It still exits 1 for pre-existing development-tool findings in `ajv`
  (Moderate, GHSA-2g4f-4pwh-qvx6) and `diff` (Low, GHSA-73rr-hh4g-fpgx).
- `npm test` passes the new security suite and compilation, then fails in the
  existing `eslint src` command: `No files matching the pattern "src" were
  found.` The same lint command fails on the baseline. The VS Code host suite
  therefore has not passed; no lint configuration or test assertion was
  relaxed to hide the failure.
- Sandbox filesystem/process restrictions initially prevented compilation
  and the hygiene checker; those commands passed with the necessary execution
  permission. A temporary npm launcher was corrected before running nested
  npm scripts. These were environment failures, not parser test failures.
- The upstream patch changes YAML work accounting; excessive merge input now
  fails with a limit exception as intended. Reverting the override/lock change
  would restore the vulnerability and is not a safe security rollback.
- The GitHub alert remains open until the fixed dependency reaches the default
  branch and GitHub rescans it. No alert dismissal or release is part of this
  local change. GitHub CI, standalone WFL program/web runners, and a release
  package build were not run for this extension development-dependency patch.
