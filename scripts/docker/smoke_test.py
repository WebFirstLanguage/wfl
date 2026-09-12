#!/usr/bin/env python3
"""Exercise the published image's real CLI, mounts, user, and exit statuses.

Requires Python 3.11+ and a local Linux Docker engine. All test-created files
live in a temporary directory; each container is removed even after a timeout.
The image must already exist locally, so this never pulls an untested tag.
"""

from __future__ import annotations

import argparse
import hashlib
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import uuid


ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests" / "fixtures" / "docker-runtime"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def snapshot(directory: Path) -> dict[str, str]:
    return {
        str(path.relative_to(directory)): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in sorted(directory.rglob("*"))
        if path.is_file()
    }


def run_container(
    image: str,
    arguments: list[str],
    *,
    options: list[str] | None = None,
    stdin: str | None = None,
    expected_status: int = 0,
) -> subprocess.CompletedProcess[str]:
    name = f"wfl-smoke-{uuid.uuid4().hex}"
    command = [
        "docker", "run", "--rm", "--pull=never", "--name", name,
        "--network", "none",
    ]
    if stdin is not None:
        command.append("--interactive")
    command.extend(options or [])
    command.extend([image, *arguments])
    try:
        result = subprocess.run(
            command, input=stdin, text=True, encoding="utf-8", capture_output=True,
            timeout=60, check=False,
        )
        require(
            result.returncode == expected_status,
            f"{arguments!r}: expected exit {expected_status}, got {result.returncode}\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}",
        )
        return result
    finally:
        # --rm handles normal exits; explicitly remove a container if the
        # client timed out while the daemon was still running it.
        subprocess.run(
            ["docker", "rm", "--force", name], capture_output=True,
            timeout=15, check=False,
        )


def check_summary(result: subprocess.CompletedProcess[str], passed: int, failed: int) -> None:
    for label, expected in (("Total", passed + failed), ("Passed", passed), ("Failed", failed)):
        require(
            re.search(rf"^{label}:\s+{expected}(?:\s|$)", result.stdout, re.MULTILINE)
            is not None,
            f"Expected {label}: {expected} in test output:\n{result.stdout}",
        )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("image", help="Locally built image or pulled immutable digest")
    parser.add_argument("--version", required=True, help="Expected Cargo package version")
    args = parser.parse_args()

    version = run_container(args.image, ["--version"])
    require(
        version.stdout.strip() == f"WebFirst Language (WFL) version {args.version}",
        f"Image version does not match {args.version}: {version.stdout!r}",
    )
    help_output = run_container(args.image, [])
    require("--test" in help_output.stdout, "Default command must show CLI help")

    user = run_container(
        args.image, ["-ec", 'test -w "$HOME"; test -w /work; test -s /etc/ssl/certs/ca-certificates.crt; id -u'],
        options=["--entrypoint", "/bin/sh"],
    )
    require(user.stdout.strip() == "10001", "Default container user must be UID 10001")
    print("PASS: exact version, default help, nonroot user, writable home/work, CA bundle")

    with tempfile.TemporaryDirectory(prefix="wfl-docker-smoke-") as temp:
        temp_path = Path(temp)
        sources = temp_path / "sources"
        shutil.copytree(FIXTURES, sources)
        sources.chmod(0o755)
        for fixture in sources.rglob("*"):
            fixture.chmod(0o755 if fixture.is_dir() else 0o644)
        output = temp_path / "output"
        output.mkdir(mode=0o777)
        output.chmod(0o777)  # Permit the image's fixed UID through the host umask.
        original_sources = snapshot(sources)
        mounts = [
            "--mount", f"type=bind,src={sources},dst=/sources,readonly",
            "--mount", f"type=bind,src={output},dst=/work",
        ]

        result = run_container(args.image, ["--test", "/sources/mounted.test.wfl"], options=mounts)
        check_summary(result, passed=3, failed=0)
        result_file = output / "result.txt"
        require(result_file.read_text(encoding="utf-8") == "container-write-ok", "WFL must write the host-mounted result")
        if os.name == "posix":
            require(result_file.stat().st_uid == 10001, "Default image must write as UID 10001")
        require(sorted(path.name for path in output.iterdir()) == ["result.txt"], "Unexpected files in the result mount")
        print("PASS: mounted test suite, relative include, filesystem result, SQLite")

        failure = run_container(
            args.image, ["--test", "/sources/failing.test.wfl"],
            options=mounts, expected_status=1,
        )
        check_summary(failure, passed=0, failed=1)
        require("Expected 1 to equal 2" in failure.stdout, "Failure must report the assertion diagnostic")
        require(snapshot(sources) == original_sources, "Read-only source contents must remain unchanged")
        require(sorted(path.name for path in output.iterdir()) == ["result.txt"], "A failed test must not create reports in the result mount")
        print("PASS: failed assertion exits 1; no unexpected source or result files")

        piped = run_container(
            args.image, ["--test", "/dev/stdin"],
            stdin=(FIXTURES / "stdin.test.wfl").read_text(encoding="utf-8"),
        )
        check_summary(piped, passed=1, failed=0)
        print("PASS: scripts piped through standard input")

        # An explicit host UID is useful when a project needs host-owned output.
        if os.name == "posix":
            result_file.unlink()
            override = [*mounts, "--user", f"{os.getuid()}:{os.getgid()}"]
            mapped = run_container(args.image, ["--test", "/sources/mounted.test.wfl"], options=override)
            check_summary(mapped, passed=3, failed=0)
            require(result_file.read_text(encoding="utf-8") == "container-write-ok", "UID override must retain filesystem behavior")
            require(result_file.stat().st_uid == os.getuid(), "--user must produce host-owned output")
            print("PASS: caller UID override creates host-owned output")

    print(f"All Docker runtime smoke tests passed for {args.image}")


if __name__ == "__main__":
    main()
