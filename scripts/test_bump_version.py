#!/usr/bin/env python3
"""Tests for tag-based YY.M.BUILD versioning in scripts/bump_version.py.

The next published version is derived from existing `vYY.M.N` tags for the
current UTC calendar month, with the committed .build_meta.json as a same-month
floor. These tests inject tag lists and UTC timestamps so they do not depend on
the live repository's tags or the wall clock.
"""
import datetime
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, os.path.abspath(os.path.dirname(__file__)))

import bump_version

UTC = datetime.timezone.utc


def utc_date(year, month, day, hour=12, minute=0, second=0):
    return datetime.datetime(year, month, day, hour, minute, second, tzinfo=UTC)


class TestCalendarUtc(unittest.TestCase):
    def test_just_after_midnight_utc_on_the_first_uses_the_new_month(self):
        yy, month, stamp = bump_version.calendar_parts(
            utc_date(2026, 11, 1, hour=0, minute=0, second=1)
        )
        self.assertEqual((yy, month), (26, 11))
        self.assertEqual(stamp, "2026-11-01")

    def test_last_instant_of_october_utc_stays_october(self):
        yy, month, stamp = bump_version.calendar_parts(
            utc_date(2026, 10, 31, hour=23, minute=59, second=59)
        )
        self.assertEqual((yy, month), (26, 10))
        self.assertEqual(stamp, "2026-10-31")

    def test_version_calendar_matches_nightly_yyyy_mm_dd_tag_date(self):
        now = utc_date(2026, 10, 9, hour=5)
        yy, month, stamp = bump_version.calendar_parts(now)
        self.assertEqual(f"{yy}.{month}", "26.10")
        self.assertEqual(stamp, "2026-10-09")
        self.assertEqual(bump_version.nightly_tag_date(now), stamp)

    def test_naive_datetime_is_treated_as_utc(self):
        naive = datetime.datetime(2026, 11, 1, 0, 0, 1)
        yy, month, stamp = bump_version.calendar_parts(naive)
        self.assertEqual((yy, month, stamp), (26, 11, "2026-11-01"))


class TestNextVersionFromTags(unittest.TestCase):
    def next_version(self, now, tags, floor=None, head_tags=None):
        return bump_version.next_version_from_tags(
            now=now,
            tag_names=tags,
            floor=floor,
            head_version_tags=head_tags,
        )

    def test_no_tags_for_current_month_starts_at_one(self):
        version = self.next_version(
            utc_date(2026, 10, 9),
            tags=["v26.9.19", "v26.8.40"],
            floor={"year": 26, "month": 9, "build": 19},
        )
        self.assertEqual(version, "26.10.1")

    def test_november_rolls_over_from_october_tags(self):
        tags = [f"v26.10.{n}" for n in range(1, 8)]
        version = self.next_version(utc_date(2026, 11, 15), tags=tags)
        self.assertEqual(version, "26.11.1")

    def test_january_rolls_over_from_december_tags(self):
        tags = [f"v26.12.{n}" for n in (1, 4, 9)]
        version = self.next_version(utc_date(2027, 1, 1, hour=0, minute=0, second=1), tags=tags)
        self.assertEqual(version, "27.1.1")

    def test_only_current_year_month_tags_are_counted(self):
        version = self.next_version(
            utc_date(2026, 10, 9),
            tags=["v26.9.19", "v26.10.3", "v26.11.8", "v25.10.40"],
        )
        self.assertEqual(version, "26.10.4")

    def test_older_month_floor_does_not_inflate_new_month(self):
        version = self.next_version(
            utc_date(2026, 10, 9),
            tags=[],
            floor={"year": 26, "month": 9, "build": 19},
        )
        self.assertEqual(version, "26.10.1")
        self.assertNotEqual(version, "26.10.20")

    def test_same_month_floor_is_respected(self):
        version = self.next_version(
            utc_date(2026, 10, 9),
            tags=["v26.10.1"],
            floor={"year": 26, "month": 10, "build": 5},
        )
        self.assertEqual(version, "26.10.5")

    def test_same_month_tags_above_floor_increment_past_the_max(self):
        version = self.next_version(
            utc_date(2026, 10, 9),
            tags=["v26.10.5"],
            floor={"year": 26, "month": 10, "build": 3},
        )
        self.assertEqual(version, "26.10.6")

    def test_single_and_double_digit_months_sort_numerically(self):
        version = self.next_version(
            utc_date(2026, 10, 9),
            tags=["v26.9.19", "v26.10.2"],
        )
        self.assertEqual(version, "26.10.3")

    def test_single_and_double_digit_build_numbers_sort_numerically(self):
        version = self.next_version(
            utc_date(2026, 10, 9),
            tags=["v26.10.9", "v26.10.10"],
        )
        self.assertEqual(version, "26.10.11")

    def test_just_after_midnight_utc_on_the_first_uses_new_month_version(self):
        version = self.next_version(
            utc_date(2026, 11, 1, hour=0, minute=0, second=1),
            tags=[f"v26.10.{n}" for n in range(1, 8)],
            floor={"year": 26, "month": 10, "build": 7},
        )
        self.assertEqual(version, "26.11.1")

    def test_reuse_existing_v_tag_on_the_same_commit(self):
        version = self.next_version(
            utc_date(2026, 10, 9),
            tags=["v26.10.1", "v26.10.2", "v26.10.3"],
            head_tags=["v26.10.2"],
        )
        self.assertEqual(version, "26.10.2")

    def test_ignores_non_version_tags(self):
        version = self.next_version(
            utc_date(2026, 10, 9),
            tags=["nightly-2026-10-09", "v26.10.1-beta", "release-26.10.9", "v26.10.4"],
        )
        self.assertEqual(version, "26.10.5")


