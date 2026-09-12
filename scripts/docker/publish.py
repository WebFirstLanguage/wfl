#!/usr/bin/env python3
"""Version-gated Docker Hub publication for the WFL nightly runtime.

Run publication under the workflow's destination-wide concurrency lock. Hub has
no compare-and-swap API for tags; every automated writer must use that lock.
Only this publisher's ``nightly-YY.MM.BUILD`` tags are eligible for deletion.

API reference: https://docs.docker.com/reference/api/hub/latest/
Tag deletion: https://docs.docker.com/security/access-tokens/organization-access-tokens/
"""

from __future__ import annotations

import argparse
from http.client import HTTPException
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time
import tomllib
import uuid
from urllib.error import HTTPError, URLError
from urllib.parse import parse_qs, urlsplit
from urllib.request import HTTPRedirectHandler, ProxyHandler, Request, build_opener


HUB_ORIGIN = "https://hub.docker.com"
EXPECTED_IMAGE = "bsbyrdwfl/wfl"
MAX_RESPONSE = 1024 * 1024
VERSION_RE = re.compile(r"(?:0|[1-9][0-9]*)\.(?:[1-9]|1[0-2])\.(?:0|[1-9][0-9]*)\Z")
DIGEST_RE = re.compile(r"sha256:[0-9a-f]{64}\Z")
MANAGED_PREFIX = "nightly-"


class PublishError(RuntimeError):
    """Safe, operator-facing error with no upstream body or credentials."""


def version_tuple(version):
    if not isinstance(version, str) or not VERSION_RE.fullmatch(version):
        raise PublishError("Invalid WFL version; expected YY.MM.BUILD")
    parts = tuple(int(part) for part in version.split("."))
    if parts[0] >= 256 or parts[2] > 65535:
        raise PublishError("WFL version exceeds supported package version limits")
    return parts


def read_version(repo):
    """Use the canonical package version, requiring the release mirrors to agree."""
    repo = Path(repo)
    try:
        with (repo / "Cargo.toml").open("rb") as source:
            package = tomllib.load(source)["package"]
        version = package["version"]
        version_tuple(version)
        metadata = json.loads((repo / ".build_meta.json").read_text(encoding="utf-8"))
        fields = [metadata[key] for key in ("year", "month", "build")]
        if any(type(value) is not int for value in fields):
            raise PublishError("Build metadata must contain integer version fields")
        mirror = ".".join(str(value) for value in fields)
        source = (repo / "src" / "version.rs").read_text(encoding="utf-8")
        constants = re.findall(r'pub const VERSION: &str = "([^"]+)";', source)
        if package.get("name") != "wfl" or mirror != version or constants != [version]:
            raise PublishError("Canonical WFL version and release mirrors disagree")
        return version
    except (OSError, ValueError, KeyError, TypeError) as error:
        raise PublishError("Cannot read valid canonical WFL version and mirrors") from None


def write_outputs(path, values):
    lines = []
    for key, value in values.items():
        if isinstance(value, bool):
            value = str(value).lower()
        if not re.fullmatch(r"[a-z_]+", key) or not isinstance(value, str) or any(
            char in value for char in "\r\n\x00"
        ):
            raise PublishError("Unsafe GitHub output value")
        lines.append(f"{key}={value}\n")
    with Path(path).open("a", encoding="utf-8", newline="\n") as output:
        output.writelines(lines)


def checked_digest(value):
    if not isinstance(value, str) or not DIGEST_RE.fullmatch(value):
        raise PublishError("Registry returned an invalid image digest")
    return value


def tag_digest(item):
    if not isinstance(item, dict):
        raise PublishError("Registry returned invalid tag metadata")
    if "digest" in item:
        return checked_digest(item["digest"])
    images = item.get("images")
    if not isinstance(images, list) or len(images) != 1:
        raise PublishError("Registry did not return one unambiguous image digest")
    image = images[0]
    if not isinstance(image, dict) or image.get("os") != "linux" or image.get("architecture") != "amd64":
        raise PublishError("Registry image is not the supported Linux amd64 runtime")
    return checked_digest(image.get("digest"))


