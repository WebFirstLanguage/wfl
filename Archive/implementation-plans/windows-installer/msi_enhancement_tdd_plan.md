# TDD Plan for MSI Installer Enhancement

## Overview
Enhance the MSI installer build script at `Tools\launch_msi_build.py` to include optional installation of:
1. **LSP Server** from `wfl-lsp/` directory
2. **VS Code Extension** from `vscode-extension/` directory
3. **Automatic Configuration** of both components

## Test Cases to Write (MUST FAIL FIRST)

### 1. Command Line Interface Tests
- [ ] Test new CLI arguments: `--include-lsp`, `--include-vscode`, `--interactive`
- [ ] Test argument validation and error handling
- [ ] Test help text includes new options
- [ ] Test backward compatibility with existing arguments

### 2. Interactive Mode Tests
- [ ] Test interactive prompts for component selection
- [ ] Test user input validation (y/n, yes/no, etc.)
- [ ] Test default selections when user presses Enter
- [ ] Test cancellation of interactive mode

### 3. LSP Server Installation Tests
- [ ] Test LSP server build process (cargo build --release -p wfl-lsp)
- [ ] Test LSP server binary detection and validation
- [ ] Test LSP server installation path configuration
- [ ] Test LSP server registration for VS Code integration
- [ ] Test error handling when LSP build fails

### 4. VS Code Extension Installation Tests
- [ ] Test VS Code extension build process (npm install, npm run compile)
- [ ] Test VSIX package creation
- [ ] Test VS Code detection on system
- [ ] Test extension installation via `code --install-extension`
- [ ] Test error handling when VS Code not found
- [ ] Test error handling when extension installation fails

### 5. Configuration Setup Tests
- [ ] Test automatic LSP server path configuration in VS Code settings
- [ ] Test WFL language association setup
- [ ] Test extension activation verification
- [ ] Test configuration file creation and validation

### 6. Installation Combination Tests
- [ ] Test WFL only installation (existing behavior)
- [ ] Test WFL + LSP server installation
- [ ] Test WFL + VS Code extension installation
- [ ] Test WFL + LSP + VS Code extension installation (all components)
- [ ] Test partial failure scenarios (one component fails, others succeed)

### 7. WiX Integration Tests
- [ ] Test WiX configuration updates for new components
- [ ] Test custom actions for LSP and VS Code installation
- [ ] Test feature selection in MSI installer UI
- [ ] Test component installation paths and registry entries

### 8. Error Handling and Recovery Tests
- [ ] Test graceful failure when dependencies missing
- [ ] Test rollback scenarios when installation partially fails
- [ ] Test user notification of installation status
- [ ] Test log file creation for troubleshooting

### 9. Backward Compatibility Tests
- [ ] Test existing MSI build functionality unchanged
- [ ] Test all existing command line arguments work
- [ ] Test existing PowerShell script integration
- [ ] Test version management integration

### 10. Integration Tests
- [ ] Test complete end-to-end installation process
- [ ] Test installed components work correctly together
- [ ] Test VS Code can communicate with LSP server
- [ ] Test WFL syntax highlighting and completion work

## Implementation Strategy

### Phase 1: Core Infrastructure (TDD)
1. Write failing tests for new CLI arguments
2. Implement minimal argument parsing to pass tests
3. Write failing tests for interactive mode
4. Implement basic interactive prompts

### Phase 2: LSP Server Integration (TDD)
1. Write failing tests for LSP server build detection
2. Implement LSP server build logic
3. Write failing tests for LSP installation
4. Implement LSP installation and configuration

### Phase 3: VS Code Extension Integration (TDD)
1. Write failing tests for VS Code detection
2. Implement VS Code detection logic
3. Write failing tests for extension installation
4. Implement extension build and installation

### Phase 4: WiX Configuration (TDD)
1. Write failing tests for WiX feature configuration
2. Update WiX files to support new components
3. Write failing tests for custom actions
4. Implement custom installation actions

### Phase 5: Integration and Testing (TDD)
1. Write failing tests for complete installation scenarios
2. Implement end-to-end installation logic
3. Write failing tests for error recovery
4. Implement robust error handling

## Files to Modify

### Python Files
- `Tools/launch_msi_build.py` - Main enhancement target
- `Tools/test_launch_msi_build.py` - New comprehensive test file

### PowerShell Scripts
- `build_msi.ps1` - May need updates for new components
- `scripts/install_vscode_extension.ps1` - May need enhancements

### WiX Configuration
- `wix.toml` - Add new features and custom actions
- `wix/main.wxs` - Add UI elements for component selection

### Documentation
- `Docs/guides/building.md` - Document new installation options
- `README.md` - Update installation instructions

## Test Data Requirements

### Mock Files and Directories
- Mock LSP server binary for testing
- Mock VS Code installation paths
- Mock extension files for testing
- Mock WiX output for validation

### Test Scenarios Data
- Valid and invalid command line arguments
- Various user input combinations
- Different system configurations
- Error conditions and edge cases

## Success Criteria

### Functional Requirements
1. All new tests pass after implementation
2. All existing tests continue to pass (backward compatibility)
3. Interactive mode provides clear user experience
4. All installation combinations work correctly
5. Error handling is robust and informative

### Quality Requirements
1. Code coverage > 90% for new functionality
2. All edge cases handled gracefully
3. Clear error messages for troubleshooting
4. Performance impact minimal on build process
5. Documentation complete and accurate

## Risk Mitigation

### Technical Risks
- **LSP build failures**: Implement fallback and clear error messages
- **VS Code detection issues**: Support multiple installation paths
- **WiX integration complexity**: Incremental testing approach
- **Cross-platform compatibility**: Focus on Windows first, document limitations

### Process Risks
- **TDD compliance**: Strict enforcement of test-first development
- **Backward compatibility**: Comprehensive regression testing
- **User experience**: Extensive interactive mode testing
- **Documentation**: Keep docs updated throughout development

## Next Steps

1. **Create failing test file** `Tools/test_launch_msi_build.py`
2. **Run tests to confirm failures** (TDD validation)
3. **Commit failing tests** as baseline
4. **Implement minimal functionality** to pass first test
5. **Iterate through TDD cycle** for each component
6. **Integration testing** with real MSI builds
7. **Documentation updates** and user guides
8. **Final validation** with clean system testing

---

**TDD Reminder**: Every implementation change must be preceded by a failing test. No exceptions.
