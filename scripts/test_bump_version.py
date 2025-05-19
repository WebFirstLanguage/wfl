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
        
        with open(self.build_meta, "w") as f:
            json.dump({"year": current_year, "build": 5}, f)
        
        with patch('datetime.datetime') as mock_datetime:
            mock_datetime.now.return_value.year = current_year
            with patch('subprocess.run'):
                bump_version.main()
        
        with open(self.build_meta, "r") as f:
            meta = json.load(f)
            self.assertEqual(meta["year"], current_year)
            self.assertEqual(meta["build"], 6)  # Incremented from 5
        
        with open(self.version_file, "r") as f:
            content = f.read()
            self.assertIn(f'"{current_year}.6"', content)

    def test_new_year_reset(self):
        current_year = datetime.datetime.now().year
        last_year = current_year - 1
        
        with open(self.build_meta, "w") as f:
            json.dump({"year": last_year, "build": 42}, f)
        
        with patch('datetime.datetime') as mock_datetime:
            mock_datetime.now.return_value.year = current_year
            with patch('subprocess.run'):
                bump_version.main()
        
        with open(self.build_meta, "r") as f:
            meta = json.load(f)
            self.assertEqual(meta["year"], current_year)
            self.assertEqual(meta["build"], 1)  # Reset to 1 for new year
        
        with open(self.version_file, "r") as f:
            content = f.read()
            self.assertIn(f'"{current_year}.1"', content)

if __name__ == "__main__":
    unittest.main()
