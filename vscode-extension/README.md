# WFL Extension for Visual Studio Code

VSCode extension providing syntax highlighting and formatting support for the WebFirst Language (WFL).

## Features

### Syntax Highlighting

The extension provides comprehensive syntax highlighting for WFL files:

- Keywords and control structures
- Operators and expressions
- String and numeric literals
- Functions/actions and their parameters
- Variables and their scopes
- I/O operations
- Comments (line and block)
- Block structures with proper nesting

### Code Formatting

The extension provides two formatting options:

1. **Built-in Formatter**: Always available, works without WFL installed
   - Handles indentation and basic code structure
   - Formats operators with proper spacing
   - Preserves comments and blank lines

2. **WFL-based Formatter**: Enhanced formatting using the WFL CLI (when available)
   - Leverages the full power of WFL's linter and fixer
   - Provides comprehensive formatting according to WFL style guidelines
   - Configurable options for indentation, line length, etc.

### Language Server Protocol (LSP) Integration

When the WFL Language Server is available:

- Real-time diagnostics and error checking
- Code completion suggestions
- Hover information
- Go to definition
- Find references
- Document symbols
- Advanced semantic highlighting

## Requirements

- VSCode 1.80.0 or newer

**Optional (for enhanced features):**
- WFL CLI (`wfl`) for enhanced formatting
- WFL Language Server (`wfl-lsp`) for IDE features

## Extension Settings

This extension contributes the following settings:

### Language Server Settings

* `wfl.serverPath`: Path to the WFL Language Server executable
* `wfl.serverArgs`: Arguments to pass to the WFL Language Server
* `wfl.versionMode`: Version compatibility handling (`warn`, `block`, or `ignore`)

### Formatting Settings

* `wfl.format.enable`: Enable/disable WFL code formatting
* `wfl.format.indentSize`: Number of spaces for indentation
* `wfl.format.maxLineLength`: Maximum allowed line length
* `wfl.format.formatOnSave`: Automatically format on save
* `wfl.format.formatOnType`: Format while typing
* `wfl.format.provider`: Formatter to use (`auto`, `builtin`, or `wfl`)

### WFL CLI Settings

* `wfl.cli.path`: Path to the WFL CLI executable
* `wfl.cli.autoDetect`: Automatically detect WFL CLI location

## Commands

This extension provides the following commands:

* `WFL: Restart Language Server`: Restart the WFL Language Server
* `WFL: Select LSP Executable...`: Choose a custom WFL Language Server executable
* `WFL: Format Document`: Format the current WFL document

## Operation Modes

The extension operates in two modes depending on the environment:

### Independent Mode (WFL not installed)

In this mode:
- Syntax highlighting works through TextMate grammar
- Built-in formatter provides basic formatting
- No LSP features available

### Enhanced Mode (WFL installed)

When WFL is detected:
- Full LSP integration provides advanced IDE features
- WFL CLI-based formatter provides enhanced formatting
- All commands are available

The extension automatically detects WFL availability and adapts accordingly.

## Working with WFL Files

### Creating New Files

1. Create a new file with a `.wfl` extension
2. The extension will automatically activate syntax highlighting
3. Use the built-in or WFL-based formatter to format your code

### Opening Existing Files

1. Open a `.wfl` file in VSCode
2. The extension automatically activates for this file
3. If WFL and the Language Server are available, you'll get enhanced features

## Troubleshooting

### Formatting Issues

If formatting does not work as expected:

1. Check that formatting is enabled (`wfl.format.enable`)
2. Verify the formatter provider setting (`wfl.format.provider`)
3. If using the WFL-based formatter, ensure WFL CLI is properly installed

### LSP Issues

If LSP features do not work:

1. Check that the WFL Language Server is installed
2. Verify the server path setting (`wfl.serverPath`)
3. Try restarting the Language Server with the command `WFL: Restart Language Server`
4. Check the WFL output channel for error messages

### WFL Detection Issues

If WFL is installed but not detected:

1. Set `wfl.cli.autoDetect` to `false`
2. Manually specify the path to WFL CLI in `wfl.cli.path`

## Development checks

From the repository root, build the real language server before running the
extension suite:

```sh
cargo build --locked -p wfl-lsp
cd vscode-extension
npm ci
npm test
```

`npm test` runs the dependency security regression, TypeScript compilation,
ESLint, and VS Code host tests. On headless Linux, use `xvfb-run -a npm test`
with Xvfb and the VS Code system libraries installed. CI runs these checks in
the required **Build, Test, Clippy** job.

The host tests download VS Code stable and use a fresh workspace and editor
profile under `target/test-artifacts/vscode-extension/`. They require the built
`target/debug/wfl-lsp` executable (`wfl-lsp.exe` on Windows); set `WFL_LSP_PATH`
to select another local build. Set `VSCODE_TEST_VERSION` to test a specific
VS Code release. Missing servers, missing diagnostics, and skipped tests fail
the suite.

For the bounded YAML dependency check alone, run `npm run test:security` after
`npm ci`.

## Release Notes

### 0.1.0

- Initial release
- Syntax highlighting for WFL
- Built-in and WFL-based formatting
- LSP integration when available
- Automatic WFL detection
