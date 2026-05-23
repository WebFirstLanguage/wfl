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

replacement1 = """            Statement::ClearListStatement { list_name, line, column } if self.get_symbol(list_name).is_none() => {
                self.errors.push(SemanticError::new(
                    format!("Variable '{list_name}' is not defined"),
                    *line,
                    *column,
                ));
            }
            Statement::ClearListStatement { .. } => {}"""

content = content.replace(to_replace1, replacement1)

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

replacement2 = """            Statement::OnSignal { handler_name, line, column } if self.current_scope.resolve(handler_name).is_none() => {
                self.errors.push(SemanticError::new(
                    format!("Undefined signal handler '{handler_name}'"),
                    *line,
                    *column,
                ));
            }
            Statement::OnSignal { .. } => {}"""

content = content.replace(to_replace2, replacement2)

with open('src/analyzer/mod.rs', 'w', encoding='utf-8') as f:
    f.write(content)

# Fix 3: Fixer in src/fixer/mod.rs
with open('src/fixer/mod.rs', 'r', encoding='utf-8') as f:
    content = f.read()

to_replace3 = """            Expression::Literal(Literal::String(s), ..) => {
                if &**s == "\\n" {
                    1
                } else {
                    0
                }
            }"""

replacement3 = """            Expression::Literal(Literal::String(s), ..) if &**s == "\\n" => 1,
            Expression::Literal(Literal::String(_), ..) => 0,"""

content = content.replace(to_replace3, replacement3)

with open('src/fixer/mod.rs', 'w', encoding='utf-8') as f:
    f.write(content)

# Fix 4: Linter in src/linter/mod.rs
with open('src/linter/mod.rs', 'r', encoding='utf-8') as f:
    content = f.read()

to_replace4 = """                Statement::FunctionDefinition { name, .. } => {
                    if !is_snake_case(name) {
                        let snake_case_name = to_snake_case(name);
                        let diagnostic = WflDiagnostic::new(
                            Severity::Warning,
                            format!(
                                "Function name '{name}' should be snake_case. Consider using '{snake_case_name}'."
                            ),
                            statement.line(),
                            statement.column(),
                            Some("NAMING".to_string()),
                        );
                        diagnostics.push(diagnostic);
                    }
                }"""

replacement4 = """                Statement::FunctionDefinition { name, .. } if !is_snake_case(name) => {
                    let snake_case_name = to_snake_case(name);
                    let diagnostic = WflDiagnostic::new(
                        Severity::Warning,
                        format!(
                            "Function name '{name}' should be snake_case. Consider using '{snake_case_name}'."
                        ),
                        statement.line(),
                        statement.column(),
                        Some("NAMING".to_string()),
                    );
                    diagnostics.push(diagnostic);
                }
                Statement::FunctionDefinition { .. } => {}"""

content = content.replace(to_replace4, replacement4)

to_replace5 = """                Statement::ActionDefinition { name, .. } => {
                    if !is_snake_case(name) {
                        let snake_case_name = to_snake_case(name);
                        let diagnostic = WflDiagnostic::new(
                            Severity::Warning,
                            format!(
                                "Action name '{name}' should be snake_case. Consider using '{snake_case_name}'."
                            ),
                            statement.line(),
                            statement.column(),
                            Some("NAMING".to_string()),
                        );
                        diagnostics.push(diagnostic);
                    }
                }"""

replacement5 = """                Statement::ActionDefinition { name, .. } if !is_snake_case(name) => {
                    let snake_case_name = to_snake_case(name);
                    let diagnostic = WflDiagnostic::new(
                        Severity::Warning,
                        format!(
                            "Action name '{name}' should be snake_case. Consider using '{snake_case_name}'."
                        ),
                        statement.line(),
                        statement.column(),
                        Some("NAMING".to_string()),
                    );
                    diagnostics.push(diagnostic);
                }
                Statement::ActionDefinition { .. } => {}"""

content = content.replace(to_replace5, replacement5)

with open('src/linter/mod.rs', 'w', encoding='utf-8') as f:
    f.write(content)
