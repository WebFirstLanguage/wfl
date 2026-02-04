use crate::analyzer::{Analyzer, Symbol, SymbolKind};
use crate::builtins;
use crate::parser::ast::{Expression, Literal, Operator, Program, Statement, Type, UnaryOperator};
use std::fmt;

#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub expected: Option<Type>,
    pub found: Option<Type>,
    pub line: usize,
    pub column: usize,
}

impl TypeError {
    pub fn new(
        message: String,
        expected: Option<Type>,
        found: Option<Type>,
        line: usize,
        column: usize,
    ) -> Self {
        TypeError {
            message,
            expected,
            found,
            line,
            column,
        }
    }
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut message = format!(
            "Type error at line {}, column {}: {}",
            self.line, self.column, self.message
        );

        if let Some(expected) = &self.expected
            && let Some(found) = &self.found
        {
            message.push_str(&format!(" - Expected {expected} but found {found}"));
        }

        write!(f, "{message}")
    }
}

impl std::error::Error for TypeError {}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Text => write!(f, "Text"),
            Type::Number => write!(f, "Number"),
            Type::Boolean => write!(f, "Boolean"),
            Type::Nothing => write!(f, "Nothing"),
            Type::Pattern => write!(f, "Pattern"),
            Type::Custom(name) => write!(f, "{name}"),
            Type::List(item_type) => write!(f, "List of {item_type}"),
            Type::Map(key_type, value_type) => write!(f, "Map from {key_type} to {value_type}"),
            Type::Function {
                parameters,
                return_type,
            } => {
                write!(f, "Function(")?;
                for (i, param) in parameters.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{param}")?;
                }
                write!(f, ") -> {return_type}")
            }
            Type::Unknown => write!(f, "Unknown"),
            Type::Error => write!(f, "Error"),
            Type::Async(t) => write!(f, "Async<{t}>"),
            Type::Any => write!(f, "Any"),
            Type::Container(name) => write!(f, "Container<{name}>"),
            Type::ContainerInstance(name) => write!(f, "Instance<{name}>"),
            Type::Interface(name) => write!(f, "Interface<{name}>"),
        }
    }
}

pub struct TypeChecker {
    analyzer: Analyzer,
    errors: Vec<TypeError>,
    analyzer_already_run: bool,
    current_container: Option<String>,
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut analyzer = Analyzer::new();

        crate::stdlib::typechecker::register_stdlib_types(&mut analyzer);

