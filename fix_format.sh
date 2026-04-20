#!/bin/bash
sed -i 's/find_symbol_at_position(&program, line, character).map(|symbol_info| format_hover_info(&symbol_info))/find_symbol_at_position(\&program, line, character)\n                .map(|symbol_info| format_hover_info(\&symbol_info))/' wfl-lsp/tests/lsp_hover_test.rs
