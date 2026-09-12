"""Publisher contracts against real local HTTP and child-process boundaries.

The server is a controllable Hub protocol peer, not the production Hub. The
Docker child is a recording adapter; the workflow separately tests real Docker
images and verifies real Hub digests on every publication.
"""

from __future__ import annotations

import contextlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlsplit


ROOT = Path(__file__).resolve().parents[2]
PUBLISHER = ROOT / "scripts" / "docker" / "publish.py"
IMAGE = "bsbyrdwfl/wfl"
USERNAME = "bsbyrdwfl"
SECRET = "test-pat-do-not-log"
BEARER = "test-bearer-do-not-log"
VERSION = "26.9.4"
OLD = "26.9.3"
DIGEST = "sha256:" + "a" * 64
OLD_DIGEST = "sha256:" + "b" * 64
OTHER_DIGEST = "sha256:" + "c" * 64
CONFIG_DIGEST = "sha256:" + "d" * 64
REPO_PATH = "/v2/namespaces/bsbyrdwfl/repositories/wfl"
TAGS_PATH = REPO_PATH + "/tags"


def tag(name, digest):
    return {
        "name": name,
        "digest": digest,
        "images": [{"os": "linux", "architecture": "amd64", "digest": digest}],
    }


class Peer:
    """Local server with persistent remote tags and injected failure points."""

    def __init__(self):
        self.tags = {}
        self.requests = []
        self.commands = []
        self.deleted = []
        self.overrides = {}
        self.repo_status = 200
        self.repo_body = {"namespace": USERNAME, "name": "wfl"}
        self.page_size = 100
        self.next_override = None
        self.fail_command = None
        self.version_output = f"WebFirst Language (WFL) version {VERSION}\n"
        self.label_version = VERSION
        self.remote_version = VERSION
        self.remote_label_version = VERSION
        self.config_digest = CONFIG_DIGEST
        self.manifest_config = CONFIG_DIGEST
        self.push_digest = DIGEST
        self.rolling_digest = DIGEST
        self.blocked_paths = set()
        self.incomplete_paths = set()
        self.bad_status_paths = set()
        self.release_responses = threading.Event()
        self.advance_after_version_push = False
        self.current_reads = 0
        self.replace_on_current_read = None
        self.server = None

    @property
    def origin(self):
        return f"http://127.0.0.1:{self.server.server_port}"

    def published(self, version=OLD, digest=OLD_DIGEST):
        self.tags = {
            "nightly": tag("nightly", digest),
            f"nightly-{version}": tag(f"nightly-{version}", digest),
        }

    def __enter__(self):
        peer = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def do_POST(self):
                self.respond()

            def do_GET(self):
                self.respond()

            def do_DELETE(self):
                self.respond()

            def respond(self):
                body = self.rfile.read(int(self.headers.get("Content-Length", 0)))
                path = urlsplit(self.path).path.rstrip("/")
                peer.requests.append((self.command, self.path, dict(self.headers), body))
                if path in peer.bad_status_paths:
                    self.wfile.write(("invalid-http-status " + SECRET + "\r\n\r\n").encode())
                    return
                if path in peer.blocked_paths:
                    peer.release_responses.wait(timeout=3)
                if path in peer.incomplete_paths:
                    self.send_response(200)
                    self.send_header("Content-Type", "application/json")
                    self.send_header("Content-Length", "100")
                    self.end_headers()
                    self.wfile.write(b"{")
                    return
                override = peer.overrides.get((self.command, path))
                if override:
                    status, response, headers = override
                    self.send_data(status, response, headers)
                    return
                if path == "/_docker":
                    self.docker(json.loads(body))
                    return
                if path == "/v2/auth/token":
                    if json.loads(body) != {"identifier": USERNAME, "secret": SECRET}:
                        self.send_data(401, {"detail": "bad credentials"})
                    else:
                        self.send_data(200, {"access_token": BEARER})
                    return
                if self.headers.get("Authorization") != f"Bearer {BEARER}":
                    self.send_data(401, {"detail": "authentication required"})
                    return
                if path == REPO_PATH:
                    self.send_data(peer.repo_status, peer.repo_body)
                    return
                if path == TAGS_PATH:
                    page = int(parse_qs(urlsplit(self.path).query).get("page", ["1"])[0])
                    ordered = sorted(peer.tags.values(), key=lambda item: item["name"])
                    offset = (page - 1) * peer.page_size
                    more = offset + peer.page_size < len(ordered)
                    next_page = (
                        f"{peer.origin}{TAGS_PATH}?page={page + 1}&page_size=100"
                        if more else None
                    )
                    if peer.next_override is not None:
                        next_page = peer.next_override
                    self.send_data(200, {
                        "count": len(ordered),
                        "next": next_page,
                        "results": ordered[offset:offset + peer.page_size],
                    })
                    return
                if path.startswith(TAGS_PATH + "/"):
                    name = path.rsplit("/", 1)[1]
                    if name == "nightly" and self.command == "GET":
                        peer.current_reads += 1
                        if peer.current_reads == peer.replace_on_current_read:
                            peer.tags["nightly"] = tag("nightly", OTHER_DIGEST)
                            peer.tags["nightly-26.9.5"] = tag("nightly-26.9.5", OTHER_DIGEST)
                    if name not in peer.tags:
                        self.send_data(404, {"detail": "tag not found"})
                    elif self.command == "DELETE":
                        del peer.tags[name]
                        peer.deleted.append(name)
                        self.send_data(204, None)
                    else:
                        self.send_data(200, peer.tags[name])
                    return
                self.send_data(404, {"detail": "unknown endpoint"})

            def docker(self, data):
                args = data["args"]
                peer.commands.append(data)
                command = " ".join(args)
                if peer.fail_command and peer.fail_command == command:
                    self.send_data(200, {"code": 1, "out": SECRET + " " + BEARER})
                    return
                if args[0] == "login":
                    result = {"code": 0, "out": "Login Succeeded"}
                elif args[:2] == ["image", "inspect"]:
                    immutable = "@sha256:" in args[2]
                    result = {"code": 0, "out": json.dumps([{
                        "Id": peer.manifest_config if immutable else peer.config_digest,
                        "Architecture": "amd64",
                        "Os": "linux",
                        "Config": {"Labels": {"org.opencontainers.image.version":
                            peer.remote_label_version if immutable else peer.label_version}},
                    }])}
                elif args[0] == "run":
                    result = {"code": 0, "out": (
                        f"WebFirst Language (WFL) version {peer.remote_version}\n"
                        if any("@sha256:" in arg for arg in args) else peer.version_output
                    )}
                elif args[:2] == ["manifest", "inspect"]:
                    result = {"code": 0, "out": json.dumps({
                        "schemaVersion": 2, "config": {"digest": peer.manifest_config},
                    })}
                elif args[0] == "tag":
                    result = {"code": 0, "out": ""}
                elif args[0] in ("pull", "smoke"):
                    result = {"code": 0, "out": "verified immutable image"}
                elif args[0] == "push":
                    name = args[1].rsplit(":", 1)[1]
                    digest = peer.rolling_digest if name == "nightly" else peer.push_digest
                    peer.tags[name] = tag(name, digest)
                    if name != "nightly" and peer.advance_after_version_push:
                        peer.tags["nightly"] = tag("nightly", OTHER_DIGEST)
                        peer.tags["nightly-26.9.5"] = tag("nightly-26.9.5", OTHER_DIGEST)
                    result = {"code": 0, "out": f"{name}: digest: {peer.push_digest} size: 1234\n"}
                else:
                    result = {"code": 1, "out": "unexpected Docker operation: " + command}
                self.send_data(200, result)

            def send_data(self, status, data, headers=None):
                payload = data if isinstance(data, bytes) else (
                    json.dumps(data).encode() if data is not None else b""
                )
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(payload)))
                for key, value in (headers or {}).items():
                    self.send_header(key, value)
                self.end_headers()
                with contextlib.suppress(BrokenPipeError, ConnectionResetError, ConnectionAbortedError):
                    self.wfile.write(payload)

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        return self

    def __exit__(self, *_args):
        self.release_responses.set()
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=3)


