# WFL Language Server Protocol (LSP)

A Language Server Protocol implementation for the WebFirst Language (WFL), providing rich IDE features for WFL development.

## Features

- **Real-time Diagnostics**: Syntax and semantic error detection
- **Code Completion**: Intelligent suggestions for variables, functions, and keywords
- **Hover Information**: Detailed symbol information and documentation
- **Standard Library Integration**: Complete WFL standard library support
- **Performance Optimized**: Fast response times for typical usage
- **Cross-Platform**: Works on Windows, macOS, and Linux

## Quick Start

### Build from Source

```bash
# From the WFL project root
cargo build --release --package wfl-lsp

# The binary will be at target/release/wfl-lsp
```

### Run the LSP Server

```bash
# Start in stdio mode (default)
./target/release/wfl-lsp --stdio

# Start with debug logging
./target/release/wfl-lsp --log-level debug

# Start on TCP port (for remote development)
./target/release/wfl-lsp --tcp 8080
```

### VS Code Integration

1. Install the WFL extension from the VS Code marketplace
2. Configure the LSP server path in settings:
   ```json
   {
       "wfl.serverPath": "/path/to/wfl-lsp"
   }
   ```

## Command Line Options

```
wfl-lsp [OPTIONS]

Options:
  --log-level <LEVEL>           Set logging level [default: warn]
                               [possible values: error, warn, info, debug, trace]
  --max-completion-items <NUM>  Maximum completion items [default: 100]
  --hover-timeout <MS>          Hover timeout in milliseconds [default: 1000]
  --stdio                       Use stdio communication (default)
  --tcp <PORT>                  Use TCP on specified port
  --version                     Show version information
  --help                        Show help message
```

## LSP Features

### Code Completion

- **Variables**: Suggests declared variables in scope
- **Functions**: Shows user-defined functions with signatures
- **Keywords**: WFL language keywords and control structures
- **Standard Library**: 30+ built-in functions with documentation
- **Context-Aware**: Different suggestions based on cursor position

### Hover Information

- **Variables**: Shows type, value, and scope information
- **Functions**: Displays function signatures and parameters
- **Keywords**: Provides documentation for WFL keywords
- **Standard Library**: Shows function signatures and descriptions

### Diagnostics

- **Syntax Errors**: Real-time syntax error detection
- **Semantic Errors**: Undefined variables and invalid operations
- **Type Errors**: Type mismatch detection and reporting
- **Position Accurate**: Precise error location highlighting

## Architecture

```
Editor (VS Code, Vim, etc.)
    ↕ LSP Protocol (JSON-RPC)
WFL Language Server
    ↕ Direct Integration
WFL Compiler Components
    ├── Lexer
    ├── Parser
    ├── Analyzer
    └── Type Checker
```

## Testing

```bash
# Run all LSP tests
cargo test --package wfl-lsp

# Run specific test suites
cargo test --package wfl-lsp --test lsp_completion_test
cargo test --package wfl-lsp --test lsp_hover_test
cargo test --package wfl-lsp --test lsp_diagnostics_test
cargo test --package wfl-lsp --test lsp_performance_stability_test

# Run with output
cargo test --package wfl-lsp -- --nocapture
```

## Performance

- **Startup Time**: < 100ms
- **Completion Response**: < 50ms for typical files
- **Hover Response**: < 100ms for complex expressions
- **Memory Usage**: ~10MB base + ~1MB per file

## Editor Support

### VS Code
- Full integration via WFL extension
- Auto-detection of LSP server
- Configuration through VS Code settings

### Vim/Neovim
```vim
" With coc.nvim
{
    "languageserver": {
        "wfl": {
            "command": "wfl-lsp",
            "args": ["--stdio"],
            "filetypes": ["wfl"]
        }
    }
}
```

### Emacs
```elisp
;; With lsp-mode
(lsp-register-client
 (make-lsp-client :new-connection (lsp-stdio-connection "wfl-lsp")
                  :major-modes '(wfl-mode)
                  :server-id 'wfl-lsp))
```

## Configuration

### VS Code Settings

```json
{
    "wfl.serverPath": "/usr/local/bin/wfl-lsp",
    "wfl.serverArgs": [
        "--log-level", "info",
        "--max-completion-items", "50"
    ]
}
```

### Performance Tuning

For large projects:
```json
{
    "wfl.serverArgs": [
        "--max-completion-items", "25",
        "--hover-timeout", "500"
    ]
}
```

## Troubleshooting

### Common Issues

1. **LSP not starting**: Check that `wfl-lsp` is in PATH or correctly configured
2. **No completions**: Ensure file is saved and has valid WFL syntax
3. **Slow performance**: Reduce `--max-completion-items` setting
4. **Missing diagnostics**: Check WFL output channel for errors

### Debug Mode

Enable debug logging:
```json
{
    "wfl.serverArgs": ["--log-level", "debug"]
}
```

## Documentation

- **[Complete LSP Guide](../Docs/guides/wfl-lsp-guide.md)** - Comprehensive usage guide
- **[Quick Reference](../Docs/guides/wfl-lsp-quick-reference.md)** - Quick reference for features
- **[Architecture Documentation](../Docs/technical/wfl-lsp-architecture.md)** - Technical implementation details

## Contributing

1. **Development Setup**:
   ```bash
   git clone https://github.com/WebFirstLanguage/wfl.git
   cd wfl
   cargo build --package wfl-lsp
   ```

2. **Run Tests**:
   ```bash
   cargo test --package wfl-lsp
   ```

3. **Add Features**: Implement new LSP handlers in `src/lib.rs`
4. **Add Tests**: Create comprehensive tests in `tests/`
5. **Update Documentation**: Update guides and README files

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../LICENSE) for details.

---

For more information, see the [WFL Documentation Index](../Docs/wfl-documentation-index.md).
