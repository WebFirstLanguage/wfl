with open('src/analyzer/mod.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Fix 1: ClearListStatement in src/analyzer/mod.rs
to_replace1 = """            Statement::ClearListStatement {
                list_name,
                line,
                column,
            } => {
                if self.get_symbol(list_name).is_none() {
                    self.errors.push(SemanticError::new(
                        format!("Variable '{list_name}' is not defined"),
                        *line,
                        *column,
                    ));
                }
            }"""

replacement1 = """            Statement::ClearListStatement { list_name, line, column } => {
                if self.get_symbol(list_name).is_none() {
                    self.errors.push(SemanticError::new(
                        format!("Variable '{list_name}' is not defined"),
                        *line,
                        *column,
                    ));
                }
            }"""

content = content.replace(replacement1, to_replace1)

# Fix 2: SignalHandler in src/analyzer/mod.rs
to_replace2 = """            Statement::OnSignal {
                handler_name,
                line,
                column,
            } => {
                // Check if the handler is defined in the current scope
                if self.current_scope.resolve(handler_name).is_none() {
                    self.errors.push(SemanticError::new(
                        format!("Undefined signal handler '{handler_name}'"),
                        *line,
                        *column,
                    ));
                }
            }"""

replacement2 = """            Statement::OnSignal { handler_name, line, column } => {
                if self.current_scope.resolve(handler_name).is_none() {
                    self.errors.push(SemanticError::new(
                        format!("Undefined signal handler '{handler_name}'"),
                        *line,
                        *column,
                    ));
                }
            }"""

content = content.replace(replacement2, to_replace2)

with open('src/analyzer/mod.rs', 'w', encoding='utf-8') as f:
    f.write(content)
