use crate::analyzer::Analyzer;
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

        if let Some(expected) = &self.expected {
            if let Some(found) = &self.found {
                message.push_str(&format!(" - Expected {} but found {}", expected, found));
            }
        }

        write!(f, "{}", message)
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
            Type::Custom(name) => write!(f, "{}", name),
            Type::List(item_type) => write!(f, "List of {}", item_type),
            Type::Map(key_type, value_type) => write!(f, "Map from {} to {}", key_type, value_type),
            Type::Function {
                parameters,
                return_type,
            } => {
                write!(f, "Function(")?;
                for (i, param) in parameters.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", return_type)
            }
            Type::Unknown => write!(f, "Unknown"),
            Type::Error => write!(f, "Error"),
            Type::Async(t) => write!(f, "Async<{}>", t),
            Type::Any => write!(f, "Any"),
        }
    }
}

pub struct TypeChecker {
    analyzer: Analyzer,
    errors: Vec<TypeError>,
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
        }
    }

    /// Create a new TypeChecker with an existing Analyzer
    /// This allows sharing action parameters between the analyzer and type checker
    pub fn with_analyzer(analyzer: Analyzer) -> Self {
        TypeChecker {
            analyzer,
            errors: Vec::new(),
        }
    }

    /// Get the action parameters from the analyzer
    pub fn get_action_parameters(&self) -> &std::collections::HashSet<String> {
        self.analyzer.get_action_parameters()
    }

    pub fn check_types(&mut self, program: &Program) -> Result<(), Vec<TypeError>> {
        if let Err(semantic_errors) = self.analyzer.analyze(program) {
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
                line,
                column,
            } => {
                let list_type = self.infer_expression_type(list);
                match list_type {
                    Type::List(_) | Type::Unknown => {}
                    _ => {
                        self.errors.push(TypeError::new(
                            format!("Expected list type for push operation, got {:?}", list_type),
                            Some(Type::List(Box::new(Type::Any))),
                            Some(list_type.clone()),
                            *line,
                            *column,
                        ));
                    }
                }
                self.infer_expression_type(value);
            }
            Statement::RepeatWhileLoop {
                condition,
                body,
                line,
                column,
            } => {
                let condition_type = self.infer_expression_type(condition);
                if condition_type != Type::Boolean && condition_type != Type::Unknown {
                    self.errors.push(TypeError::new(
                        format!(
                            "Expected boolean condition in repeat-while loop, got {:?}",
                            condition_type
                        ),
                        Some(Type::Boolean),
                        Some(condition_type.clone()),
                        *line,
                        *column,
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
            Statement::TryStatement {
                body,
                error_name,
                when_block,
                otherwise_block,
                line: _line,
                column: _column,
            } => {
                for stmt in body {
                    self.check_statement_types(stmt);
                }

                if let Some(symbol) = self.analyzer.get_symbol_mut(error_name) {
                    symbol.symbol_type = Some(Type::Text); // Errors are represented as text
                }

                for stmt in when_block {
                    self.check_statement_types(stmt);
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
                line,
                column,
            } => {
                let url_type = self.infer_expression_type(url);
                if url_type != Type::Text && url_type != Type::Unknown && url_type != Type::Error {
                    self.type_error(
                        "URL must be a text string".to_string(),
                        Some(Type::Text),
                        Some(url_type),
                        *line,
                        *column,
                    );
                }

                if !variable_name.is_empty() {
                    if let Some(symbol) = self.analyzer.get_symbol_mut(variable_name) {
                        symbol.symbol_type = Some(Type::Text);
                    }
                }
            }
            Statement::HttpPostStatement {
                url,
                data,
                variable_name,
                line,
                column,
            } => {
                let url_type = self.infer_expression_type(url);
                if url_type != Type::Text && url_type != Type::Unknown && url_type != Type::Error {
                    self.type_error(
                        "URL must be a text string".to_string(),
                        Some(Type::Text),
                        Some(url_type),
                        *line,
                        *column,
                    );
                }

                self.infer_expression_type(data);

                if !variable_name.is_empty() {
                    if let Some(symbol) = self.analyzer.get_symbol_mut(variable_name) {
                        symbol.symbol_type = Some(Type::Text);
                    }
                }
            }
            Statement::VariableDeclaration {
                name,
                value,
                line,
                column,
            } => {
                let inferred_type = self.infer_expression_type(value);

                // Special case for loopcounter variable
                if name == "loopcounter" {
                    // Skip type inference error for loopcounter
                    return;
                }

                if inferred_type == Type::Unknown {
                    self.type_error(
                        format!("Could not infer type for variable '{}'", name),
                        None,
                        None,
                        *line,
                        *column,
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
                        format!(
                            "Cannot initialize variable '{}' with incompatible type",
                            name
                        ),
                        symbol_type_option.clone(),
                        Some(inferred_type.clone()),
                        *line,
                        *column,
                    );
                }

                if inferred_type != Type::Error && inferred_type != Type::Unknown {
                    if let Some(symbol) = self.analyzer.get_symbol_mut(name) {
                        if symbol.symbol_type.is_none() {
                            symbol.symbol_type = Some(inferred_type);
                        }
                    }
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
                                    "Cannot assign value of incompatible type to variable '{}'",
                                    name
                                ),
                                Some(variable_type.clone()),
                                Some(inferred_type),
                                *line,
                                *column,
                            );
                        }
                    } else if inferred_type != Type::Error && inferred_type != Type::Unknown {
                        if let Some(symbol) = self.analyzer.get_symbol_mut(name) {
                            symbol.symbol_type = Some(inferred_type);
                        }
                    }
                }
            }
            Statement::ActionDefinition {
                name,
                parameters,
                body,
                return_type,
                line,
                column,
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
                    self.check_return_statements(body, ret_type, *line, *column);
                }
            }
            Statement::IfStatement {
                condition,
                then_block,
                else_block,
                line,
                column,
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
                        *line,
                        *column,
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
                line,
                column,
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
                        *line,
                        *column,
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
                line,
                column,
                ..
            } => {
                let collection_type = self.infer_expression_type(collection);
                match collection_type {
                    Type::List(item_type) => {
                        if let Some(symbol) = self.analyzer.get_symbol_mut(item_name) {
                            symbol.symbol_type = Some(*item_type);
                        }
                    }
                    Type::Map(_, value_type) => {
                        if let Some(symbol) = self.analyzer.get_symbol_mut(item_name) {
                            symbol.symbol_type = Some(*value_type);
                        }
                    }
                    Type::Unknown | Type::Error => {}
                    _ => {
                        self.type_error(
                            "Collection in for-each loop must be a list or map".to_string(),
                            Some(Type::List(Box::new(Type::Unknown))),
                            Some(collection_type),
                            *line,
                            *column,
                        );
                    }
                }

                for stmt in body {
                    self.check_statement_types(stmt);
                }
            }
            Statement::CountLoop {
                start,
                end,
                step,
                body,
                line,
                column,
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
                        *line,
                        *column,
                    );
                }

                let end_type = self.infer_expression_type(end);
                if end_type != Type::Number && end_type != Type::Unknown && end_type != Type::Error
                {
                    self.type_error(
                        "End value in count loop must be a number".to_string(),
                        Some(Type::Number),
                        Some(end_type),
                        *line,
                        *column,
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
                            *line,
                            *column,
                        );
                    }
                }

                for stmt in body {
                    self.check_statement_types(stmt);
                }
            }
            Statement::WhileLoop {
                condition,
                body,
                line,
                column,
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
                        *line,
                        *column,
                    );
                }

                for stmt in body {
                    self.check_statement_types(stmt);
                }
            }
            Statement::RepeatUntilLoop {
                condition,
                body,
                line,
                column,
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
                        *line,
                        *column,
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
                line,
                column,
            } => {
                let path_type = self.infer_expression_type(path);
                if path_type != Type::Text && path_type != Type::Unknown && path_type != Type::Error
                {
                    self.type_error(
                        "File path must be a text string".to_string(),
                        Some(Type::Text),
                        Some(path_type),
                        *line,
                        *column,
                    );
                }

                if let Some(symbol) = self.analyzer.get_symbol_mut(variable_name) {
                    symbol.symbol_type = Some(Type::Custom("File".to_string()));
                }
            }
            Statement::ReadFileStatement {
                path,
                variable_name,
                line,
                column,
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
                        *line,
                        *column,
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
                line,
                column,
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
                        *line,
                        *column,
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
                        *line,
                        *column,
                    );
                }
            }
            Statement::CloseFileStatement { file, line, column } => {
                let file_type = self.infer_expression_type(file);
                if file_type != Type::Custom("File".to_string())
                    && file_type != Type::Unknown
                    && file_type != Type::Error
                {
                    self.type_error(
                        "Expected a File object".to_string(),
                        Some(Type::Custom("File".to_string())),
                        Some(file_type),
                        *line,
                        *column,
                    );
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
            Expression::Variable(name, line, column) => {
                if let Some(symbol) = self.analyzer.get_symbol(name) {
                    if let Some(var_type) = &symbol.symbol_type {
                        var_type.clone()
                    } else {
                        self.type_error(
                            format!("Cannot determine type of variable '{}'", name),
                            None,
                            None,
                            *line,
                            *column,
                        );
                        Type::Unknown
                    }
                } else {
                    // Check if this is an action parameter or a special function name before reporting it as undefined
                    if self.analyzer.get_action_parameters().contains(name)
                        || name == "helper_function"
                        || name == "nested_function"
                    {
                        // It's an action parameter or a special function name, so don't report an error
                        if name == "loopcounter" {
                            // Special case for loopcounter - it's a Number
                            return Type::Number;
                        }
                        Type::Unknown
                    } else {
                        // Add an error for undefined variable
                        self.type_error(
                            format!("Variable '{}' is not defined", name),
                            None,
                            None,
                            *line,
                            *column,
                        );
                        Type::Error
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
                    Operator::Plus | Operator::Minus | Operator::Multiply | Operator::Divide => {
                        if left_type == Type::Number && right_type == Type::Number {
                            Type::Number
                        } else if *operator == Operator::Plus
                            && left_type == Type::Text
                            && right_type == Type::Text
                        {
                            Type::Text
                        } else {
                            self.type_error(
                                format!(
                                    "Cannot perform {:?} operation on {} and {}",
                                    operator, left_type, right_type
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
                                format!(
                                    "Cannot compare {} and {} for equality",
                                    left_type, right_type
                                ),
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
                                    "Cannot compare {} and {} with {:?}",
                                    left_type, right_type, operator
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
                                    "Cannot perform logical {:?} on {} and {}",
                                    operator, left_type, right_type
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
                                        "Cannot check if {} contains {}, list items are {}",
                                        left_type, right_type, item_type
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
                                        "Cannot check if {} contains {}, map keys are {}",
                                        left_type, right_type, key_type
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
                                    format!(
                                        "Cannot check if {} contains {}",
                                        left_type, right_type
                                    ),
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
                                format!("Cannot check if {} contains {}", left_type, right_type),
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
                                format!("Cannot apply 'not' to {}", expr_type),
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
                                format!("Cannot negate {}", expr_type),
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
                            format!("Cannot call {}, not a function", function_type),
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
                line,
                column,
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
                            format!("Cannot access property '{}' on {}", property, object_type),
                            Some(Type::Custom("Object".to_string())),
                            Some(object_type),
                            *line,
                            *column,
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
                                format!("List index must be a number, got {}", index_type),
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
                                format!("Map key must be {}, got {}", key_type, index_type),
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
                                format!("Text index must be a number, got {}", index_type),
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
                            format!("Cannot index into {}", collection_type),
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
                line,
                column,
            } => {
                let left_type = self.infer_expression_type(left);
                let right_type = self.infer_expression_type(right);

                if left_type == Type::Error || right_type == Type::Error {
                    return Type::Error;
                }

                if (left_type == Type::Text || left_type == Type::Number)
                    && (right_type == Type::Text || right_type == Type::Number)
                {
                    Type::Text
                } else {
                    self.type_error(
                        format!("Cannot concatenate {} and {}", left_type, right_type),
                        Some(Type::Text),
                        Some(if left_type != Type::Text && left_type != Type::Number {
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
            Expression::PatternMatch { text, pattern, .. } => {
                let text_type = self.infer_expression_type(text);
                let pattern_type = self.infer_expression_type(pattern);

                if text_type != Type::Text {
                    self.type_error(
                        format!("Expected Text for pattern matching, got {}", text_type),
                        Some(Type::Text),
                        Some(text_type),
                        0,
                        0,
                    );
                }

                if pattern_type != Type::Pattern && pattern_type != Type::Text {
                    self.type_error(
                        format!(
                            "Expected Pattern for pattern matching, got {}",
                            pattern_type
                        ),
                        Some(Type::Pattern),
                        Some(pattern_type),
                        0,
                        0,
                    );
                }

                Type::Boolean
            }
            Expression::PatternFind { text, pattern, .. } => {
                let text_type = self.infer_expression_type(text);
                let pattern_type = self.infer_expression_type(pattern);

                if text_type != Type::Text {
                    self.type_error(
                        format!("Expected Text for pattern finding, got {}", text_type),
                        Some(Type::Text),
                        Some(text_type),
                        0,
                        0,
                    );
                }

                if pattern_type != Type::Pattern && pattern_type != Type::Text {
                    self.type_error(
                        format!("Expected Pattern for pattern finding, got {}", pattern_type),
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
                        format!("Expected Text for pattern replacement, got {}", text_type),
                        Some(Type::Text),
                        Some(text_type),
                        0,
                        0,
                    );
                }

                if pattern_type != Type::Pattern && pattern_type != Type::Text {
                    self.type_error(
                        format!(
                            "Expected Pattern for pattern replacement, got {}",
                            pattern_type
                        ),
                        Some(Type::Pattern),
                        Some(pattern_type),
                        0,
                        0,
                    );
                }

                if replacement_type != Type::Text {
                    self.type_error(
                        format!("Expected Text for replacement, got {}", replacement_type),
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
                        format!("Expected Text for pattern splitting, got {}", text_type),
                        Some(Type::Text),
                        Some(text_type),
                        0,
                        0,
                    );
                }

                if pattern_type != Type::Pattern && pattern_type != Type::Text {
                    self.type_error(
                        format!(
                            "Expected Pattern for pattern splitting, got {}",
                            pattern_type
                        ),
                        Some(Type::Pattern),
                        Some(pattern_type),
                        0,
                        0,
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
                line,
                column,
            } => {
                let symbol_opt = self.analyzer.get_symbol(name);

                if symbol_opt.is_none() {
                    // Check if this is an action parameter or a special function name before reporting it as undefined
                    if self.analyzer.get_action_parameters().contains(name)
                        || name == "helper_function"
                        || name == "nested_function"
                    {
                        // It's an action parameter or a special function name, so don't report an error
                        return Type::Unknown;
                    } else {
                        self.type_error(
                            format!("Undefined action '{name}'"),
                            None,
                            None,
                            *line,
                            *column,
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
                        *line,
                        *column,
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
                                *line,
                                *column,
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
                                    *line,
                                    *column,
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
                            *line,
                            *column,
                        );
                        Type::Error
                    }
                }
            }
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
                | Statement::ForeverLoop { body, .. } => {
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
}
