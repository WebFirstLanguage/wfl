# Docker Hub tag-count repair — validation record

Date: 2026-09-12. Risk: **R3** (untrusted registry metadata, publication, and
owned-tag deletion). Base: `7be2cbe598739008c3a51c4520f8bd53318b05e0`.

## Observed provider failure

[Nightly 34701542605](https://github.com/WebFirstLanguage/wfl/actions/runs/34701542605)
at `3b98aa3bbef827d90bc691b03831a5926beb5fe2` passed the full reusable CI,
Linux and Windows release builds, and real container acceptance tests.
[Docker job 103578170273](https://github.com/WebFirstLanguage/wfl/actions/runs/34701542605/job/103578170273)
then failed with `Docker Hub returned an incomplete tag list` after uploading
`nightly-26.9.4`. The rolling tag was not promoted.

Public GET requests to the configured repository's tag endpoint returned:

```json
{
  "count": 0,
  "next": null,
  "results": [{
    "name": "nightly-26.9.4",
    "digest": "sha256:4301d2d74656420603010551606b27c121676eff55734a1da3a6d53237d6c7d7"
  }]
}
```

The same mismatch persisted for several minutes and with page sizes 10 and
100. The [Docker Hub API schema](https://docs.docker.com/reference/api/hub/latest/)
describes `count` as the total across pages, but the observed aggregate was
stale. The original test peer always generated an exact total and therefore
missed this real provider response. The failed run is retained; no required
test is retried or relabeled as passing.

## Contract and acceptance criteria

The publisher follows `next` links to enumerate tags. It permits a stable
aggregate count below the actual enumerated records, which does not itself
indicate missing records. An overcount still indicates an incomplete list and
fails. A full final page with an undercount is ambiguous and also fails.
Page and actual-record limits, duplicate rejection, fixed-origin pagination,
cross-page consistency, and direct current-tag digest checks remain enforced.
Every page explicitly supplies `next`; each following link advances exactly
one page and retains `page_size=100`. Missing, blank, duplicate, or extra query
fields cannot normalize into an accepted page link.

Before deleting any older owned tag, cleanup additionally requires the
enumerated rolling and current-version entries to match their expected digest.
An incomplete or stale view cannot silently report cleanup success. The change
does not alter version ordering, tag ownership, credentials, workflow gates,
or immutable-image recovery.

Regression tests exercise the provider's stale aggregate through the real
loopback HTTP boundary, including first publication, interrupted recovery,
replacement, and same-version cleanup. Negative cases cover missing records,
ambiguous truncation, excessive response records, and stale current entries.
The Docker command peer remains a process double; live publication is separate
provider-boundary evidence.

## Validation and recovery

Test-only Red commit `0e8809fb` ran 48 publisher tests in 43.667 seconds:
five expected errors reproduced stale-count rejection during first upload,
staged recovery, replacement, unchanged cleanup, and multipage enumeration.
Five assertion failures exposed acceptance of an oversized page and four
missing/mismatched current-tag cleanup views. Production code was unchanged.
Existing overcount, malformed, duplicate, unsafe-origin, loop, and ambiguous
full-page cases remained rejecting.

Anonymous registry verification of `nightly-26.9.4` passed after the failed
deployment: manifest and config payload hashes match their descriptors, the
config identifies Linux amd64 and version 26.9.4, and its source revision is
`3b98aa3bbef827d90bc691b03831a5926beb5fe2`. The public digest matches the one
recorded above. This checks public metadata access, not remote container
execution; the real container smoke tests passed in the linked nightly job.

Local Green: `python -m unittest discover -s tests/tooling -p 'test_docker*.py'
-v` passed all 53 tests (48 publisher, five workflow) in 45.696 seconds.
`python scripts/validate_docs_examples.py --ci --force` passed all 36 examples.
Final CI results are attached to the repair PR before merge. The complete
presubmit workflow and credential-free Docker acceptance run on the final
proposed commit. Independent R3 review covers the asymmetric count rule and
deletion protections.

Independent R3 review approved the final publisher diff with no blocking
findings and separately ran the eight new regression tests: all passed in
11.977 seconds. Staged-tree repository hygiene and `git diff --check` passed.

[PR #729 review](https://github.com/WebFirstLanguage/wfl/pull/729#discussion_r3996750220)
identified that a page jump or smaller page size could hide records despite
the new undercount handling. Test-only commit `c076b09a` reproduced seven
failures in 3.752 seconds across two test methods: skipped pages, changed page
size, missing parameters, a duplicate blank parameter, an empty extra
parameter, and missing `next`. The repair now requires sequential pages, the
fixed size, and explicit termination, preserving blank query values during
validation. Updated independent R3 review approved this delta and ran both
new methods plus valid multipage enumeration: three tests passed in 4.198
seconds. No workflow or required-test retry was added.

Final local Green after the pagination repair: the same Docker test command
passed all 55 tests in 48.926 seconds. Final remote checks must identify the
subsequent Green commit, rather than the earlier PR revision.

Recovery proceeds through the existing serialized nightly publisher after the
repair passes its checks. If the requested version already exists, the
publisher pulls and retests that immutable digest before promotion. Otherwise
it builds the new canonical version and verifies the replacement before
removing older owned tags. No manually fabricated version tag is used to test
deletion. Live rolling publication, replacement, cleanup, and unchanged-version
skip remain pending until recorded on the repair PR.

Repository-wide coverage and extended-soak gaps are unchanged from `testing.md`.
The changed protocol paths require positive and negative regression coverage;
runtime language, database, UI, and performance behavior are unaffected.