        TypeChecker {
            analyzer,
            errors: Vec::new(),
            analyzer_already_run: false,
            current_container: None,
        }
    }

    /// Create a new TypeChecker with an existing Analyzer
    /// This allows sharing action parameters between the analyzer and type checker
    pub fn with_analyzer(analyzer: Analyzer) -> Self {
        TypeChecker {
            analyzer,
            errors: Vec::new(),
            analyzer_already_run: true, // Analyzer has already been run when passed in
            current_container: None,
        }
    }

    /// Get the action parameters from the analyzer
    pub fn get_action_parameters(&self) -> &std::collections::HashSet<String> {
        self.analyzer.get_action_parameters()
    }

    /// Get the return type for builtin functions
    fn get_builtin_function_type(&self, name: &str, _arg_count: usize) -> Type {
        match name {
            // Type functions
            "typeof" | "type_of" => Type::Text,
            "isnothing" | "is_nothing" => Type::Boolean,

            // Math functions
            "abs" | "round" | "floor" | "ceil" | "clamp" | "min" | "max" | "power" | "sqrt"
            | "sin" | "cos" | "tan" => Type::Number,

            // Random functions
            "random" | "random_between" | "random_int" => Type::Number,
            "random_boolean" => Type::Boolean,
            "random_from" => Type::Any, // Returns element from list, so type depends on list
            "random_seed" => Type::Nothing, // Void function

            // Text functions
            "length" | "indexof" | "index_of" | "lastindexof" | "last_index_of" => Type::Number,
            "touppercase" | "tolowercase" | "substring" | "replace" | "trim" | "padleft"
            | "padright" | "capitalize" | "reverse" => Type::Text,
            "contains" | "startswith" | "starts_with" | "endswith" | "ends_with" => Type::Boolean,
            "split" => Type::List(Box::new(Type::Text)),
            "join" => Type::Text,

            // List functions
            "push" | "pop" | "shift" | "unshift" | "removeat" | "remove_at" | "insertat"
            | "insert_at" | "slice" | "concat" | "unique" | "sort" | "reverse_list" | "clear"
            | "filter" | "map" => Type::List(Box::new(Type::Any)),
            "find" => Type::Any,
            "count" | "size" => Type::Number,
            "includes" => Type::Boolean,

            // Time functions
            "now" | "today" | "time" | "date" | "year" | "month" | "day" | "hour" | "minute"
            | "second" | "dayofweek" | "day_of_week" | "adddays" | "add_days" | "addmonths"
            | "add_months" | "addyears" | "add_years" | "addhours" | "add_hours" | "addminutes"
            | "add_minutes" | "addseconds" | "add_seconds" => Type::Number,
            "formatdate" | "format_date" | "formattime" | "format_time" => Type::Text,
            "parsedate" | "parse_date" | "isleapyear" | "is_leap_year" => Type::Number,
            "daysbetween" | "days_between" | "monthsbetween" | "months_between"
            | "yearsbetween" | "years_between" => Type::Number,

            // Pattern functions
            "pattern" | "match" | "test" | "replace_pattern" | "extract" => Type::Text,
            "ismatch" | "is_match" => Type::Boolean,
            "findall" | "find_all" => Type::List(Box::new(Type::Text)),

            // Crypto functions
            "wflhash256" | "wflhash512" | "wflhash256_with_salt" | "wflmac256" => Type::Text,

            _ => Type::Unknown,
        }
    }

    pub fn check_types(&mut self, program: &Program) -> Result<(), Vec<TypeError>> {
        // Only run the analyzer if it hasn't been run already
        // When created with with_analyzer(), the analyzer has already been run,
        // so we don't need to analyze again. This prevents duplicate symbol registration.
        if !self.analyzer_already_run
            && let Err(semantic_errors) = self.analyzer.analyze(program)
        {
            for error in semantic_errors {
                self.errors.push(TypeError::new(
                    error.message,
                    None,
                    None,
                    error.line,
                    error.column,
                ));
            }
            return Err(self.errors.clone());
        }

        for statement in &program.statements {
            self.check_statement_types(statement);
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn check_statement_types(&mut self, statement: &Statement) {
        match statement {
            Statement::PushStatement {
                list,
                value,
                line: _line,
                column: _column,
            } => {
                let list_type = self.infer_expression_type(list);
                match list_type {
                    Type::List(_) | Type::Unknown => {}
                    _ => {
                        self.errors.push(TypeError::new(
                            format!("Expected list type for push operation, got {list_type:?}"),
                            Some(Type::List(Box::new(Type::Any))),
                            Some(list_type.clone()),
                            *_line,
                            *_column,
                        ));
                    }
                }
                self.infer_expression_type(value);
            }
            Statement::RepeatWhileLoop {
                condition,
                body,
                line: _line,
                column: _column,
            } => {
                let condition_type = self.infer_expression_type(condition);
                if condition_type != Type::Boolean && condition_type != Type::Unknown {
                    self.errors.push(TypeError::new(
                        format!(
                            "Expected boolean condition in repeat-while loop, got {condition_type:?}"
                        ),
                        Some(Type::Boolean),
                        Some(condition_type.clone()),
                        *_line,
                        *_column,
                    ));
                }

                for stmt in body {
                    self.check_statement_types(stmt);
                }
            }
            Statement::ExitStatement { line: _, column: _ } => {}
            Statement::WaitForStatement {
                inner,
                line: _line,
                column: _column,
            } => {
                self.check_statement_types(inner);
            }
            Statement::WaitForDurationStatement {
                duration,
                line: _line,
                column: _column,
                ..
            } => {
                let duration_type = self.infer_expression_type(duration);
                if duration_type != Type::Number
                    && duration_type != Type::Unknown
                    && duration_type != Type::Error
                {
                    self.type_error(
                        "Expected a number for wait duration".to_string(),
                        Some(Type::Number),
                        Some(duration_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::TryStatement {
                body,
                when_clauses,
                otherwise_block,
                line: _line,
                column: _column,
            } => {
                for stmt in body {
                    self.check_statement_types(stmt);
                }

                // Type check each when clause
                for when_clause in when_clauses {
                    if let Some(symbol) = self.analyzer.get_symbol_mut(&when_clause.error_name) {
                        symbol.symbol_type = Some(Type::Text); // Errors are represented as text
                    }

                    for stmt in &when_clause.body {
                        self.check_statement_types(stmt);
                    }
                }

                if let Some(otherwise_stmts) = otherwise_block {
                    for stmt in otherwise_stmts {
                        self.check_statement_types(stmt);
                    }
                }
            }
            Statement::HttpGetStatement {
                url,
                variable_name,
                line: _line,
                column: _column,
            } => {
                let url_type = self.infer_expression_type(url);
                if url_type != Type::Text && url_type != Type::Unknown && url_type != Type::Error {
                    self.type_error(
                        "URL must be a text string".to_string(),
                        Some(Type::Text),
                        Some(url_type),
                        *_line,
                        *_column,
                    );
                }

                if !variable_name.is_empty()
                    && let Some(symbol) = self.analyzer.get_symbol_mut(variable_name)
                {
                    symbol.symbol_type = Some(Type::Text);
                }
            }
            Statement::HttpPostStatement {
                url,
                data,
                variable_name,
                line: _line,
                column: _column,
            } => {
                let url_type = self.infer_expression_type(url);
                if url_type != Type::Text && url_type != Type::Unknown && url_type != Type::Error {
                    self.type_error(
                        "URL must be a text string".to_string(),
                        Some(Type::Text),
                        Some(url_type),
                        *_line,
                        *_column,
                    );
                }

                self.infer_expression_type(data);

                if !variable_name.is_empty()
                    && let Some(symbol) = self.analyzer.get_symbol_mut(variable_name)
                {
                    symbol.symbol_type = Some(Type::Text);
                }
            }
            Statement::VariableDeclaration {
                name,
                value,
                is_constant: _,
                line: _line,
                column: _column,
            } => {
                let inferred_type = self.infer_expression_type(value);

                // Special case for loopcounter variable
                if name == "loopcounter" {
                    // Skip type inference error for loopcounter
                    return;
                }

                // Check if this is a container property assignment within a method
                // In this case, we might know the property type from the container definition
                let mut is_container_property_assignment = false;
                if inferred_type == Type::Unknown {
                    // Check if we're in a container method and this is a property assignment
                    if let Some(ref container_name) = self.current_container
                        && let Some(container_info) = self.analyzer.get_container(container_name)
                        && container_info.properties.contains_key(name)
                    {
                        // This is a container property assignment
                        is_container_property_assignment = true;
                    }

                    // Also check if the analyzer has this symbol (fallback)
                    if !is_container_property_assignment
                        && let Some(symbol) = self.analyzer.get_symbol(name)
                        && symbol.symbol_type.is_some()
                    {
                        // Variable already exists with a known type
                        is_container_property_assignment = true;
                    }
                }

                if inferred_type == Type::Unknown && !is_container_property_assignment {
                    self.type_error(
                        format!("Could not infer type for variable '{name}'"),
                        None,
                        None,
                        *_line,
                        *_column,
                    );
                }

                let symbol_type_option = if let Some(symbol) = self.analyzer.get_symbol(name) {
                    symbol.symbol_type.clone()
                } else {
                    None
                };

                let need_type_error = if let Some(declared_type) = &symbol_type_option {
                    !self.are_types_compatible(declared_type, &inferred_type)
                } else {
                    false
                };

                if need_type_error {
                    self.type_error(
                        format!("Cannot initialize variable '{name}' with incompatible type"),
                        symbol_type_option.clone(),
                        Some(inferred_type.clone()),
                        *_line,
                        *_column,
                    );
                }

                if inferred_type != Type::Error
                    && inferred_type != Type::Unknown
                    && let Some(symbol) = self.analyzer.get_symbol_mut(name)
                    && symbol.symbol_type.is_none()
                {
                    symbol.symbol_type = Some(inferred_type);
                }
            }
            Statement::Assignment {
                name,
                value,
                line,
                column,
            } => {
                let inferred_type = self.infer_expression_type(value);

                if let Some(symbol) = self.analyzer.get_symbol(name) {
                    if let Some(variable_type) = &symbol.symbol_type {
                        if !self.are_types_compatible(variable_type, &inferred_type) {
                            self.type_error(
                                format!(
                                    "Cannot assign value of incompatible type to variable '{name}'"
                                ),
                                Some(variable_type.clone()),
                                Some(inferred_type),
                                *line,
                                *column,
                            );
                        }
                    } else if inferred_type != Type::Error
                        && inferred_type != Type::Unknown
                        && let Some(symbol) = self.analyzer.get_symbol_mut(name)
                    {
                        symbol.symbol_type = Some(inferred_type);
                    }
                }
            }
            Statement::ActionDefinition {
                name,
                parameters,
                body,
                return_type,
                line: _line,
                column: _column,
            } => {
                let param_types = parameters
                    .iter()
                    .map(|p| p.param_type.clone().unwrap_or(Type::Unknown))
                    .collect::<Vec<Type>>();

                let return_type_value = return_type.clone().unwrap_or(Type::Nothing);

                if let Some(symbol) = self.analyzer.get_symbol_mut(name) {
                    symbol.symbol_type = Some(Type::Function {
                        parameters: param_types,
                        return_type: Box::new(return_type_value),
                    });
                }

                for stmt in body {
                    self.check_statement_types(stmt);
                }

                if let Some(ret_type) = return_type {
                    self.check_return_statements(body, ret_type, *_line, *_column);
                }
            }
            Statement::IfStatement {
                condition,
                then_block,
                else_block,
                line: _line,
                column: _column,
            } => {
                let condition_type = self.infer_expression_type(condition);
                if condition_type != Type::Boolean
                    && condition_type != Type::Unknown
                    && condition_type != Type::Error
                {
                    self.type_error(
                        "Condition must be a boolean expression".to_string(),
                        Some(Type::Boolean),
                        Some(condition_type),
                        *_line,
                        *_column,
                    );
                }

                for stmt in then_block {
                    self.check_statement_types(stmt);
                }

                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.check_statement_types(stmt);
                    }
                }
            }
            Statement::SingleLineIf {
                condition,
                then_stmt,
                else_stmt,
                line: _line,
                column: _column,
            } => {
                let condition_type = self.infer_expression_type(condition);
                if condition_type != Type::Boolean
                    && condition_type != Type::Unknown
                    && condition_type != Type::Error
                {
                    self.type_error(
                        "Condition must be a boolean expression".to_string(),
                        Some(Type::Boolean),
                        Some(condition_type),
                        *_line,
                        *_column,
                    );
                }

                self.check_statement_types(then_stmt);

                if let Some(else_stmt) = else_stmt {
                    self.check_statement_types(else_stmt);
                }
            }
            Statement::ForEachLoop {
                item_name,
                collection,
                body,
                line: _line,
                column: _column,
                ..
            } => {
                let collection_type = self.infer_expression_type(collection);
                let mut item_type_inferred = Type::Unknown;

                match collection_type {
                    Type::List(item_type) => {
                        item_type_inferred = *item_type;
                    }
                    Type::Map(_, value_type) => {
                        item_type_inferred = *value_type;
                    }
                    Type::Unknown | Type::Error => {}
                    _ => {
                        self.type_error(
                            "Collection in for-each loop must be a list or map".to_string(),
                            Some(Type::List(Box::new(Type::Unknown))),
                            Some(collection_type),
                            *_line,
                            *_column,
                        );
                    }
                }

                // Push a new scope for the loop body
                self.analyzer.push_scope();

                // Define the loop variable in the new scope
                let symbol = Symbol {
                    name: item_name.clone(),
                    kind: SymbolKind::Variable { mutable: false },
                    symbol_type: Some(item_type_inferred),
                    line: *_line,
                    column: *_column,
                };

                // Ignore errors (e.g., if already defined, though in a new scope it shouldn't be)
                let _ = self.analyzer.define_symbol(symbol);

                for stmt in body {
                    self.check_statement_types(stmt);
                }

                // Pop the scope
                self.analyzer.pop_scope();
            }
            Statement::CountLoop {
                start,
                end,
                step,
                body,
                line: _line,
                column: _column,
                ..
            } => {
                let start_type = self.infer_expression_type(start);
                if start_type != Type::Number
                    && start_type != Type::Unknown
                    && start_type != Type::Error
                {
                    self.type_error(
                        "Start value in count loop must be a number".to_string(),
                        Some(Type::Number),
                        Some(start_type),
                        *_line,
                        *_column,
                    );
                }

                let end_type = self.infer_expression_type(end);
                if end_type != Type::Number && end_type != Type::Unknown && end_type != Type::Error
                {
                    self.type_error(
                        "End value in count loop must be a number".to_string(),
                        Some(Type::Number),
                        Some(end_type),
                        *_line,
                        *_column,
                    );
                }

                if let Some(step_expr) = step {
                    let step_type = self.infer_expression_type(step_expr);
                    if step_type != Type::Number
                        && step_type != Type::Unknown
                        && step_type != Type::Error
                    {
                        self.type_error(
                            "Step value in count loop must be a number".to_string(),
                            Some(Type::Number),
                            Some(step_type),
                            *_line,
                            *_column,
                        );
                    }
                }

                // Register the "count" variable with type Number
                if let Some(symbol) = self.analyzer.get_symbol_mut("count") {
                    symbol.symbol_type = Some(Type::Number);
                }

                for stmt in body {
                    self.check_statement_types(stmt);
                }
            }
            Statement::WhileLoop {
                condition,
                body,
                line: _line,
                column: _column,
            } => {
                let condition_type = self.infer_expression_type(condition);
                if condition_type != Type::Boolean
                    && condition_type != Type::Unknown
                    && condition_type != Type::Error
                {
                    self.type_error(
                        "Condition in while loop must be a boolean expression".to_string(),
                        Some(Type::Boolean),
                        Some(condition_type),
                        *_line,
                        *_column,
                    );
                }

                for stmt in body {
                    self.check_statement_types(stmt);
                }
            }
            Statement::RepeatUntilLoop {
                condition,
                body,
                line: _line,
                column: _column,
            } => {
                let condition_type = self.infer_expression_type(condition);
                if condition_type != Type::Boolean
                    && condition_type != Type::Unknown
                    && condition_type != Type::Error
                {
                    self.type_error(
                        "Condition in repeat-until loop must be a boolean expression".to_string(),
                        Some(Type::Boolean),
                        Some(condition_type),
                        *_line,
                        *_column,
                    );
                }

                for stmt in body {
                    self.check_statement_types(stmt);
                }
            }
            Statement::ForeverLoop { body, .. } => {
                for stmt in body {
                    self.check_statement_types(stmt);
                }
            }
            Statement::MainLoop { body, .. } => {
                for stmt in body {
                    self.check_statement_types(stmt);
                }
            }
            Statement::DisplayStatement { value, .. } => {
                self.infer_expression_type(value);
            }
            Statement::ReturnStatement {
                value,
                line: _,
                column: _,
            } => {
                if let Some(expr) = value {
                    self.infer_expression_type(expr);
                }
            }
            Statement::ExpressionStatement { expression, .. } => {
                self.infer_expression_type(expression);
            }
            Statement::BreakStatement { .. } | Statement::ContinueStatement { .. } => {}
            Statement::OpenFileStatement {
                path,
                variable_name,
                mode: _mode,
                line: _line,
                column: _column,
            } => {
                let path_type = self.infer_expression_type(path);
                if path_type != Type::Text && path_type != Type::Unknown && path_type != Type::Error
                {
                    self.type_error(
                        "File path must be a text string".to_string(),
                        Some(Type::Text),
                        Some(path_type),
                        *_line,
                        *_column,
                    );
                }

                if let Some(symbol) = self.analyzer.get_symbol_mut(variable_name) {
                    symbol.symbol_type = Some(Type::Custom("File".to_string()));
                }
            }
            Statement::ReadFileStatement {
                path,
                variable_name,
                line: _line,
                column: _column,
            } => {
                let file_type = self.infer_expression_type(path);
                if file_type != Type::Custom("File".to_string())
                    && file_type != Type::Unknown
                    && file_type != Type::Error
                {
                    self.type_error(
                        "Expected a File object".to_string(),
                        Some(Type::Custom("File".to_string())),
                        Some(file_type),
                        *_line,
                        *_column,
                    );
                }

                if let Some(symbol) = self.analyzer.get_symbol_mut(variable_name) {
                    symbol.symbol_type = Some(Type::Text);
                }
            }
            Statement::WriteFileStatement {
                file,
                content,
                mode: _,
                line: _line,
                column: _column,
            } => {
                let file_type = self.infer_expression_type(file);
                if file_type != Type::Custom("File".to_string())
                    && file_type != Type::Unknown
                    && file_type != Type::Error
                {
                    self.type_error(
                        "Expected a File object".to_string(),
                        Some(Type::Custom("File".to_string())),
                        Some(file_type),
                        *_line,
                        *_column,
                    );
                }

                let content_type = self.infer_expression_type(content);
                if content_type != Type::Text
                    && content_type != Type::Unknown
                    && content_type != Type::Error
                {
                    self.type_error(
                        "File content must be a text string".to_string(),
                        Some(Type::Text),
                        Some(content_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::CloseFileStatement {
                file,
                line: _line,
                column: _column,
            } => {
                let file_type = self.infer_expression_type(file);
                if file_type != Type::Custom("File".to_string())
                    && file_type != Type::Unknown
                    && file_type != Type::Error
                {
                    self.type_error(
                        "Expected a File object".to_string(),
                        Some(Type::Custom("File".to_string())),
                        Some(file_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::CreateDirectoryStatement {
                path,
                line: _line,
                column: _column,
            } => {
                let path_type = self.infer_expression_type(path);
                if path_type != Type::Text && path_type != Type::Unknown && path_type != Type::Error
                {
                    self.type_error(
                        "Expected string for directory path".to_string(),
                        Some(Type::Text),
                        Some(path_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::CreateFileStatement {
                path,
                content,
                line: _line,
                column: _column,
            } => {
                let path_type = self.infer_expression_type(path);
                if path_type != Type::Text && path_type != Type::Unknown && path_type != Type::Error
                {
                    self.type_error(
                        "Expected string for file path".to_string(),
                        Some(Type::Text),
                        Some(path_type),
                        *_line,
                        *_column,
                    );
                }
                let _content_type = self.infer_expression_type(content); // Content can be any type
            }
            Statement::DeleteFileStatement {
                path,
                line: _line,
                column: _column,
            } => {
                let path_type = self.infer_expression_type(path);
                if path_type != Type::Text && path_type != Type::Unknown && path_type != Type::Error
                {
                    self.type_error(
                        "Expected string for file path".to_string(),
                        Some(Type::Text),
                        Some(path_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::DeleteDirectoryStatement {
                path,
                line: _line,
                column: _column,
            } => {
                let path_type = self.infer_expression_type(path);
                if path_type != Type::Text && path_type != Type::Unknown && path_type != Type::Error
                {
                    self.type_error(
                        "Expected string for directory path".to_string(),
                        Some(Type::Text),
                        Some(path_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::LoadModuleStatement {
                path,
                line: _line,
                column: _column,
                ..
            } => {
                let path_type = self.infer_expression_type(path);
                if path_type != Type::Text && path_type != Type::Unknown && path_type != Type::Error
                {
                    self.type_error(
                        "Expected string for module path".to_string(),
                        Some(Type::Text),
                        Some(path_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::ExecuteCommandStatement {
                command,
                arguments,
                variable_name: _,
                use_shell: _,
                line: _line,
                column: _column,
            } => {
                let cmd_type = self.infer_expression_type(command);
                if cmd_type != Type::Text && cmd_type != Type::Unknown && cmd_type != Type::Error {
                    self.type_error(
                        "Expected string for command".to_string(),
                        Some(Type::Text),
                        Some(cmd_type),
                        *_line,
                        *_column,
                    );
                }
                if let Some(args) = arguments {
                    let _args_type = self.infer_expression_type(args);
                    // Arguments can be a list or a single string
                }
            }
            Statement::SpawnProcessStatement {
                command,
                arguments,
                variable_name: _,
                use_shell: _,
                line: _line,
                column: _column,
            } => {
                let cmd_type = self.infer_expression_type(command);
                if cmd_type != Type::Text && cmd_type != Type::Unknown && cmd_type != Type::Error {
                    self.type_error(
                        "Expected string for command".to_string(),
                        Some(Type::Text),
                        Some(cmd_type),
                        *_line,
                        *_column,
                    );
                }
                if let Some(args) = arguments {
                    let _args_type = self.infer_expression_type(args);
                }
            }
            Statement::ReadProcessOutputStatement {
                process_id,
                variable_name: _,
                line: _line,
                column: _column,
            } => {
                let proc_type = self.infer_expression_type(process_id);
                if proc_type != Type::Text && proc_type != Type::Unknown && proc_type != Type::Error
                {
                    self.type_error(
                        "Expected string for process ID".to_string(),
                        Some(Type::Text),
                        Some(proc_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::KillProcessStatement {
                process_id,
                line: _line,
                column: _column,
            } => {
                let proc_type = self.infer_expression_type(process_id);
                if proc_type != Type::Text && proc_type != Type::Unknown && proc_type != Type::Error
                {
                    self.type_error(
                        "Expected string for process ID".to_string(),
                        Some(Type::Text),
                        Some(proc_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::WaitForProcessStatement {
                process_id,
                variable_name: _,
                line: _line,
                column: _column,
            } => {
                let proc_type = self.infer_expression_type(process_id);
                if proc_type != Type::Text && proc_type != Type::Unknown && proc_type != Type::Error
                {
                    self.type_error(
                        "Expected string for process ID".to_string(),
                        Some(Type::Text),
                        Some(proc_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::WriteToStatement {
                content,
                file,
                line: _line,
                column: _column,
            } => {
                let _content_type = self.infer_expression_type(content); // Content can be any type
                let file_type = self.infer_expression_type(file);
                if file_type != Type::Custom("File".to_string())
                    && file_type != Type::Text  // Allow string file handles
                    && file_type != Type::Unknown
                    && file_type != Type::Error
                {
                    self.type_error(
                        "Expected a file handle or string".to_string(),
                        Some(Type::Custom("File".to_string())),
                        Some(file_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::WriteContentStatement {
                content,
                target,
                line: _line,
                column: _column,
            } => {
                let _content_type = self.infer_expression_type(content); // Content can be any type
                let target_type = self.infer_expression_type(target);
                if target_type != Type::Custom("File".to_string())
                    && target_type != Type::Text  // Allow string file handles
                    && target_type != Type::Unknown
                    && target_type != Type::Error
                {
                    self.type_error(
                        "Expected a file handle or string".to_string(),
                        Some(Type::Custom("File".to_string())),
                        Some(target_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::CreateListStatement {
                name,
                initial_values,
                line,
                column,
            } => {
                // Infer the element type from initial values
                let mut element_type = Type::Unknown;
                for value in initial_values {
                    let value_type = self.infer_expression_type(value);
                    if element_type == Type::Unknown {
                        element_type = value_type;
                    } else if element_type != value_type && value_type != Type::Unknown {
                        self.type_error(
                            format!("Mixed types in list initialization. Expected {element_type:?}, got {value_type:?}"),
                            Some(element_type.clone()),
                            Some(value_type),
                            *line,
                            *column,
                        );
                    }
                }

                // If empty list, element type remains Unknown
                let list_type = Type::List(Box::new(element_type));
                if let Some(symbol) = self.analyzer.get_symbol_mut(name) {
                    symbol.symbol_type = Some(list_type);
                }
            }
            Statement::AddToListStatement {
                value,
                list_name,
                line,
                column,
            } => {
                let value_type = self.infer_expression_type(value);

                if let Some(symbol) = self.analyzer.get_symbol(list_name) {
                    match &symbol.symbol_type {
                        Some(Type::List(element_type)) => {
                            if **element_type != Type::Unknown
                                && **element_type != value_type
                                && value_type != Type::Unknown
                            {
                                self.type_error(
                                    format!(
                                        "Cannot add {value_type:?} to list of {element_type:?}"
                                    ),
                                    Some((**element_type).clone()),
                                    Some(value_type),
                                    *line,
                                    *column,
                                );
                            }
                        }
                        Some(Type::Number) => {
                            // This is arithmetic add
                            if value_type != Type::Number && value_type != Type::Unknown {
                                self.type_error(
                                    "Cannot add non-numeric value to number".to_string(),
                                    Some(Type::Number),
                                    Some(value_type),
                                    *line,
                                    *column,
                                );
                            }
                        }
                        _ => {
                            // Variable might not be a list
                            if symbol.symbol_type != Some(Type::Unknown) {
                                self.type_error(
                                    format!("Cannot add to non-list variable '{list_name}'"),
                                    Some(Type::List(Box::new(Type::Any))),
                                    symbol.symbol_type.clone(),
                                    *line,
                                    *column,
                                );
                            }
                        }
                    }
                }
            }
            Statement::RemoveFromListStatement {
                value,
                list_name,
                line,
                column,
            } => {
                let _value_type = self.infer_expression_type(value);

                if let Some(symbol) = self.analyzer.get_symbol(list_name)
                    && !matches!(
                        symbol.symbol_type,
                        Some(Type::List(_)) | Some(Type::Unknown)
                    )
                {
                    self.type_error(
                        format!("Cannot remove from non-list variable '{list_name}'"),
                        Some(Type::List(Box::new(Type::Any))),
                        symbol.symbol_type.clone(),
                        *line,
                        *column,
                    );
                }
            }
            Statement::ClearListStatement {
                list_name,
                line,
                column,
            } => {
                if let Some(symbol) = self.analyzer.get_symbol(list_name)
                    && !matches!(
                        symbol.symbol_type,
                        Some(Type::List(_)) | Some(Type::Unknown)
                    )
                {
                    self.type_error(
                        format!("Cannot clear non-list variable '{list_name}'"),
                        Some(Type::List(Box::new(Type::Any))),
                        symbol.symbol_type.clone(),
                        *line,
                        *column,
                    );
                }
            }
            // Container-related statements
            Statement::ContainerDefinition {
                name: _name,
                extends,
                implements,
                properties,
                methods,
                events: _events,
                static_properties: _static_properties,
                static_methods: _static_methods,
                line,
                column,
            } => {
                if let Some(parent_name) = extends {
                    if let Some(parent_symbol) = self.analyzer.get_symbol(parent_name) {
                        if parent_symbol.symbol_type != Some(Type::Container(parent_name.clone())) {
                            self.type_error(
                                format!("'{parent_name}' is not a container type"),
                                Some(Type::Container(parent_name.clone())),
                                parent_symbol.symbol_type.clone(),
                                *line,
                                *column,
                            );
                        }
                    } else {
                        self.type_error(
                            format!("Parent container '{parent_name}' not found"),
                            Some(Type::Container(parent_name.clone())),
                            None,
                            *line,
                            *column,
                        );
                    }
                }

                for interface_name in implements {
                    if let Some(interface_symbol) = self.analyzer.get_symbol(interface_name) {
                        if interface_symbol.symbol_type
                            != Some(Type::Interface(interface_name.clone()))
                        {
                            self.type_error(
                                format!("'{interface_name}' is not an interface type"),
                                Some(Type::Interface(interface_name.clone())),
                                interface_symbol.symbol_type.clone(),
                                *line,
                                *column,
                            );
                        }
                    } else {
                        self.type_error(
                            format!("Interface '{interface_name}' not found"),
                            Some(Type::Interface(interface_name.clone())),
                            None,
                            *line,
                            *column,
                        );
                    }
                }

                for property in properties {
                    if let Some(default_expr) = &property.default_value {
                        let default_type = self.infer_expression_type(default_expr);
                        if let Some(declared_type) = &property.property_type
                            && !self.are_types_compatible(&default_type, declared_type)
                        {
                            self.type_error(
                                    format!(
                                        "Default value type {default_type:?} incompatible with declared type {declared_type:?}"
                                    ),
                                    Some(declared_type.clone()),
                                    Some(default_type),
                                    property.line,
                                    property.column,
                                );
                        }
                    }
                }

                for method in methods {
                    if let Statement::ActionDefinition { body, .. } = method {
                        // Set container context for method body analysis
                        let previous_container = self.current_container.clone();
                        self.current_container = Some(_name.clone());

                        for stmt in body {
                            self.check_statement_types(stmt);
                        }

                        // Restore previous container context
                        self.current_container = previous_container;
                    }
                }

                // Container type registration would be handled by analyzer
            }
            Statement::ContainerInstantiation {
                container_type,
                instance_name: _instance_name,
                arguments: _arguments,
                property_initializers,
                line,
                column,
            } => {
                if let Some(container_symbol) = self.analyzer.get_symbol(container_type) {
                    if container_symbol.symbol_type != Some(Type::Container(container_type.clone()))
                    {
                        self.type_error(
                            format!("'{container_type}' is not a container type"),
                            Some(Type::Container(container_type.clone())),
                            container_symbol.symbol_type.clone(),
                            *line,
                            *column,
                        );
                    }
                } else {
                    self.type_error(
                        format!("Container type '{container_type}' not found"),
                        Some(Type::Container(container_type.clone())),
                        None,
                        *line,
                        *column,
                    );
                }

                for initializer in property_initializers {
                    let _init_type = self.infer_expression_type(&initializer.value);
                }
            }
            Statement::InterfaceDefinition {
                name: _name,
                extends: _extends,
                required_actions: _required_actions,
                line: _line,
                column: _column,
            } => {
                // Interface type registration would be handled by analyzer
            }
            Statement::EventDefinition {
                name: _name,
                parameters: _parameters,
                line: _line,
                column: _column,
            } => {}
            Statement::EventTrigger {
                name: _name,
                arguments: _arguments,
                line: _line,
                column: _column,
            } => {}
            Statement::EventHandler {
                event_name: _event_name,
                event_source: _event_source,
                handler_body,
                line: _line,
                column: _column,
            } => {
                for stmt in handler_body {
                    self.check_statement_types(stmt);
                }
            }
            Statement::ParentMethodCall {
                method_name: _method_name,
                arguments: _arguments,
                line: _line,
                column: _column,
            } => {}
            Statement::PatternDefinition { .. } => {
                // TODO: Add type checking for pattern definitions
                // For now, patterns are valid without additional type checking
            }
            Statement::MapCreation {
                name: _name,
                entries,
                line: _line,
                column: _column,
            } => {
                // Check each entry value type
                for (_key, value) in entries {
                    self.infer_expression_type(value);
                }
                // The map will be added to the environment at runtime
            }
            Statement::CreateDateStatement {
                name: _name,
                value,
                line: _line,
                column: _column,
            } => {
                // Check the value expression if provided
                if let Some(expr) = value {
                    self.infer_expression_type(expr);
                }
                // The date will be added to the environment at runtime
            }
            Statement::CreateTimeStatement {
                name: _name,
                value,
                line: _line,
                column: _column,
            } => {
                // Check the value expression if provided
                if let Some(expr) = value {
                    self.infer_expression_type(expr);
                }
                // The time will be added to the environment at runtime
            }
            Statement::ListenStatement {
                port,
                server_name: _server_name,
                line: _line,
                column: _column,
            } => {
                let port_type = self.infer_expression_type(port);
                if port_type != Type::Number
                    && port_type != Type::Unknown
                    && port_type != Type::Error
                {
                    self.type_error(
                        "Port must be a number".to_string(),
                        Some(Type::Number),
                        Some(port_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::WaitForRequestStatement {
                server,
                request_name: _request_name,
                timeout,
                line: _line,
                column: _column,
            } => {
                self.validate_server_operand(server, *_line, *_column);

                if let Some(timeout_expr) = timeout {
                    let timeout_type = self.infer_expression_type(timeout_expr);
                    if timeout_type != Type::Number
                        && timeout_type != Type::Unknown
                        && timeout_type != Type::Error
                    {
                        self.type_error(
                            "Timeout must be a number".to_string(),
                            Some(Type::Number),
                            Some(timeout_type),
                            *_line,
                            *_column,
                        );
                    }
                }
            }
            Statement::RespondStatement {
                request: _request,
                content,
                status,
                content_type,
                line: _line,
                column: _column,
            } => {
                // Check content type (should be text)
                let content_type_result = self.infer_expression_type(content);
                if content_type_result != Type::Text
                    && content_type_result != Type::Unknown
                    && content_type_result != Type::Error
                {
                    self.type_error(
                        "Response content must be text".to_string(),
                        Some(Type::Text),
                        Some(content_type_result),
                        *_line,
                        *_column,
                    );
                }

                // Check status if provided (should be number)
                if let Some(status_expr) = status {
                    let status_type = self.infer_expression_type(status_expr);
                    if status_type != Type::Number
                        && status_type != Type::Unknown
                        && status_type != Type::Error
                    {
                        self.type_error(
                            "HTTP status must be a number".to_string(),
                            Some(Type::Number),
                            Some(status_type),
                            *_line,
                            *_column,
                        );
                    }
                }

                // Check content_type if provided (should be text)
                if let Some(ct_expr) = content_type {
                    let ct_type = self.infer_expression_type(ct_expr);
                    if ct_type != Type::Text && ct_type != Type::Unknown && ct_type != Type::Error {
                        self.type_error(
                            "Content type must be text".to_string(),
                            Some(Type::Text),
                            Some(ct_type),
                            *_line,
                            *_column,
                        );
                    }
                }
            }
            // Graceful shutdown and signal handling statements
            Statement::RegisterSignalHandlerStatement {
                signal_type,
                handler_name,
                line,
                column,
            } => {
                self.validate_signal_handler_statement(signal_type, handler_name, *line, *column);
            }
            Statement::StopAcceptingConnectionsStatement {
                server,
                line: _line,
                column: _column,
            } => {
                self.validate_server_operand(server, *_line, *_column);
            }
            Statement::CloseServerStatement {
                server,
                line: _line,
                column: _column,
            } => {
                self.validate_server_operand(server, *_line, *_column);
            }
            // Test framework statements
            Statement::DescribeBlock {
                description: _description,
                setup,
                teardown,
                tests,
                line: _line,
                column: _column,
            } => {
                // Type check setup block if present
                if let Some(setup_stmts) = setup {
                    for stmt in setup_stmts {
                        self.check_statement_types(stmt);
                    }
                }

                // Type check all test blocks
                for test in tests {
                    self.check_statement_types(test);
                }

                // Type check teardown block if present
                if let Some(teardown_stmts) = teardown {
                    for stmt in teardown_stmts {
                        self.check_statement_types(stmt);
                    }
                }
            }
            Statement::TestBlock {
                description: _description,
                body,
                line: _line,
                column: _column,
            } => {
                // Type check test body
                for stmt in body {
                    self.check_statement_types(stmt);
                }
            }
            Statement::ExpectStatement {
                subject,
                assertion,
                line: _line,
                column: _column,
            } => {
                // Type check the subject expression
                let subject_type = self.infer_expression_type(subject);

                // Perform compile-time type checking for assertions where possible
                use crate::parser::ast::Assertion;
                match assertion {
                    Assertion::Equal(expr) | Assertion::Be(expr) => {
                        // Type check the expected value
                        self.infer_expression_type(expr);
                    }
                    Assertion::GreaterThan(expr) | Assertion::LessThan(expr) => {
                        // Check that subject is a number
                        if subject_type != Type::Number
                            && subject_type != Type::Unknown
                            && subject_type != Type::Error
                        {
                            self.type_error(
                                "Comparison assertions require numeric types".to_string(),
                                Some(Type::Number),
                                Some(subject_type.clone()),
                                *_line,
                                *_column,
                            );
                        }
                        // Type check the comparison value
                        let expr_type = self.infer_expression_type(expr);
                        if expr_type != Type::Number
                            && expr_type != Type::Unknown
                            && expr_type != Type::Error
                        {
                            self.type_error(
                                "Comparison value must be numeric".to_string(),
                                Some(Type::Number),
                                Some(expr_type),
                                *_line,
                                *_column,
                            );
                        }
                    }
                    Assertion::BeYes | Assertion::BeNo => {
                        // Truthiness checks work on any type, no validation needed
                    }
                    Assertion::Exist => {
                        // Existence checks work on any type
                    }
                    Assertion::Contain(expr) => {
                        // Check that subject is a list or text
                        if !matches!(
                            subject_type,
                            Type::List(_) | Type::Text | Type::Unknown | Type::Error
                        ) {
                            self.type_error(
                                "contain assertion requires List or Text type".to_string(),
                                None,
                                Some(subject_type.clone()),
                                *_line,
                                *_column,
                            );
                        }
                        // Type check the item expression
                        self.infer_expression_type(expr);
                    }
                    Assertion::BeEmpty => {
                        // Check that subject is a list or text
                        if !matches!(
                            subject_type,
                            Type::List(_) | Type::Text | Type::Unknown | Type::Error
                        ) {
                            self.type_error(
                                "be empty assertion requires List or Text type".to_string(),
                                None,
                                Some(subject_type.clone()),
                                *_line,
                                *_column,
                            );
                        }
                    }
                    Assertion::HaveLength(expr) => {
                        // Check that subject is a list or text
                        if !matches!(
                            subject_type,
                            Type::List(_) | Type::Text | Type::Unknown | Type::Error
                        ) {
                            self.type_error(
                                "have length assertion requires List or Text type".to_string(),
                                None,
                                Some(subject_type.clone()),
                                *_line,
                                *_column,
                            );
                        }
                        // Type check the length value (should be number)
                        let length_type = self.infer_expression_type(expr);
                        if length_type != Type::Number
                            && length_type != Type::Unknown
                            && length_type != Type::Error
                        {
                            self.type_error(
                                "Length value must be numeric".to_string(),
                                Some(Type::Number),
                                Some(length_type),
                                *_line,
                                *_column,
                            );
                        }
                    }
                    Assertion::BeOfType(_type_name) => {
                        // Type name is validated at runtime
                    }
                }
            }

            Statement::IncludeStatement {
                path, line, column, ..
            } => {
                // Type check the path expression - must be a string
                let path_type = self.infer_expression_type(path);
                if path_type != Type::Text && path_type != Type::Unknown && path_type != Type::Error
                {
                    self.type_error(
                        "Expected string for include path".to_string(),
                        Some(Type::Text),
                        Some(path_type),
                        *line,
                        *column,
                    );
                }
                // Note: Include statements execute in parent scope, making their symbols available
                // at runtime. However, the type checker doesn't currently parse included files,
                // which can result in false "not found" errors for symbols defined in included files.
                // Future improvement: Parse and analyze included files during type checking
                // to register their symbols in the current scope for more accurate diagnostics.
            }

            Statement::ExportStatement {
                export_type,
                name,
                line,
                column,
                ..
            } => {
                // Basic type checking for export statements
                // Check if the exported item exists in the current scope
                match export_type {
                    crate::parser::ast::ExportType::Container => {
                        if let Some(_container) = self.analyzer.get_container(name) {
                            // Container exists - export is valid
                        } else {
                            self.type_error(
                                format!("Container '{}' not found for export", name),
                                None,
                                None,
                                *line,
                                *column,
                            );
                        }
                    }
                    crate::parser::ast::ExportType::Action => {
                        // Check if action exists as a symbol in the current scope
                        if let Some(symbol) = self.analyzer.get_symbol(name) {
                            match &symbol.kind {
                                crate::analyzer::SymbolKind::Function { .. } => {
                                    // Action exists - export is valid
                                }
                                _ => {
                                    self.type_error(
                                        format!(
                                            "'{}' is not an action and cannot be exported as one",
                                            name
                                        ),
                                        None,
                                        None,
                                        *line,
                                        *column,
                                    );
                                }
                            }
                        } else {
                            self.type_error(
                                format!("Action '{}' not found for export", name),
                                None,
                                None,
                                *line,
                                *column,
                            );
                        }
                    }
                    crate::parser::ast::ExportType::Constant => {
                        // Check if variable exists as a symbol in the current scope
                        if let Some(symbol) = self.analyzer.get_symbol(name) {
                            match &symbol.kind {
                                crate::analyzer::SymbolKind::Variable { mutable } => {
                                    // Only immutable variables can be exported as constants
                                    if *mutable {
                                        self.type_error(
                                            format!(
                                                "'{}' is mutable and cannot be exported as constant",
                                                name
                                            ),
                                            None,
                                            None,
                                            *line,
                                            *column,
                                        );
                                    }
                                    // Otherwise, immutable variable is valid for constant export
                                }
                                _ => {
                                    self.type_error(
                                        format!("'{}' is not a variable and cannot be exported as constant", name),
                                        None,
                                        None,
                                        *line,
                                        *column,
                                    );
                                }
                            }
                        } else {
                            self.type_error(
                                format!("Constant '{}' not found for export", name),
                                None,
                                None,
                                *line,
                                *column,
                            );
                        }
                    }
                }
            }
        }
    }

    fn infer_expression_type(&mut self, expression: &Expression) -> Type {
        match expression {
            Expression::Literal(literal, _, _) => match literal {
                Literal::String(_) => Type::Text,
                Literal::Integer(_) => Type::Number,
                Literal::Float(_) => Type::Number,
                Literal::Boolean(_) => Type::Boolean,
                Literal::Nothing => Type::Nothing,
                Literal::Pattern(_) => Type::Pattern,
                Literal::List(_) => Type::List(Box::new(Type::Any)),
            },
            Expression::Variable(name, _line, _column) => {
                if let Some(symbol) = self.analyzer.get_symbol(name) {
                    if let Some(var_type) = &symbol.symbol_type {
                        var_type.clone()
                    } else {
                        self.type_error(
                            format!("Cannot determine type of variable '{name}'"),
                            None,
                            None,
                            *_line,
                            *_column,
                        );
                        Type::Unknown
                    }
                } else {
                    // Check if this is an action parameter, builtin function, or special function name before reporting it as undefined
                    if self.analyzer.get_action_parameters().contains(name)
                        || Analyzer::is_builtin_function(name)
                        || name == "helper_function"
                        || name == "nested_function"
                    {
                        // It's an action parameter or a special function name, so don't report an error
                        if name == "loopcounter" || name == "count" {
                            // Special case for loopcounter and count - they're Numbers
                            return Type::Number;
                        }

                        // For builtin functions, return their proper type
                        if Analyzer::is_builtin_function(name) {
                            let param_count = builtins::get_function_arity(name);
                            return Type::Function {
                                parameters: vec![Type::Any; param_count],
                                return_type: Box::new(
                                    self.get_builtin_function_type(name, param_count),
                                ),
                            };
                        }

                        Type::Unknown
                    } else {
                        // The analyzer already reports undefined variables, so we don't need to duplicate the error
                        // Return Unknown type to continue type checking without cascading errors
                        Type::Unknown
                    }
                }
            }
            Expression::BinaryOperation {
                left,
                operator,
                right,
                line,
                column,
            } => {
                let left_type = self.infer_expression_type(left);
                let right_type = self.infer_expression_type(right);

                if left_type == Type::Error || right_type == Type::Error {
                    return Type::Error;
                }

                if left_type == Type::Unknown || right_type == Type::Unknown {
                    return Type::Unknown;
                }

                match operator {
                    Operator::Plus => {
                        // Plus operation allows:
                        // - Number + Number = Number
                        // - Text + Text = Text
                        // - Text + Number = Text (automatic conversion)
                        // - Number + Text = Text (automatic conversion)
                        if left_type == Type::Number && right_type == Type::Number {
                            Type::Number
                        } else if left_type == Type::Text || right_type == Type::Text {
                            // If either operand is Text, the result is Text (automatic conversion)
                            Type::Text
                        } else {
                            self.type_error(
                                format!(
                                    "Cannot perform {operator:?} operation on {left_type} and {right_type}"
                                ),
                                Some(Type::Text),
                                Some(if left_type != Type::Text {
                                    left_type
                                } else {
                                    right_type
                                }),
                                *line,
                                *column,
                            );
                            Type::Error
                        }
                    }
                    Operator::Minus | Operator::Multiply | Operator::Divide | Operator::Modulo => {
                        // These operations require both operands to be numbers
                        if left_type == Type::Number && right_type == Type::Number {
                            Type::Number
                        } else {
                            self.type_error(
                                format!(
                                    "Cannot perform {operator:?} operation on {left_type} and {right_type}"
                                ),
                                Some(Type::Number),
                                Some(if left_type != Type::Number {
                                    left_type
                                } else {
                                    right_type
                                }),
                                *line,
                                *column,
                            );
                            Type::Error
                        }
                    }
                    Operator::Equals | Operator::NotEquals => {
                        if !self.are_types_compatible(&left_type, &right_type)
                            && !self.are_types_compatible(&right_type, &left_type)
                        {
                            self.type_error(
                                format!("Cannot compare {left_type} and {right_type} for equality"),
                                Some(left_type.clone()),
                                Some(right_type),
                                *line,
                                *column,
                            );
                            Type::Error
                        } else {
                            Type::Boolean
                        }
                    }
                    Operator::GreaterThan
                    | Operator::LessThan
                    | Operator::GreaterThanOrEqual
                    | Operator::LessThanOrEqual => {
                        if (left_type == Type::Number && right_type == Type::Number)
                            || (left_type == Type::Text && right_type == Type::Text)
                        {
                            Type::Boolean
                        } else {
                            self.type_error(
                                format!(
                                    "Cannot compare {left_type} and {right_type} with {operator:?}"
                                ),
                                Some(if left_type == Type::Number || left_type == Type::Text {
                                    left_type.clone()
                                } else {
                                    Type::Number
                                }),
                                Some(right_type),
                                *line,
                                *column,
                            );
                            Type::Error
                        }
                    }
                    Operator::And | Operator::Or => {
                        if left_type == Type::Boolean && right_type == Type::Boolean {
                            Type::Boolean
                        } else {
                            self.type_error(
                                format!(
                                    "Cannot perform logical {operator:?} on {left_type} and {right_type}"
                                ),
                                Some(Type::Boolean),
                                Some(if left_type != Type::Boolean {
                                    left_type
                                } else {
                                    right_type
                                }),
                                *line,
                                *column,
                            );
                            Type::Error
                        }
                    }
                    Operator::Contains => match &left_type {
                        Type::List(item_type) => {
                            if !self.are_types_compatible(item_type, &right_type) {
                                self.type_error(
                                    format!(
                                        "Cannot check if {left_type} contains {right_type}, list items are {item_type}"
                                    ),
                                    Some(*item_type.clone()),
                                    Some(right_type),
                                    *line,
                                    *column,
                                );
                                Type::Error
                            } else {
                                Type::Boolean
                            }
                        }
                        Type::Map(key_type, _) => {
                            if !self.are_types_compatible(key_type, &right_type) {
                                self.type_error(
                                    format!(
                                        "Cannot check if {left_type} contains {right_type}, map keys are {key_type}"
                                    ),
                                    Some(*key_type.clone()),
                                    Some(right_type),
                                    *line,
                                    *column,
                                );
                                Type::Error
                            } else {
                                Type::Boolean
                            }
                        }
                        Type::Text => {
                            if right_type != Type::Text {
                                self.type_error(
                                    format!("Cannot check if {left_type} contains {right_type}"),
                                    Some(Type::Text),
                                    Some(right_type),
                                    *line,
                                    *column,
                                );
                                Type::Error
                            } else {
                                Type::Boolean
                            }
                        }
                        _ => {
                            self.type_error(
                                format!("Cannot check if {left_type} contains {right_type}"),
                                Some(Type::List(Box::new(Type::Unknown))),
                                Some(left_type),
                                *line,
                                *column,
                            );
                            Type::Error
                        }
                    },
                }
            }
            Expression::UnaryOperation {
                operator,
                expression,
                line,
                column,
            } => {
                let expr_type = self.infer_expression_type(expression);

                if expr_type == Type::Error {
                    return Type::Error;
                }

                match operator {
                    UnaryOperator::Not => {
                        if expr_type == Type::Boolean {
                            Type::Boolean
                        } else {
                            self.type_error(
                                format!("Cannot apply 'not' to {expr_type}"),
                                Some(Type::Boolean),
                                Some(expr_type),
                                *line,
                                *column,
                            );
                            Type::Error
                        }
                    }
                    UnaryOperator::Minus => {
                        if expr_type == Type::Number {
                            Type::Number
                        } else {
                            self.type_error(
                                format!("Cannot negate {expr_type}"),
                                Some(Type::Number),
                                Some(expr_type),
                                *line,
                                *column,
                            );
                            Type::Error
                        }
                    }
                }
            }
            Expression::FunctionCall {
                function,
                arguments,
                line,
                column,
            } => {
                let function_type = self.infer_expression_type(function);

                match function_type {
                    Type::Function {
                        parameters,
                        return_type,
                    } => {
                        if arguments.len() != parameters.len() {
                            self.type_error(
                                format!(
                                    "Function expects {} arguments, but {} were provided",
                                    parameters.len(),
                                    arguments.len()
                                ),
                                None,
                                None,
                                *line,
                                *column,
                            );
                            return Type::Error;
                        }

                        let mut has_type_error = false;
                        for (i, (arg, param_type)) in
                            arguments.iter().zip(parameters.iter()).enumerate()
                        {
                            let arg_type = self.infer_expression_type(&arg.value);
                            if !self.are_types_compatible(param_type, &arg_type) {
                                self.type_error(
                                    format!(
                                        "Argument {} has incorrect type: expected {}, found {}",
                                        i + 1,
                                        param_type,
                                        arg_type
                                    ),
                                    Some(param_type.clone()),
                                    Some(arg_type),
                                    *line,
                                    *column,
                                );
                                has_type_error = true;
                            }
                        }

                        if has_type_error {
                            Type::Error
                        } else {
                            *return_type
                        }
                    }
                    Type::Unknown | Type::Error => Type::Unknown,
                    _ => {
                        self.type_error(
                            format!("Cannot call {function_type}, not a function"),
                            Some(Type::Function {
                                parameters: vec![],
                                return_type: Box::new(Type::Unknown),
                            }),
                            Some(function_type),
                            *line,
                            *column,
                        );
                        Type::Error
                    }
                }
            }
            Expression::MemberAccess {
                object,
                property,
                line: _line,
                column: _column,
            } => {
                let object_type = self.infer_expression_type(object);

                if object_type == Type::Error {
                    return Type::Error;
                }

                match object_type {
                    Type::Custom(_) => Type::Unknown,
                    Type::Unknown => Type::Unknown,
                    _ => {
                        self.type_error(
                            format!("Cannot access property '{property}' on {object_type}"),
                            Some(Type::Custom("Object".to_string())),
                            Some(object_type),
                            *_line,
                            *_column,
                        );
                        Type::Error
                    }
                }
            }
            Expression::IndexAccess {
                collection,
                index,
                line,
                column,
            } => {
                let collection_type = self.infer_expression_type(collection);
                let index_type = self.infer_expression_type(index);

                if collection_type == Type::Error || index_type == Type::Error {
                    return Type::Error;
                }

                match collection_type {
                    Type::List(item_type) => {
                        if index_type != Type::Number {
                            self.type_error(
                                format!("List index must be a number, got {index_type}"),
                                Some(Type::Number),
                                Some(index_type),
                                *line,
                                *column,
                            );
                            Type::Error
                        } else {
                            *item_type
                        }
                    }
                    Type::Map(key_type, value_type) => {
                        if !self.are_types_compatible(&key_type, &index_type) {
                            self.type_error(
                                format!("Map key must be {key_type}, got {index_type}"),
                                Some(*key_type.clone()),
                                Some(index_type),
                                *line,
                                *column,
                            );
                            Type::Error
                        } else {
                            *value_type
                        }
                    }
                    Type::Text => {
                        if index_type != Type::Number {
                            self.type_error(
                                format!("Text index must be a number, got {index_type}"),
                                Some(Type::Number),
                                Some(index_type),
                                *line,
                                *column,
                            );
                            Type::Error
                        } else {
                            Type::Text
                        }
                    }
                    Type::Unknown => Type::Unknown,
                    _ => {
                        self.type_error(
                            format!("Cannot index into {collection_type}"),
                            Some(Type::List(Box::new(Type::Unknown))),
                            Some(collection_type),
                            *line,
                            *column,
                        );
                        Type::Error
                    }
                }
            }
            Expression::Concatenation {
                left,
                right,
                line: _line,
                column: _column,
            } => {
                let left_type = self.infer_expression_type(left);
                let right_type = self.infer_expression_type(right);

                if left_type == Type::Error || right_type == Type::Error {
                    return Type::Error;
                }

                // Allow concatenation of any types - they will be converted to text at runtime
                // This matches the interpreter's behavior which converts values to strings
                Type::Text
            }
            Expression::PatternMatch {
                text,
                pattern,
                line,
                column,
            } => {
                let text_type = self.infer_expression_type(text);
                let pattern_type = self.infer_expression_type(pattern);

                if text_type != Type::Text && text_type != Type::Unknown {
                    self.type_error(
                        format!("Expected Text for pattern matching, got {text_type}"),
                        Some(Type::Text),
                        Some(text_type),
                        *line,
                        *column,
                    );
                }

                if pattern_type != Type::Pattern
                    && pattern_type != Type::Text
                    && pattern_type != Type::Unknown
                {
                    self.type_error(
                        format!("Expected Pattern for pattern matching, got {pattern_type}"),
                        Some(Type::Pattern),
                        Some(pattern_type),
                        *line,
                        *column,
                    );
                }

                Type::Boolean
            }
            Expression::PatternFind { text, pattern, .. } => {
                let text_type = self.infer_expression_type(text);
                let pattern_type = self.infer_expression_type(pattern);

                if text_type != Type::Text {
                    self.type_error(
                        format!("Expected Text for pattern finding, got {text_type}"),
                        Some(Type::Text),
                        Some(text_type),
                        0,
                        0,
                    );
                }

                if pattern_type != Type::Pattern && pattern_type != Type::Text {
                    self.type_error(
                        format!("Expected Pattern for pattern finding, got {pattern_type}"),
                        Some(Type::Pattern),
                        Some(pattern_type),
                        0,
                        0,
                    );
                }

                Type::Map(Box::new(Type::Text), Box::new(Type::Nothing))
            }
            Expression::PatternReplace {
                text,
                pattern,
                replacement,
                ..
            } => {
                let text_type = self.infer_expression_type(text);
                let pattern_type = self.infer_expression_type(pattern);
                let replacement_type = self.infer_expression_type(replacement);

                if text_type != Type::Text {
                    self.type_error(
                        format!("Expected Text for pattern replacement, got {text_type}"),
                        Some(Type::Text),
                        Some(text_type),
                        0,
                        0,
                    );
                }

                if pattern_type != Type::Pattern && pattern_type != Type::Text {
                    self.type_error(
                        format!("Expected Pattern for pattern replacement, got {pattern_type}"),
                        Some(Type::Pattern),
                        Some(pattern_type),
                        0,
                        0,
                    );
                }

                if replacement_type != Type::Text {
                    self.type_error(
                        format!("Expected Text for replacement, got {replacement_type}"),
                        Some(Type::Text),
                        Some(replacement_type),
                        0,
                        0,
                    );
                }

                Type::Text
            }
            Expression::PatternSplit { text, pattern, .. } => {
                let text_type = self.infer_expression_type(text);
                let pattern_type = self.infer_expression_type(pattern);

                if text_type != Type::Text {
                    self.type_error(
                        format!("Expected Text for pattern splitting, got {text_type}"),
                        Some(Type::Text),
                        Some(text_type),
                        0,
                        0,
                    );
                }

                if pattern_type != Type::Pattern && pattern_type != Type::Text {
                    self.type_error(
                        format!("Expected Pattern for pattern splitting, got {pattern_type}"),
                        Some(Type::Pattern),
                        Some(pattern_type),
                        0,
                        0,
                    );
                }

                Type::List(Box::new(Type::Text))
            }
            Expression::StringSplit {
                text,
                delimiter,
                line,
                column,
            } => {
                let text_type = self.infer_expression_type(text);
                let delimiter_type = self.infer_expression_type(delimiter);

                if text_type != Type::Text {
                    self.type_error(
                        format!("Expected Text for string splitting, got {text_type}"),
                        Some(Type::Text),
                        Some(text_type),
                        *line,
                        *column,
                    );
                }

                if delimiter_type != Type::Text {
                    self.type_error(
                        format!("Expected Text for delimiter, got {delimiter_type}"),
                        Some(Type::Text),
                        Some(delimiter_type),
                        *line,
                        *column,
                    );
                }

                Type::List(Box::new(Type::Text))
            }
            Expression::AwaitExpression {
                expression,
                line,
                column,
            } => {
                let expr_type = self.infer_expression_type(expression);

                match expr_type {
                    Type::Async(inner_type) => *inner_type,
                    _ => {
                        self.type_error(
                            format!("Cannot await non-async value of type {expr_type}"),
                            Some(Type::Async(Box::new(Type::Unknown))),
                            Some(expr_type),
                            *line,
                            *column,
                        );
                        Type::Error
                    }
                }
            }
            Expression::ActionCall {
                name,
                arguments,
                line: _line,
                column: _column,
            } => {
                // For builtin functions, use special handling (variadic support, etc.)
                if Analyzer::is_builtin_function(name) {
                    return self.get_builtin_function_type(name, arguments.len());
                }

                let symbol_opt = self.analyzer.get_symbol(name);

                if symbol_opt.is_none() {
                    // Check if this is an action parameter, builtin function, or special function name before reporting it as undefined
                    if self.analyzer.get_action_parameters().contains(name)
                        || Analyzer::is_builtin_function(name)
                        || name == "helper_function"
                        || name == "nested_function"
                    {
                        // It's an action parameter or a special function name, so don't report an error
                        // For builtin functions, return their proper type
                        if Analyzer::is_builtin_function(name) {
                            return self.get_builtin_function_type(name, arguments.len());
                        }
                        return Type::Unknown;
                    } else {
                        self.type_error(
                            format!("Undefined action '{name}'"),
                            None,
                            None,
                            *_line,
                            *_column,
                        );
                        return Type::Error;
                    }
                }

                let symbol = symbol_opt.unwrap();

                if symbol.symbol_type.is_none() {
                    self.type_error(
                        format!("Cannot determine type of action '{name}'"),
                        None,
                        None,
                        *_line,
                        *_column,
                    );
                    return Type::Unknown;
                }

                let symbol_type = symbol.symbol_type.clone().unwrap();

                match symbol_type {
                    Type::Function {
                        parameters,
                        return_type,
                    } => {
                        if arguments.len() != parameters.len() {
                            self.type_error(
                                format!(
                                    "Action '{}' expects {} arguments, but {} were provided",
                                    name,
                                    parameters.len(),
                                    arguments.len()
                                ),
                                None,
                                None,
                                *_line,
                                *_column,
                            );
                            return Type::Error;
                        }

                        let mut arg_types = Vec::with_capacity(arguments.len());
                        for arg in arguments {
                            arg_types.push(self.infer_expression_type(&arg.value));
                        }

                        for (i, (param_type, arg_type)) in
                            parameters.iter().zip(arg_types.iter()).enumerate()
                        {
                            if !self.are_types_compatible(param_type, arg_type) {
                                self.type_error(
                                    format!(
                                        "Argument {} of action '{}' expects {}, but got {}",
                                        i + 1,
                                        name,
                                        param_type,
                                        arg_type
                                    ),
                                    Some(param_type.clone()),
                                    Some(arg_type.clone()),
                                    *_line,
                                    *_column,
                                );
                                return Type::Error;
                            }
                        }

                        *return_type
                    }
                    _ => {
                        self.type_error(
                            format!("'{name}' is not an action"),
                            Some(Type::Function {
                                parameters: vec![],
                                return_type: Box::new(Type::Unknown),
                            }),
                            Some(symbol_type),
                            *_line,
                            *_column,
                        );
                        Type::Error
                    }
                }
            }
            Expression::StaticMemberAccess {
                container,
                member,
                line,
                column,
            } => {
                // Look up the container in the analyzer's registry
                if let Some(container_info) = self.analyzer.get_container(container) {
                    // First check static properties
                    if let Some(prop_info) = container_info.static_properties.get(member) {
                        return prop_info.property_type.clone();
                    }

                    // Then check static methods
                    if let Some(method_info) = container_info.static_methods.get(member) {
                        return Type::Function {
                            parameters: method_info
                                .parameters
                                .iter()
                                .map(|p| p.param_type.as_ref().cloned().unwrap_or(Type::Unknown))
                                .collect(),
                            return_type: Box::new(method_info.return_type.clone()),
                        };
                    }

                    // Member not found
                    self.errors.push(TypeError::new(
                        format!("Static member '{member}' not found in container '{container}'"),
                        None,
                        None,
                        *line,
                        *column,
                    ));
                    Type::Error
                } else {
                    // Container not found
                    self.errors.push(TypeError::new(
                        format!("Container '{container}' not found"),
                        None,
                        None,
                        *line,
                        *column,
                    ));
                    Type::Error
                }
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                line,
                column,
            } => {
                // First, determine the type of the object
                let object_type = self.infer_expression_type(object);

                // Check if the object is a container instance
                match object_type {
                    Type::ContainerInstance(container_name) => {
                        // Look up the container in the analyzer's registry
                        if let Some(container_info) = self.analyzer.get_container(&container_name) {
                            // Look up the method in the container
                            if let Some(method_info) = container_info.methods.get(method) {
                                // Check argument count
                                let param_count = method_info.parameters.len();
                                let return_type = method_info.return_type.clone();
                                let method_params = method_info.parameters.clone();

                                if arguments.len() != param_count {
                                    self.errors.push(TypeError::new(
                                        format!(
                                            "Method '{}' expects {} arguments but {} were provided",
                                            method,
                                            param_count,
                                            arguments.len()
                                        ),
                                        None,
                                        None,
                                        *line,
                                        *column,
                                    ));
                                }

                                // Check argument types
                                for (i, (arg, param)) in
                                    arguments.iter().zip(&method_params).enumerate()
                                {
                                    let arg_type = self.infer_expression_type(&arg.value);
                                    let expected_type =
                                        param.param_type.as_ref().cloned().unwrap_or(Type::Unknown);

                                    if arg_type != Type::Unknown
                                        && expected_type != Type::Unknown
                                        && arg_type != expected_type
                                    {
                                        self.errors.push(TypeError::new(
                                            format!(
                                                "Argument {} of method '{}' has type {} but expected {}",
                                                i + 1,
                                                method,
                                                arg_type,
                                                expected_type
                                            ),
                                            Some(expected_type),
                                            Some(arg_type),
                                            *line,
                                            *column,
                                        ));
                                    }
                                }

                                // Return the method's return type
                                return_type
                            } else {
                                // Check parent containers if the method is not found
                                let mut current_container = container_info.extends.as_ref();
                                let mut found_method = None;

                                while let Some(parent_name) = current_container {
                                    if let Some(parent_info) =
                                        self.analyzer.get_container(parent_name)
                                    {
                                        if let Some(method_info) = parent_info.methods.get(method) {
                                            found_method = Some((
                                                method_info.parameters.clone(),
                                                method_info.return_type.clone(),
                                            ));
                                            break;
                                        }
                                        current_container = parent_info.extends.as_ref();
                                    } else {
                                        break;
                                    }
                                }

                                if let Some((method_params, return_type)) = found_method {
                                    // Found in parent - do the same checks
                                    if arguments.len() != method_params.len() {
                                        self.errors.push(TypeError::new(
                                            format!(
                                                "Method '{}' expects {} arguments but {} were provided",
                                                method,
                                                method_params.len(),
                                                arguments.len()
                                            ),
                                            None,
                                            None,
                                            *line,
                                            *column,
                                        ));
                                    }

                                    for (i, (arg, param)) in
                                        arguments.iter().zip(&method_params).enumerate()
                                    {
                                        let arg_type = self.infer_expression_type(&arg.value);
                                        let expected_type = param
                                            .param_type
                                            .as_ref()
                                            .cloned()
                                            .unwrap_or(Type::Unknown);

                                        if arg_type != Type::Unknown
                                            && expected_type != Type::Unknown
                                            && arg_type != expected_type
                                        {
                                            self.errors.push(TypeError::new(
                                                format!(
                                                    "Argument {} of method '{}' has type {} but expected {}",
                                                    i + 1,
                                                    method,
                                                    arg_type,
                                                    expected_type
                                                ),
                                                Some(expected_type),
                                                Some(arg_type),
                                                *line,
                                                *column,
                                            ));
                                        }
                                    }

                                    return_type
                                } else {
                                    self.errors.push(TypeError::new(
                                        format!(
                                            "Method '{method}' not found in container '{container_name}'"
                                        ),
                                        None,
                                        None,
                                        *line,
                                        *column,
                                    ));
                                    Type::Error
                                }
                            }
                        } else {
                            self.errors.push(TypeError::new(
                                format!("Container '{container_name}' not found"),
                                None,
                                None,
                                *line,
                                *column,
                            ));
                            Type::Error
                        }
                    }
                    _ => {
                        self.type_error(
                            format!(
                                "Cannot call method '{method}' on non-container type {object_type}"
                            ),
                            Some(Type::ContainerInstance(String::from("Unknown"))),
                            Some(object_type),
                            *line,
                            *column,
                        );
                        Type::Error
                    }
                }
            }
            Expression::PropertyAccess {
                object,
                property,
                line,
                column,
            } => {
                let object_type = self.infer_expression_type(object);
                match object_type {
                    Type::ContainerInstance(container_name) => {
                        // Look up the container in the analyzer's registry
                        if let Some(container_info) = self.analyzer.get_container(&container_name) {
                            // Look up the property in the container
                            if let Some(prop_info) = container_info.properties.get(property) {
                                prop_info.property_type.clone()
                            } else {
                                // Check parent containers if property not found
                                let mut current_container = container_info.extends.as_ref();
                                let mut found = false;
                                let mut prop_type = Type::Unknown;

                                while let Some(parent_name) = current_container {
                                    if let Some(parent_info) =
                                        self.analyzer.get_container(parent_name)
                                    {
                                        if let Some(prop_info) =
                                            parent_info.properties.get(property)
                                        {
                                            found = true;
                                            prop_type = prop_info.property_type.clone();
                                            break;
                                        }
                                        current_container = parent_info.extends.as_ref();
                                    } else {
                                        break;
                                    }
                                }

                                if !found {
                                    self.errors.push(TypeError::new(
                                        format!(
                                            "Property '{property}' not found in container '{container_name}'"
                                        ),
                                        None,
                                        None,
                                        *line,
                                        *column,
                                    ));
                                    Type::Error
                                } else {
                                    prop_type
                                }
                            }
                        } else {
                            self.errors.push(TypeError::new(
                                format!("Container '{container_name}' not found"),
                                None,
                                None,
                                *line,
                                *column,
                            ));
                            Type::Error
                        }
                    }
                    _ => {
                        self.type_error(
                            format!(
                                "Cannot access property '{property}' on non-container type {object_type}"
                            ),
                            Some(Type::ContainerInstance("Unknown".to_string())),
                            Some(object_type),
                            *line,
                            *column,
                        );
                        Type::Error
                    }
                }
            }
            Expression::FileExists { .. } => Type::Boolean,
            Expression::DirectoryExists { .. } => Type::Boolean,
            Expression::ListFiles { .. } => Type::List(Box::new(Type::Text)),
            Expression::ReadContent { .. } => Type::Text,
            Expression::ListFilesRecursive { .. } => Type::List(Box::new(Type::Text)),
            Expression::ListFilesFiltered { .. } => Type::List(Box::new(Type::Text)),
            Expression::HeaderAccess { .. } => Type::Text,
            Expression::CurrentTimeMilliseconds { .. } => Type::Number,
            Expression::CurrentTimeFormatted { .. } => Type::Text,
            Expression::ProcessRunning { .. } => Type::Boolean,
        }
    }

    fn validate_signal_handler_statement(
        &mut self,
        signal_type: &str,
        handler_name: &str,
        line: usize,
        column: usize,
    ) {
        // Validate signal type
        // List of valid signals based on common POSIX signals that are handleable
        // SIGKILL and SIGSTOP cannot be handled, so they are excluded
        let valid_signals = [
            "SIGINT", "SIGTERM", "SIGHUP", "SIGQUIT", "SIGABRT", "SIGALRM", "SIGCHLD", "SIGCONT",
            "SIGPIPE", "SIGUSR1", "SIGUSR2", "INT", "TERM", "HUP", "QUIT", "ABRT", "ALRM", "CHLD",
            "CONT", "PIPE", "USR1", "USR2",
        ];

        if !valid_signals.contains(&signal_type) {
            self.type_error(
                format!(
                    "Invalid signal type: '{signal_type}'. Supported signals include: {}",
                    valid_signals.join(", ")
                ),
                None,
                None,
                line,
                column,
            );
        }

        // Validate handler name
        if let Some(symbol) = self.analyzer.get_symbol(handler_name) {
            if let Some(symbol_type) = &symbol.symbol_type {
                match symbol_type {
                    Type::Function {
                        parameters,
                        return_type: _,
                    } => {
                        // Handler should accept 0 or 1 arguments
                        if parameters.len() > 1 {
                            self.type_error(
                                format!(
                                    "Signal handler '{handler_name}' must accept 0 or 1 arguments, but accepts {}",
                                    parameters.len()
                                ),
                                None,
                                None,
                                line,
                                column,
                            );
                        } else if parameters.len() == 1 {
                            // If it accepts an argument, it must be a Number (the signal number)
                            // Also allow Unknown for backward compatibility with untyped parameters
                            let param_type = &parameters[0];
                            if *param_type != Type::Number && *param_type != Type::Unknown {
                                self.type_error(
                                    format!(
                                        "Signal handler parameter must be a Number (signal code), but got {}",
                                        param_type
                                    ),
                                    Some(Type::Number),
                                    Some(param_type.clone()),
                                    line,
                                    column,
                                );
                            }
                        }
                    }
                    _ => {
                        self.type_error(
                            format!("'{handler_name}' is not a function"),
                            Some(Type::Function {
                                parameters: vec![],
                                return_type: Box::new(Type::Nothing),
                            }),
                            Some(symbol_type.clone()),
                            line,
                            column,
                        );
                    }
                }
            } else {
                self.type_error(
                    format!("Cannot determine type of signal handler '{handler_name}'"),
                    None,
                    None,
                    line,
                    column,
                );
            }
        } else {
            self.type_error(
                format!("Undefined signal handler '{handler_name}'"),
                None,
                None,
                line,
                column,
            );
        }
    }

    fn validate_server_operand(&mut self, server: &Expression, line: usize, column: usize) {
        let server_type = self.infer_expression_type(server);
        if server_type != Type::Custom("Server".to_string())
            && server_type != Type::Text
            && server_type != Type::Unknown
            && server_type != Type::Error
        {
            self.type_error(
                "Expected a Server object or server name (text)".to_string(),
                Some(Type::Custom("Server".to_string())),
                Some(server_type),
                line,
                column,
            );
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn check_return_statements(
        &mut self,
        statements: &[Statement],
        expected_type: &Type,
        line: usize,
        column: usize,
    ) {
        for statement in statements {
            match statement {
                Statement::ReturnStatement {
                    value,
                    line,
                    column,
                } => {
                    if let Some(expr) = value {
                        let return_type = self.infer_expression_type(expr);
                        if !self.are_types_compatible(expected_type, &return_type) {
                            self.type_error(
                                "Return statement has incorrect type".to_string(),
                                Some(expected_type.clone()),
                                Some(return_type),
                                *line,
                                *column,
                            );
                        }
                    } else if *expected_type != Type::Nothing {
                        self.type_error(
                            "Function must return a value".to_string(),
                            Some(expected_type.clone()),
                            Some(Type::Nothing),
                            *line,
                            *column,
                        );
                    }
                }
                Statement::IfStatement {
                    then_block,
                    else_block,
                    ..
                } => {
                    self.check_return_statements(then_block, expected_type, line, column);
                    if let Some(else_stmts) = else_block {
                        self.check_return_statements(else_stmts, expected_type, line, column);
                    }
                }
                Statement::SingleLineIf {
                    then_stmt,
                    else_stmt,
                    ..
                } => {
                    self.check_return_statements(
                        &[*(*then_stmt).clone()],
                        expected_type,
                        line,
                        column,
                    );
                    if let Some(else_stmt) = else_stmt {
                        self.check_return_statements(
                            &[*(*else_stmt).clone()],
                            expected_type,
                            line,
                            column,
                        );
                    }
                }
                Statement::ForEachLoop { body, .. }
                | Statement::CountLoop { body, .. }
                | Statement::WhileLoop { body, .. }
                | Statement::RepeatUntilLoop { body, .. }
                | Statement::ForeverLoop { body, .. }
                | Statement::MainLoop { body, .. } => {
                    self.check_return_statements(body, expected_type, line, column);
                }
                _ => {}
            }
        }
    }

    fn type_error(
        &mut self,
        message: String,
        expected: Option<Type>,
        found: Option<Type>,
        line: usize,
        column: usize,
    ) {
        self.errors
            .push(TypeError::new(message, expected, found, line, column));
    }

    fn are_types_compatible(&self, target_type: &Type, source_type: &Type) -> bool {
        #[allow(clippy::only_used_in_recursion)]
        let _self = self; // Suppress the warning for self parameter
        match (target_type, source_type) {
            (a, b) if a == b => true,

            (Type::Unknown, _) => true,
            (_, Type::Unknown) => true, // Unknown can be assigned to any type

            (Type::Any, _) => true, // Any can accept any type
            (_, Type::Any) => true, // Any can be assigned to any type

            (_, Type::Nothing) => true,

            (_, Type::Error) => true,

            (inner, Type::Async(async_type)) => self.are_types_compatible(inner, async_type),

            (Type::List(a), Type::List(b)) => self.are_types_compatible(a, b),
            (Type::Map(a_key, a_val), Type::Map(b_key, b_val)) => {
                self.are_types_compatible(a_key, b_key) && self.are_types_compatible(a_val, b_val)
            }

            (
                Type::Function {
                    parameters: a_params,
                    return_type: a_ret,
                },
                Type::Function {
                    parameters: b_params,
                    return_type: b_ret,
                },
            ) => {
                if a_params.len() != b_params.len() {
                    return false;
                }

                for (a, b) in a_params.iter().zip(b_params.iter()) {
                    if !self.are_types_compatible(a, b) {
                        return false;
                    }
                }

                self.are_types_compatible(a_ret, b_ret)
            }

            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Argument, Expression, Literal, Parameter, Program, Statement, Type};

    #[test]
    fn test_variable_declaration_type_inference() {
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

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&program);
        assert!(result.is_ok(), "Expected no type errors");
    }

    #[test]
    fn test_type_mismatch_in_assignment() {
        let program = Program {
            statements: vec![
                Statement::VariableDeclaration {
                    name: "x".to_string(),
                    value: Expression::Literal(Literal::Integer(10), 1, 1),
                    is_constant: false,
                    line: 1,
                    column: 1,
                },
                Statement::Assignment {
                    name: "x".to_string(),
                    value: Expression::Literal(Literal::String("hello".to_string()), 2, 1),
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&program);
        assert!(result.is_err(), "Expected type error for mismatched types");

        let errors = result.err().unwrap();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("incompatible type"))
        );
    }

    #[test]
    fn test_binary_operation_type_checking() {
        let program = Program {
            statements: vec![Statement::VariableDeclaration {
                name: "x".to_string(),
                is_constant: false,
                value: Expression::BinaryOperation {
                    left: Box::new(Expression::Literal(
                        Literal::String("hello".to_string()),
                        1,
                        5,
                    )),
                    operator: crate::parser::ast::Operator::Plus,
                    right: Box::new(Expression::Literal(
                        Literal::String("world".to_string()),
                        1,
                        10,
                    )),
                    line: 1,
                    column: 5,
                },
                line: 1,
                column: 1,
            }],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&program);
        assert!(
            result.is_ok(),
            "Expected no type errors for string concatenation with +"
        );

        let program = Program {
            statements: vec![Statement::VariableDeclaration {
                name: "x".to_string(),
                is_constant: false,
                value: Expression::BinaryOperation {
                    left: Box::new(Expression::Literal(Literal::Integer(10), 1, 5)),
                    operator: crate::parser::ast::Operator::Minus,
                    right: Box::new(Expression::Literal(
                        Literal::String("hello".to_string()),
                        1,
                        10,
                    )),
                    line: 1,
                    column: 5,
                },
                line: 1,
                column: 1,
            }],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&program);
        assert!(
            result.is_err(),
            "Expected type error for incompatible operation"
        );

        let errors = result.err().unwrap();
        assert!(errors.iter().any(|e| e.message.contains("Cannot perform")));
    }

    #[test]
    fn test_function_call_type_checking() {
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
                            value: Expression::Literal(Literal::Integer(123), 3, 7),
                        }],
                        line: 3,
                        column: 1,
                    },
                    line: 3,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&program);
        assert!(
            result.is_err(),
            "Expected type error for wrong argument type"
        );

        let errors = result.err().unwrap();
        assert!(errors.iter().any(|e| e.message.contains("incorrect type")));
    }

    #[test]
    fn test_conditional_type_checking() {
        let program = Program {
            statements: vec![Statement::IfStatement {
                condition: Expression::Literal(Literal::Integer(1), 1, 10),
                then_block: vec![],
                else_block: None,
                line: 1,
                column: 1,
            }],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&program);
        assert!(
            result.is_err(),
            "Expected type error for non-boolean condition"
        );

        let errors = result.err().unwrap();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Condition must be a boolean"))
        );
    }

    #[test]
    fn test_async_type_compatibility() {
        assert!(
            TypeChecker::new()
                .are_types_compatible(&Type::Number, &Type::Async(Box::new(Type::Number)))
        );

        assert!(
            !TypeChecker::new()
                .are_types_compatible(&Type::Text, &Type::Async(Box::new(Type::Number)))
        );
    }

    #[test]
    fn test_foreach_type_inference() {
        let program = Program {
            statements: vec![
                Statement::CreateListStatement {
                    name: "numbers".to_string(),
                    initial_values: vec![Expression::Literal(Literal::Integer(1), 1, 1)],
                    line: 1,
                    column: 1,
                },
                Statement::ForEachLoop {
                    item_name: "item".to_string(),
                    collection: Expression::Variable("numbers".to_string(), 2, 10),
                    reversed: false,
                    body: vec![Statement::ExpressionStatement {
                        expression: Expression::BinaryOperation {
                            left: Box::new(Expression::Variable("item".to_string(), 3, 5)),
                            operator: crate::parser::ast::Operator::Minus,
                            right: Box::new(Expression::Literal(
                                Literal::String("text".to_string()),
                                3,
                                12,
                            )),
                            line: 3,
                            column: 10,
                        },
                        line: 3,
                        column: 1,
                    }],
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&program);
        assert!(
            result.is_err(),
            "Expected type error for incompatible operation on loop variable"
        );

        let errors = result.err().unwrap();
        assert!(
            errors.iter().any(|e| e.message.contains("Cannot perform")),
            "Expected error about invalid operation, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_server_statement_type_checking() {
        // Test case 1: Valid server variable (mocked as Custom("Server"))
        let valid_server_var = Program {
            statements: vec![Statement::WaitForRequestStatement {
                server: Expression::Variable("myServer".to_string(), 1, 30),
                request_name: "req".to_string(),
                timeout: None,
                line: 1,
                column: 1,
            }],
        };

        let mut type_checker = TypeChecker::new();
        // Manually inject the server symbol to simulate a ListenStatement having run
        type_checker
            .analyzer
            .define_symbol(Symbol {
                name: "myServer".to_string(),
                kind: SymbolKind::Variable { mutable: false },
                symbol_type: Some(Type::Custom("Server".to_string())),
                line: 0,
                column: 0,
            })
            .unwrap();

        let result = type_checker.check_types(&valid_server_var);
        assert!(
            result.is_ok(),
            "Expected valid server variable to pass type checking"
        );

        // Test case 2: Valid text server name
        let valid_text_server = Program {
            statements: vec![Statement::WaitForRequestStatement {
                server: Expression::Literal(Literal::String("server_name".to_string()), 1, 30),
                request_name: "req".to_string(),
                timeout: None,
                line: 1,
                column: 1,
            }],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&valid_text_server);
        assert!(
            result.is_ok(),
            "Expected valid text server name to pass type checking"
        );

        // Test case 3: Invalid server type (Number)
        let invalid_server_type = Program {
            statements: vec![Statement::WaitForRequestStatement {
                server: Expression::Literal(Literal::Integer(123), 1, 30),
                request_name: "req".to_string(),
                timeout: None,
                line: 1,
                column: 1,
            }],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&invalid_server_type);
        assert!(
            result.is_err(),
            "Expected type error for numeric server name"
        );
        let errors = result.err().unwrap();
        assert!(
            errors.iter().any(|e| e
                .message
                .contains("Expected a Server object or server name")),
            "Error message should mention expected types"
        );

        // Test case 4: Valid timeout
        let valid_timeout = Program {
            statements: vec![Statement::WaitForRequestStatement {
                server: Expression::Literal(Literal::String("server".to_string()), 1, 30),
                request_name: "req".to_string(),
                timeout: Some(Expression::Literal(Literal::Integer(5000), 1, 50)),
                line: 1,
                column: 1,
            }],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&valid_timeout);
        assert!(
            result.is_ok(),
            "Expected valid timeout to pass type checking"
        );

        // Test case 5: Invalid timeout type
        let invalid_timeout = Program {
            statements: vec![Statement::WaitForRequestStatement {
                server: Expression::Literal(Literal::String("server".to_string()), 1, 30),
                request_name: "req".to_string(),
                timeout: Some(Expression::Literal(
                    Literal::String("5s".to_string()),
                    1,
                    50,
                )),
                line: 1,
                column: 1,
            }],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&invalid_timeout);
        assert!(
            result.is_err(),
            "Expected type error for invalid timeout type"
        );
        let errors = result.err().unwrap();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Timeout must be a number")),
            "Error message should mention timeout type requirement"
        );

        // Test case 6: Close server validation
        let invalid_close = Program {
            statements: vec![Statement::CloseServerStatement {
                server: Expression::Literal(Literal::Integer(123), 1, 15),
                line: 1,
                column: 1,
            }],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&invalid_close);
        assert!(
            result.is_err(),
            "Expected type error for closing numeric server"
        );
    }

    #[test]
    fn test_signal_handler_type_checking() {
        // Test case 1: Valid registration
        let valid_program = Program {
            statements: vec![
                Statement::ActionDefinition {
                    name: "handler".to_string(),
                    parameters: vec![],
                    body: vec![],
                    return_type: None,
                    line: 1,
                    column: 1,
                },
                Statement::RegisterSignalHandlerStatement {
                    signal_type: "SIGINT".to_string(),
                    handler_name: "handler".to_string(),
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&valid_program);
        assert!(
            result.is_ok(),
            "Expected valid signal registration to pass type checking"
        );

        // Test case 2: Invalid signal name (using unhandleable signal)
        let invalid_signal = Program {
            statements: vec![
                Statement::ActionDefinition {
                    name: "handler".to_string(),
                    parameters: vec![],
                    body: vec![],
                    return_type: None,
                    line: 1,
                    column: 1,
                },
                Statement::RegisterSignalHandlerStatement {
                    signal_type: "SIGKILL".to_string(),
                    handler_name: "handler".to_string(),
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&invalid_signal);
        assert!(
            result.is_err(),
            "Expected type error for unhandleable signal name (SIGKILL)"
        );
        let errors = result.err().unwrap();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Invalid signal type"))
        );

        // Test case 3: Undefined handler
        let undefined_handler = Program {
            statements: vec![Statement::RegisterSignalHandlerStatement {
                signal_type: "SIGINT".to_string(),
                handler_name: "unknown_handler".to_string(),
                line: 1,
                column: 1,
            }],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&undefined_handler);
        assert!(result.is_err(), "Expected type error for undefined handler");
        let errors = result.err().unwrap();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Undefined signal handler"))
        );

        // Test case 4: Handler is not a function
        let not_a_function = Program {
            statements: vec![
                Statement::VariableDeclaration {
                    name: "handler".to_string(),
                    value: Expression::Literal(Literal::Integer(1), 1, 1),
                    is_constant: false,
                    line: 1,
                    column: 1,
                },
                Statement::RegisterSignalHandlerStatement {
                    signal_type: "SIGINT".to_string(),
                    handler_name: "handler".to_string(),
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&not_a_function);
        assert!(
            result.is_err(),
            "Expected type error when handler is not a function"
        );
        let errors = result.err().unwrap();
        assert!(errors.iter().any(|e| e.message.contains("not a function")));

        // Test case 5: Handler has too many parameters
        let too_many_params = Program {
            statements: vec![
                Statement::ActionDefinition {
                    name: "handler".to_string(),
                    parameters: vec![
                        Parameter {
                            name: "a".to_string(),
                            param_type: Some(Type::Text),
                            default_value: None,
                            line: 1,
                            column: 1,
                        },
                        Parameter {
                            name: "b".to_string(),
                            param_type: Some(Type::Text),
                            default_value: None,
                            line: 1,
                            column: 1,
                        },
                    ],
                    body: vec![],
                    return_type: None,
                    line: 1,
                    column: 1,
                },
                Statement::RegisterSignalHandlerStatement {
                    signal_type: "SIGINT".to_string(),
                    handler_name: "handler".to_string(),
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&too_many_params);
        assert!(
            result.is_err(),
            "Expected type error when handler has too many parameters"
        );
        let errors = result.err().unwrap();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("must accept 0 or 1 arguments"))
        );

        // Test case 6: Handler has wrong parameter type
        let wrong_param_type = Program {
            statements: vec![
                Statement::ActionDefinition {
                    name: "handler".to_string(),
                    parameters: vec![Parameter {
                        name: "a".to_string(),
                        param_type: Some(Type::Text),
                        default_value: None,
                        line: 1,
                        column: 1,
                    }],
                    body: vec![],
                    return_type: None,
                    line: 1,
                    column: 1,
                },
                Statement::RegisterSignalHandlerStatement {
                    signal_type: "SIGINT".to_string(),
                    handler_name: "handler".to_string(),
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&wrong_param_type);
        assert!(
            result.is_err(),
            "Expected type error when handler has wrong parameter type"
        );
        let errors = result.err().unwrap();
        assert!(errors.iter().any(|e| {
            e.message
                .contains("Signal handler parameter must be a Number")
        }));
    }
}
