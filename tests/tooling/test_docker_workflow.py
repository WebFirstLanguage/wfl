"""Release-policy checks for the nightly consumer test image.

Container execution is checked by the credential-free Docker image workflow;
these guards cover trust, ordering, and serialization in the publish workflow.
"""

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


def job(text, name):
    match = re.search(rf"^  {re.escape(name)}:\n(.*?)(?=^  [\w-]+:|\Z)", text, re.M | re.S)
    if not match:
        raise AssertionError(f"Required workflow job {name!r} is missing")
    return match.group(1)


class DockerWorkflowPolicyTests(unittest.TestCase):
    def setUp(self):
        self.nightly = (ROOT / ".github/workflows/nightly.yml").read_text(encoding="utf-8")

    def test_publication_is_main_only_and_serialized_for_one_destination(self):
        publish = job(self.nightly, "docker-nightly")
        self.assertIn("github.ref == 'refs/heads/main'", publish)
        self.assertIn("group: dockerhub-wfl-nightly", publish)
        self.assertIn("cancel-in-progress: false", publish)
        self.assertIn("DOCKERHUB_IMAGE: bsbyrdwfl/wfl", publish)
        self.assertNotIn("pull_request_target", self.nightly)

    def test_docker_bootstrap_does_not_depend_on_github_release_marker(self):
        check = job(self.nightly, "check-for-changes")
        self.assertIn("docker_should_build:", check)
        self.assertIn("publish.py plan", check)
        linux = job(self.nightly, "build-linux")
        self.assertIn("outputs.docker_should_build == 'true'", linux)
        self.assertIn("outputs.should_build == 'true'", linux)

    def test_candidate_is_gated_and_tested_before_registry_mutations(self):
        publish = job(self.nightly, "docker-nightly")
        self.assertLess(publish.index("publish.py plan"), publish.index("useblacksmith/build-push-action"))
        self.assertLess(publish.index("smoke_test.py"), publish.index("publish.py publish"))
        self.assertIn("steps.plan.outputs.should_build == 'true'", publish)
        self.assertIn("load: true", publish)
        self.assertIn("push: false", publish)
        self.assertIn("cache-key: scripts/docker/Dockerfile", publish)
        self.assertNotIn("cache-to:", publish)
        self.assertNotIn("cache-from:", publish)

    def test_publication_waits_for_same_revision_full_ci_and_windows_build(self):
        checks = job(self.nightly, "docker-release-checks")
        self.assertIn("uses: ./.github/workflows/ci.yml", checks)
        self.assertIn("outputs.docker_should_build == 'true'", checks)
        publish = job(self.nightly, "docker-nightly")
        self.assertIn("needs: [check-for-changes, build, build-linux, docker-release-checks]", publish)
        self.assertIn("needs.build.result == 'success'", publish)
        self.assertIn("needs.docker-release-checks.result == 'success'", publish)
        ci = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        self.assertIn("workflow_call:", ci)
        bump = job(ci, "bump-version")
        self.assertIn("github.event_name == 'push'", bump)

    def test_pull_requests_exercise_real_container_without_hub_credentials(self):
        validation = (ROOT / ".github/workflows/docker-image.yml").read_text(encoding="utf-8")
        self.assertIn("pull_request:", validation)
        self.assertIn("workflow_dispatch:", validation)
        self.assertIn("smoke_test.py", validation)
        self.assertIn("x86_64-unknown-linux-musl", validation)
        self.assertNotIn("secrets.", validation)
        self.assertNotIn("publish.py publish", validation)
        self.assertIn("push: false", validation)


if __name__ == "__main__":
    unittest.main()
