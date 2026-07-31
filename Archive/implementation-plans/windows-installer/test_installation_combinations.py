#!/usr/bin/env python3
"""
Test script for all MSI installation combinations
Tests WFL only, WFL + LSP, WFL + VS Code extension, and all components
"""

import os
import sys
import subprocess
import tempfile
import shutil
from pathlib import Path

# Add the Tools directory to the path
sys.path.insert(0, os.path.abspath(os.path.dirname(__file__)))

import launch_msi_build

def test_cli_arguments():
    """Test all CLI argument combinations."""
    print("=== Testing CLI Arguments ===")
    
    test_cases = [
        # Basic arguments
        [],
        ['--verbose'],
        ['--skip-tests'],
        
        # Component arguments
        ['--include-lsp'],
        ['--include-vscode'],
        ['--include-lsp', '--include-vscode'],
        
        # Combined arguments
        ['--include-lsp', '--verbose'],
        ['--include-vscode', '--skip-tests'],
        ['--include-lsp', '--include-vscode', '--verbose', '--skip-tests'],
        
        # Version arguments
        ['--bump-version'],
        ['--version-override', '2025.9'],
        
        # Output directory
        ['--output-dir', 'test_output'],
    ]
    
    passed = 0
    failed = 0
    
    for i, args in enumerate(test_cases):
        try:
            print(f"Test {i+1}: {' '.join(args) if args else '(no args)'}")
            parsed_args = launch_msi_build.parse_arguments(args)
            
            # Validate parsed arguments
            assert hasattr(parsed_args, 'include_lsp')
            assert hasattr(parsed_args, 'include_vscode')
            assert hasattr(parsed_args, 'interactive')
            assert hasattr(parsed_args, 'verbose')
            assert hasattr(parsed_args, 'skip_tests')
            
            print(f"  ✓ LSP: {parsed_args.include_lsp}, VS Code: {parsed_args.include_vscode}")
            passed += 1
            
        except Exception as e:
            print(f"  ✗ Failed: {e}")
            failed += 1
    
    print(f"\nCLI Arguments Test Results: {passed} passed, {failed} failed")
    return failed == 0

def test_component_detection():
    """Test component detection and validation."""
    print("\n=== Testing Component Detection ===")
    
    tests = [
        ("LSP Server Buildable", launch_msi_build.check_lsp_server_buildable),
        ("VS Code Detection", launch_msi_build.detect_vscode_installation),
        ("Dependencies Check", launch_msi_build.check_dependencies),
    ]
    
    passed = 0
    failed = 0
    
    for test_name, test_func in tests:
        try:
            print(f"Testing {test_name}...")
            result = test_func()
            print(f"  ✓ Result: {result}")
            passed += 1
        except Exception as e:
            print(f"  ✗ Failed: {e}")
            failed += 1
    
    print(f"\nComponent Detection Test Results: {passed} passed, {failed} failed")
    return failed == 0

def test_installation_combinations():
    """Test different installation combinations."""
    print("\n=== Testing Installation Combinations ===")
    
    combinations = [
        {"name": "WFL Only", "include_lsp": False, "include_vscode": False},
        {"name": "WFL + LSP", "include_lsp": True, "include_vscode": False},
        {"name": "WFL + VS Code", "include_lsp": False, "include_vscode": True},
        {"name": "All Components", "include_lsp": True, "include_vscode": True},
    ]
    
    passed = 0
    failed = 0
    
    for combo in combinations:
        try:
            print(f"Testing {combo['name']}...")
            options = {
                'include_lsp': combo['include_lsp'],
                'include_vscode': combo['include_vscode']
            }
            
            # Mock subprocess calls to avoid actual builds
            import unittest.mock
            with unittest.mock.patch('subprocess.run') as mock_run:
                mock_run.return_value.returncode = 0
                mock_run.return_value.stdout = "mock output"
                
                result = launch_msi_build.run_installation_with_options(options)
                
                # Validate result structure
                assert isinstance(result, dict)
                assert 'core_success' in result
                assert 'lsp_success' in result
                assert 'vscode_success' in result
                
                print(f"  ✓ Core: {result['core_success']}, LSP: {result['lsp_success']}, VS Code: {result['vscode_success']}")
                passed += 1
                
        except Exception as e:
            print(f"  ✗ Failed: {e}")
            failed += 1
    
    print(f"\nInstallation Combinations Test Results: {passed} passed, {failed} failed")
    return failed == 0