def managed_version(tag):
    if not isinstance(tag, str) or not tag.startswith(MANAGED_PREFIX):
        return None
    value = tag[len(MANAGED_PREFIX):]
    try:
        version_tuple(value)
    except PublishError:
        return None
    return value


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, *_args, **_kwargs):
        return None


class Hub:
    def __init__(self, username, token, image=EXPECTED_IMAGE, *, origin=HUB_ORIGIN,
                 allow_loopback=False, timeout=30):
        # Changing the publication destination requires a reviewed code change;
        # a secret, environment variable, or API response cannot redirect it.
        if image != EXPECTED_IMAGE or username != EXPECTED_IMAGE.split("/")[0]:
            raise PublishError("Docker Hub destination or username is outside the configured scope")
        if not isinstance(token, str) or not token or any(char.isspace() for char in token):
            raise PublishError("A Docker Hub personal access token is required")
        parsed = urlsplit(origin)
        safe_loopback = (
            allow_loopback and parsed.scheme == "http" and parsed.hostname == "127.0.0.1"
            and parsed.port is not None and parsed.path == "" and parsed.query == ""
            and parsed.fragment == "" and parsed.username is None and parsed.password is None
        )
        if origin != HUB_ORIGIN and not safe_loopback:
            raise PublishError("Docker Hub API origin is outside the configured scope")
        self.origin = origin
        self.image = image
        self.username = username
        self.token = token
        self.timeout = timeout
        self.bearer = None
        self.authenticated_at = 0.0
        self.repo_path = f"/v2/namespaces/{username}/repositories/wfl"
        self.tags_path = self.repo_path + "/tags"
        # No ambient proxy can receive credentials; TLS verification is default.
        self.opener = build_opener(ProxyHandler({}), NoRedirect())

    def request(self, method, path, *, body=None, missing=False):
        if not path.startswith("/") or path.startswith("//"):
            raise PublishError("Invalid Docker Hub API path")
        if path != "/v2/auth/token":
            self.authenticate()
        headers = {"Accept": "application/json", "User-Agent": "wfl-nightly-publisher"}
        data = None
        if body is not None:
            data = json.dumps(body).encode("utf-8")
            headers["Content-Type"] = "application/json"
        if self.bearer and path != "/v2/auth/token":
            headers["Authorization"] = "Bearer " + self.bearer
        request = Request(self.origin + path, data=data, headers=headers, method=method)
        try:
            with self.opener.open(request, timeout=self.timeout) as response:
                status = response.status
                raw = response.read(MAX_RESPONSE + 1)
        except HTTPError as error:
            status = error.code
            error.close()
            if status == 404 and missing:
                return None
            raise PublishError(f"Docker Hub {method} request failed with HTTP {status}") from None
        except (URLError, OSError, ValueError, HTTPException):
            raise PublishError("Docker Hub connection failed or timed out") from None
        if status not in (200, 204) or (method == "DELETE" and status != 204):
            raise PublishError(f"Unexpected Docker Hub response status {status}")
        if len(raw) > MAX_RESPONSE:
            raise PublishError("Docker Hub response exceeded the size limit")
        if status == 204:
            return None
        try:
            result = json.loads(raw)
        except (UnicodeDecodeError, ValueError):
            raise PublishError("Docker Hub returned malformed JSON") from None
        if not isinstance(result, dict):
            raise PublishError("Docker Hub returned an unexpected JSON structure")
        return result

    def authenticate(self):
        # Hub's access token lasts ten minutes. Refresh proactively with two
        # minutes of headroom after long Docker operations; never retry a 401.
        if self.bearer is None or time.monotonic() - self.authenticated_at >= 480:
            result = self.request("POST", "/v2/auth/token", body={
                "identifier": self.username, "secret": self.token,
            })
            bearer = result.get("access_token")
            if not isinstance(bearer, str) or not bearer or any(char.isspace() for char in bearer):
                raise PublishError("Docker Hub returned no valid access token")
            self.bearer = bearer
            self.authenticated_at = time.monotonic()

    def repository(self):
        self.authenticate()
        result = self.request("GET", self.repo_path)
        if result.get("namespace") != self.username or result.get("name") != "wfl":
            raise PublishError("Docker Hub repository identity does not match the expected destination")

    def get_tag(self, name, *, missing=False):
        if name != "nightly" and managed_version(name) is None:
            raise PublishError("Tag read is outside the managed namespace")
        result = self.request("GET", self.tags_path + "/" + name, missing=missing)
        if result is not None:
            if result.get("name") != name:
                raise PublishError("Docker Hub returned the wrong tag")
            tag_digest(result)
        return result

    def list_tags(self):
        path = self.tags_path + "?page=1&page_size=100"
        seen = set()
        tags = {}
        expected_count = None
        while path:
            if path in seen or len(seen) >= 100:
                raise PublishError("Docker Hub pagination loop or page limit exceeded")
            seen.add(path)
            page = self.request("GET", path)
            results = page.get("results")
            count = page.get("count")
            if not isinstance(results, list) or type(count) is not int or not 0 <= count <= 10000:
                raise PublishError("Docker Hub returned malformed tag pagination")
            if expected_count is not None and count != expected_count:
                raise PublishError("Docker Hub tag listing changed during pagination; run again")
            expected_count = count
            for item in results:
                if not isinstance(item, dict) or not isinstance(item.get("name"), str):
                    raise PublishError("Docker Hub returned malformed tag metadata")
                name = item["name"]
                if name in tags:
                    raise PublishError("Docker Hub returned duplicate tags during pagination")
                # Unrelated tag metadata has no bearing on this publisher.
                if name == "nightly" or managed_version(name) is not None:
                    tag_digest(item)
                tags[name] = item
            following = page.get("next")
            if following is None:
                path = None
                continue
            if not isinstance(following, str):
                raise PublishError("Docker Hub returned an invalid pagination URL")
            parsed = urlsplit(following)
            origin = urlsplit(self.origin)
            query = parse_qs(parsed.query)
            if (
                parsed.scheme != origin.scheme or parsed.netloc != origin.netloc
                or parsed.path.rstrip("/") != self.tags_path or parsed.fragment
                or not query or set(query) - {"page", "page_size"}
                or any(len(values) != 1 or not values[0].isdigit() for values in query.values())
            ):
                raise PublishError("Docker Hub pagination URL is outside the expected tag endpoint")
            path = self.tags_path + "?" + parsed.query
        if expected_count != len(tags):
            raise PublishError("Docker Hub returned an incomplete tag list")
        return tags

    def delete_tag(self, name):
        if managed_version(name) is None:
            raise PublishError("Refusing to delete a tag outside the owned version namespace")
        self.request("DELETE", self.tags_path + "/" + name)
        if self.get_tag(name, missing=True) is not None:
            raise PublishError("Docker Hub still reports the deleted tag; cleanup is incomplete")


