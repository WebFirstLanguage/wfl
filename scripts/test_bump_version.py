#!/usr/bin/env python3
import unittest
import os
import json
import tempfile
import shutil
import sys
import datetime
from unittest.mock import patch

sys.path.insert(0, os.path.abspath(os.path.dirname(__file__)))

import bump_version

class TestBumpVersion(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.original_dir = os.getcwd()
        os.chdir(self.temp_dir)
        
        self.build_meta = os.path.join(self.temp_dir, ".build_meta.json")
        self.version_file = os.path.join(self.temp_dir, "src/version.rs")
        
        os.makedirs(os.path.dirname(self.version_file), exist_ok=True)
        
        bump_version.BUILD_META_FILE = self.build_meta
        bump_version.VERSION_FILE = self.version_file
        
        os.system("git init .")
        os.system("git config user.name 'Test User'")
        os.system("git config user.email 'test@example.com'")
    
    def tearDown(self):
        os.chdir(self.original_dir)
        shutil.rmtree(self.temp_dir)

    def test_same_year_increment(self):
        current_year = datetime.datetime.now().year
        current_month = datetime.datetime.now().month

        with open(self.build_meta, "w") as f:
            json.dump({"year": current_year, "month": current_month, "build": 5}, f)

        with patch('datetime.datetime') as mock_datetime:
            mock_datetime.now.return_value.year = current_year
            mock_datetime.now.return_value.month = current_month
            with patch('subprocess.run'):
                with patch('sys.argv', ['bump_version.py']):
                    bump_version.main()

        with open(self.build_meta, "r") as f:
            meta = json.load(f)
            self.assertEqual(meta["year"], current_year)
            self.assertEqual(meta["month"], current_month)
            self.assertEqual(meta["build"], 6)  # Incremented from 5

        with open(self.version_file, "r") as f:
            content = f.read()
            self.assertIn(f'"{current_year}.{current_month}.6"', content)

    def test_new_year_reset(self):
        current_year = datetime.datetime.now().year
        current_month = datetime.datetime.now().month
        last_year = current_year - 1

        with open(self.build_meta, "w") as f:
            json.dump({"year": last_year, "month": current_month, "build": 42}, f)

        with patch('datetime.datetime') as mock_datetime:
            mock_datetime.now.return_value.year = current_year
            mock_datetime.now.return_value.month = current_month
            with patch('subprocess.run'):
                with patch('sys.argv', ['bump_version.py']):
                    bump_version.main()

        with open(self.build_meta, "r") as f:
            meta = json.load(f)
            self.assertEqual(meta["year"], current_year)
            self.assertEqual(meta["month"], current_month)
            self.assertEqual(meta["build"], 1)  # Reset to 1 for new year

        with open(self.version_file, "r") as f:
            content = f.read()
            self.assertIn(f'"{current_year}.{current_month}.1"', content)

    def test_wfl_lsp_version_updated(self):
        """Test that wfl-lsp/Cargo.toml is updated with --update-all"""
        current_year = datetime.datetime.now().year
        current_month = datetime.datetime.now().month

        # Create wfl-lsp directory structure
        wfl_lsp_dir = os.path.join(self.temp_dir, "wfl-lsp")
        os.makedirs(wfl_lsp_dir, exist_ok=True)
        wfl_lsp_cargo_toml = os.path.join(wfl_lsp_dir, "Cargo.toml")

        # Create wfl-lsp/Cargo.toml with version 0.1.0
        with open(wfl_lsp_cargo_toml, "w") as f:
            f.write('[package]\nname = "wfl-lsp"\nversion = "0.1.0"\nedition = "2024"\n')

        # Create main Cargo.toml
        cargo_toml = os.path.join(self.temp_dir, "Cargo.toml")
        with open(cargo_toml, "w") as f:
            f.write('[package]\nname = "wfl"\nversion = "1.0.0"\n')

        # Create build meta
        with open(self.build_meta, "w") as f:
            json.dump({"year": current_year, "month": current_month, "build": 10}, f)

        # Run bump_version with --update-all --skip-bump
        with patch('subprocess.run'):
            with patch('sys.argv', ['bump_version.py', '--update-all', '--skip-bump', '--skip-git']):
                bump_version.main()

        # Verify wfl-lsp/Cargo.toml was updated
        with open(wfl_lsp_cargo_toml, "r") as f:
            content = f.read()
            expected_version = f"{current_year}.{current_month}.10"
            self.assertIn(f'version = "{expected_version}"', content)

    def test_wfl_lsp_cargo_lock_synchronized(self):
        """Test that Cargo.lock contains correct wfl-lsp version"""
        current_year = datetime.datetime.now().year
        current_month = datetime.datetime.now().month

        # Create workspace structure
        wfl_lsp_dir = os.path.join(self.temp_dir, "wfl-lsp")
        os.makedirs(wfl_lsp_dir, exist_ok=True)
        wfl_lsp_cargo_toml = os.path.join(wfl_lsp_dir, "Cargo.toml")

        with open(wfl_lsp_cargo_toml, "w") as f:
            f.write('[package]\nname = "wfl-lsp"\nversion = "0.1.0"\n')

        cargo_toml = os.path.join(self.temp_dir, "Cargo.toml")
        with open(cargo_toml, "w") as f:
            f.write('[package]\nname = "wfl"\nversion = "1.0.0"\n')

        cargo_lock = os.path.join(self.temp_dir, "Cargo.lock")
        expected_version = f"{current_year}.{current_month}.10"

        with open(self.build_meta, "w") as f:
            json.dump({"year": current_year, "month": current_month, "build": 10}, f)

        # Mock cargo update to create Cargo.lock
        def mock_run(*args, **kwargs):
            # Create a mock Cargo.lock with both packages
            with open(cargo_lock, "w") as f:
                f.write(f'''[[package]]
name = "wfl"
version = "{expected_version}"

[[package]]
name = "wfl-lsp"
version = "{expected_version}"
''')
            return type('obj', (object,), {'returncode': 0, 'stdout': '', 'stderr': ''})()

        with patch('subprocess.run', side_effect=mock_run):
            with patch('sys.argv', ['bump_version.py', '--update-all', '--skip-bump', '--skip-git']):
                bump_version.main()

        # Verify Cargo.lock contains correct wfl-lsp version
        with open(cargo_lock, "r") as f:
            content = f.read()
            self.assertIn(f'name = "wfl-lsp"', content)
            self.assertIn(f'version = "{expected_version}"', content)

    def test_all_versions_synchronized(self):
        """Integration test: verify all files have matching versions"""
        current_year = datetime.datetime.now().year
        current_month = datetime.datetime.now().month

        # Create full workspace structure
        wfl_lsp_dir = os.path.join(self.temp_dir, "wfl-lsp")
        os.makedirs(wfl_lsp_dir, exist_ok=True)
        wfl_lsp_cargo_toml = os.path.join(wfl_lsp_dir, "Cargo.toml")

        with open(wfl_lsp_cargo_toml, "w") as f:
            f.write('[package]\nname = "wfl-lsp"\nversion = "0.1.0"\n')

        cargo_toml = os.path.join(self.temp_dir, "Cargo.toml")
        with open(cargo_toml, "w") as f:
            f.write('[package]\nname = "wfl"\nversion = "1.0.0"\n')

        cargo_lock = os.path.join(self.temp_dir, "Cargo.lock")

        with open(self.build_meta, "w") as f:
            json.dump({"year": current_year, "month": current_month, "build": 10}, f)

        expected_version = f"{current_year}.{current_month}.10"

        # Mock cargo update
        def mock_run(*args, **kwargs):
            with open(cargo_lock, "w") as f:
                f.write(f'''[[package]]
name = "wfl"
version = "{expected_version}"

[[package]]
name = "wfl-lsp"
version = "{expected_version}"
''')
            return type('obj', (object,), {'returncode': 0, 'stdout': '', 'stderr': ''})()

        with patch('subprocess.run', side_effect=mock_run):
            with patch('sys.argv', ['bump_version.py', '--update-all', '--skip-bump', '--skip-git']):
                bump_version.main()

        # Verify all versions match
        with open(self.build_meta, "r") as f:
            meta = json.load(f)
            version_from_meta = f"{meta['year']}.{meta['month']}.{meta['build']}"
            self.assertEqual(version_from_meta, expected_version)

        with open(cargo_toml, "r") as f:
            content = f.read()
            self.assertIn(f'version = "{expected_version}"', content)

        with open(wfl_lsp_cargo_toml, "r") as f:
            content = f.read()
            self.assertIn(f'version = "{expected_version}"', content)

        with open(self.version_file, "r") as f:
            content = f.read()
            self.assertIn(f'"{expected_version}"', content)

if __name__ == "__main__":
    unittest.main()