DOCKER_ADAPTER = r'''
import json, os, pathlib, sys, urllib.request
args = sys.argv[1:]
if args[0] == "--smoke":
    config = os.environ["DOCKER_CONFIG"]
    args = ["smoke"] + args[1:]
else:
    assert args[0] == "--config", args
    config = args[1]
    args = args[2:]
stdin = sys.stdin.read() if args[0] == "login" else ""
if args[0] == "login":
    assert args == ["login", "docker.io", "--username", "bsbyrdwfl", "--password-stdin"], args
    assert stdin.strip() == "test-pat-do-not-log"
    pathlib.Path(config, "config.json").write_text(stdin)
request = urllib.request.Request(
    os.environ["TEST_DOCKER_PEER"] + "/_docker",
    data=json.dumps({"args": args, "config": config, "stdin": stdin,
                     "token_in_environment": "DOCKERHUB_TOKEN" in os.environ}).encode(),
    headers={"Content-Type": "application/json"},
)
with urllib.request.urlopen(request, timeout=3) as response:
    result = json.load(response)
print(result["out"])
sys.exit(result["code"])
'''


class PublisherTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if PUBLISHER.is_file():
            spec = importlib.util.spec_from_file_location("wfl_docker_publish", PUBLISHER)
            cls.publisher = importlib.util.module_from_spec(spec)
            sys.modules[spec.name] = cls.publisher
            spec.loader.exec_module(cls.publisher)
        else:
            cls.publisher = None

    def setUp(self):
        self.assertIsNotNone(
            self.publisher,
            "WFL has no Docker nightly publisher: version gating and safe publication are absent",
        )
        self.temp = tempfile.TemporaryDirectory(prefix="wfl-docker-publish-test-")
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        (self.repo / "src").mkdir()
        self.write_version(VERSION)
        self.adapter = self.repo / "docker_adapter.py"
        self.adapter.write_text(DOCKER_ADAPTER, encoding="utf-8")

    def write_version(self, version):
        (self.repo / "Cargo.toml").write_text(
            '[package]\nname = "wfl"\nversion = "' + version + '"\n', encoding="utf-8",
        )
        year, month, build = [int(part) for part in version.split(".")]
        (self.repo / ".build_meta.json").write_text(
            json.dumps({"year": year, "month": month, "build": build}), encoding="utf-8",
        )
        (self.repo / "src" / "version.rs").write_text(
            f'pub const VERSION: &str = "{version}";\n', encoding="utf-8",
        )

    def hub(self, peer):
        return self.publisher.Hub(
            USERNAME, SECRET, IMAGE, origin=peer.origin, allow_loopback=True, timeout=2,
        )

    @contextlib.contextmanager
    def docker(self, peer):
        previous = {key: os.environ.get(key) for key in ["TEST_DOCKER_PEER", "DOCKERHUB_TOKEN"]}
        os.environ["TEST_DOCKER_PEER"] = peer.origin
        os.environ["DOCKERHUB_TOKEN"] = SECRET
        try:
            with self.publisher.Docker(
                USERNAME, SECRET, command=(sys.executable, str(self.adapter)), timeout=5,
            ) as docker:
                docker.smoke_command = (sys.executable, str(self.adapter), "--smoke")
                yield docker
        finally:
            for key, value in previous.items():
                if value is None:
                    os.environ.pop(key, None)
                else:
                    os.environ[key] = value

    def publish(self, peer, candidate="wfl-nightly-candidate:test"):
        with self.docker(peer) as docker:
            return self.publisher.publish(self.hub(peer), VERSION, candidate, docker)

    def assert_no_mutation(self, peer):
        self.assertFalse(peer.deleted)
        self.assertFalse([item for item in peer.commands if item["args"][0] in ("push", "tag")])

    def test_version_uses_cargo_and_requires_matching_mirrors(self):
        self.assertEqual(VERSION, self.publisher.read_version(self.repo))
        (self.repo / "src" / "version.rs").write_text(
            f'pub const VERSION: &str = "{OLD}";\n', encoding="utf-8",
        )
        with self.assertRaises(self.publisher.PublishError):
            self.publisher.read_version(self.repo)

    def test_build_metadata_drift_is_rejected(self):
        (self.repo / ".build_meta.json").write_text('{"year":26,"month":9,"build":3}')
        with self.assertRaises(self.publisher.PublishError):
            self.publisher.read_version(self.repo)

    def test_invalid_versions_are_rejected(self):
        for version in ("26.09.4", "26.13.4", "256.9.4", "26.9.4-rc", "26.9.4\nshould_build=true"):
            with self.subTest(version=version):
                with self.assertRaises(self.publisher.PublishError):
                    self.publisher.version_tuple(version)

    def test_cli_version_writes_only_actual_version(self):
        result = subprocess.run(
            [sys.executable, str(PUBLISHER), "version", "--repo", str(self.repo)],
            capture_output=True, text=True, timeout=5, check=False,
        )
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertEqual(VERSION, result.stdout.strip())

    def test_plan_output_is_github_safe_and_machine_readable(self):
        output = self.repo / "github-output"
        self.publisher.write_outputs(output, {
            "version": VERSION, "should_build": True, "reason": "version changed",
        })
        self.assertEqual(
            {"version=26.9.4", "should_build=true", "reason=version changed"},
            set(output.read_text().splitlines()),
        )
        with self.assertRaises(self.publisher.PublishError):
            self.publisher.write_outputs(output, {"reason": "bad\nshould_build=true"})

    def test_first_publish_requires_existing_expected_repository(self):
        with Peer() as peer:
            result = self.publisher.plan(self.hub(peer), VERSION)
            self.assertTrue(result["should_build"])
            self.assertEqual(VERSION, result["version"])
            self.assert_no_mutation(peer)
            peer.repo_status = 404
            with self.assertRaises(self.publisher.PublishError):
                self.publisher.plan(self.hub(peer), VERSION)

    def test_repository_identity_mismatch_fails_closed(self):
        with Peer() as peer:
            peer.repo_body = {"namespace": "someoneelse", "name": "wfl"}
            with self.assertRaises(self.publisher.PublishError):
                self.publisher.plan(self.hub(peer), VERSION)

    def test_authentication_uses_pat_body_and_bearer_header(self):
        with Peer() as peer:
            self.publisher.plan(self.hub(peer), VERSION)
            login = peer.requests[0]
            self.assertEqual(("POST", "/v2/auth/token"), login[:2])
            self.assertEqual({"identifier": USERNAME, "secret": SECRET}, json.loads(login[3]))
            for request in peer.requests[1:]:
                self.assertEqual(f"Bearer {BEARER}", request[2].get("Authorization"))
                self.assertNotIn(SECRET, request[1])

    def test_expiring_bearer_is_refreshed_before_next_authenticated_request(self):
        with Peer() as peer:
            hub = self.hub(peer)
            self.publisher.plan(hub, VERSION)
            hub.authenticated_at = time.monotonic() - 601
            hub.get_tag("nightly", missing=True)
            authentications = [item for item in peer.requests if item[1] == "/v2/auth/token"]
            self.assertEqual(2, len(authentications))

    def test_same_and_older_version_do_not_build(self):
        for current in (VERSION, "26.9.10", "26.10.1", "27.1.1"):
            with self.subTest(current=current), Peer() as peer:
                peer.published(current)
                self.assertFalse(self.publisher.plan(self.hub(peer), VERSION)["should_build"])
                self.assert_no_mutation(peer)

    def test_new_version_builds_even_if_old_and_new_have_same_sha(self):
        with Peer() as peer:
            peer.published()
            self.assertTrue(self.publisher.plan(self.hub(peer), VERSION)["should_build"])

    def test_unknown_or_ambiguous_rolling_version_fails_closed(self):
        for ambiguous in (False, True):
            with self.subTest(ambiguous=ambiguous), Peer() as peer:
                peer.tags["nightly"] = tag("nightly", OLD_DIGEST)
                if ambiguous:
                    peer.tags["nightly-26.9.1"] = tag("nightly-26.9.1", OLD_DIGEST)
                    peer.tags["nightly-26.9.2"] = tag("nightly-26.9.2", OLD_DIGEST)
                with self.assertRaises(self.publisher.PublishError):
                    self.publisher.plan(self.hub(peer), VERSION)

    def test_registry_error_statuses_never_become_first_publish(self):
        for status in (401, 403, 429, 500, 503):
            with self.subTest(status=status), Peer() as peer:
                peer.overrides[("GET", TAGS_PATH + "/nightly")] = (
                    status, {"detail": SECRET + BEARER}, {},
                )
                with self.assertRaises(self.publisher.PublishError) as caught:
                    self.publisher.plan(self.hub(peer), VERSION)
                self.assertNotIn(SECRET, str(caught.exception))
                self.assertNotIn(BEARER, str(caught.exception))
                self.assert_no_mutation(peer)

    def test_registry_timeout_does_not_become_first_publish(self):
        with Peer() as peer:
            peer.blocked_paths.add(TAGS_PATH + "/nightly")
            hub = self.publisher.Hub(
                USERNAME, SECRET, IMAGE, origin=peer.origin,
                allow_loopback=True, timeout=0.05,
            )
            with self.assertRaises(self.publisher.PublishError):
                self.publisher.plan(hub, VERSION)
            self.assert_no_mutation(peer)

    def test_incomplete_http_response_is_reported_as_safe_publication_failure(self):
        with Peer() as peer:
            peer.incomplete_paths.add(TAGS_PATH)
            with self.assertRaises(self.publisher.PublishError):
                self.publisher.plan(self.hub(peer), VERSION)
            self.assert_no_mutation(peer)

    def test_malformed_http_status_cannot_escape_with_secret_in_error(self):
        with Peer() as peer:
            peer.bad_status_paths.add(TAGS_PATH)
            with self.assertRaises(self.publisher.PublishError) as caught:
                self.publisher.plan(self.hub(peer), VERSION)
            self.assertNotIn(SECRET, str(caught.exception))
            self.assert_no_mutation(peer)

    def test_redirects_are_not_followed_even_on_same_origin(self):
        with Peer() as peer:
            peer.overrides[("POST", "/v2/auth/token")] = (
                307, b"", {"Location": peer.origin + "/credential-sink"},
            )
            with self.assertRaises(self.publisher.PublishError):
                self.publisher.plan(self.hub(peer), VERSION)
            self.assertFalse([item for item in peer.requests if "credential-sink" in item[1]])

    def test_malformed_and_oversized_responses_fail_closed(self):
        for response in (b"invalid JSON " + SECRET.encode(), b"x" * (2 * 1024 * 1024)):
            with self.subTest(size=len(response)), Peer() as peer:
                peer.overrides[("GET", TAGS_PATH)] = (200, response, {})
                with self.assertRaises(self.publisher.PublishError) as caught:
                    self.publisher.plan(self.hub(peer), VERSION)
                self.assertNotIn(SECRET, str(caught.exception))
                self.assert_no_mutation(peer)

    def test_malformed_tag_digest_fails_closed(self):
        with Peer() as peer:
            peer.published()
            peer.tags["nightly"]["digest"] = "not-a-digest"
            with self.assertRaises(self.publisher.PublishError):
                self.publisher.plan(self.hub(peer), VERSION)

    def test_pagination_cannot_send_bearer_to_another_origin_or_endpoint(self):
        for next_url in ("https://example.com/steal", "http://127.0.0.1:1/steal"):
            with self.subTest(next_url=next_url), Peer() as peer:
                peer.next_override = next_url
                with self.assertRaises(self.publisher.PublishError):
                    self.publisher.plan(self.hub(peer), VERSION)
                self.assert_no_mutation(peer)

    def test_pagination_loop_fails_instead_of_running_forever(self):
        with Peer() as peer:
            peer.next_override = peer.origin + TAGS_PATH + "?page=1&page_size=100"
            with self.assertRaises(self.publisher.PublishError):
                self.publisher.plan(self.hub(peer), VERSION)
            self.assertLess(len(peer.requests), 10)

    def test_destination_and_loopback_overrides_require_explicit_safe_scope(self):
        for image in ("evil.example/wfl", "bsbyrdwfl/wfl:nightly", "other/wfl", "bsbyrdwfl/../wfl"):
            with self.subTest(image=image):
                with self.assertRaises(self.publisher.PublishError):
                    self.publisher.Hub(USERNAME, SECRET, image)
        for origin, allow in (("http://127.0.0.1:1", False), ("https://example.com", True),
                              ("http://localhost:1", True), ("https://hub.docker.com.evil", False)):
            with self.subTest(origin=origin):
                with self.assertRaises(self.publisher.PublishError):
                    self.publisher.Hub(USERNAME, SECRET, IMAGE, origin=origin, allow_loopback=allow)

    def test_new_image_is_verified_before_old_managed_tags_are_deleted(self):
        with Peer() as peer:
            peer.published()
            peer.tags["stable"] = tag("stable", OLD_DIGEST)
            peer.tags["nightly-not-a-version"] = tag("nightly-not-a-version", OLD_DIGEST)
            peer.tags["nightly-26.10.1"] = tag("nightly-26.10.1", OTHER_DIGEST)
            result = self.publish(peer)
            self.assertEqual("published", result["action"])
            self.assertEqual(DIGEST, peer.tags["nightly"]["digest"])
            self.assertEqual(DIGEST, peer.tags[f"nightly-{VERSION}"]["digest"])
            self.assertEqual([f"nightly-{OLD}"], peer.deleted)
            self.assertIn("stable", peer.tags)
            self.assertIn("nightly-not-a-version", peer.tags)
            self.assertIn("nightly-26.10.1", peer.tags)
            pushes = [item["args"][1] for item in peer.commands if item["args"][0] == "push"]
            self.assertEqual([IMAGE + ":nightly-" + VERSION, IMAGE + ":nightly"], pushes)
            first_delete = next(i for i, item in enumerate(peer.requests) if item[0] == "DELETE")
            self.assertTrue(any(
                item[0] == "GET" and item[1].rstrip("/") == TAGS_PATH + "/nightly"
                for item in peer.requests[:first_delete]
            ))

    def test_same_version_repairs_cleanup_without_candidate_login_or_push(self):
        with Peer() as peer:
            peer.published(VERSION, DIGEST)
            peer.tags[f"nightly-{OLD}"] = tag(f"nightly-{OLD}", OLD_DIGEST)
            result = self.publish(peer, candidate=None)
            self.assertEqual("unchanged", result["action"])
            self.assertEqual([f"nightly-{OLD}"], peer.deleted)
            self.assertFalse(peer.commands)

    def test_stale_publisher_does_not_touch_newer_current_image(self):
        with Peer() as peer:
            peer.published("26.9.5", OTHER_DIGEST)
            result = self.publish(peer)
            self.assertEqual("newer-published", result["action"])
            self.assert_no_mutation(peer)
            self.assertFalse(peer.commands)

    def test_newer_publication_during_versioned_push_prevents_rolling_overwrite(self):
        with Peer() as peer:
            peer.published()
            peer.advance_after_version_push = True
            with self.assertRaises(self.publisher.PublishError):
                self.publish(peer)
            self.assertEqual(OTHER_DIGEST, peer.tags["nightly"]["digest"])
            pushes = [item["args"][1] for item in peer.commands if item["args"][0] == "push"]
            self.assertEqual([IMAGE + ":nightly-" + VERSION], pushes)
            self.assertFalse(peer.deleted)

    def test_current_image_change_during_cleanup_prevents_deletion(self):
        with Peer() as peer:
            peer.published(VERSION, DIGEST)
            peer.tags[f"nightly-{OLD}"] = tag(f"nightly-{OLD}", OLD_DIGEST)
            peer.replace_on_current_read = 3
            with self.assertRaises(self.publisher.PublishError):
                self.publish(peer, candidate=None)
            self.assertEqual(OTHER_DIGEST, peer.tags["nightly"]["digest"])
            self.assertIn(f"nightly-{OLD}", peer.tags)
            self.assertFalse(peer.deleted)

    def test_wrong_candidate_version_or_label_cannot_be_published(self):
        for mismatch in ("binary", "label"):
            with self.subTest(mismatch=mismatch), Peer() as peer:
                peer.published()
                if mismatch == "binary":
                    peer.version_output = f"WebFirst Language (WFL) version {OLD}\n"
                else:
                    peer.label_version = OLD
                with self.assertRaises(self.publisher.PublishError):
                    self.publish(peer)
                self.assert_no_mutation(peer)

    def test_failed_versioned_push_keeps_current_and_old_tags(self):
        with Peer() as peer:
            peer.published()
            peer.fail_command = "push " + IMAGE + ":nightly-" + VERSION
            with self.assertRaises(self.publisher.PublishError) as caught:
                self.publish(peer)
            self.assertEqual(OLD_DIGEST, peer.tags["nightly"]["digest"])
            self.assertIn(f"nightly-{OLD}", peer.tags)
            self.assertFalse(peer.deleted)
            self.assertNotIn(SECRET, str(caught.exception))
            self.assertNotIn(BEARER, str(caught.exception))

    def test_failed_promotion_keeps_previous_image_and_all_version_tags(self):
        with Peer() as peer:
            peer.published()
            peer.fail_command = "push " + IMAGE + ":nightly"
            with self.assertRaises(self.publisher.PublishError):
                self.publish(peer)
            self.assertEqual(OLD_DIGEST, peer.tags["nightly"]["digest"])
            self.assertIn(f"nightly-{VERSION}", peer.tags)
            self.assertIn(f"nightly-{OLD}", peer.tags)
            self.assertFalse(peer.deleted)

    def test_digest_mismatch_after_promotion_prevents_deletion(self):
        with Peer() as peer:
            peer.published()
            peer.rolling_digest = OTHER_DIGEST
            with self.assertRaises(self.publisher.PublishError):
                self.publish(peer)
            self.assertIn(f"nightly-{OLD}", peer.tags)
            self.assertFalse(peer.deleted)

    def test_existing_candidate_can_recover_promotion_without_overwriting_it(self):
        with Peer() as peer:
            peer.published()
            peer.tags[f"nightly-{VERSION}"] = tag(f"nightly-{VERSION}", DIGEST)
            self.publish(peer)
            pushes = [item["args"][1] for item in peer.commands if item["args"][0] == "push"]
            self.assertEqual([IMAGE + ":nightly"], pushes)
            self.assertEqual(DIGEST, peer.tags["nightly"]["digest"])

    def test_different_rebuild_recovers_and_smoke_tests_immutable_candidate(self):
        with Peer() as peer:
            peer.published()
            peer.tags[f"nightly-{VERSION}"] = tag(f"nightly-{VERSION}", DIGEST)
            peer.manifest_config = OTHER_DIGEST
            self.publish(peer)
            immutable = IMAGE + "@" + DIGEST
            args = [item["args"] for item in peer.commands]
            self.assertIn(["pull", immutable], args)
            self.assertIn(["smoke", immutable, "--version", VERSION], args)
            self.assertIn(["tag", immutable, IMAGE + ":nightly"], args)
            self.assertFalse([item for item in args if item == ["push", IMAGE + ":nightly-" + VERSION]])
            self.assertLess(args.index(["smoke", immutable, "--version", VERSION]),
                            args.index(["push", IMAGE + ":nightly"]))

    def test_recovery_smoke_failure_keeps_previous_and_staged_images(self):
        with Peer() as peer:
            peer.published()
            peer.tags[f"nightly-{VERSION}"] = tag(f"nightly-{VERSION}", DIGEST)
            peer.fail_command = "smoke " + IMAGE + "@" + DIGEST + " --version " + VERSION
            with self.assertRaises(self.publisher.PublishError):
                self.publish(peer, candidate=None)
            self.assertEqual(OLD_DIGEST, peer.tags["nightly"]["digest"])
            self.assertEqual(DIGEST, peer.tags[f"nightly-{VERSION}"]["digest"])
            self.assert_no_mutation(peer)

    def test_recovery_rejects_wrong_remote_binary_version(self):
        with Peer() as peer:
            peer.published()
            peer.tags[f"nightly-{VERSION}"] = tag(f"nightly-{VERSION}", DIGEST)
            peer.remote_version = OLD
            with self.assertRaises(self.publisher.PublishError):
                self.publish(peer, candidate=None)
            self.assert_no_mutation(peer)

    def test_cleanup_failure_is_reported_and_later_same_version_repairs_it(self):
        with Peer() as peer:
            peer.published()
            peer.overrides[("DELETE", TAGS_PATH + f"/nightly-{OLD}")] = (503, {}, {})
            with self.assertRaises(self.publisher.PublishError):
                self.publish(peer)
            self.assertEqual(DIGEST, peer.tags["nightly"]["digest"])
            self.assertIn(f"nightly-{OLD}", peer.tags)
            peer.overrides.clear()
            peer.commands.clear()
            self.publish(peer, candidate=None)
            self.assertNotIn(f"nightly-{OLD}", peer.tags)
            self.assertFalse(peer.commands)

    def test_cleanup_reads_all_pages_and_preserves_unrelated_tags(self):
        with Peer() as peer:
            peer.published(VERSION, DIGEST)
            for build in range(1, 4):
                peer.tags[f"nightly-26.9.{build}"] = tag(f"nightly-26.9.{build}", OLD_DIGEST)
            peer.tags["user-image"] = tag("user-image", OTHER_DIGEST)
            peer.page_size = 2
            self.publish(peer, candidate=None)
            self.assertEqual({"nightly", f"nightly-{VERSION}", "user-image"}, set(peer.tags))
            self.assertEqual(3, len(peer.deleted))

    def test_delete_allowlist_rejects_rolling_unrelated_and_malformed_tags(self):
        with Peer() as peer:
            hub = self.hub(peer)
            for name in ("nightly", "stable", "nightly-26.9.4/../../wfl", "nightly-26.13.1"):
                with self.subTest(name=name), self.assertRaises(self.publisher.PublishError):
                    hub.delete_tag(name)
            self.assertFalse(peer.deleted)

    def test_docker_credentials_are_stdin_only_and_temporary_config_is_removed(self):
        with Peer() as peer:
            peer.published()
            self.publish(peer)
            login = next(item for item in peer.commands if item["args"][0] == "login")
            self.assertEqual(SECRET, login["stdin"].strip())
            for command in peer.commands:
                self.assertNotIn(SECRET, " ".join(command["args"]))
                self.assertFalse(command["token_in_environment"])
                self.assertFalse(Path(command["config"]).exists())


if __name__ == "__main__":
    unittest.main()