def plan(hub, version):
    wanted = version_tuple(version)
    hub.repository()
    rolling = hub.get_tag("nightly", missing=True)
    tags = hub.list_tags()
    current = None
    digest = None
    if rolling is None:
        if "nightly" in tags:
            raise PublishError("Docker Hub current tag changed during planning; run again")
        reason = "first publish"
        should_build = True
    else:
        digest = tag_digest(rolling)
        listed = tags.get("nightly")
        if listed is None or tag_digest(listed) != digest:
            raise PublishError("Docker Hub current tag changed during planning; run again")
        matches = [managed_version(name) for name, item in tags.items()
                   if managed_version(name) is not None and tag_digest(item) == digest]
        if len(matches) != 1:
            raise PublishError("Cannot identify exactly one version for the current nightly image")
        current = matches[0]
        should_build = wanted > version_tuple(current)
        reason = "version changed" if should_build else (
            "version unchanged" if version == current else "published version newer"
        )
    return {"version": version, "should_build": should_build, "reason": reason,
            "current_version": current, "digest": digest}


class Docker:
    def __init__(self, username, token, *, command=("docker",), timeout=600):
        self.username = username
        self.token = token
        self.command = list(command)
        self.timeout = timeout
        self.directory = None
        self.smoke_command = (sys.executable, str(Path(__file__).with_name("smoke_test.py")))

    def __enter__(self):
        self.directory = tempfile.TemporaryDirectory(prefix="wfl-docker-auth-")
        return self

    def __exit__(self, *_args):
        self.directory.cleanup()

    def execute(self, command, operation, *, stdin=None, timeout=None):
        environment = os.environ.copy()
        for key in ("DOCKERHUB_TOKEN", "DOCKERHUB_USERNAME", "GITHUB_TOKEN", "GH_TOKEN", "VAULT_TOKEN"):
            environment.pop(key, None)
        environment["DOCKER_CONFIG"] = self.directory.name
        try:
            result = subprocess.run(
                command,
                input=stdin, capture_output=True, text=True, encoding="utf-8",
                env=environment, timeout=self.timeout if timeout is None else timeout, check=False,
            )
        except (OSError, subprocess.TimeoutExpired, UnicodeError):
            raise PublishError("Docker command failed to start, timed out, or returned invalid output") from None
        if result.returncode != 0:
            # Docker errors may include upstream bodies, credentials or registry
            # URLs. Preserve the operation and status, never those raw outputs.
            raise PublishError(f"Docker {operation} failed with exit status {result.returncode}")
        return result.stdout

    def run(self, args, *, stdin=None, timeout=None):
        return self.execute(
            self.command + ["--config", self.directory.name] + list(args),
            args[0], stdin=stdin, timeout=timeout,
        )

    def login(self):
        self.run(["login", "docker.io", "--username", self.username, "--password-stdin"],
                 stdin=self.token + "\n")

    def inspect_candidate(self, candidate, version):
        if not isinstance(candidate, str) or not re.fullmatch(r"wfl-nightly-candidate:[a-zA-Z0-9_.-]+", candidate):
            raise PublishError("Expected a local wfl-nightly-candidate image reference")
        return self.inspect_image(candidate, version)

    def inspect_image(self, reference, version):
        try:
            items = json.loads(self.run(["image", "inspect", reference]))
            if not isinstance(items, list) or len(items) != 1:
                raise ValueError()
            item = items[0]
            if (item["Architecture"] != "amd64" or item["Os"] != "linux"
                    or item["Config"]["Labels"]["org.opencontainers.image.version"] != version):
                raise PublishError("Candidate architecture or version label does not match WFL")
            config = checked_digest(item["Id"])
        except (ValueError, KeyError, TypeError):
            raise PublishError("Docker returned invalid candidate image metadata") from None
        container = "wfl-version-" + uuid.uuid4().hex
        try:
            actual = self.run([
                "run", "--rm", "--pull=never", "--network", "none", "--name", container,
                "--entrypoint", "wfl", reference, "--version",
            ]).strip()
        finally:
            try:
                # --rm handles normal completion; client timeout does not stop
                # the daemon's container. A missing container is harmless.
                self.run(["rm", "--force", container], timeout=15)
            except PublishError:
                pass
        if actual != f"WebFirst Language (WFL) version {version}":
            raise PublishError("Candidate binary version does not match canonical WFL version")
        return config

    def pull_and_test(self, reference, version):
        self.run(["pull", reference])
        self.inspect_image(reference, version)
        # Test exactly the retained registry bytes. Rebuilding can change image
        # config (apt metadata, build times, source revision) without changing
        # WFL version, and must neither strand recovery nor overwrite that tag.
        self.execute(list(self.smoke_command) + [reference, "--version", version], "recovery smoke test")

    def tag(self, candidate, reference):
        self.run(["tag", candidate, reference])

    def push(self, reference):
        output = self.run(["push", reference])
        digests = re.findall(r"\bdigest: (sha256:[0-9a-f]{64})\b", output)
        if len(set(digests)) != 1:
            raise PublishError("Docker push did not report one unambiguous manifest digest")
        return checked_digest(digests[0])


