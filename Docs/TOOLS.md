# WFL Tools Documentation

This document describes the various tools and utilities available for the Web First Language (WFL) project.

## File Combiner (WFL version)

The WFL File Combiner is a command-line tool that merges Markdown ("Docs") and/or Rust ("src") files into consolidated `.md` and `.txt` deliverables. This is a WFL port of the original Python utility `wfl_md_combiner.py`.

### Usage

```bash
./combine_files.wfl [OPTIONS]
```

### Current Implementation Status

**Note**: The current WFL implementation is a basic proof-of-concept that demonstrates the core file combination logic using only supported WFL syntax. Full functionality requires parser support for stdlib functions with the `function of argument` syntax pattern.

The current version:
- ✅ Demonstrates proper file combination structure
- ✅ Generates table of contents with proper formatting
- ✅ Creates file headers and separators
- ✅ Adds start/end banners for each file
- ✅ Supports basic string concatenation and counter logic
- ❌ Does not yet support actual file I/O operations
- ❌ Does not yet support command-line argument parsing
- ❌ Uses hardcoded content instead of reading actual files

### Planned Command-Line Flags

When fully implemented, the WFL combiner will support the following flags (matching the Python version):

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--type` | choice | `docs` | File type to process: `docs`, `src`, or `both` |
| `--output` | string | `combined` | Output directory base name |
| `--sort` | choice | `alpha` | Sort method: `alpha`, `time`, or custom list |
| `--recursive` | boolean | `yes` | Search directories recursively |
| `--no-toc` | boolean | `no` | Disable table of contents generation |
| `--header-level` | number | `2` | Header level for file sections (1-6) |
| `--separator` | string | `---` | Separator between files |
| `--all-files` | boolean | `no` | Include all files (not just `wfl-` prefixed) |
| `--no-txt` | boolean | `no` | Skip generating `.txt` output files |
| `--help` | boolean | `no` | Show usage information |

### Migration Notes from Python Version

When the WFL version is fully implemented, users migrating from the Python version should note:

#### Key Differences
- **Interactive prompt removed**: Defaults to `--type docs` instead of prompting
- **Path normalization**: All paths use `/` separators for consistent TOC anchors
- **Streaming writes**: Handles large repositories without memory issues
- **Symlink behavior**: Does not follow symlinks (matches Python default)
- **Separator control**: `--separator ""` disables separators entirely (handy when embedding in other docs)

#### CI Pipeline Migration
If you previously relied on the Python combiner prompting for type, add `--type both` (or `docs`, `src`) explicitly to avoid CI pipeline stalls.

**Before (Python version):**
```bash
python3 Tools/wfl_md_combiner.py  # Would prompt for type
```

**After (WFL version):**
```bash
./combine_files.wfl --type both  # Explicit type specification
```

### Output Structure

The combiner generates files in the following structure:

```
combined/
├── docs.md     # Combined documentation files
├── docs.txt    # Plain text version (unless --no-txt)
├── src.md      # Combined source files (if --type src or both)
└── src.txt     # Plain text version (unless --no-txt)
```

### File Processing Logic

1. **Discovery**: Finds files matching the specified type and extension
2. **Filtering**: Applies `wfl-` prefix filter for docs (unless `--all-files`)
3. **Sorting**: Orders files by alpha, time, or custom list
4. **TOC Generation**: Creates numbered table of contents (unless `--no-toc`)
5. **Content Processing**: 
   - Adds file headers with configurable level
   - Wraps source files in appropriate code fences
   - Includes start/end banners for each file
   - Inserts separators between files
6. **Output**: Writes both Markdown and plain text versions

### Testing

The WFL combiner includes a comprehensive test suite in `test_combiner.wfl`:

```bash
./test_combiner.wfl
```

The test suite validates:
- String concatenation functionality
- File counter logic
- Header generation
- Banner generation  
- Table of contents generation

### Development Status

The WFL File Combiner is currently in development. The basic implementation demonstrates the intended functionality, but full feature parity with the Python version requires:

1. **Parser Enhancement**: Support for `function of argument` syntax
2. **Stdlib Completion**: Full implementation of file I/O, CLI parsing, and path utilities
3. **Integration Testing**: Byte-for-byte comparison with Python version outputs

### Future Enhancements

Planned improvements include:
- Performance optimization for large repositories
- Cross-platform path handling
- Enhanced error reporting
- Configuration file support
- Plugin system for custom processors

## Other Tools

### Python File Combiner (Legacy)

The original Python implementation is available at `Tools/wfl_md_combiner.py`. This version provides full functionality and serves as the reference implementation for the WFL port.

For detailed usage of the Python version, run:
```bash
python3 Tools/wfl_md_combiner.py --help
```

---

*This documentation is part of the WFL project. For more information about WFL, see the main project documentation.*
