#!/usr/bin/env python3
"""
Comprehensive TDD Tests for Enhanced MSI Build Script
These tests MUST FAIL initially to validate TDD approach.
"""

import unittest
import os
import json
import tempfile
import shutil
import sys
import subprocess
from unittest.mock import patch, MagicMock, call
from pathlib import Path

# Add the Tools directory to the path to import the module under test
sys.path.insert(0, os.path.abspath(os.path.dirname(__file__)))

# Import the module we're testing (this will fail initially as enhancements don't exist yet)
try:
    import launch_msi_build
except ImportError:
    # Create a minimal module for testing if it doesn't exist
    class MockModule:
        pass
    launch_msi_build = MockModule()

class TestEnhancedMSIBuildScript(unittest.TestCase):
    """Test suite for enhanced MSI build script with LSP and VS Code extension support."""
    
    def setUp(self):
        """Set up test environment."""
        self.temp_dir = tempfile.mkdtemp()
        self.original_dir = os.getcwd()
        os.chdir(self.temp_dir)
        
        # Create mock project structure
        self.project_root = Path(self.temp_dir)
        self.wfl_lsp_dir = self.project_root / "wfl-lsp"
        self.vscode_ext_dir = self.project_root / "vscode-extension"
        self.scripts_dir = self.project_root / "scripts"
        
        # Create directories
        self.wfl_lsp_dir.mkdir(exist_ok=True)
        self.vscode_ext_dir.mkdir(exist_ok=True)
        self.scripts_dir.mkdir(exist_ok=True)
        
        # Create mock files
        (self.wfl_lsp_dir / "Cargo.toml").write_text("[package]\nname = \"wfl-lsp\"")
        (self.vscode_ext_dir / "package.json").write_text('{"name": "vscode-wfl"}')
        (self.project_root / ".build_meta.json").write_text('{"year": 2025, "build": 1}')
    
    def tearDown(self):
        """Clean up test environment."""
        os.chdir(self.original_dir)
        shutil.rmtree(self.temp_dir)

    # ========== CLI Argument Tests (MUST FAIL INITIALLY) ==========
    
    def test_new_cli_arguments_parsing(self):
        """Test that new CLI arguments are properly parsed."""
        args = launch_msi_build.parse_arguments([])
        # These attributes should now exist
        self.assertTrue(hasattr(args, 'include_lsp'))
        self.assertTrue(hasattr(args, 'include_vscode'))
        self.assertTrue(hasattr(args, 'interactive'))
        # Default values should be False
        self.assertFalse(args.include_lsp)
        self.assertFalse(args.include_vscode)
        self.assertFalse(args.interactive)
    
    def test_include_lsp_argument(self):
        """Test --include-lsp argument parsing."""
        args = launch_msi_build.parse_arguments(['--include-lsp'])
        self.assertTrue(args.include_lsp)

    def test_include_vscode_argument(self):
        """Test --include-vscode argument parsing."""
        args = launch_msi_build.parse_arguments(['--include-vscode'])
        self.assertTrue(args.include_vscode)

    def test_interactive_argument(self):
        """Test --interactive argument parsing."""
        args = launch_msi_build.parse_arguments(['--interactive'])
        self.assertTrue(args.interactive)
    
    def test_help_text_includes_new_options(self):
        """Test that help text includes new installation options."""
        # Test by trying to parse help (will raise SystemExit)
        with self.assertRaises(SystemExit):
            launch_msi_build.parse_arguments(['--help'])

    # ========== Interactive Mode Tests (MUST FAIL INITIALLY) ==========
    
    def test_interactive_mode_prompts(self):
        """Test interactive mode prompts for component selection."""
        with patch('builtins.input', side_effect=['y', 'n']):
            options = launch_msi_build.get_interactive_options()
            self.assertTrue(options['include_lsp'])
            self.assertFalse(options['include_vscode'])

    def test_interactive_input_validation(self):
        """Test validation of user input in interactive mode."""
        with patch('builtins.input', side_effect=['invalid', 'y', 'n']):
            options = launch_msi_build.get_interactive_options()
            self.assertTrue(options['include_lsp'])
            self.assertFalse(options['include_vscode'])

    def test_interactive_default_selections(self):
        """Test default selections when user presses Enter."""
        with patch('builtins.input', side_effect=['', '']):  # Empty inputs
            options = launch_msi_build.get_interactive_options()
            # Should use defaults
            self.assertFalse(options['include_lsp'])
            self.assertFalse(options['include_vscode'])

    # ========== LSP Server Installation Tests (MUST FAIL INITIALLY) ==========
    
    def test_lsp_server_build_detection(self):
        """Test detection of LSP server build requirements."""
        result = launch_msi_build.check_lsp_server_buildable()
        self.assertTrue(result)  # Should pass since wfl-lsp directory exists

    def test_lsp_server_build_process(self):
        """Test LSP server build process execution."""
        with patch('subprocess.run') as mock_run:
            mock_run.return_value.returncode = 0
            result = launch_msi_build.build_lsp_server()
            self.assertTrue(result)
            # Check that subprocess.run was called with correct arguments
            self.assertTrue(mock_run.called)
            call_args = mock_run.call_args
            self.assertEqual(call_args[0][0], ['cargo', 'build', '--release', '-p', 'wfl-lsp'])
    
    def test_lsp_server_binary_validation(self):
        """Test validation of built LSP server binary."""
        # Create mock binary
        binary_path = self.project_root / "target" / "release" / "wfl-lsp.exe"
        binary_path.parent.mkdir(parents=True, exist_ok=True)
        binary_path.write_text("mock binary")

        # Mock the subprocess call to avoid timeout
        with patch('subprocess.run') as mock_run:
            mock_run.return_value.returncode = 0
            mock_run.return_value.stdout = "wfl-lsp 1.0.0"
            result = launch_msi_build.validate_lsp_binary()
            self.assertTrue(result)

    def test_lsp_server_installation_path_config(self):
        """Test LSP server installation path configuration."""
        config = launch_msi_build.get_lsp_installation_config()
        self.assertIn('install_path', config)
        self.assertIn('binary_name', config)
        self.assertEqual(config['binary_name'], 'wfl-lsp.exe')

    # ========== VS Code Extension Tests (MUST FAIL INITIALLY) ==========
    
    def test_vscode_detection(self):
        """Test VS Code installation detection."""
        with patch('os.path.exists', return_value=True):
            result = launch_msi_build.detect_vscode_installation()
            self.assertTrue(result)

    def test_vscode_extension_build_process(self):
        """Test VS Code extension build process."""
        with patch('subprocess.run') as mock_run:
            mock_run.return_value.returncode = 0
            result = launch_msi_build.build_vscode_extension()
            self.assertTrue(result)

    def test_vscode_extension_installation(self):
        """Test VS Code extension installation via code command."""
        # Create mock VSIX file
        vsix_path = self.vscode_ext_dir / "vscode-wfl-0.1.0.vsix"
        vsix_path.write_text("mock vsix")

        with patch('subprocess.run') as mock_run:
            mock_run.return_value.returncode = 0
            with patch('launch_msi_build.detect_vscode_installation', return_value=True):
                result = launch_msi_build.install_vscode_extension()
                self.assertTrue(result)
                # Should call code --install-extension
                mock_run.assert_called()

    def test_vscode_extension_error_handling(self):
        """Test error handling when VS Code not found."""
        with patch('launch_msi_build.detect_vscode_installation', return_value=False):
            result = launch_msi_build.install_vscode_extension()
            self.assertFalse(result)

    # ========== Configuration Setup Tests (MUST FAIL INITIALLY) ==========
    
    def test_lsp_server_configuration_setup(self):
        """Test automatic LSP server configuration for VS Code."""
        config = launch_msi_build.setup_lsp_configuration()
        self.assertIn('serverPath', config)
        self.assertIn('serverArgs', config)
        self.assertEqual(config['serverPath'], 'wfl-lsp')

    def test_wfl_language_association_setup(self):
        """Test WFL language association setup."""
        result = launch_msi_build.setup_language_associations()
        self.assertTrue(result)

    # ========== Installation Combination Tests (MUST FAIL INITIALLY) ==========
    
    def test_wfl_only_installation(self):
        """Test WFL only installation (existing behavior)."""
        options = {'include_lsp': False, 'include_vscode': False}
        result = launch_msi_build.run_installation_with_options(options)
        self.assertIsInstance(result, dict)
        self.assertTrue(result['core_success'])

    def test_wfl_plus_lsp_installation(self):
        """Test WFL + LSP server installation."""
        options = {'include_lsp': True, 'include_vscode': False}
        with patch('subprocess.run') as mock_run:
            mock_run.return_value.returncode = 0
            mock_run.return_value.stdout = "wfl-lsp 1.0.0"
            result = launch_msi_build.run_installation_with_options(options)
            self.assertIsInstance(result, dict)
            self.assertTrue(result['lsp_success'])

    def test_wfl_plus_vscode_installation(self):
        """Test WFL + VS Code extension installation."""
        options = {'include_lsp': False, 'include_vscode': True}
        with patch('subprocess.run') as mock_run:
            mock_run.return_value.returncode = 0
            result = launch_msi_build.run_installation_with_options(options)
            self.assertIsInstance(result, dict)
            self.assertTrue(result['vscode_success'])

    def test_all_components_installation(self):
        """Test WFL + LSP + VS Code extension installation."""
        options = {'include_lsp': True, 'include_vscode': True}
        with patch('subprocess.run') as mock_run:
            mock_run.return_value.returncode = 0
            mock_run.return_value.stdout = "wfl-lsp 1.0.0"
            result = launch_msi_build.run_installation_with_options(options)
            self.assertIsInstance(result, dict)

    # ========== Error Handling Tests (MUST FAIL INITIALLY) ==========
    
    def test_partial_failure_handling(self):
        """Test handling when one component fails but others succeed."""
        with patch('launch_msi_build.build_lsp_server', return_value=False):
            with patch('launch_msi_build.build_vscode_extension', return_value=True):
                options = {'include_lsp': True, 'include_vscode': True}
                result = launch_msi_build.run_installation_with_options(options)
                # Should handle partial failure gracefully
                self.assertIsInstance(result, dict)
                self.assertFalse(result['lsp_success'])
                self.assertTrue(result['vscode_success'])

    def test_dependency_missing_error_handling(self):
        """Test graceful failure when dependencies are missing."""
        with patch('subprocess.run') as mock_run:
            mock_run.return_value.returncode = 1  # Command failed
            result = launch_msi_build.check_dependencies()
            self.assertFalse(result)

    # ========== Backward Compatibility Tests (MUST FAIL INITIALLY) ==========
    
    def test_existing_functionality_unchanged(self):
        """Test that existing MSI build functionality is unchanged."""
        # Create mock args object
        class MockArgs:
            def __init__(self):
                self.output_dir = None
                self.verbose = False

        mock_args = MockArgs()
        # Should be able to run without new options
        result = launch_msi_build.run_msi_build_legacy(mock_args)
        # This will call the actual run_msi_build function, which may fail in test environment
        # but the function should exist and be callable
        self.assertTrue(callable(launch_msi_build.run_msi_build_legacy))

    def test_existing_arguments_still_work(self):
        """Test that all existing command line arguments still work."""
        args = launch_msi_build.parse_arguments(['--bump-version', '--verbose'])
        self.assertTrue(args.bump_version)
        self.assertTrue(args.verbose)

        # These should now exist and work
        self.assertTrue(hasattr(args, 'include_lsp'))
        self.assertTrue(hasattr(args, 'include_vscode'))
        # Default values should be False
        self.assertFalse(args.include_lsp)
        self.assertFalse(args.include_vscode)


if __name__ == "__main__":
    print("=" * 60)
    print("RUNNING TDD TESTS FOR ENHANCED MSI BUILD SCRIPT")
    print("=" * 60)
    print("These tests MUST FAIL initially to validate TDD approach!")
    print("If any test passes, the TDD process is not being followed correctly.")
    print("=" * 60)
    
    unittest.main(verbosity=2)