def test_configuration_setup():
    """Test configuration setup functions."""
    print("\n=== Testing Configuration Setup ===")
    
    tests = [
        ("LSP Configuration", launch_msi_build.setup_lsp_configuration),
        ("Language Associations", launch_msi_build.setup_language_associations),
        ("LSP Installation Config", launch_msi_build.get_lsp_installation_config),
    ]
    
    passed = 0
    failed = 0
    
    for test_name, test_func in tests:
        try:
            print(f"Testing {test_name}...")
            result = test_func()
            print(f"  ✓ Result type: {type(result)}")
            if isinstance(result, dict):
                print(f"    Keys: {list(result.keys())}")
            passed += 1
        except Exception as e:
            print(f"  ✗ Failed: {e}")
            failed += 1
    
    print(f"\nConfiguration Setup Test Results: {passed} passed, {failed} failed")
    return failed == 0

def test_error_handling():
    """Test error handling scenarios."""
    print("\n=== Testing Error Handling ===")
    
    passed = 0
    failed = 0
    
    # Test missing dependencies
    try:
        print("Testing dependency check with missing tools...")
        import unittest.mock
        with unittest.mock.patch('subprocess.run') as mock_run:
            mock_run.return_value.returncode = 1  # Command failed
            result = launch_msi_build.check_dependencies()
            assert result == False
            print("  ✓ Correctly detected missing dependencies")
            passed += 1
    except Exception as e:
        print(f"  ✗ Failed: {e}")
        failed += 1
    
    # Test partial failure handling
    try:
        print("Testing partial failure handling...")
        import unittest.mock
        with unittest.mock.patch('launch_msi_build.build_lsp_server', return_value=False):
            with unittest.mock.patch('launch_msi_build.build_vscode_extension', return_value=True):
                options = {'include_lsp': True, 'include_vscode': True}
                result = launch_msi_build.run_installation_with_options(options)
                assert result['lsp_success'] == False
                assert result['vscode_success'] == True
                print("  ✓ Correctly handled partial failure")
                passed += 1
    except Exception as e:
        print(f"  ✗ Failed: {e}")
        failed += 1
    
    print(f"\nError Handling Test Results: {passed} passed, {failed} failed")
    return failed == 0

def test_backward_compatibility():
    """Test backward compatibility with existing functionality."""
    print("\n=== Testing Backward Compatibility ===")
    
    passed = 0
    failed = 0
    
    # Test existing arguments still work
    try:
        print("Testing existing arguments...")
        args = launch_msi_build.parse_arguments(['--bump-version', '--verbose'])
        assert args.bump_version == True
        assert args.verbose == True
        # New arguments should have default values
        assert args.include_lsp == False
        assert args.include_vscode == False
        print("  ✓ Existing arguments work with new defaults")
        passed += 1
    except Exception as e:
        print(f"  ✗ Failed: {e}")
        failed += 1
    
    # Test legacy MSI build function exists
    try:
        print("Testing legacy MSI build function...")
        class MockArgs:
            def __init__(self):
                self.output_dir = None
                self.verbose = False
        
        mock_args = MockArgs()
        assert callable(launch_msi_build.run_msi_build_legacy)
        print("  ✓ Legacy MSI build function is callable")
        passed += 1
    except Exception as e:
        print(f"  ✗ Failed: {e}")
        failed += 1
    
    print(f"\nBackward Compatibility Test Results: {passed} passed, {failed} failed")
    return failed == 0

def main():
    """Run all installation combination tests."""
    print("=" * 60)
    print("COMPREHENSIVE MSI INSTALLER TESTING")
    print("=" * 60)
    
    all_tests = [
        test_cli_arguments,
        test_component_detection,
        test_installation_combinations,
        test_configuration_setup,
        test_error_handling,
        test_backward_compatibility,
    ]
    
    passed_tests = 0
    total_tests = len(all_tests)
    
    for test_func in all_tests:
        if test_func():
            passed_tests += 1
    
    print("\n" + "=" * 60)
    print(f"OVERALL RESULTS: {passed_tests}/{total_tests} test suites passed")
    print("=" * 60)
    
    if passed_tests == total_tests:
        print("🎉 All tests passed! MSI installer enhancement is working correctly.")
        return 0
    else:
        print("❌ Some tests failed. Please review the output above.")
        return 1

if __name__ == "__main__":
    sys.exit(main())