def require_tag(hub, name, digest):
    item = hub.get_tag(name)
    if tag_digest(item) != digest:
        raise PublishError("Published tag digest does not match the verified candidate")


def cleanup(hub, version, digest):
    current_tag = MANAGED_PREFIX + version
    require_tag(hub, "nightly", digest)
    require_tag(hub, current_tag, digest)
    tags = hub.list_tags()
    obsolete = [name for name in tags if managed_version(name) is not None
                and version_tuple(managed_version(name)) < version_tuple(version)]
    deleted = []
    for name in sorted(obsolete, key=lambda value: version_tuple(managed_version(value))):
        # Detect out-of-band writers before each destructive operation. The
        # workflow lock is still required because Hub does not offer tag CAS.
        require_tag(hub, "nightly", digest)
        require_tag(hub, current_tag, digest)
        item = hub.get_tag(name, missing=True)
        if item is None:
            continue
        if tag_digest(item) != tag_digest(tags[name]):
            raise PublishError("Old tag changed during cleanup; refusing deletion")
        hub.delete_tag(name)
        deleted.append(name)
    return deleted


def publish(hub, version, candidate, docker):
    initial = plan(hub, version)
    if not initial["should_build"]:
        if initial["current_version"] != version:
            return {"action": "newer-published", "version": version, "deleted": []}
        removed = cleanup(hub, version, initial["digest"])
        return {"action": "unchanged", "version": version, "deleted": removed}
    name = MANAGED_PREFIX + version
    reference = hub.image + ":" + name
    existing = hub.get_tag(name, missing=True)
    if existing is not None:
        digest = tag_digest(existing)
        source = hub.image + "@" + digest
        docker.login()
        docker.pull_and_test(source, version)
    else:
        if candidate is None:
            raise PublishError("A tested local candidate is required for a new WFL version")
        docker.inspect_candidate(candidate, version)
        source = candidate
        docker.login()
        docker.tag(source, reference)
        digest = docker.push(reference)
    require_tag(hub, name, digest)
    # A prior workflow-level plan may be stale. Re-read before the rolling tag
    # mutation, even though this job already owns the destination-wide lock.
    latest = plan(hub, version)
    if latest["current_version"] != initial["current_version"] or latest["digest"] != initial["digest"]:
        raise PublishError("Current nightly changed during publication; refusing promotion")
    docker.tag(source, hub.image + ":nightly")
    promoted = docker.push(hub.image + ":nightly")
    if promoted != digest:
        raise PublishError("Rolling push digest differs from the versioned candidate")
    require_tag(hub, "nightly", digest)
    require_tag(hub, name, digest)
    removed = cleanup(hub, version, digest)
    return {"action": "published", "version": version, "digest": digest, "deleted": removed}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    for command in ("version", "plan", "publish"):
        child = commands.add_parser(command)
        child.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[2])
        if command == "plan":
            child.add_argument("--github-output", type=Path)
        if command == "publish":
            child.add_argument("--candidate")
    args = parser.parse_args(argv)
    try:
        version = read_version(args.repo)
        if args.command == "version":
            print(version)
            return 0
        username = os.environ.get("DOCKERHUB_USERNAME", "")
        token = os.environ.get("DOCKERHUB_TOKEN", "")
        hub = Hub(username, token, os.environ.get("DOCKERHUB_IMAGE", EXPECTED_IMAGE))
        if args.command == "plan":
            result = plan(hub, version)
            if args.github_output:
                write_outputs(args.github_output, {key: result[key] for key in ("version", "should_build", "reason")})
        else:
            with Docker(username, token) as docker:
                result = publish(hub, version, args.candidate, docker)
        print(json.dumps(result, sort_keys=True))
        return 0
    except (PublishError, OSError) as error:
        message = str(error) if isinstance(error, PublishError) else "Cannot read or write publisher files"
        print("Docker publication failed: " + message, file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
