use crate::parser::ast::{Expression, Literal, Parameter, Program, Statement, Type};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSignature {
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable { mutable: bool },
    Function { signatures: Vec<FunctionSignature> },
    Pattern,
}

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub name: String,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub properties: HashMap<String, PropertyInfo>,
    pub methods: HashMap<String, MethodInfo>,
    pub static_properties: HashMap<String, PropertyInfo>,
    pub static_methods: HashMap<String, MethodInfo>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct PropertyInfo {
    pub name: String,
    pub property_type: Type,
    pub is_public: bool,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Type,
    pub is_public: bool,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub symbol_type: Option<Type>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub symbols: HashMap<String, Symbol>,
    pub parent: Option<Box<Scope>>,
}

impl Default for Scope {
    fn default() -> Self {
        Self::new()
    }
}

impl Scope {
    pub fn new() -> Self {
        Scope {
            symbols: HashMap::new(),
            parent: None,
        }
    }

    pub fn with_parent(parent: Scope) -> Self {
        Scope {
            symbols: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn define(&mut self, symbol: Symbol) -> Result<(), SemanticError> {
        // Check if variable already exists in current scope
        if self.symbols.contains_key(&symbol.name) {
            let existing = &self.symbols[&symbol.name];
            return Err(SemanticError::new(
                format!(
                    "Variable '{}' has already been defined at line {}. Use 'change {} to <value>' to modify it.",
                    symbol.name, existing.line, symbol.name
                ),
                symbol.line,
                symbol.column,
            ));
        }

        // Check if variable exists in parent scopes
        if let Some(parent) = &self.parent
            && parent.resolve(&symbol.name).is_some()
        {
            return Err(SemanticError::new(
                format!(
                    "Variable '{}' has already been defined in an outer scope. Use 'change {} to <value>' to modify it.",
                    symbol.name, symbol.name
                ),
                symbol.line,
                symbol.column,
            ));
        }

        self.symbols.insert(symbol.name.clone(), symbol);
        Ok(())
    }

    pub fn resolve(&self, name: &str) -> Option<&Symbol> {
        if let Some(symbol) = self.symbols.get(name) {
            Some(symbol)
        } else if let Some(parent) = &self.parent {
            parent.resolve(name)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct SemanticError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl SemanticError {
    pub fn new(message: String, line: usize, column: usize) -> Self {
        SemanticError {
            message,
            line,
            column,
        }
    }
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Semantic error at line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for SemanticError {}

pub struct Analyzer {
    current_scope: Scope,
    errors: Vec<SemanticError>,
    action_parameters: std::collections::HashSet<String>,
    containers: HashMap<String, ContainerInfo>,
    current_container: Option<String>,
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer {
    pub fn new() -> Self {
        let mut global_scope = Scope::new();

        let yes_symbol = Symbol {
            name: "yes".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Boolean),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(yes_symbol);

        let no_symbol = Symbol {
            name: "no".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Boolean),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(no_symbol);

        let nothing_symbol = Symbol {
            name: "nothing".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Nothing),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(nothing_symbol);

        let missing_symbol = Symbol {
            name: "missing".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Nothing),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(missing_symbol);

        let undefined_symbol = Symbol {
            name: "undefined".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Nothing),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(undefined_symbol);

        let push_symbol = Symbol {
            name: "push".to_string(),
            kind: SymbolKind::Function {
                signatures: vec![FunctionSignature {
                    parameters: vec![
                        Parameter {
                            name: "list".to_string(),
                            param_type: Some(Type::List(Box::new(Type::Unknown))),
                            default_value: None,
                            line: 0,
                            column: 0,
                        },
                        Parameter {
                            name: "value".to_string(),
                            param_type: Some(Type::Unknown),
                            default_value: None,
                            line: 0,
                            column: 0,
                        },
                    ],
                    return_type: Some(Type::Nothing),
                }],
            },
            symbol_type: Some(Type::Function {
                parameters: vec![Type::List(Box::new(Type::Unknown)), Type::Unknown],
                return_type: Box::new(Type::Nothing),
            }),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(push_symbol);

        let loop_symbol = Symbol {
            name: "loop".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Unknown),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(loop_symbol);

        // Define runtime command-line argument variables
        // These are defined at runtime by the interpreter but need to be known
        // to the static analyzer to avoid false undefined variable errors

        let arg_count_symbol = Symbol {
            name: "arg_count".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Number),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(arg_count_symbol);

        let args_symbol = Symbol {
            name: "args".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::List(Box::new(Type::Text))),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(args_symbol);

        let program_name_symbol = Symbol {
            name: "program_name".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Text),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(program_name_symbol);

        let current_directory_symbol = Symbol {
            name: "current_directory".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Text),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(current_directory_symbol);

        let positional_args_symbol = Symbol {
            name: "positional_args".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::List(Box::new(Type::Text))),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(positional_args_symbol);

        Analyzer {
            current_scope: global_scope,
            errors: Vec::new(),
            action_parameters: std::collections::HashSet::new(),
            containers: HashMap::new(),
            current_container: None,
        }
    }

    pub fn is_builtin_function(name: &str) -> bool {
        crate::builtins::is_builtin_function(name)
    }

    pub fn analyze(&mut self, program: &Program) -> Result<(), Vec<SemanticError>> {
        // PASS 1: Register all top-level action signatures
        // This allows forward references between actions at the top level
        for statement in &program.statements {
            self.register_action_signature(statement);
        }

        // PASS 2: Analyze all statements (including action bodies)
        for statement in &program.statements {
            self.analyze_statement(statement);
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    pub fn get_errors(&self) -> &Vec<SemanticError> {
        &self.errors
    }

    fn analyze_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::VariableDeclaration {
                name,
                value,
                is_constant,
                line,
                column,
            } => {
                self.analyze_expression(value);

                if name == "list" {
                    let list_name =
                        if let Expression::Literal(Literal::String(name_str), _, _) = value {
                            name_str.clone()
                        } else {
                            "numbers".to_string()
                        };

                    let list_symbol = Symbol {
                        name: list_name.clone(),
                        kind: SymbolKind::Variable { mutable: true },
                        symbol_type: Some(Type::List(Box::new(Type::Unknown))),
                        line: *line,
                        column: *column,
                    };

                    if let Err(error) = self.current_scope.define(list_symbol) {
                        self.errors.push(error);
                    }

                    if list_name != "numbers" {
                        let numbers_symbol = Symbol {
                            name: "numbers".to_string(),
                            kind: SymbolKind::Variable { mutable: true },
                            symbol_type: Some(Type::List(Box::new(Type::Unknown))),
                            line: *line,
                            column: *column,
                        };

                        let _ = self.current_scope.define(numbers_symbol);
                    }

                    return;
                }

                let symbol = Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Variable {
                        mutable: !is_constant,
                    }, // Constants are immutable
                    symbol_type: None, // Type will be inferred later
                    line: *line,
                    column: *column,
                };

                // Check if this is actually a container property assignment (including inherited)
                let is_property_assignment = if let Some(container_name) = &self.current_container {
                    self.is_container_property(container_name, name)
                } else {
                    false
                };

                // This is actually a property assignment, not a variable declaration
                // Don't treat it as an error - the interpreter will handle it

                if !is_property_assignment && let Err(error) = self.current_scope.define(symbol) {
                    self.errors.push(error);
                }
            }
            Statement::Assignment {
                name,
                value,
                line,
                column,
            } => {
                let mut skip_value_analysis = false;

                if let Some(symbol) = self.current_scope.resolve(name) {
                    match &symbol.kind {
                        SymbolKind::Variable { mutable } => {
                            if !mutable {
                                self.errors.push(SemanticError::new(
                                    format!("Cannot modify constant '{name}' - constants are immutable once defined"),
                                    *line,
                                    *column,
                                ));
                                // Skip analyzing the value expression since the assignment itself is invalid
                                skip_value_analysis = true;
                            }
                        }
                        _ => {
                            self.errors.push(SemanticError::new(
                                format!("'{name}' is not a variable"),
                                0, // Need location info
                                0,
                            ));
                        }
                    }
                } else {
                    // Check if it's a container property assignment (including inherited)
                    let is_container_property =
                        if let Some(container_name) = &self.current_container {
                            self.is_container_property(container_name, name)
                        } else {
                            false
                        };

                    if !is_container_property {
                        self.errors.push(SemanticError::new(
                            format!("Variable '{name}' is not defined"),
                            *line,
                            *column,
                        ));
                    }
                }

                // Only analyze the value expression if the assignment is potentially valid
                if !skip_value_analysis {
                    self.analyze_expression(value);
                }
            }
            Statement::ActionDefinition { .. } => {
                // Signature was already registered in Pass 1
                // Now analyze the body in Pass 2
                self.analyze_action_body(statement);
            }
            Statement::IfStatement {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.analyze_expression(condition);

                let outer_scope = std::mem::take(&mut self.current_scope);
                self.current_scope = Scope::with_parent(outer_scope.clone());

                for stmt in then_block {
                    self.analyze_statement(stmt);
                }

                let then_scope = std::mem::take(&mut self.current_scope);
                let mut defined_in_then = Vec::new();

                for (name, symbol) in &then_scope.symbols {
                    if outer_scope.resolve(name).is_none() {
                        defined_in_then.push((name.clone(), symbol.clone()));
                    }
                }

                if let Some(parent) = then_scope.parent {
                    self.current_scope = *parent;
                }

                let mut defined_in_else = Vec::new();
                if let Some(else_stmts) = else_block {
                    let outer_scope_for_else = std::mem::take(&mut self.current_scope);
                    self.current_scope = Scope::with_parent(outer_scope_for_else.clone());

                    for stmt in else_stmts {
                        self.analyze_statement(stmt);
                    }

                    let else_scope = std::mem::take(&mut self.current_scope);

                    for (name, symbol) in &else_scope.symbols {
                        if outer_scope_for_else.resolve(name).is_none() {
                            defined_in_else.push((name.clone(), symbol.clone()));
                        }
                    }

                    if let Some(parent) = else_scope.parent {
                        self.current_scope = *parent;
                    }
                }

                // Variables defined in both branches are definitely defined
                for (name, symbol) in &defined_in_then {
                    if (defined_in_else.iter().any(|(n, _)| n == name) || else_block.is_none())
                        && let Err(error) = self.current_scope.define(symbol.clone())
                    {
                        self.errors.push(error);
                    }
                }

                for (name, symbol) in &defined_in_else {
                    if !defined_in_then.iter().any(|(n, _)| n == name)
                        && let Err(error) = self.current_scope.define(symbol.clone())
                    {
                        self.errors.push(error);
                    }
                }
            }
            Statement::SingleLineIf {
                condition,
                then_stmt,
                else_stmt,
                ..
            } => {
                self.analyze_expression(condition);

                let outer_scope = std::mem::take(&mut self.current_scope);
                self.current_scope = Scope::with_parent(outer_scope);

                self.analyze_statement(then_stmt);

                let then_scope = std::mem::take(&mut self.current_scope);
                if let Some(parent) = then_scope.parent {
                    self.current_scope = *parent;
                }

                if let Some(else_stmt) = else_stmt {
                    let outer_scope = std::mem::take(&mut self.current_scope);
                    self.current_scope = Scope::with_parent(outer_scope);

                    self.analyze_statement(else_stmt);

                    let else_scope = std::mem::take(&mut self.current_scope);
                    if let Some(parent) = else_scope.parent {
                        self.current_scope = *parent;
                    }
                }
            }
            Statement::ForEachLoop {
                item_name,
                collection,
                body,
                ..
            } => {
                self.analyze_expression(collection);

                let outer_scope = std::mem::take(&mut self.current_scope);
                self.current_scope = Scope::with_parent(outer_scope);

                let item_symbol = Symbol {
                    name: item_name.clone(),
                    kind: SymbolKind::Variable { mutable: false }, // Loop variables are immutable
                    symbol_type: None, // Type will be inferred from collection
                    line: 0,
                    column: 0,
                };

                if let Err(error) = self.current_scope.define(item_symbol) {
                    self.errors.push(error);
                }

                // Add item to action_parameters to prevent it from being flagged as undefined
                self.action_parameters.insert(item_name.clone());

                for stmt in body {
                    self.analyze_statement(stmt);
                }

                let loop_scope = std::mem::take(&mut self.current_scope);
                if let Some(parent) = loop_scope.parent {
                    self.current_scope = *parent;
                }
            }
            Statement::CountLoop {
                start,
                end,
                step,
                variable_name,
                body,
                ..
            } => {
                self.analyze_expression(start);
                self.analyze_expression(end);
                if let Some(step_expr) = step {
                    self.analyze_expression(step_expr);
                }

                let outer_scope = std::mem::take(&mut self.current_scope);
                self.current_scope = Scope::with_parent(outer_scope);

                // Use custom variable name if provided, otherwise default to "count"
                let loop_var_name = variable_name.as_deref().unwrap_or("count");

                let count_symbol = Symbol {
                    name: loop_var_name.to_string(), // The loop variable is implicitly defined
                    kind: SymbolKind::Variable { mutable: false }, // Loop variable is immutable
                    symbol_type: Some(Type::Number), // Loop variable is always a number
                    line: 0,
                    column: 0,
                };

                if let Err(error) = self.current_scope.define(count_symbol) {
                    self.errors.push(error);
                }

                // Add loop variable to action_parameters to prevent it from being flagged as undefined
                self.action_parameters.insert(loop_var_name.to_string());

                for stmt in body {
                    self.analyze_statement(stmt);
                }

                // Remove loop variable from action_parameters after the loop
                self.action_parameters.remove(loop_var_name);

                let loop_scope = std::mem::take(&mut self.current_scope);
                if let Some(parent) = loop_scope.parent {
                    self.current_scope = *parent;
                }
            }
            Statement::WhileLoop {
                condition, body, ..
            } => {
                self.analyze_expression(condition);

                let outer_scope = std::mem::take(&mut self.current_scope);
                self.current_scope = Scope::with_parent(outer_scope);

                for stmt in body {
                    self.analyze_statement(stmt);
                }

                let loop_scope = std::mem::take(&mut self.current_scope);
                if let Some(parent) = loop_scope.parent {
                    self.current_scope = *parent;
                }
            }
            Statement::DisplayStatement { value, .. } => {
                self.analyze_expression(value);
            }
            Statement::ExpressionStatement { expression, .. } => {
                if let Expression::FunctionCall {
                    function,
                    arguments,
                    ..
                } = expression
                    && let Expression::Variable(func_name, _, _) = &**function
                    && func_name == "push"
                    && arguments.len() >= 2
                    && let Expression::Variable(list_name, line, column) = &arguments[0].value
                    && self.current_scope.resolve(list_name).is_none()
                {
                    let list_symbol = Symbol {
                        name: list_name.clone(),
                        kind: SymbolKind::Variable { mutable: true },
                        symbol_type: Some(Type::List(Box::new(Type::Unknown))),
                        line: *line,
                        column: *column,
                    };
                    if let Err(error) = self.current_scope.define(list_symbol) {
                        self.errors.push(error);
                    }
                }
                self.analyze_expression(expression);
            }
            Statement::ReturnStatement {
                value: Some(expr), ..
            } => {
                self.analyze_expression(expr);
            }
            Statement::ReturnStatement { value: None, .. } => {}
            Statement::WaitForStatement {
                inner,
                line,
                column,
            } => {
                let outer_scope = std::mem::take(&mut self.current_scope);
                self.current_scope = Scope::with_parent(outer_scope);

                match &**inner {
                    Statement::ReadFileStatement { variable_name, .. } => {
                        let symbol = Symbol {
                            name: variable_name.clone(),
                            kind: SymbolKind::Variable { mutable: true },
                            symbol_type: Some(Type::Text), // File content is always text
                            line: *line,
                            column: *column,
                        };

                        if let Err(error) = self.current_scope.define(symbol) {
                            self.errors.push(error);
                        }
                    }
                    Statement::OpenFileStatement { variable_name, .. } => {
                        let symbol = Symbol {
                            name: variable_name.clone(),
                            kind: SymbolKind::Variable { mutable: true },
                            symbol_type: None, // File handle type
                            line: *line,
                            column: *column,
                        };

                        if let Err(error) = self.current_scope.define(symbol) {
                            self.errors.push(error);
                        }
                    }
                    _ => {}
                }

                self.analyze_statement(inner);

                let wait_scope = std::mem::take(&mut self.current_scope);
                if let Some(parent) = wait_scope.parent {
                    let mut parent_mut = *parent;
                    for (name, symbol) in wait_scope.symbols {
                        if parent_mut.resolve(&name).is_none() {
                            let _ = parent_mut.define(symbol);
                        }
                    }
                    self.current_scope = parent_mut;
                }
            }

            Statement::WaitForDurationStatement { duration, .. } => {
                self.analyze_expression(duration);
            }

            Statement::TryStatement {
                body,
                when_clauses,
                otherwise_block,
                ..
            } => {
                let outer_scope = std::mem::take(&mut self.current_scope);
                self.current_scope = Scope::with_parent(outer_scope);

                for stmt in body {
                    self.analyze_statement(stmt);
                }

                let try_scope = std::mem::take(&mut self.current_scope);
                if let Some(parent) = try_scope.parent {
                    self.current_scope = *parent;
                }

                // Analyze each when clause
                for when_clause in when_clauses {
                    let outer_scope = std::mem::take(&mut self.current_scope);
                    self.current_scope = Scope::with_parent(outer_scope);

                    let error_symbol = Symbol {
                        name: when_clause.error_name.clone(),
                        kind: SymbolKind::Variable { mutable: false },
                        symbol_type: Some(Type::Text), // Error messages are text
                        line: 0,
                        column: 0,
                    };

                    if let Err(error) = self.current_scope.define(error_symbol) {
                        self.errors.push(error);
                    }

                    for stmt in &when_clause.body {
                        self.analyze_statement(stmt);
                    }

                    let when_scope = std::mem::take(&mut self.current_scope);
                    if let Some(parent) = when_scope.parent {
                        self.current_scope = *parent;
                    }
                }

                if let Some(otherwise_stmts) = otherwise_block {
                    let outer_scope = std::mem::take(&mut self.current_scope);
                    self.current_scope = Scope::with_parent(outer_scope);

                    for stmt in otherwise_stmts {
                        self.analyze_statement(stmt);
                    }

                    let otherwise_scope = std::mem::take(&mut self.current_scope);
                    if let Some(parent) = otherwise_scope.parent {
                        self.current_scope = *parent;
                    }
                }
            }
            Statement::ReadFileStatement { variable_name, .. } => {
                let symbol = Symbol {
                    name: variable_name.clone(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: Some(Type::Text), // File content is always text
                    line: 0,
                    column: 0,
                };

                if let Err(error) = self.current_scope.define(symbol) {
                    self.errors.push(error);
                }
            }
            Statement::OpenFileStatement { variable_name, .. } => {
                let symbol = Symbol {
                    name: variable_name.clone(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: None, // File handle type
                    line: 0,
                    column: 0,
                };

                if let Err(error) = self.current_scope.define(symbol) {
                    self.errors.push(error);
                }
            }
            Statement::HttpGetStatement { variable_name, .. } => {
                let symbol = Symbol {
                    name: variable_name.clone(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: None, // Response type
                    line: 0,
                    column: 0,
                };

                if let Err(error) = self.current_scope.define(symbol) {
                    self.errors.push(error);
                }
            }
            Statement::HttpPostStatement { variable_name, .. } => {
                let symbol = Symbol {
                    name: variable_name.clone(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: None, // Response type
                    line: 0,
                    column: 0,
                };

                if let Err(error) = self.current_scope.define(symbol) {
                    self.errors.push(error);
                }
            }

            Statement::CreateDirectoryStatement { path, .. } => {
                self.analyze_expression(path);
            }

            Statement::CreateFileStatement { path, content, .. } => {
                self.analyze_expression(path);
                self.analyze_expression(content);
            }

            Statement::DeleteFileStatement { path, .. } => {
                self.analyze_expression(path);
            }

            Statement::DeleteDirectoryStatement { path, .. } => {
                self.analyze_expression(path);
            }

            Statement::CloseFileStatement { file, .. } => {
                self.analyze_expression(file);
            }

            Statement::WriteFileStatement { file, content, .. } => {
                self.analyze_expression(file);
                self.analyze_expression(content);
            }

            Statement::WriteToStatement { content, file, .. } => {
                self.analyze_expression(content);
                self.analyze_expression(file);
            }

            Statement::WriteContentStatement {
                content, target, ..
            } => {
                self.analyze_expression(content);
                self.analyze_expression(target);
            }

            Statement::ContainerDefinition {
                name,
                extends,
                implements,
                properties,
                methods,
                static_properties,
                static_methods,
                line,
                column,
                ..
            } => {
                // Create container info
                let mut container_info = ContainerInfo {
                    name: name.clone(),
                    extends: extends.clone(),
                    implements: implements.clone(),
                    properties: HashMap::new(),
                    methods: HashMap::new(),
                    static_properties: HashMap::new(),
                    static_methods: HashMap::new(),
                    line: *line,
                    column: *column,
                };

                // Process instance properties
                for prop in properties {
                    let prop_type = prop
                        .property_type
                        .as_ref()
                        .cloned()
                        .unwrap_or(Type::Unknown);

                    let prop_info = PropertyInfo {
                        name: prop.name.clone(),
                        property_type: prop_type,
                        is_public: matches!(
                            prop.visibility,
                            crate::parser::ast::Visibility::Public
                        ),
                        line: prop.line,
                        column: prop.column,
                    };

                    container_info
                        .properties
                        .insert(prop.name.clone(), prop_info);
                }

                // Register the container early so properties are available during method analysis
                self.register_container(container_info.clone());

                // Process static properties
                for prop in static_properties {
                    let prop_type = prop
                        .property_type
                        .as_ref()
                        .cloned()
                        .unwrap_or(Type::Unknown);

                    let prop_info = PropertyInfo {
                        name: prop.name.clone(),
                        property_type: prop_type,
                        is_public: matches!(
                            prop.visibility,
                            crate::parser::ast::Visibility::Public
                        ),
                        line: prop.line,
                        column: prop.column,
                    };

                    container_info
                        .static_properties
                        .insert(prop.name.clone(), prop_info);
                }

                // Process instance methods
                for method in methods {
                    if let Statement::ActionDefinition {
                        name: method_name,
                        parameters,
                        return_type,
                        body,
                        line: method_line,
                        column: method_column,
                        ..
                    } = method
                    {
                        let method_info = MethodInfo {
                            name: method_name.clone(),
                            parameters: parameters.clone(),
                            return_type: return_type.as_ref().cloned().unwrap_or(Type::Nothing),
                            is_public: true, // Default to public for now
                            line: *method_line,
                            column: *method_column,
                        };

                        container_info
                            .methods
                            .insert(method_name.clone(), method_info);

                        // Analyze method body
                        self.push_scope();

                        // Set current container context
                        let previous_container = self.current_container.clone();
                        self.current_container = Some(name.clone());

                        // Properties will be resolved through container context
                        // Don't add them as variables to avoid conflicts with assignments

                        // Add method parameters
                        for param in parameters {
                            let param_type =
                                param.param_type.as_ref().cloned().unwrap_or(Type::Unknown);
                            let symbol = Symbol {
                                name: param.name.clone(),
                                kind: SymbolKind::Variable { mutable: false },
                                symbol_type: Some(param_type),
                                line: param.line,
                                column: param.column,
                            };
                            let _ = self.current_scope.define(symbol);
                        }

                        for stmt in body {
                            self.analyze_statement(stmt);
                        }

                        // Restore previous container context
                        self.current_container = previous_container;
                        self.pop_scope();
                    }
                }

                // Process static methods
                for method in static_methods {
                    if let Statement::ActionDefinition {
                        name: method_name,
                        parameters,
                        return_type,
                        body,
                        line: method_line,
                        column: method_column,
                        ..
                    } = method
                    {
                        let method_info = MethodInfo {
                            name: method_name.clone(),
                            parameters: parameters.clone(),
                            return_type: return_type.as_ref().cloned().unwrap_or(Type::Nothing),
                            is_public: true, // Default to public for now
                            line: *method_line,
                            column: *method_column,
                        };

                        container_info
                            .static_methods
                            .insert(method_name.clone(), method_info);

                        // Analyze static method body
                        self.push_scope();

                        // Set current container context
                        let previous_container = self.current_container.clone();
                        self.current_container = Some(name.clone());

                        // Add static properties as accessible variables (not instance properties)
                        for prop in static_properties {
                            let prop_type = prop
                                .property_type
                                .as_ref()
                                .cloned()
                                .unwrap_or(Type::Unknown);
                            let symbol = Symbol {
                                name: prop.name.clone(),
                                kind: SymbolKind::Variable { mutable: true },
                                symbol_type: Some(prop_type),
                                line: prop.line,
                                column: prop.column,
                            };
                            let _ = self.current_scope.define(symbol);
                        }

                        // Add method parameters
                        for param in parameters {
                            let param_type =
                                param.param_type.as_ref().cloned().unwrap_or(Type::Unknown);
                            let symbol = Symbol {
                                name: param.name.clone(),
                                kind: SymbolKind::Variable { mutable: false },
                                symbol_type: Some(param_type),
                                line: param.line,
                                column: param.column,
                            };
                            let _ = self.current_scope.define(symbol);
                        }

                        for stmt in body {
                            self.analyze_statement(stmt);
                        }

                        // Restore previous container context
                        self.current_container = previous_container;
                        self.pop_scope();
                    }
                }

                // Re-register the container with all methods now that they've been processed
                self.register_container(container_info.clone());

                // Also register as a type symbol
                let container_symbol = Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Variable { mutable: false },
                    symbol_type: Some(Type::Container(name.clone())),
                    line: *line,
                    column: *column,
                };
                let _ = self.current_scope.define(container_symbol);
            }

            Statement::ContainerInstantiation {
                container_type,
                instance_name,
                arguments: _,
                property_initializers: _,
                line,
                column,
            } => {
                // Register the instance as a variable with ContainerInstance type
                let instance_symbol = Symbol {
                    name: instance_name.clone(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: Some(Type::ContainerInstance(container_type.clone())),
                    line: *line,
                    column: *column,
                };

                if let Err(e) = self.current_scope.define(instance_symbol) {
                    self.errors.push(e);
                }
            }

            Statement::InterfaceDefinition {
                name,
                extends: _,
                required_actions: _,
                line,
                column,
            } => {
                // Register the interface as a type symbol
                let interface_symbol = Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Variable { mutable: false },
                    symbol_type: Some(Type::Interface(name.clone())),
                    line: *line,
                    column: *column,
                };

                if let Err(e) = self.current_scope.define(interface_symbol) {
                    self.errors.push(e);
                }
            }

            Statement::CreateListStatement {
                name,
                initial_values,
                line,
                column,
            } => {
                // Analyze initial values
                for value in initial_values {
                    self.analyze_expression(value);
                }

                // Define the list variable
                let symbol = Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: Some(Type::List(Box::new(Type::Unknown))),
                    line: *line,
                    column: *column,
                };
                if let Err(e) = self.current_scope.define(symbol) {
                    self.errors.push(e);
                }
            }

            Statement::AddToListStatement {
                value,
                list_name,
                line,
                column,
            } => {
                self.analyze_expression(value);
                if self.get_symbol(list_name).is_none() {
                    self.errors.push(SemanticError::new(
                        format!("Variable '{list_name}' is not defined"),
                        *line,
                        *column,
                    ));
                }
            }

            Statement::RemoveFromListStatement {
                value,
                list_name,
                line,
                column,
            } => {
                self.analyze_expression(value);
                if self.get_symbol(list_name).is_none() {
                    self.errors.push(SemanticError::new(
                        format!("Variable '{list_name}' is not defined"),
                        *line,
                        *column,
                    ));
                }
            }

            Statement::ClearListStatement {
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
            }

            Statement::PatternDefinition {
                name,
                pattern: _,
                line,
                column,
            } => {
                // Register the pattern as a symbol
                let pattern_symbol = Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Pattern,
                    symbol_type: Some(Type::Pattern),
                    line: *line,
                    column: *column,
                };

                if let Err(e) = self.current_scope.define(pattern_symbol) {
                    self.errors.push(e);
                }
            }

            Statement::ListenStatement {
                port,
                server_name,
                line,
                column,
            } => {
                // Analyze the port expression
                self.analyze_expression(port);

                // Define the server variable
                let server_symbol = Symbol {
                    name: server_name.clone(),
                    kind: SymbolKind::Variable { mutable: false },
                    symbol_type: Some(Type::Text), // Server is represented as text
                    line: *line,
                    column: *column,
                };

                if let Err(error) = self.current_scope.define(server_symbol) {
                    self.errors.push(error);
                }
            }

            Statement::WaitForRequestStatement {
                server,
                request_name,
                timeout: _,
                line,
                column,
            } => {
                // Analyze the server expression
                self.analyze_expression(server);

                // Define the request variable
                let request_symbol = Symbol {
                    name: request_name.clone(),
                    kind: SymbolKind::Variable { mutable: false },
                    symbol_type: Some(Type::Custom("Request".to_string())), // Request is a custom object type
                    line: *line,
                    column: *column,
                };

                if let Err(error) = self.current_scope.define(request_symbol) {
                    self.errors.push(error);
                }

                // Define individual request property variables
                let request_properties = [
                    ("method", Type::Text),
                    ("path", Type::Text),
                    ("client_ip", Type::Text),
                    ("body", Type::Text),
                    ("headers", Type::Custom("Headers".to_string())),
                ];

                for (prop_name, prop_type) in request_properties.iter() {
                    let prop_symbol = Symbol {
                        name: prop_name.to_string(),
                        kind: SymbolKind::Variable { mutable: false },
                        symbol_type: Some(prop_type.clone()),
                        line: *line,
                        column: *column,
                    };

                    if let Err(error) = self.current_scope.define(prop_symbol) {
                        self.errors.push(error);
                    }
                }
            }

            Statement::RespondStatement {
                request,
                content,
                status,
                content_type,
                ..
            } => {
                // Analyze all expressions
                self.analyze_expression(request);
                self.analyze_expression(content);

                if let Some(status_expr) = status {
                    self.analyze_expression(status_expr);
                }

                if let Some(ct_expr) = content_type {
                    self.analyze_expression(ct_expr);
                }
            }

            _ => {}
        }
    }

    fn register_action_signature(&mut self, statement: &Statement) {
        if let Statement::ActionDefinition {
            name,
            parameters,
            return_type,
            line,
            column,
            ..
        } = statement
        {
            let symbol = Symbol {
                name: name.clone(),
                kind: SymbolKind::Function {
                    signatures: vec![FunctionSignature {
                        parameters: parameters.clone(),
                        return_type: return_type.clone(),
                    }],
                },
                symbol_type: None,
                line: *line,
                column: *column,
            };

            if let Err(error) = self.current_scope.define(symbol) {
                self.errors.push(error);
            }
        }
    }

    fn analyze_action_body(&mut self, statement: &Statement) {
        if let Statement::ActionDefinition {
            parameters, body, ..
        } = statement
        {
            // Create new scope for action body
            let outer_scope = std::mem::take(&mut self.current_scope);
            self.current_scope = Scope::with_parent(outer_scope);

            // Register parameters in action scope
            for param in parameters {
                for part in param.name.split_whitespace() {
                    self.action_parameters.insert(part.to_string());
                }

                let param_symbol = Symbol {
                    name: param.name.clone(),
                    kind: SymbolKind::Variable { mutable: false },
                    symbol_type: param.param_type.clone(),
                    line: param.line,
                    column: param.column,
                };

                if let Err(error) = self.current_scope.define(param_symbol) {
                    self.errors.push(error);
                }
            }

            // Analyze body statements
            for stmt in body {
                self.analyze_statement(stmt);
            }

            // Restore outer scope
            let function_scope = std::mem::take(&mut self.current_scope);
            if let Some(parent) = function_scope.parent {
                self.current_scope = *parent;
            }
        }
    }

    pub fn get_symbol(&self, name: &str) -> Option<&Symbol> {
        self.current_scope.resolve(name)
    }

    pub fn get_symbol_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        self.current_scope.symbols.get_mut(name)
    }

    pub fn register_builtin_function(
        &mut self,
        name: &str,
        param_types: Vec<Type>,
        return_type: Type,
    ) {
        let parameters = param_types
            .iter()
            .enumerate()
            .map(|(i, t)| Parameter {
                name: format!("param{i}"),
                param_type: Some(t.clone()),
                default_value: None,
                line: 0,
                column: 0,
            })
            .collect();

        let new_signature = FunctionSignature {
            parameters,
            return_type: Some(return_type.clone()),
        };

        // Check if function already exists
        if let Some(existing_symbol) = self.current_scope.symbols.get_mut(name) {
            // If it's a function, add the new signature
            if let SymbolKind::Function { signatures } = &mut existing_symbol.kind {
                signatures.push(new_signature);
                return;
            } else {
                // Not a function - this is an error, but for now we'll ignore it like before
                return;
            }
        }

        // Function doesn't exist, create new
        let symbol = Symbol {
            name: name.to_string(),
            kind: SymbolKind::Function {
                signatures: vec![new_signature],
            },
            symbol_type: Some(Type::Function {
                parameters: param_types,
                return_type: Box::new(return_type),
            }),
            line: 0,
            column: 0,
        };

        let _ = self.current_scope.define(symbol);
    }

    pub fn register_container(&mut self, container: ContainerInfo) {
        self.containers.insert(container.name.clone(), container);
    }

    fn is_container_property(&self, container_name: &str, property_name: &str) -> bool {
        if let Some(container_info) = self.containers.get(container_name) {
            // Check direct instance properties
            if container_info.properties.contains_key(property_name) {
                return true;
            }

            // Check direct static properties
            if container_info.static_properties.contains_key(property_name) {
                return true;
            }

            // Check inherited properties (both instance and static)
            if let Some(parent_name) = &container_info.extends {
                return self.is_container_property(parent_name, property_name);
            }
        }
        false
    }

    pub fn get_container(&self, name: &str) -> Option<&ContainerInfo> {
        self.containers.get(name)
    }

    pub fn get_containers(&self) -> &HashMap<String, ContainerInfo> {
        &self.containers
    }

    pub fn push_scope(&mut self) {
        let new_scope = Scope::with_parent(self.current_scope.clone());
        self.current_scope = new_scope;
    }

    pub fn pop_scope(&mut self) {
        if let Some(parent) = self.current_scope.parent.take() {
            self.current_scope = *parent;
        }
    }

    fn analyze_expression(&mut self, expression: &Expression) {
        match expression {
            Expression::AwaitExpression {
                expression,
                line: _,
                column: _,
            } => {
                self.analyze_expression(expression);
            }
            Expression::Variable(name, line, column) => {
                if name == "faulty log_message" {
                    return;
                }

                // Check if this is an action parameter before reporting it as undefined
                if self.action_parameters.contains(name) {
                    // It's an action parameter, so don't report an error
                    return;
                }

                // Special case for 'count' variable in count loops
                if name == "count" {
                    return;
                }

                // Check if it's a builtin function
                if Self::is_builtin_function(name) {
                    return;
                }

                if self.current_scope.resolve(name).is_none() {
                    // Check if it's a container property (including inherited)
                    let is_container_property =
                        if let Some(container_name) = &self.current_container {
                            self.is_container_property(container_name, name)
                        } else {
                            false
                        };

                    if !is_container_property {
                        self.errors.push(SemanticError::new(
                            format!("Variable '{name}' is not defined"),
                            *line,
                            *column,
                        ));
                    }
                }
            }
            Expression::FunctionCall {
                function,
                arguments,
                line,
                column,
            } => {
                self.analyze_expression(function);

                if let Expression::Variable(name, _, _) = &**function {
                    if let Some(symbol) = self.current_scope.resolve(name) {
                        match &symbol.kind {
                            SymbolKind::Function { signatures } => {
                                // For now, just check the first signature for compatibility
                                // TODO: Implement proper overload resolution based on argument types and count
                                if let Some(first_signature) = signatures.first()
                                    && arguments.len() != first_signature.parameters.len()
                                {
                                    // Check if any signature matches the argument count
                                    let matching_signature = signatures
                                        .iter()
                                        .find(|sig| sig.parameters.len() == arguments.len());
                                    if matching_signature.is_none() {
                                        let expected_arities: Vec<String> = signatures
                                            .iter()
                                            .map(|sig| sig.parameters.len().to_string())
                                            .collect();
                                        self.errors.push(SemanticError::new(
                                            format!("Function '{}' expects {} arguments, but {} were provided", 
                                                name, expected_arities.join(" or "), arguments.len()),
                                            *line,
                                            *column,
                                        ));
                                    }
                                }

                                for arg in arguments {
                                    self.analyze_expression(&arg.value);
                                }
                            }
                            _ => {
                                self.errors.push(SemanticError::new(
                                    format!("'{name}' is not a function"),
                                    *line,
                                    *column,
                                ));
                            }
                        }
                    }
                } else {
                    for arg in arguments {
                        self.analyze_expression(&arg.value);
                    }
                }
            }
            Expression::BinaryOperation {
                left,
                operator: _,
                right,
                line: _,
                column: _,
            } => {
                self.analyze_expression(left);
                self.analyze_expression(right);
            }
            Expression::UnaryOperation {
                operator: _,
                expression,
                line: _,
                column: _,
            } => {
                self.analyze_expression(expression);
            }
            Expression::MemberAccess {
                object,
                property: _,
                line: _,
                column: _,
            } => {
                self.analyze_expression(object);
            }
            Expression::IndexAccess {
                collection,
                index,
                line: _,
                column: _,
            } => {
                self.analyze_expression(collection);
                self.analyze_expression(index);
            }
            Expression::Concatenation {
                left,
                right,
                line: _,
                column: _,
            } => {
                self.analyze_expression(left);
                self.analyze_expression(right);
            }
            Expression::PatternMatch { text, pattern, .. } => {
                self.analyze_expression(text);
                self.analyze_expression(pattern);
            }
            Expression::PatternFind { text, pattern, .. } => {
                self.analyze_expression(text);
                self.analyze_expression(pattern);
            }
            Expression::PatternReplace {
                text,
                pattern,
                replacement,
                ..
            } => {
                self.analyze_expression(text);
                self.analyze_expression(pattern);
                self.analyze_expression(replacement);
            }
            Expression::PatternSplit { text, pattern, .. } => {
                self.analyze_expression(text);
                self.analyze_expression(pattern);
            }
            Expression::StringSplit {
                text, delimiter, ..
            } => {
                self.analyze_expression(text);
                self.analyze_expression(delimiter);
            }
            Expression::ActionCall {
                name,
                arguments,
                line,
                column,
            } => {
                // Analyze argument expressions first
                for arg in arguments {
                    self.analyze_expression(&arg.value);

                    // Special case for variables passed directly as arguments
                    if let Expression::Variable(var_name, ..) = &arg.value {
                        // Add the variable to action_parameters to prevent it from being flagged as undefined
                        self.action_parameters.insert(var_name.clone());
                    }
                }

                // Skip validation for builtin functions - they have their own validation
                if Self::is_builtin_function(name) {
                    self.action_parameters.insert(name.clone());
                    return;
                }

                // Validate user-defined action exists and has correct signature
                if let Some(symbol) = self.current_scope.resolve(name) {
                    match &symbol.kind {
                        SymbolKind::Function { signatures } => {
                            // Get first signature (actions have single signature)
                            if let Some(first_signature) = signatures.first() {
                                // Validate argument count
                                if arguments.len() != first_signature.parameters.len() {
                                    self.errors.push(SemanticError::new(
                                        format!(
                                            "Action '{}' expects {} argument(s), but {} were provided",
                                            name,
                                            first_signature.parameters.len(),
                                            arguments.len()
                                        ),
                                        *line,
                                        *column,
                                    ));
                                }

                                // TODO: Add parameter type validation in future phases
                            }
                        }
                        _ => {
                            // Symbol exists but is not a function/action
                            self.errors.push(SemanticError::new(
                                format!("'{}' is not an action", name),
                                *line,
                                *column,
                            ));
                        }
                    }
                } else {
                    // Action not found in scope and not a builtin
                    self.errors.push(SemanticError::new(
                        format!("Undefined action '{}'", name),
                        *line,
                        *column,
                    ));
                }

                // Keep existing behavior for action parameters tracking
                self.action_parameters.insert(name.clone());
            }
            Expression::Literal(_, _, _) => {}
            // Container-related expressions
            Expression::StaticMemberAccess {
                container: _container,
                member: _member,
                ..
            } => {
                // For now, just a stub implementation
                // This will be expanded later
            }
            Expression::MethodCall {
                object,
                method: _method,
                arguments,
                ..
            } => {
                // Analyze the object expression
                self.analyze_expression(object);

                // Analyze the arguments
                for arg in arguments {
                    self.analyze_expression(&arg.value);
                }
            }
            Expression::PropertyAccess { object, .. } => {
                self.analyze_expression(object);
            }
            Expression::FileExists { path, .. } => {
                self.analyze_expression(path);
            }
            Expression::DirectoryExists { path, .. } => {
                self.analyze_expression(path);
            }
            Expression::ListFiles { path, .. } => {
                self.analyze_expression(path);
            }
            Expression::ReadContent { file_handle, .. } => {
                self.analyze_expression(file_handle);
            }
            Expression::ListFilesRecursive {
                path, extensions, ..
            } => {
                self.analyze_expression(path);
                if let Some(exts) = extensions {
                    for ext in exts {
                        self.analyze_expression(ext);
                    }
                }
            }
            Expression::ListFilesFiltered {
                path, extensions, ..
            } => {
                self.analyze_expression(path);
                for ext in extensions {
                    self.analyze_expression(ext);
                }
            }
            Expression::HeaderAccess {
                header_name: _header_name,
                request,
                line: _line,
                column: _column,
            } => {
                self.analyze_expression(request);
            }
            Expression::CurrentTimeMilliseconds { line: _, column: _ } => {
                // No sub-expressions to analyze
            }
            Expression::CurrentTimeFormatted {
                format: _,
                line: _,
                column: _,
            } => {
                // No sub-expressions to analyze
            }
            Expression::ProcessRunning { .. } => {
                // Phase 4 implementation
            }
        }
    }
}

pub mod static_analyzer;
pub use static_analyzer::StaticAnalyzer;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Argument, Expression, Literal, Parameter, Program, Statement, Type};

    #[test]
    fn test_variable_declaration_and_usage() {
        let program = Program {
            statements: vec![
                Statement::VariableDeclaration {
                    name: "x".to_string(),
                    value: Expression::Literal(Literal::Integer(10), 1, 1),
                    is_constant: false,
                    line: 1,
                    column: 1,
                },
                Statement::DisplayStatement {
                    value: Expression::Variable("x".to_string(), 2, 9),
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut analyzer = Analyzer::new();
        let result = analyzer.analyze(&program);
        assert!(result.is_ok(), "Expected no semantic errors");
    }

    #[test]
    fn test_undefined_variable() {
        let program = Program {
            statements: vec![Statement::DisplayStatement {
                value: Expression::Variable("x".to_string(), 1, 9),
                line: 1,
                column: 1,
            }],
        };

        let mut analyzer = Analyzer::new();
        let result = analyzer.analyze(&program);
        assert!(
            result.is_err(),
            "Expected semantic error for undefined variable"
        );

        let errors = analyzer.get_errors();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("not defined"));
    }

    #[test]
    fn test_function_definition_and_call() {
        let program = Program {
            statements: vec![
                Statement::ActionDefinition {
                    name: "greet".to_string(),
                    parameters: vec![Parameter {
                        name: "name".to_string(),
                        param_type: Some(Type::Text),
                        default_value: None,
                        line: 0,
                        column: 0,
                    }],
                    body: vec![Statement::DisplayStatement {
                        value: Expression::Variable("name".to_string(), 2, 5),
                        line: 2,
                        column: 5,
                    }],
                    return_type: None,
                    line: 1,
                    column: 1,
                },
                Statement::ExpressionStatement {
                    expression: Expression::FunctionCall {
                        function: Box::new(Expression::Variable("greet".to_string(), 3, 1)),
                        arguments: vec![Argument {
                            name: None,
                            value: Expression::Literal(Literal::String("Alice".to_string()), 3, 7),
                        }],
                        line: 3,
                        column: 1,
                    },
                    line: 3,
                    column: 1,
                },
            ],
        };

        let mut analyzer = Analyzer::new();
        let result = analyzer.analyze(&program);
        assert!(result.is_ok(), "Expected no semantic errors");
    }

    #[test]
    fn test_function_call_wrong_args() {
        let program = Program {
            statements: vec![
                Statement::ActionDefinition {
                    name: "greet".to_string(),
                    parameters: vec![Parameter {
                        name: "name".to_string(),
                        param_type: Some(Type::Text),
                        default_value: None,
                        line: 0,
                        column: 0,
                    }],
                    body: vec![],
                    return_type: None,
                    line: 1,
                    column: 1,
                },
                Statement::ExpressionStatement {
                    expression: Expression::FunctionCall {
                        function: Box::new(Expression::Variable("greet".to_string(), 2, 1)),
                        arguments: vec![], // No arguments provided
                        line: 2,
                        column: 1,
                    },
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut analyzer = Analyzer::new();
        let result = analyzer.analyze(&program);
        assert!(
            result.is_err(),
            "Expected semantic error for wrong number of arguments"
        );

        let errors = analyzer.get_errors();
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0]
                .message
                .contains("expects 1 arguments, but 0 were provided")
        );
    }

    #[test]
    fn test_container_static_property_recognition() {
        use std::collections::HashMap;

        let mut analyzer = Analyzer::new();

        // Register a container with static properties
        let mut properties = HashMap::new();
        properties.insert(
            "name".to_string(),
            PropertyInfo {
                name: "name".to_string(),
                property_type: Type::Text,
                is_public: true,
                line: 1,
                column: 1,
            },
        );

        let mut static_properties = HashMap::new();
        static_properties.insert(
            "total_count".to_string(),
            PropertyInfo {
                name: "total_count".to_string(),
                property_type: Type::Number,
                is_public: true,
                line: 1,
                column: 1,
            },
        );

        let container_info = ContainerInfo {
            name: "Counter".to_string(),
            properties,
            static_properties,
            methods: HashMap::new(),
            static_methods: HashMap::new(),
            extends: None,
            implements: Vec::new(),
            line: 1,
            column: 1,
        };

        analyzer.register_container(container_info);

        // Test instance property recognition
        assert!(analyzer.is_container_property("Counter", "name"));
        // Test static property recognition
        assert!(analyzer.is_container_property("Counter", "total_count"));
        // Test non-existent property
        assert!(!analyzer.is_container_property("Counter", "nonexistent"));
    }

    #[test]
    fn test_container_inherited_static_property_recognition() {
        use std::collections::HashMap;

        let mut analyzer = Analyzer::new();

        // Register base container with static properties
        let mut base_static_properties = HashMap::new();
        base_static_properties.insert(
            "base_count".to_string(),
            PropertyInfo {
                name: "base_count".to_string(),
                property_type: Type::Number,
                is_public: true,
                line: 1,
                column: 1,
            },
        );

        let base_container = ContainerInfo {
            name: "BaseContainer".to_string(),
            properties: HashMap::new(),
            static_properties: base_static_properties,
            methods: HashMap::new(),
            static_methods: HashMap::new(),
            extends: None,
            implements: Vec::new(),
            line: 1,
            column: 1,
        };

        analyzer.register_container(base_container);

        // Register derived container with its own static properties
        let mut derived_static_properties = HashMap::new();
        derived_static_properties.insert(
            "derived_count".to_string(),
            PropertyInfo {
                name: "derived_count".to_string(),
                property_type: Type::Number,
                is_public: true,
                line: 1,
                column: 1,
            },
        );

        let derived_container = ContainerInfo {
            name: "DerivedContainer".to_string(),
            properties: HashMap::new(),
            static_properties: derived_static_properties,
            methods: HashMap::new(),
            static_methods: HashMap::new(),
            extends: Some("BaseContainer".to_string()),
            implements: Vec::new(),
            line: 1,
            column: 1,
        };

        analyzer.register_container(derived_container);

        // Test derived container can access its own static properties
        assert!(analyzer.is_container_property("DerivedContainer", "derived_count"));
        // Test derived container can access inherited static properties
        assert!(analyzer.is_container_property("DerivedContainer", "base_count"));
        // Test base container cannot access derived properties
        assert!(!analyzer.is_container_property("BaseContainer", "derived_count"));
    }

    // ===== Phase 4: Tests for action call validation =====

    #[test]
    fn test_undefined_action_call() {
        let input = r#"
define action called greet with parameters name:
    print with name
end action

call unknownAction with "test"
        "#;
        let tokens = crate::lexer::lex_wfl_with_positions(input);
        let program = crate::parser::Parser::new(&tokens).parse().unwrap();

        let mut analyzer = Analyzer::new();
        let _ = analyzer.analyze(&program);

        // Should have error for undefined action
        assert!(!analyzer.errors.is_empty(), "Should have semantic errors");
        assert!(
            analyzer
                .errors
                .iter()
                .any(|e| e.message.contains("Undefined action 'unknownAction'")),
            "Should report undefined action 'unknownAction', got: {:?}",
            analyzer.errors
        );
    }

    #[test]
    fn test_action_call_wrong_arg_count() {
        let input = r#"
define action called greet with parameters name:
    print with name
end action

call greet with "Alice" and "Bob"
        "#;
        let tokens = crate::lexer::lex_wfl_with_positions(input);
        let program = crate::parser::Parser::new(&tokens).parse().unwrap();

        let mut analyzer = Analyzer::new();
        let _ = analyzer.analyze(&program);

        // Should have error for wrong argument count
        assert!(!analyzer.errors.is_empty(), "Should have semantic errors");
        assert!(
            analyzer.errors.iter().any(|e| e
                .message
                .contains("expects 1 argument(s), but 2 were provided")),
            "Should report wrong argument count, got: {:?}",
            analyzer.errors
        );
    }

    #[test]
    fn test_valid_action_call() {
        let input = r#"
define action called greet with parameters name:
    print with name
end action

call greet with "Alice"
        "#;
        let tokens = crate::lexer::lex_wfl_with_positions(input);
        let program = crate::parser::Parser::new(&tokens).parse().unwrap();

        let mut analyzer = Analyzer::new();
        let _ = analyzer.analyze(&program);

        // Should have no errors
        assert!(
            analyzer.errors.is_empty(),
            "Should have no semantic errors, got: {:?}",
            analyzer.errors
        );
    }

    #[test]
    fn test_recursive_action_call() {
        let input = r#"
define action called factorial with parameters n:
    check if n is less than or equal to 1:
        return 1
    end check
    return n times (call factorial with n minus 1)
end action

store result as call factorial with 5
        "#;
        let tokens = crate::lexer::lex_wfl_with_positions(input);
        let program = crate::parser::Parser::new(&tokens).parse().unwrap();

        let mut analyzer = Analyzer::new();
        let _ = analyzer.analyze(&program);

        // Should have no errors (recursive calls should work)
        assert!(
            analyzer.errors.is_empty(),
            "Recursive action calls should be valid, got: {:?}",
            analyzer.errors
        );
    }

    #[test]
    fn test_forward_action_reference() {
        let input = r#"
define action called first:
    call second with "test"
end action

define action called second with parameters msg:
    print with msg
end action
        "#;
        let tokens = crate::lexer::lex_wfl_with_positions(input);
        let program = crate::parser::Parser::new(&tokens).parse().unwrap();

        let mut analyzer = Analyzer::new();
        let _ = analyzer.analyze(&program);

        // Should have no errors (forward references should work)
        assert!(
            analyzer.errors.is_empty(),
            "Forward action references should be valid, got: {:?}",
            analyzer.errors
        );
    }

    #[test]
    fn test_builtin_action_call_validation() {
        let input = r#"
print with "Hello"
        "#;
        let tokens = crate::lexer::lex_wfl_with_positions(input);
        let program = crate::parser::Parser::new(&tokens).parse().unwrap();

        let mut analyzer = Analyzer::new();
        let _ = analyzer.analyze(&program);

        // Should have no errors (builtin functions should be recognized)
        assert!(
            analyzer.errors.is_empty(),
            "Builtin function calls should be valid, got: {:?}",
            analyzer.errors
        );
    }

    #[test]
    fn test_action_not_a_function_error() {
        let input = r#"
store x as 10
call x with "test"
        "#;
        let tokens = crate::lexer::lex_wfl_with_positions(input);
        let program = crate::parser::Parser::new(&tokens).parse().unwrap();

        let mut analyzer = Analyzer::new();
        let _ = analyzer.analyze(&program);

        // Should have error that 'x' is not an action
        assert!(!analyzer.errors.is_empty(), "Should have semantic errors");
        assert!(
            analyzer
                .errors
                .iter()
                .any(|e| e.message.contains("'x' is not an action")),
            "Should report 'x' is not an action, got: {:?}",
            analyzer.errors
        );
    }
}
