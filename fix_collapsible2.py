import re

with open('src/analyzer/mod.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Fix 1: ClearListStatement in src/analyzer/mod.rs (line 1291)
content = re.sub(
    r'Statement::ClearListStatement \{\s*list_name,\s*line,\s*column,\s*\} => \{\s*if self\.get_symbol\(list_name\)\.is_none\(\) \{\s*self\.errors\.push\(SemanticError::new\(\s*format!\("Variable \'\{list_name\}' is not defined"\),\s*\*line,\s*\*column,\s*\)\);\s*\}\s*\}',
    r'Statement::ClearListStatement { list_name, line, column } if self.get_symbol(list_name).is_none() => {\n                self.errors.push(SemanticError::new(\n                    format!("Variable \'{list_name}\' is not defined"),\n                    *line,\n                    *column,\n                ));\n            }\n            Statement::ClearListStatement { .. } => {}',
    content
)

# Fix 2: SignalHandler in src/analyzer/mod.rs (line 1420)
content = re.sub(
    r'Statement::OnSignal \{\s*handler_name,\s*line,\s*column,\s*\} => \{\s*// Check if the handler is defined in the current scope\s*if self\.current_scope\.resolve\(handler_name\)\.is_none\(\) \{\s*self\.errors\.push\(SemanticError::new\(\s*format!\("Undefined signal handler \'\{handler_name\}'"\),\s*\*line,\s*\*column,\s*\)\);\s*\}\s*\}',
    r'Statement::OnSignal { handler_name, line, column } if self.current_scope.resolve(handler_name).is_none() => {\n                self.errors.push(SemanticError::new(\n                    format!("Undefined signal handler \'{handler_name}\'"),\n                    *line,\n                    *column,\n                ));\n            }\n            Statement::OnSignal { .. } => {}',
    content
)

with open('src/analyzer/mod.rs', 'w', encoding='utf-8') as f:
    f.write(content)
