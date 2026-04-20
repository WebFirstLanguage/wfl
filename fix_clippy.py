import re

with open('src/analyzer/mod.rs', 'r') as f:
    analyzer_content = f.read()

analyzer_content = re.sub(
    r'Statement::ClearListStatement \{\s*list_name,\s*line,\s*column,\s*\}\s*=>\s*\{\s*if self\.get_symbol\(list_name\)\.is_none\(\)\s*\{([^}]+)\}\s*\}',
    r'Statement::ClearListStatement { list_name, line, column } if self.get_symbol(list_name).is_none() => {\1}',
    analyzer_content
)

analyzer_content = re.sub(
    r'Statement::ConnectSignal \{\s*signal_name: _,\s*handler_name,\s*line,\s*column,\s*\}\s*=>\s*\{\s*// Check if the handler is defined in the current scope\s*if self\.current_scope\.resolve\(handler_name\)\.is_none\(\)\s*\{([^}]+)\}\s*\}',
    r'Statement::ConnectSignal { signal_name: _, handler_name, line, column } if self.current_scope.resolve(handler_name).is_none() => {\1}',
    analyzer_content
)

with open('src/analyzer/mod.rs', 'w') as f:
    f.write(analyzer_content)


with open('src/fixer/mod.rs', 'r') as f:
    fixer_content = f.read()

fixer_content = re.sub(
    r'Expression::Literal\(Literal::String\(s\), \.\.\)\s*=>\s*\{\s*if &\*\*s == "\\n"\s*\{\s*1\s*\}\s*else\s*\{\s*0\s*\}\s*\}',
    r'Expression::Literal(Literal::String(s), ..) if &**s == "\\n" => { 1 } Expression::Literal(Literal::String(_), ..) => { 0 }',
    fixer_content
)

with open('src/fixer/mod.rs', 'w') as f:
    f.write(fixer_content)


with open('src/linter/mod.rs', 'r') as f:
    linter_content = f.read()

linter_content = re.sub(
    r'Statement::FunctionDefinition \{\s*name,\s*\.\.\s*\}\s*=>\s*\{\s*if !is_snake_case\(name\)\s*\{([^\}]+)\}\s*\}',
    r'Statement::FunctionDefinition { name, .. } if !is_snake_case(name) => {\1}',
    linter_content
)

linter_content = re.sub(
    r'Statement::VariableDeclaration \{\s*name,\s*\.\.\s*\}\s*=>\s*\{\s*if !is_snake_case\(name\)\s*\{([^\}]+)\}\s*\}',
    r'Statement::VariableDeclaration { name, .. } if !is_snake_case(name) => {\1}',
    linter_content
)

with open('src/linter/mod.rs', 'w') as f:
    f.write(linter_content)
