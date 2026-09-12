# Run WFL tests in Docker

`bsbyrdwfl/wfl:nightly` is a reusable WFL runtime for other projects. Mount
that project's source and pass its test script to `--test`; WFL's built-in
`describe`, `test`, and `expect` blocks run inside the container.

The image supports Linux x86-64 (`linux/amd64`). It contains WFL, Debian
Bookworm, a shell, HTTPS certificate authorities, and WFL's bundled SQLite
support. Docker Desktop must use Linux containers. Rust and the WFL source
repository are not required on the consuming machine.

## Run a project's tests

From a project containing `tests/example.test.wfl`, on Linux:

```bash
docker run --rm --pull=always \
  --user "$(id -u):$(id -g)" \
  --mount "type=bind,src=$PWD,dst=/work" \
  bsbyrdwfl/wfl:nightly --test tests/example.test.wfl
```

On PowerShell with Docker Desktop using Linux containers:

```powershell
docker run --rm --pull=always `
  --mount "type=bind,src=$($PWD.Path),dst=/work" `
  bsbyrdwfl/wfl:nightly --test tests/example.test.wfl
if ($LASTEXITCODE -ne 0) { throw "WFL tests failed" }
```

The image uses `/work` as its working directory and defaults to UID/GID
`10001:10001`. Linux's `--user` option makes generated results belong to the
caller. Pass WFL options before the script path; remaining arguments are script
arguments. Failed assertions return exit status `1` to Docker and CI.

Run an ordinary script by omitting `--test`, or inspect the installed version:

```bash
docker run --rm bsbyrdwfl/wfl:nightly --version
```

For a source tree that should remain read-only, mount it at `/sources` with
`,readonly`, then mount a separate writable results directory at `/work`.
Module includes resolve relative to the source file; relative output paths
resolve from `/work`. The closest project `.wflcfg` overrides the image's global
defaults. The image disables execution logs and debug reports by default.

WFL accepts a file path for source input. A Linux container can read a piped
script through `/dev/stdin`:

```bash
docker run --rm -i bsbyrdwfl/wfl:nightly --test /dev/stdin < tests/example.test.wfl
```

## Use in another GitHub repository

This example checks out the project, pulls the current nightly, and runs its
test script. Docker returns WFL's failing status, so failed assertions fail
the step.

```yaml
name: WFL tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run WFL tests
        run: |
          docker run --rm --pull=always \
            --user "$(id -u):$(id -g)" \
            --mount "type=bind,src=$GITHUB_WORKSPACE,dst=/work" \
            bsbyrdwfl/wfl:nightly --test tests/example.test.wfl
```

## Versions and nightly updates

The nightly workflow compares the version in WFL's root `Cargo.toml` with
the image currently published on Docker Hub. A newer WFL version produces a
new image; an unchanged version does not build or push an image. An older
queued nightly cannot replace a newer published version. Image-only changes
therefore need a WFL version change before they are published.

The workflow publishes two tags in `bsbyrdwfl/wfl`:

- `nightly`: the current tested WFL runtime.
- `nightly-<version>`: the same image identified by its WFL version.

The new image must run the container tests before it is pushed. The workflow
then verifies the published digest before deleting older `nightly-<version>`
tags. Other tags are left alone. Only the current nightly version is retained;
projects that need a permanent historical image should mirror it in their
own registry. Docker Hub controls when unreferenced image layers are reclaimed.

A failed build, failed container test, or failed publication check prevents
old-tag cleanup. If a run stops during cleanup, the next run can finish that
cleanup without rebuilding an unchanged WFL version. Registry authentication,
network, and response errors fail the job rather than being treated as a
missing image.

For image maintenance, local building, and verification, see
[the packaging README](../../scripts/docker/README.md). For WFL assertions,
see [the testing guide](testing-guide.md).