class TestLsRemoteParsing(unittest.TestCase):
    SAMPLE = """\
aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\trefs/tags/nightly-2026-10-09
bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\trefs/tags/v26.9.19
cccccccccccccccccccccccccccccccccccccccc\trefs/tags/v26.9.19^{}
dddddddddddddddddddddddddddddddddddddddd\trefs/tags/v26.10.9
eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee\trefs/tags/v26.10.10
ffffffffffffffffffffffffffffffffffffffff\trefs/tags/v26.10.10^{}
"""

    def test_parses_version_tags_and_skips_peeled_and_nightly(self):
        names = bump_version.version_tag_names_from_ls_remote(self.SAMPLE)
        self.assertEqual(names, ["v26.9.19", "v26.10.9", "v26.10.10"])

    def test_head_reuse_matches_peeled_annotated_tag(self):
        head_tags = bump_version.head_version_tags_from_ls_remote(
            self.SAMPLE, "cccccccccccccccccccccccccccccccccccccccc"
        )
        self.assertEqual(head_tags, ["v26.9.19"])

    def test_head_reuse_matches_lightweight_tag(self):
        head_tags = bump_version.head_version_tags_from_ls_remote(
            self.SAMPLE, "dddddddddddddddddddddddddddddddddddddddd"
        )
        self.assertEqual(head_tags, ["v26.10.9"])


