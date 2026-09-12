# 2026-09-12 — A Docker runtime for projects testing WFL scripts

Other repositories need to execute their own WFL `describe`/`test` suites
without building or installing the WFL toolchain. The nightly pipeline now
packages its tested static Linux x86-64 binary as `bsbyrdwfl/wfl:nightly`.
The image uses a nonroot user, a writable working directory, and the existing
quiet test configuration. WFL remains the entrypoint, so assertion failures
propagate through `docker run` to the consuming CI job.

Publication is driven by the canonical Cargo product version independently
of GitHub's existing release marker. An unchanged version does not build or
push another image, and an older queued version cannot downgrade the rolling
tag. A single publication lock covers the destination; the publisher checks
registry state again inside that lock. The replacement is tested and its
published digest verified before older owned version tags are removed.

Risk class is R3: credentials, publication, cleanup, and concurrent scheduled
runs require failure-path and real-boundary evidence. Tooling tests exercise
version and registry policy, and a separate credential-free Blacksmith
workflow runs the actual container with mounted project fixtures. The smoke
suite covers assertions, failure exit status, relative includes, file output,
SQLite, standard input, caller UID mapping, and unexpected output files.

Usage is documented in [the Docker guide](../../../Docs/guides/docker-testing.md).
Validation and release evidence are recorded in
[the testing record](../../../Engineering/evidence/docker-nightly-testing.md).

## 2026-09-12 addendum — Docker Hub tag-count interoperability

The first live nightly passed its release gates and container tests, then
stopped before promoting `nightly`. Docker Hub had accepted `nightly-26.9.4`
but returned a tag-list total of zero with one actual result. Repeated public
reads with page sizes 10 and 100 confirmed the mismatch. The original exact
count assertion treated a stale aggregate as a truncated list.

The repair accepts an underreported total while retaining strict page and
record bounds, digest checks, and rejection of genuinely incomplete listings.
Cleanup also requires the enumerated current tags to match their verified
digests. The failed deployment remains evidence, and recovery uses the already
tested immutable version image. Regression and deployment evidence are in
[the repair record](../../../Engineering/evidence/docker-hub-tag-count-repair.md).
