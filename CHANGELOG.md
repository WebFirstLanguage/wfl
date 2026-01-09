# Changelog

All notable changes to the WFL project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project uses a calendar-based versioning scheme: **YY.MM.BUILD**.

## [25.9.1] - 2025-09-20

### Added
- Comprehensive documentation consolidation and optimization
- New consolidated development guide for AI assistants
- Enhanced README with GitHub-optimized navigation
- Table of contents and collapsible sections for better browsing
- Improved cross-linking between documentation files

### Changed
- Updated version scheme documentation with current examples
- Reorganized documentation structure for better GitHub navigation
- Consolidated AI assistant instructions into single comprehensive guide
- Enhanced project status display with collapsible details

### Removed
- Redundant AGENTS.md and CLAUDE.md files (consolidated into .augment/rules/DEVELOPMENT.md)
- Outdated version references throughout documentation

### Fixed
- Version consistency across all documentation files
- Broken or outdated links in documentation
- Documentation navigation structure

## [25.8.11] - 2025-08-12

### Added
- Enhanced bracket array indexing support
- Comprehensive pattern matching with natural language syntax
- Improved error reporting with source context
- Advanced async/await functionality
- Container system for object-oriented programming

### Fixed
- Fixed bracket array indexing parsing issues
- Improved memory management in parser
- Enhanced error recovery in lexer
- Fixed static analyzer variable usage detection

## [25.5.30] - 2025-05-30

### Added
- Configuration validation & auto-fix flags (`--configCheck` and `--configFix`)
- Enhanced SDK integration and bug reporting system
- Improved development tooling and debugging capabilities

### Fixed
- Fixed memory leak in closures with weak references to parent environments
- Improved file I/O with append-mode operations instead of read-modify-write
- Optimized parser memory allocations to reduce heap churn
- Fixed static analyzer incorrectly flagging variables as unused in action definitions

## [25.4.20] - 2025-04-20

### Added
- Nightly build and installer pipeline for Windows, Linux, and macOS
- Automated installers: MSI for Windows, tar.gz/deb for Linux, pkg for macOS
- Skip-if-unchanged logic to avoid unnecessary builds
- Default configuration files included in installers
- Documentation for building and releasing WFL

### Changed
- Updated build system to support cross-platform compilation
- Updated documentation to clarify sequential wait-for behavior

### Fixed
- Fixed memory leak in closures by using weak references for captured environments
- Improved debug report to return a Result and show appropriate error messages
- Hardened `.clear` REPL command against stdout failures

## Version Scheme

WFL uses a calendar-based version scheme: **YY.MM.BUILD**

- **YY**: Two-digit year (e.g., 25 for 2025)
- **MM**: Month number (1-12)
- **BUILD**: Build number within the month (resets each month)

Example: `25.9.1` means Year 2025, September, Build 1

This format ensures compatibility with Windows MSI installers while providing clear release date information.
