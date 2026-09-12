# WFL Docker runtime

The nightly image runs WFL programs and their built-in `describe`, `test`, and
`expect` suites. It contains the nightly's static Linux x86-64 WFL binary,
Debian Bookworm, a shell, and HTTPS certificate authorities. The runtime image
supports `linux/amd64`; it does not contain Rust or the WFL repository's tests.

For usage and the nightly publication contract, see
[the Docker guide](../../Docs/guides/docker-testing.md).

## Build and verify locally

The Docker build consumes a small, explicitly staged context. The `wfl`
binary must already be built for `x86_64-unknown-linux-musl`; the nightly
workflow asserts that it is static and checks its version before publication.

```bash
mkdir -p target/docker-context
cp target/x86_64-unknown-linux-musl/release/wfl target/docker-context/wfl
cp LICENSE .wflcfg target/docker-context/
VERSION="$(python3 -c 'import tomllib; print(tomllib.load(open("Cargo.toml", "rb"))["package"]["version"])')"
docker build --platform linux/amd64 \
  --file scripts/docker/Dockerfile \
  --build-arg "VERSION=$VERSION" \
  --build-arg "REVISION=$(git rev-parse HEAD)" \
  --tag wfl-runtime:local target/docker-context
python3 scripts/docker/smoke_test.py wfl-runtime:local --version "$VERSION"
```

The smoke harness uses real containers with networking disabled. It checks
the exact version, default help, nonroot execution, writable home/work,
certificate bundle, mounted WFL assertions, relative includes, file results,
SQLite, failed-test exit propagation, standard input, caller UID overrides,
and the absence of unexpected output files. It requires a local Linux Docker
engine and Python 3.11+. Temporary directories and containers are removed on
success, failure, and timeout.

The Debian image is pinned by digest. Update that digest in the Dockerfile
when updating the base, then rerun the container tests. The publication gate
still requires a changed WFL version; changing only the Dockerfile does not
replace a published version.

## Publication configuration

The nightly workflow targets `bsbyrdwfl/wfl` and reads the GitHub Actions
secrets `DOCKERHUB_USERNAME` and `DOCKERHUB_TOKEN`. The Docker Hub token needs
repository read, write, and delete permissions for publication and owned-tag
cleanup. Keep the credential in GitHub's secret store; it is not an image
build argument and never enters consumer scripts or the Docker build context.

`publish.py version --repo .` validates the product-version mirrors.
`publish.py plan --repo . --github-output "$GITHUB_OUTPUT"` reports whether
an image build is needed. `publish.py publish --repo . --candidate IMAGE`
rechecks current registry state before publishing a tested local image and
cleaning up older managed tags. The workflow serializes all publishers with
one destination-specific concurrency group and does not cancel a running
publisher.

Before any new-version publication, the nightly runs the complete reusable CI
workflow at the same commit and waits for the Windows and Linux build jobs.
If an upload stopped after the version tag was written, recovery pulls that
immutable digest and repeats the container acceptance tests before promotion;
it never overwrites the retained version tag with newly rebuilt bytes.

Docker Hub can return a stale aggregate tag count immediately after a push.
The publisher follows the returned page links and validates the actual tag
records. A total below the enumerated records is accepted; a total above them,
an ambiguous full final page, changing pagination, or missing current-image
records stops publication or cleanup. Page links must advance exactly one page
and retain the requested page size. Current and versioned digests are checked
independently before promotion and each deletion. A provider failure leaves any
already uploaded version tag available for the recovery path above.

Only this workflow should manage the `nightly` and `nightly-<version>` tags.
Run `python -m unittest discover -s tests/tooling -p 'test_docker*.py' -v`
for transport, version, publication, and workflow policy tests. The
`Docker Runtime Validation` workflow exercises the real Linux image on
Blacksmith without Docker Hub secrets or publication.

Image construction uses the Docker CLI installed on the Blacksmith runner.
This works within the organization's existing GitHub Actions allowlist and
does not require Blacksmith's separate Docker-builder actions or remote layer
cache. Rust compilation still uses the existing Cargo cache.
