# 2026-09-16 — Same-version nightly MSI republish skips instead of failing

Scheduled Nightly Build stayed red after 26.9.6 published. The Linux tarball
is bit-identical across rebuilds of the same commit; the Windows MSI is not.
`publish_spaces.sh` treated a different-byte collision on an immutable
versioned key as a hard error, so
[run 570](https://github.com/WebFirstLanguage/wfl/actions/runs/35058982253)
failed at `Publish artifacts to DigitalOcean Spaces` with:

```text
refusing to overwrite releases/wfl-26.9.6.msi: it is already published with
different bytes (published b48097e0…, built f2d8e1cf…).
```

The Docker job on the same run took the version-unchanged path and succeeded.

Root cause, verified from that run's logs (not guessed):

- `check-for-changes` still diffs `HEAD` (`3bf6521f`) against the latest
  GitHub nightly tag `nightly-2026-09-12` (`3b98aa3b`), so `should_build`
  stays true and Windows rebuilds the MSI.
- Spaces already holds `releases/wfl-26.9.6.msi`. The rebuild's bytes differ,
  so the immutable-key guard aborted the whole publish.
- The Linux object `releases/wfl-26.9.6-linux-x86_64-3bf6521f.tar.gz` matched
  and was correctly left unwritten.

The guard now skips a colliding versioned artifact, leaves it and its
`.sha256` sidecar untouched, and does not move that artifact's rolling
`latest` pointer onto the rejected rebuild. Identical bytes stay a no-op
success. A later new version is still a new key and still publishes. An empty
artifact directory still fails closed.

Risk class **R2** (release tooling / published-artifact immutability). The
change is conservative: it never overwrites a versioned key. Residual: the
first scheduled nightly after this lands will still rebuild Windows because
`should_build` remains true against `nightly-2026-09-12`; Spaces will skip
the MSI and the release job can then write a new date tag so later unchanged
nights skip the binary jobs.

Verified with `./scripts/test_publish_spaces.sh` (88 passed, 0 failed).

## Docker runtime investigation (same request)

Pulled the published image by digest and ran the real container smoke
harness against it (not only CI publish metadata):

```text
bsbyrdwfl/wfl@sha256:a078e0a1eee7b967ff9ee99ec819b96d13c169aa860711b2636b27f6a2bba8dc
```

Anonymous Hub reads and local `docker image inspect` agree: tags
`nightly` and `nightly-26.9.6` share that digest; config
`sha256:f6ee3b65fd0bac0de7e059ed94831372e25209e8482eaf8365af330e44f04008`;
linux/amd64; USER `10001:10001`; ENTRYPOINT `wfl`; CMD `--help`;
WORKDIR `/work`; version label `26.9.6`; revision `3bf6521f`.

`python3 scripts/docker/smoke_test.py <digest> --version 26.9.6` passed:
exact version, default help, nonroot + writable home/work + CA bundle,
mounted tests (relative include, file I/O, SQLite), failed-assertion
exit 1, stdin, and `--user` host-owned output. A docs-style
`--user $(id -u):$(id -g)` bind-mount of a project test file also
passed. No runtime defect found; no image change in this PR.

Residual (unchanged from the #728/#729 rollout): linux/amd64 only; no
image vulnerability scan; Dockerfile-only fixes still need a WFL
version bump before they publish; Hub controls layer GC.
