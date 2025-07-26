# WFL Editor Design Document

## Overview

This document outlines the design and implementation strategy for a nimble, efficient WFL code editor built in Rust for Windows. The editor will provide syntax highlighting, auto-completion, and integration with the WFL language server.

## Technology Stack

### GUI Framework: egui

After extensive research, **egui** is recommended for the following reasons:

- **Performance**: Only 30MB memory usage with minimal CPU overhead
- **Bundle Size**: Small native binary without WebView overhead
- **Text Editing**: Existing `egui_code_editor` crate with syntax highlighting
- **Development Speed**: Immediate mode GUI is simple and quick to iterate
- **Active Ecosystem**: Well-maintained with regular updates

### Alternative: Tauri

If richer features are needed (advanced IntelliSense, debugging UI), **Tauri** is recommended:
- Can integrate Monaco Editor (VS Code's editor)
- Web-based UI with native performance
- Larger memory footprint (50-200MB) but more features

## Architecture

### Core Components

1. **Editor Core** (`wfledit-core`)
   - Text buffer management
   - Undo/redo system
   - File I/O operations
   - WFL syntax definitions

2. **GUI Layer** (`wfledit-gui`)
   - egui-based user interface
   - Syntax highlighting via `egui_code_editor`
   - Menu system and shortcuts
   - File tree sidebar

3. **LSP Integration** (`wfledit-lsp`)
   - Communication with `wfl-lsp`
   - Auto-completion
   - Error diagnostics
   - Go-to-definition

4. **Configuration** (`wfledit-config`)
   - User preferences
   - Theme management
   - Keyboard shortcuts

## Implementation Plan

### Phase 1: Basic Editor (Week 1-2)

```rust
// Cargo.toml dependencies
[dependencies]
egui = "0.24"
eframe = "0.24"
egui_code_editor = "0.2"
syntect = "5.0"  // For advanced syntax highlighting
rfd = "0.12"     // Native file dialogs
```

```rust
// Basic editor structure
use egui_code_editor::{CodeEditor, ColorTheme, Syntax};

pub struct WflEditor {
    code: String,
    file_path: Option<PathBuf>,
    modified: bool,
}

impl WflEditor {
    pub fn ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Menu bar
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open...").clicked() {
                        self.open_file();
                    }
                    if ui.button("Save").clicked() {
                        self.save_file();
                    }
                });
            });
            
            // Code editor
            CodeEditor::default()
                .with_fontsize(14.0)
                .with_theme(ColorTheme::GRUVBOX)
                .with_syntax(self.wfl_syntax())
                .with_numlines(true)
                .show(ui, &mut self.code);
        });
    }
}
```

### Phase 2: WFL Integration (Week 3-4)

1. **Syntax Highlighting**
   ```rust
   fn wfl_syntax() -> Syntax {
       Syntax::new("WFL")
           .with_keywords(vec![
               "store", "as", "display", "check", "if", "then",
               "otherwise", "end", "count", "from", "to", "for",
               "each", "in", "define", "action", "needs", "returns",
               "container", "with", "property", "extending", "async",
               "await", "try", "catch", "throw", "import", "export"
           ])
           .with_types(vec!["text", "number", "boolean", "list", "null"])
           .with_special(vec!["true", "false", "null"])
   }
   ```

2. **Auto-completion**
   - Integration with WFL lexer tokens
   - Context-aware suggestions
   - Snippet support

### Phase 3: LSP Integration (Week 5-6)

```rust
use lsp_types::*;
use tokio::net::TcpStream;

pub struct LspClient {
    connection: TcpStream,
}

impl LspClient {
    pub async fn initialize(&mut self) -> Result<InitializeResult> {
        // Initialize LSP connection
    }
    
    pub async fn get_completions(&mut self, position: Position) -> Vec<CompletionItem> {
        // Request completions from wfl-lsp
    }
}
```

### Phase 4: Advanced Features (Week 7-8)

1. **File Explorer**
   - Tree view of project files
   - Quick file switching (Ctrl+P)
   - Search in files

2. **Error Diagnostics**
   - Real-time error highlighting
   - Error panel with quick fixes
   - Integration with WFL analyzer

3. **Debugging Support**
   - Breakpoint management
   - Variable inspection
   - Step-through debugging

## UI Design

### Layout

```
┌─────────────────────────────────────────────────┐
│ File  Edit  View  Run  Help                     │
├────────────┬────────────────────────────────────┤
│            │ example.wfl                         │
│ Explorer   ├────────────────────────────────────┤
│            │  1 | store message as "Hello"      │
│ ▼ project  │  2 | display message                │
│   main.wfl │  3 |                               │
│   lib.wfl  │  4 | count from 1 to 10:          │
│            │  5 |     display index             │
│            │  6 | end count                     │
│            │                                    │
├────────────┴────────────────────────────────────┤
│ Problems (0)  Output  Terminal                   │
└─────────────────────────────────────────────────┘
```

### Keyboard Shortcuts

- `Ctrl+S` - Save
- `Ctrl+O` - Open
- `Ctrl+N` - New file
- `Ctrl+P` - Quick open
- `Ctrl+Shift+P` - Command palette
- `Ctrl+Space` - Trigger completion
- `F12` - Go to definition
- `Shift+F12` - Find references
- `Ctrl+/` - Toggle comment
- `Alt+Up/Down` - Move line

## Performance Targets

- **Startup Time**: < 100ms
- **Memory Usage**: < 50MB for typical projects
- **File Open**: < 50ms for files under 1MB
- **Syntax Highlighting**: Real-time with no perceptible lag
- **Auto-completion**: < 100ms response time

## Build Configuration

```toml
# Cargo.toml
[package]
name = "wfledit"
version = "0.1.0"
edition = "2021"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true

[target.'cfg(windows)'.build-dependencies]
winres = "0.1"

[package.metadata.winres]
ProductName = "WFL Editor"
FileDescription = "Lightweight code editor for WFL"
```

## Distribution

1. **Standalone Binary**
   - Single .exe file
   - No installer required
   - < 10MB size

2. **MSI Installer** (Optional)
   - Windows installer
   - Start menu integration
   - File association (.wfl files)

## Testing Strategy

1. **Unit Tests**
   - Text buffer operations
   - Syntax highlighting rules
   - File I/O

2. **Integration Tests**
   - LSP communication
   - End-to-end editing scenarios
   - Performance benchmarks

3. **Manual Testing**
   - Large file handling
   - Unicode support
   - Accessibility features

## Future Enhancements

1. **Plugin System**
   - Lua or Rhai scripting
   - Custom themes
   - Language extensions

2. **Collaboration Features**
   - Live share support
   - Git integration
   - Diff viewer

3. **AI Integration**
   - Code completion via AI
   - Natural language to WFL
   - Code explanation

## Conclusion

This design provides a solid foundation for a nimble, efficient WFL editor that balances simplicity with essential features. The egui-based approach ensures excellent performance and a small footprint, while the modular architecture allows for future expansion.

The editor will serve as both a learning tool for WFL beginners and a productive environment for experienced users, maintaining the language's philosophy of accessibility and simplicity.