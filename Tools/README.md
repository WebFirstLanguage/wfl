# WFL Tools

This directory contains utility tools for the WFL project.

## Available Tools

### MSI Build Launcher (`launch_msi_build.py`)

A utility for launching MSI build sessions for the WFL project.

#### Features
- Coordinates version management using `scripts/bump_version.py`
- Executes the MSI build process using `build_msi.ps1`
- Creates Windows MSI installer with .wfl file associations
- Automatically updates documentation in implementation progress files
- Provides clear feedback on build success/failure

#### Usage
```bash
python launch_msi_build.py [options]
```

Options:
- `--bump-version`: Increment the build number
- `--version-override VALUE`: Override the version number (format: YYYY.MM)
- `--output-dir DIR`: Specify custom output directory for the MSI file
- `--skip-tests`: Skip running tests before building
- `--verbose`: Show detailed output during execution

#### Examples

Launch a build with the current version:
```bash
python launch_msi_build.py
```

Launch a build with an incremented version number:
```bash
python launch_msi_build.py --bump-version
```

Launch a build with a specific version:
```bash
python launch_msi_build.py --version-override 2025.6
```

### WFL Configuration Checker (`wfl_config_checker.py`)

A utility for checking and fixing WFL configuration files.

#### Features
- Checks for existence and correctness of all `.wflcfg` files
- Validates configuration settings against expected types and values
- Provides detailed reports of any issues found
- Can automatically fix common configuration issues

#### Usage
```bash
python wfl_config_checker.py [options]
```

Options:
- `--project-dir DIR, -d DIR`: Specify project directory to check (default: current directory)
- `--fix`: Automatically fix issues found (creates missing files and corrects invalid settings)
- `--verbose, -v`: Show detailed information during execution

#### Examples

Check configuration in current directory:
```bash
python wfl_config_checker.py
```

Check configuration in a specific directory and fix issues:
```bash
python wfl_config_checker.py --project-dir /path/to/wfl --fix
```

### Rust Line Counter (`rust_loc_counter.py`)

A utility for counting lines of Rust code in the project.

#### Features
- Counts total lines, code lines, comments, and blank lines
- Provides a breakdown by directory and file
- Generates a formatted report as markdown

#### Usage
```bash
python rust_loc_counter.py
```

### WFL Markdown Combiner (`wfl_md_combiner.py`)

A Python utility for combining markdown files into a single document.

#### Usage
See the script's internal documentation for details.

### WFL File Combiner (`wfl_combiner.wfl`)

A WFL implementation of the markdown combiner, demonstrating WFL's file I/O capabilities.

#### Features
- Combines multiple markdown (.md) files from the Docs directory
- Creates a single output file with proper headers and separators
- Demonstrates advanced WFL programming techniques
- Serves as both a practical tool and a showcase of WFL capabilities

#### Usage
```bash
wfl Tools/wfl_combiner.wfl
```

This will:
- Read all .md files from the `./Docs` directory
- Combine them into `./combined/wfl_docs_combined.md`
- Include proper formatting with file headers and separators

#### Technical Notes
This script demonstrates:
- File discovery using `list files in` with extension filtering
- File I/O operations with `open file at`, `read content from`, `create file at`
- Error handling with `try/when error/end try` blocks
- String concatenation and manipulation
- Loop iteration over file lists
- Working around WFL's variable scoping in loops

The WFL version serves as a practical port of the Python script and showcases WFL's capabilities for real-world file processing tasks.