class TestSetVersion(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.original_dir = os.getcwd()
        os.chdir(self.temp_dir)
        bump_version.MODIFIED_FILES.clear()
        Path("src").mkdir()
        Path("fuzz").mkdir()
        Path("vscode-extension").mkdir()
        Path(".build_meta.json").write_text(
            json.dumps({"year": 26, "month": 9, "build": 19}, indent=2),
            encoding="utf-8",
        )
        Path("src/version.rs").write_text(
            'pub const VERSION: &str = "26.9.19";\n', encoding="utf-8"
        )
        Path("Cargo.toml").write_text(
            '[package]\nname = "wfl"\nversion = "26.9.19"\n', encoding="utf-8"
        )
        Path("Cargo.lock").write_text(
            '[[package]]\nname = "wfl"\nversion = "26.9.19"\n', encoding="utf-8"
        )
        Path("fuzz/Cargo.lock").write_text(
            '[[package]]\nname = "wfl"\nversion = "26.9.19"\n', encoding="utf-8"
        )
        Path("wix.toml").write_text(
            '[package]\nversion = "26.9.19.0" # Updated by bump_version.py\n',
            encoding="utf-8",
        )
        Path("vscode-extension/package.json").write_text(
            json.dumps({"name": "vscode-wfl", "version": "26.9.19"}, indent=2),
            encoding="utf-8",
        )
        Path("vscode-extension/package-lock.json").write_text(
            json.dumps(
                {
                    "name": "vscode-wfl",
                    "version": "26.9.19",
                    "packages": {"": {"name": "vscode-wfl", "version": "26.9.19"}},
                },
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )
        subprocess.run(["git", "init", "-q"], check=True)
        subprocess.run(["git", "config", "user.name", "Test"], check=True)
        subprocess.run(["git", "config", "user.email", "test@example.com"], check=True)
        subprocess.run(["git", "add", "."], check=True)
        subprocess.run(["git", "commit", "-q", "-m", "seed"], check=True)
        self.seed = subprocess.check_output(
            ["git", "rev-parse", "HEAD"], text=True
        ).strip()

    def tearDown(self):
        os.chdir(self.original_dir)
        bump_version.MODIFIED_FILES.clear()

    def test_set_version_writes_every_mirror_without_committing(self):
        rc = bump_version.main_with_args(
            ["--set-version", "26.10.1", "--update-all", "--skip-git"]
        )
        self.assertEqual(rc, 0)
        meta = json.loads(Path(".build_meta.json").read_text(encoding="utf-8"))
        self.assertEqual(meta, {"year": 26, "month": 10, "build": 1})
        self.assertEqual(
            Path("src/version.rs").read_text(encoding="utf-8"),
            'pub const VERSION: &str = "26.10.1";\n',
        )
        self.assertIn('version = "26.10.1"', Path("Cargo.toml").read_text(encoding="utf-8"))
        self.assertIn('version = "26.10.1"', Path("Cargo.lock").read_text(encoding="utf-8"))
        self.assertIn(
            'version = "26.10.1"', Path("fuzz/Cargo.lock").read_text(encoding="utf-8")
        )
        self.assertIn(
            'version = "26.10.1.0"', Path("wix.toml").read_text(encoding="utf-8")
        )
        pkg = json.loads(Path("vscode-extension/package.json").read_text(encoding="utf-8"))
        self.assertEqual(pkg["version"], "26.10.1")
        lock = json.loads(
            Path("vscode-extension/package-lock.json").read_text(encoding="utf-8")
        )
        self.assertEqual(lock["version"], "26.10.1")
        self.assertEqual(lock["packages"][""]["version"], "26.10.1")
        head = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
        self.assertEqual(head, self.seed)

    def test_from_tags_print_does_not_write_files(self):
        before = Path(".build_meta.json").read_text(encoding="utf-8")
        with patch.object(
            bump_version,
            "next_version_from_tags",
            return_value="26.10.1",
        ):
            buf = io.StringIO()
            with redirect_stdout(buf):
                rc = bump_version.main_with_args(["--from-tags", "--print"])
        self.assertEqual(rc, 0)
        self.assertEqual(buf.getvalue().strip(), "26.10.1")
        self.assertEqual(Path(".build_meta.json").read_text(encoding="utf-8"), before)


class TestFromTagsCliUsesUtcAndFloor(unittest.TestCase):
    def test_from_tags_print_uses_injected_discovery(self):
        with patch.object(
            bump_version,
            "discover_tag_state",
            return_value=(
                ["v26.9.19"],
                [],
                {"year": 26, "month": 9, "build": 19},
            ),
        ), patch.object(
            bump_version, "utc_now", return_value=utc_date(2026, 10, 9)
        ):
            buf = io.StringIO()
            with redirect_stdout(buf):
                rc = bump_version.main_with_args(["--from-tags", "--print"])
        self.assertEqual(rc, 0)
        self.assertEqual(buf.getvalue().strip(), "26.10.1")


if __name__ == "__main__":
    unittest.main()
