use crate::analyzer::Analyzer;
use crate::lexer::lex_wfl_with_positions;
use crate::parser::Parser;
use crate::parser::ast::{
    Expression, Literal, Operator, Program, Statement, Type, UnaryOperator, ValidationRuleType,
    Visibility,
};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

mod source;
pub use source::{validate_source, write_fixed_file};

pub struct CodeFixer {
    indent_size: usize,
    max_line_length: usize,
    max_concatenation_chain: usize,
    snake_case_variables: bool,
    trailing_whitespace: bool,
    consistent_keyword_case: bool,
}

pub enum FixerOutputMode {
    Stdout,  // Print fixed code to stdout
    InPlace, // Overwrite the input file
    Diff,    // Generate a unified diff
}

#[derive(Debug, Default)]
pub struct FixerSummary {
    pub lines_reformatted: usize,
    pub vars_renamed: usize,
    pub dead_code_removed: usize,
    pub concatenations_fixed: usize,
}

impl FixerSummary {
    /// Count reported edits; an unchanged source produces zero.
    pub fn total(&self) -> usize {
        self.lines_reformatted
            + self.vars_renamed
            + self.dead_code_removed
            + self.concatenations_fixed
    }
}

impl CodeFixer {
    /// Use the default WFL indentation, naming, and whitespace rules.
    pub fn new() -> Self {
        Self {
            indent_size: 4,
            max_line_length: 100,
            max_concatenation_chain: 5,
            snake_case_variables: true,
            trailing_whitespace: false,
            consistent_keyword_case: true,
        }
    }

    pub fn set_indent_size(&mut self, size: usize) {
        self.indent_size = size;
    }

    pub fn set_max_line_length(&mut self, length: usize) {
        self.max_line_length = length;
    }

    pub fn set_max_concatenation_chain(&mut self, max_chain: usize) {
        self.max_concatenation_chain = max_chain;
    }

    /// Format parsed source without discarding comments, literals, or syntax.
    /// File/CLI callers use `fix_checked` so a validation failure is an error.
    pub fn fix(&self, program: &Program, source: &str) -> (String, FixerSummary) {
        if source.is_empty() && !program.statements.is_empty() {
            // Retain AST-only printing for library clients constructing an AST.
            return self.print_program(program);
        }
        self.fix_checked(program, source)
            .unwrap_or_else(|_| (source.to_string(), FixerSummary::default()))
    }

    /// Return source-preserving edits only after lexical and parse validation.
    ///
    /// Unlike `fix`, validation and configured resource-limit failures propagate
    /// to the caller. The supplied program must describe the supplied source.
    pub fn fix_checked(
        &self,
        program: &Program,
        source: &str,
    ) -> io::Result<(String, FixerSummary)> {
        source::fix_source(self, program, source)
    }

    /// Retain the legacy printer for callers constructing an AST without source.
    fn print_program(&self, program: &Program) -> (String, FixerSummary) {
        let _analyzer = Analyzer::new();
        let dead_code = Vec::new();

        let simplified_program = self.simplify_program(program, &dead_code);

        let mut output = String::new();
        let mut summary = FixerSummary {
            lines_reformatted: 0,
            vars_renamed: 0,
            dead_code_removed: dead_code.len(),
            concatenations_fixed: 0,
        };

        self.pretty_print(&simplified_program, &mut output, 0, &mut summary);

        let tokens = lex_wfl_with_positions(&output);
        let mut parser = Parser::new(&tokens);
        match parser.parse() {
            Ok(_new_program) => {}
            Err(_) => {
                eprintln!(
                    "Warning: Re-parsing the fixed code resulted in errors. This is a bug in the code fixer."
                );
            }
        }

        (output, summary)
    }

    /// Validate and format one UTF-8 file, then publish the selected output.
    ///
    /// Source and patch modes write only to stdout. In-place mode delegates to
    /// `write_fixed_file`, which refuses invalid or stale input before replacement.
    pub fn fix_file(&self, path: &Path, mode: FixerOutputMode) -> io::Result<FixerSummary> {
        let source = fs::read_to_string(path)?;
        validate_source(&source)?;

        let tokens = lex_wfl_with_positions(&source);
        let mut parser = Parser::new(&tokens);
        let program = match parser.parse() {
            Ok(program) => program,
            Err(err) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to parse file: {err:?}"),
                ));
            }
        };

        let (fixed_code, summary) = self.fix_checked(&program, &source)?;

        match mode {
            FixerOutputMode::Stdout => {
                io::stdout().write_all(fixed_code.as_bytes())?;
            }
            FixerOutputMode::InPlace => {
                write_fixed_file(path, &source, &fixed_code)?;
            }
            FixerOutputMode::Diff => {
                let diff = self.diff_for_path(path, &source, &fixed_code);
                io::stdout().write_all(diff.as_bytes())?;
            }
        }

        Ok(summary)
    }

    fn simplify_program(&self, program: &Program, dead_code: &[usize]) -> Program {
        let mut simplified_statements = Vec::new();

        for (i, statement) in program.statements.iter().enumerate() {
            if dead_code.contains(&i) {
                continue;
            }

            let simplified = self.simplify_statement(statement);
            simplified_statements.push(simplified);
        }

        Program {
            statements: simplified_statements,
        }
    }

    fn simplify_statement(&self, statement: &Statement) -> Statement {
        match statement {
            Statement::IfStatement {
                condition,
                then_block,
                else_block,
                line,
                column,
            } => {
                let simplified_condition = self.simplify_boolean_expression(condition);

                let mut simplified_then = Vec::new();
                for stmt in then_block {
                    simplified_then.push(self.simplify_statement(stmt));
                }

                let simplified_else = if let Some(else_stmts) = else_block {
                    let mut simplified = Vec::new();
                    for stmt in else_stmts {
                        simplified.push(self.simplify_statement(stmt));
                    }
                    Some(simplified)
                } else {
                    None
                };

                Statement::IfStatement {
                    condition: simplified_condition,
                    then_block: simplified_then,
                    else_block: simplified_else,
                    line: *line,
                    column: *column,
                }
            }
            _ => statement.clone(),
        }
    }
    #[allow(clippy::only_used_in_recursion)]
    fn simplify_boolean_expression(&self, expression: &Expression) -> Expression {
        match expression {
            Expression::BinaryOperation {
                left,
                operator,
                right,
                line,
                column,
            } => {
                let simplified_left = self.simplify_boolean_expression(left);
                let simplified_right = self.simplify_boolean_expression(right);

                Expression::BinaryOperation {
                    left: Box::new(simplified_left),
                    operator: operator.clone(),
                    right: Box::new(simplified_right),
                    line: *line,
                    column: *column,
                }
            }
            _ => expression.clone(),
        }
    }

    fn pretty_print(
        &self,
        program: &Program,
        output: &mut String,
        indent_level: usize,
        summary: &mut FixerSummary,
    ) {
        for statement in &program.statements {
            self.pretty_print_statement(statement, output, indent_level, summary);
        }
    }

    fn pretty_print_statement(
        &self,
        statement: &Statement,
        output: &mut String,
        indent_level: usize,
        summary: &mut FixerSummary,
    ) {
        let indent = " ".repeat(indent_level * self.indent_size);

        match statement {
            Statement::VariableDeclaration { name, value, .. } => {
                let fixed_name = self.fix_identifier_name(name, summary);
                output.push_str(&indent);
                output.push_str("store ");
                output.push_str(&fixed_name);
                output.push_str(" as ");
                self.pretty_print_expression(value, output, indent_level, summary);
                output.push('\n');
                summary.lines_reformatted += 1;
            }
            Statement::Assignment { name, value, .. } => {
                let fixed_name = self.fix_identifier_name(name, summary);
                output.push_str(&indent);
                output.push_str("change ");
                output.push_str(&fixed_name);
                output.push_str(" to ");
                self.pretty_print_expression(value, output, indent_level, summary);
                output.push('\n');
                summary.lines_reformatted += 1;
            }
            Statement::ActionDefinition {
                name,
                parameters,
                body,
                return_type,
                ..
            } => {
                let fixed_name = self.fix_identifier_name(name, summary);
                output.push_str(&indent);
                output.push_str("define action called ");
                output.push_str(&fixed_name);

                if !parameters.is_empty() {
                    output.push_str(" with parameters ");
                    for (i, param) in parameters.iter().enumerate() {
                        if i > 0 {
                            output.push_str(" and ");
                        }
                        let fixed_param_name = self.fix_identifier_name(&param.name, summary);
                        output.push_str(&fixed_param_name);

                        if let Some(param_type) = &param.param_type {
                            output.push_str(" as ");
                            output.push_str(&self.format_type(param_type));
                        }

                        if let Some(default_value) = &param.default_value {
                            output.push_str(" default ");
                            self.pretty_print_expression(
                                default_value,
                                output,
                                indent_level,
                                summary,
                            );
                        }
                    }
                }

                if let Some(ret_type) = return_type {
                    output.push_str(": ");
                    output.push_str(&self.format_action_return_type(ret_type));
                }

                output.push_str(":\n");

                for stmt in body {
                    self.pretty_print_statement(stmt, output, indent_level + 1, summary);
                }

                output.push_str(&indent);
                output.push_str("end action\n");
                summary.lines_reformatted += 1;
            }
            Statement::IfStatement {
                condition,
                then_block,
                else_block,
                ..
            } => {
                output.push_str(&indent);
                output.push_str("check if ");
                self.pretty_print_expression(condition, output, indent_level, summary);
                output.push_str(":\n");

                for stmt in then_block {
                    self.pretty_print_statement(stmt, output, indent_level + 1, summary);
                }

                if let Some(else_stmts) = else_block {
                    output.push_str(&indent);
                    output.push_str("otherwise:\n");

                    for stmt in else_stmts {
                        self.pretty_print_statement(stmt, output, indent_level + 1, summary);
                    }
                }

                output.push_str(&indent);
                output.push_str("end check\n");
                summary.lines_reformatted += 1;
            }
            Statement::SingleLineIf {
                condition,
                then_stmt,
                else_stmt,
                ..
            } => {
                output.push_str(&indent);
                output.push_str("if ");
                self.pretty_print_expression(condition, output, indent_level, summary);
                output.push_str(" then ");

                let mut then_output = String::new();
                self.pretty_print_statement(then_stmt, &mut then_output, 0, summary);
                let then_str = then_output.trim();
                output.push_str(then_str);

                if let Some(else_stmt) = else_stmt {
                    output.push_str(" otherwise ");

                    let mut else_output = String::new();
                    self.pretty_print_statement(else_stmt, &mut else_output, 0, summary);
                    let else_str = else_output.trim();
                    output.push_str(else_str);
                }

                output.push('\n');
                summary.lines_reformatted += 1;
            }
            Statement::ForEachLoop {
                item_name,
                collection,
                body,
                ..
            } => {
                let fixed_item_name = self.fix_identifier_name(item_name, summary);
                output.push_str(&indent);
                output.push_str("for each ");
                output.push_str(&fixed_item_name);
                output.push_str(" in ");
                self.pretty_print_expression(collection, output, indent_level, summary);
                output.push_str(":\n");

                for stmt in body {
                    self.pretty_print_statement(stmt, output, indent_level + 1, summary);
                }

                output.push_str(&indent);
                output.push_str("end for each\n");
                summary.lines_reformatted += 1;
            }
            Statement::CountLoop {
                start,
                end,
                step,
                variable_name,
                body,
                ..
            } => {
                output.push_str(&indent);
                output.push_str("count from ");
                self.pretty_print_expression(start, output, indent_level, summary);
                output.push_str(" to ");
                self.pretty_print_expression(end, output, indent_level, summary);

                if let Some(step_expr) = step {
                    output.push_str(" by ");
                    self.pretty_print_expression(step_expr, output, indent_level, summary);
                }

                // Add custom variable name if present
                if let Some(var_name) = variable_name {
                    output.push_str(" as ");
                    output.push_str(var_name);
                }

                output.push_str(":\n");

                for stmt in body {
                    self.pretty_print_statement(stmt, output, indent_level + 1, summary);
                }

                output.push_str(&indent);
                output.push_str("end count\n");
                summary.lines_reformatted += 1;
            }
            Statement::WhileLoop {
                condition, body, ..
            } => {
                output.push_str(&indent);
                output.push_str("while ");
                self.pretty_print_expression(condition, output, indent_level, summary);
                output.push_str(":\n");

                for stmt in body {
                    self.pretty_print_statement(stmt, output, indent_level + 1, summary);
                }

                output.push_str(&indent);
                output.push_str("end while\n");
                summary.lines_reformatted += 1;
            }
            Statement::DisplayStatement { value, .. } => {
                output.push_str(&indent);
                output.push_str("display ");
                self.pretty_print_expression(value, output, indent_level, summary);
                output.push('\n');
                summary.lines_reformatted += 1;
            }
            Statement::ReturnStatement { value, .. } => {
                output.push_str(&indent);
                output.push_str("return");

                if let Some(expr) = value {
                    output.push(' ');
                    self.pretty_print_expression(expr, output, indent_level, summary);
                }

                output.push('\n');
                summary.lines_reformatted += 1;
            }
            Statement::ExpressionStatement { expression, .. } => {
                output.push_str(&indent);
                self.pretty_print_expression(expression, output, indent_level, summary);
                output.push('\n');
                summary.lines_reformatted += 1;
            }
            Statement::ContainerDefinition {
                name,
                extends,
                implements,
                properties,
                methods,
                events,
                static_properties,
                static_methods,
                ..
            } => {
                output.push_str(&indent);
                output.push_str("create container ");
                output.push_str(name);

                // Handle inheritance
                if let Some(parent) = extends {
                    output.push_str(" extends ");
                    output.push_str(parent);
                }

                // Handle interfaces
                if !implements.is_empty() {
                    output.push_str(" implements ");
                    output.push_str(&implements.join(", "));
                }

                output.push_str(":\n");

                // Format static properties. Property names are deliberately
                // NOT snake_case-normalized: initializers in 'create new'
                // blocks and member accesses print the original spelling, so
                // renaming only the definition would break the program.
                for prop in static_properties {
                    output.push_str(&format!("{indent}    "));
                    output.push_str("static property ");
                    output.push_str(&prop.name);

                    if let Some(prop_type) = &prop.property_type {
                        output.push_str(": ");
                        output.push_str(&self.format_type(prop_type));
                    }

                    if let Some(default) = &prop.default_value {
                        output.push_str(" defaults ");
                        self.pretty_print_expression(default, output, indent_level + 1, summary);
                    }

                    output.push('\n');
                }

                // Format instance properties
                for prop in properties {
                    output.push_str(&format!("{indent}    "));
                    if prop.visibility == Visibility::Private {
                        output.push_str("private ");
                    }
                    output.push_str("property ");
                    output.push_str(&prop.name);

                    if let Some(prop_type) = &prop.property_type {
                        output.push_str(": ");
                        output.push_str(&self.format_type(prop_type));
                    }

                    if let Some(default) = &prop.default_value {
                        output.push_str(" defaults ");
                        self.pretty_print_expression(default, output, indent_level + 1, summary);
                    }

                    // Format validation rules
                    for rule in &prop.validation_rules {
                        output.push('\n');
                        output.push_str(&format!("{indent}        "));
                        match &rule.rule_type {
                            ValidationRuleType::NotEmpty => output.push_str("must not be empty"),
                            ValidationRuleType::MinLength => {
                                output.push_str("minimum length ");
                                if let Some(param) = rule.parameters.first() {
                                    self.pretty_print_expression(
                                        param,
                                        output,
                                        indent_level + 2,
                                        summary,
                                    );
                                }
                            }
                            ValidationRuleType::MaxLength => {
                                output.push_str("maximum length ");
                                if let Some(param) = rule.parameters.first() {
                                    self.pretty_print_expression(
                                        param,
                                        output,
                                        indent_level + 2,
                                        summary,
                                    );
                                }
                            }
                            ValidationRuleType::ExactLength => {
                                output.push_str("exact length ");
                                if let Some(param) = rule.parameters.first() {
                                    self.pretty_print_expression(
                                        param,
                                        output,
                                        indent_level + 2,
                                        summary,
                                    );
                                }
                            }
                            ValidationRuleType::MinValue => {
                                output.push_str("minimum value ");
                                if let Some(param) = rule.parameters.first() {
                                    self.pretty_print_expression(
                                        param,
                                        output,
                                        indent_level + 2,
                                        summary,
                                    );
                                }
                            }
                            ValidationRuleType::MaxValue => {
                                output.push_str("maximum value ");
                                if let Some(param) = rule.parameters.first() {
                                    self.pretty_print_expression(
                                        param,
                                        output,
                                        indent_level + 2,
                                        summary,
                                    );
                                }
                            }
                            ValidationRuleType::Pattern => {
                                output.push_str("must match pattern ");
                                if let Some(param) = rule.parameters.first() {
                                    self.pretty_print_expression(
                                        param,
                                        output,
                                        indent_level + 2,
                                        summary,
                                    );
                                }
                            }
                            ValidationRuleType::Custom => {
                                output.push_str("custom validation");
                                if !rule.parameters.is_empty() {
                                    output.push_str(" with ");
                                    for (i, param) in rule.parameters.iter().enumerate() {
                                        if i > 0 {
                                            output.push_str(", ");
                                        }
                                        self.pretty_print_expression(
                                            param,
                                            output,
                                            indent_level + 2,
                                            summary,
                                        );
                                    }
                                }
                            }
                        }
                    }

                    output.push('\n');
                }

                // Format events in the grammar the container-body parser
                // accepts: 'event <name> [needs a: T, b: T]'.
                for event in events {
                    output.push_str(&format!("{indent}    "));
                    output.push_str("event ");
                    output.push_str(&event.name);

                    if !event.parameters.is_empty() {
                        output.push_str(" needs ");
                        for (i, param) in event.parameters.iter().enumerate() {
                            if i > 0 {
                                output.push_str(", ");
                            }
                            output.push_str(&param.name);
                            if let Some(param_type) = &param.param_type {
                                output.push_str(": ");
                                output.push_str(&self.format_type(param_type));
                            }
                        }
                    }

                    output.push('\n');
                }

                // Add spacing between sections
                if (!static_properties.is_empty() || !properties.is_empty() || !events.is_empty())
                    && (!methods.is_empty() || !static_methods.is_empty())
                {
                    output.push('\n');
                }

                // Format methods in container-body grammar ('action <name>
                // ...: ... end'), not the standalone 'define action called'
                // form, which the container-body parser rejects.
                for method in static_methods {
                    self.pretty_print_container_action(
                        method,
                        output,
                        indent_level + 1,
                        true,
                        summary,
                    );
                }

                for method in methods {
                    self.pretty_print_container_action(
                        method,
                        output,
                        indent_level + 1,
                        false,
                        summary,
                    );
                }

                output.push_str(&indent);
                output.push_str("end\n");
                summary.lines_reformatted += 1;
            }
            Statement::ContainerInstantiation {
                container_type,
                instance_name,
                arguments,
                property_initializers,
                ..
            } => {
                output.push_str(&indent);
                output.push_str("create new ");
                output.push_str(container_type);

                // Handle constructor arguments
                if !arguments.is_empty() {
                    output.push_str(" with ");
                    for (i, arg) in arguments.iter().enumerate() {
                        if i > 0 {
                            output.push_str(" and ");
                        }
                        self.pretty_print_expression(&arg.value, output, indent_level, summary);
                    }
                }

                output.push_str(" as ");
                output.push_str(instance_name);

                // Only add the block if there are property initializers
                if !property_initializers.is_empty() {
                    output.push_str(":\n");

                    // Format property initializers
                    for initializer in property_initializers {
                        output.push_str(&format!("{indent}    "));
                        output.push_str(&initializer.name);
                        output.push_str(" is ");
                        self.pretty_print_expression(
                            &initializer.value,
                            output,
                            indent_level + 1,
                            summary,
                        );
                        output.push('\n');
                    }

                    output.push_str(&indent);
                    output.push_str("end\n");
                } else {
                    output.push('\n');
                }

                summary.lines_reformatted += 1;
            }
            Statement::InterfaceDefinition {
                name,
                extends,
                required_actions,
                ..
            } => {
                output.push_str(&indent);
                output.push_str("create interface ");
                output.push_str(name);

                // Handle interface inheritance
                if !extends.is_empty() {
                    output.push_str(" extends ");
                    output.push_str(&extends.join(", "));
                }

                // A bare interface (no requirements) is spelled without a
                // colon or 'end' — the parser only opens a body after ':'.
                if required_actions.is_empty() {
                    output.push('\n');
                    summary.lines_reformatted += 1;
                } else {
                    output.push_str(":\n");

                    // Format required actions in the grammar the parser
                    // accepts: 'requires action <name> [needs a: T, b: T]
                    // [: ReturnType]'.
                    for action in required_actions {
                        output.push_str(&format!("{indent}    "));
                        output.push_str("requires action ");
                        output.push_str(&action.name);

                        if !action.parameters.is_empty() {
                            output.push_str(" needs ");
                            for (i, param) in action.parameters.iter().enumerate() {
                                if i > 0 {
                                    output.push_str(", ");
                                }
                                output.push_str(&param.name);
                                if let Some(param_type) = &param.param_type {
                                    output.push_str(": ");
                                    output.push_str(&self.format_type(param_type));
                                }
                            }
                        }

                        if let Some(return_type) = &action.return_type {
                            output.push_str(": ");
                            output.push_str(&self.format_type(return_type));
                        }

                        output.push('\n');
                    }

                    output.push_str(&indent);
                    output.push_str("end\n");
                    summary.lines_reformatted += 1;
                }
            }
            Statement::EventDefinition {
                name, parameters, ..
            } => {
                output.push_str(&indent);
                output.push_str("event ");
                output.push_str(name);

                // Format parameters
                if !parameters.is_empty() {
                    output.push_str(" with ");
                    for (i, param) in parameters.iter().enumerate() {
                        if i > 0 {
                            output.push_str(" and ");
                        }
                        output.push_str(&param.name);
                        if let Some(param_type) = &param.param_type {
                            output.push_str(" as ");
                            output.push_str(&self.format_type(param_type));
                        }
                    }
                }

                output.push('\n');
                summary.lines_reformatted += 1;
            }
            _ => {
                output.push_str(&indent);
                output.push_str(&format!("{statement:?}\n"));
                summary.lines_reformatted += 1;
            }
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn pretty_print_expression(
        &self,
        expression: &Expression,
        output: &mut String,
        indent_level: usize,
        summary: &mut FixerSummary,
    ) {
        match expression {
            Expression::Literal(literal, ..) => match literal {
                Literal::String(s) => {
                    output.push('"');
                    output.push_str(s);
                    output.push('"');
                }
                Literal::Integer(n) => {
                    output.push_str(&n.to_string());
                }
                Literal::Float(f) => {
                    output.push_str(&f.to_string());
                }
                Literal::Boolean(b) => {
                    output.push_str(if *b { "yes" } else { "no" });
                }
                Literal::Nothing => {
                    output.push_str("nothing");
                }
                Literal::Pattern(p) => {
                    output.push('/');
                    output.push_str(p);
                    output.push('/');
                }
                Literal::List(elements) => {
                    output.push('[');
                    for (i, element) in elements.iter().enumerate() {
                        if i > 0 {
                            output.push_str(" and ");
                        }
                        self.pretty_print_expression(element, output, indent_level, summary);
                    }
                    output.push(']');
                }
            },
            Expression::Variable(name, ..) => {
                let fixed_name = self.fix_identifier_name(name, summary);
                output.push_str(&fixed_name);
            }
            Expression::BinaryOperation {
                left,
                operator,
                right,
                ..
            } => {
                output.push('(');
                self.pretty_print_expression(left, output, indent_level, summary);

                match operator {
                    Operator::Plus => output.push_str(" + "),
                    Operator::Minus => output.push_str(" - "),
                    Operator::Multiply => output.push_str(" * "),
                    Operator::Divide => output.push_str(" / "),
                    Operator::Modulo => output.push_str(" % "),
                    Operator::Equals => output.push_str(" == "),
                    Operator::NotEquals => output.push_str(" != "),
                    Operator::LessThan => output.push_str(" < "),
                    Operator::LessThanOrEqual => output.push_str(" <= "),
                    Operator::GreaterThan => output.push_str(" > "),
                    Operator::GreaterThanOrEqual => output.push_str(" >= "),
                    Operator::And => output.push_str(" and "),
                    Operator::Or => output.push_str(" or "),
                    Operator::Contains => output.push_str(" contains "),
                }

                self.pretty_print_expression(right, output, indent_level, summary);
                output.push(')');
            }
            Expression::UnaryOperation {
                operator,
                expression: expr,
                ..
            } => {
                match operator {
                    UnaryOperator::Minus => output.push('-'),
                    UnaryOperator::Not => output.push_str("not "),
                }

                self.pretty_print_expression(expr, output, indent_level, summary);
            }
            Expression::FunctionCall {
                function,
                arguments,
                ..
            } => {
                self.pretty_print_expression(function, output, indent_level, summary);
                output.push('(');

                for (i, arg) in arguments.iter().enumerate() {
                    if i > 0 {
                        output.push_str(", ");
                    }

                    if let Some(name) = &arg.name {
                        let fixed_name = self.fix_identifier_name(name, summary);
                        output.push_str(&fixed_name);
                        output.push_str(": ");
                    }

                    self.pretty_print_expression(&arg.value, output, indent_level, summary);
                }

                output.push(')');
            }
            Expression::MemberAccess {
                object, property, ..
            } => {
                self.pretty_print_expression(object, output, indent_level, summary);
                output.push('.');
                output.push_str(property);
            }
            Expression::IndexAccess {
                collection, index, ..
            } => {
                self.pretty_print_expression(collection, output, indent_level, summary);
                output.push('[');
                self.pretty_print_expression(index, output, indent_level, summary);
                output.push(']');
            }
            Expression::Concatenation { left, right, .. } => {
                if self.should_reformat_concatenation(expression) {
                    let chain_length = self.count_concatenation_chain(expression);
                    let is_multiline = chain_length > 3;
                    let formatted = self.format_concatenation_chain(expression, is_multiline);
                    output.push_str(&formatted);
                    summary.concatenations_fixed += 1;
                } else {
                    self.pretty_print_expression(left, output, indent_level, summary);
                    output.push_str(" with ");
                    self.pretty_print_expression(right, output, indent_level, summary);
                }
            }
            Expression::PatternMatch { text, pattern, .. } => {
                self.pretty_print_expression(text, output, indent_level, summary);
                output.push_str(" matches ");
                self.pretty_print_expression(pattern, output, indent_level, summary);
            }
            Expression::PatternFind { text, pattern, .. } => {
                output.push_str("find ");
                self.pretty_print_expression(pattern, output, indent_level, summary);
                output.push_str(" in ");
                self.pretty_print_expression(text, output, indent_level, summary);
            }
            Expression::PatternReplace {
                text,
                pattern,
                replacement,
                ..
            } => {
                output.push_str("replace ");
                self.pretty_print_expression(pattern, output, indent_level, summary);
                output.push_str(" with ");
                self.pretty_print_expression(replacement, output, indent_level, summary);
                output.push_str(" in ");
                self.pretty_print_expression(text, output, indent_level, summary);
            }
            Expression::PatternSplit { text, pattern, .. } => {
                output.push_str("split ");
                self.pretty_print_expression(text, output, indent_level, summary);
                output.push_str(" on pattern ");
                self.pretty_print_expression(pattern, output, indent_level, summary);
            }
            Expression::StringSplit {
                text, delimiter, ..
            } => {
                output.push_str("split ");
                self.pretty_print_expression(text, output, indent_level, summary);
                output.push_str(" by ");
                self.pretty_print_expression(delimiter, output, indent_level, summary);
            }
            Expression::AwaitExpression {
                expression: expr, ..
            } => {
                output.push_str("await ");
                self.pretty_print_expression(expr, output, indent_level, summary);
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                self.pretty_print_expression(object, output, indent_level, summary);
                output.push('.');
                output.push_str(method);
                output.push('(');

                for (i, arg) in arguments.iter().enumerate() {
                    if i > 0 {
                        output.push_str(", ");
                    }

                    if let Some(name) = &arg.name {
                        let fixed_name = self.fix_identifier_name(name, summary);
                        output.push_str(&fixed_name);
                        output.push_str(": ");
                    }

                    self.pretty_print_expression(&arg.value, output, indent_level, summary);
                }

                output.push(')');
            }
            #[allow(unreachable_patterns)]
            _ => {
                output.push_str(&format!("{expression:?}"));
            }
        }
    }

    /// Print a container method in the grammar the container-body parser
    /// accepts: `action <name> [needs a: T, b: T][: ReturnType]:` + body +
    /// `end`. Method and parameter names are deliberately NOT snake_case
    /// normalized — method-call sites and interface `requires action` names
    /// print the original spelling, so renaming only the definition would
    /// break the fixed program (calls and interface conformance alike).
    fn pretty_print_container_action(
        &self,
        method: &Statement,
        output: &mut String,
        indent_level: usize,
        is_static: bool,
        summary: &mut FixerSummary,
    ) {
        let Statement::ActionDefinition {
            name,
            parameters,
            body,
            return_type,
            ..
        } = method
        else {
            return;
        };

        let indent = "    ".repeat(indent_level);
        output.push_str(&indent);
        if is_static {
            output.push_str("static ");
        }
        output.push_str("action ");
        output.push_str(name);

        if !parameters.is_empty() {
            output.push_str(" needs ");
            for (i, param) in parameters.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                output.push_str(&param.name);
                if let Some(param_type) = &param.param_type {
                    output.push_str(": ");
                    output.push_str(&self.format_type(param_type));
                }
            }
        }

        // The colon doubles as the body marker; a return type follows it
        // directly ('action get_area: Number'), matching the parser.
        output.push(':');
        if let Some(return_type) = return_type {
            output.push(' ');
            output.push_str(&self.format_type(return_type));
        }
        output.push('\n');

        for statement in body {
            self.pretty_print_statement(statement, output, indent_level + 1, summary);
        }

        output.push_str(&indent);
        output.push_str("end\n");
        summary.lines_reformatted += 1;
    }

    fn fix_identifier_name(&self, name: &str, summary: &mut FixerSummary) -> String {
        if !self.is_snake_case(name) {
            summary.vars_renamed += 1;
            self.to_snake_case(name)
        } else {
            name.to_string()
        }
    }

    fn is_snake_case(&self, s: &str) -> bool {
        !s.contains(char::is_uppercase) && !s.contains(' ')
    }

    fn to_snake_case(&self, s: &str) -> String {
        let mut result = String::new();
        let mut previous_char_is_lowercase = false;

        for (i, c) in s.char_indices() {
            if c.is_uppercase() {
                if i > 0 && previous_char_is_lowercase {
                    result.push('_');
                }
                result.push(c.to_lowercase().next().unwrap());
            } else if c == ' ' {
                result.push('_');
            } else {
                result.push(c);
            }

            previous_char_is_lowercase = c.is_lowercase();
        }

        result
    }

    /// Analyzes a concatenation expression to determine if it needs reformatting
    fn should_reformat_concatenation(&self, expr: &Expression) -> bool {
        let chain_length = self.count_concatenation_chain(expr);
        // Only reformat if we have a very long chain (more than 8 elements)
        // or if we have genuinely poor formatting patterns
        chain_length > 8 || self.has_genuinely_poor_formatting(expr)
    }

    /// Counts the length of a concatenation chain
    #[allow(clippy::only_used_in_recursion)]
    fn count_concatenation_chain(&self, expr: &Expression) -> usize {
        match expr {
            Expression::Concatenation { left, right, .. } => {
                1 + self.count_concatenation_chain(left) + self.count_concatenation_chain(right)
            }
            _ => 0,
        }
    }

    /// Checks if concatenation has genuinely poor formatting that needs fixing
    fn has_genuinely_poor_formatting(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Concatenation { .. } => {
                // Look for very specific poor patterns like the original problematic case:
                // multiline strings with embedded newlines that span multiple actual lines
                self.has_problematic_multiline_pattern(expr)
            }
            _ => false,
        }
    }

    /// Detects specific problematic patterns like the original wfl_combiner.wfl issue
    fn has_problematic_multiline_pattern(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Concatenation { .. } => {
                // Look for patterns where we have multiple string literals with newlines
                // concatenated in a way that suggests the original multiline format
                self.count_newline_literals(expr) > 4 // More than 4 "\n" literals suggests poor formatting
            }
            _ => false,
        }
    }

    /// Counts the number of "\n" literal strings in a concatenation chain
    #[allow(clippy::only_used_in_recursion)]
    fn count_newline_literals(&self, expr: &Expression) -> usize {
        match expr {
            Expression::Literal(Literal::String(s), ..) if &**s == "\n" => 1,
            Expression::Literal(Literal::String(_), ..) => 0,
            Expression::Concatenation { left, right, .. } => {
                self.count_newline_literals(left) + self.count_newline_literals(right)
            }
            _ => 0,
        }
    }

    /// Formats a concatenation chain in a more readable way
    fn format_concatenation_chain(&self, expr: &Expression, is_multiline: bool) -> String {
        match expr {
            Expression::Concatenation { left, right, .. } => {
                let left_str = match **left {
                    Expression::Concatenation { .. } => {
                        self.format_concatenation_chain(left, is_multiline)
                    }
                    _ => self.format_single_expression_for_concatenation(left),
                };

                let right_str = match **right {
                    Expression::Concatenation { .. } => {
                        self.format_concatenation_chain(right, is_multiline)
                    }
                    _ => self.format_single_expression_for_concatenation(right),
                };

                if is_multiline {
                    format!("{left_str} with\n    {right_str}")
                } else {
                    format!("{left_str} with {right_str}")
                }
            }
            _ => self.format_single_expression_for_concatenation(expr),
        }
    }

    /// Formats a single expression within a concatenation chain
    fn format_single_expression_for_concatenation(&self, expr: &Expression) -> String {
        match expr {
            Expression::Literal(Literal::String(s), ..) => {
                format!("\"{s}\"")
            }
            Expression::Variable(name, ..) => name.clone(),
            _ => format!("{expr:?}"), // Fallback for other expressions
        }
    }

    /// Format the recursively representable action-return surface types.
    ///
    /// This is intentionally separate from `format_type`: changing the shared
    /// spellings would also change parameters and properties. Only list,
    /// map/binary, and optional nesting are added to the action-header
    /// round-trip contract; other internal type spellings retain their existing
    /// behavior.
    fn format_action_return_type(&self, type_val: &Type) -> String {
        match type_val {
            Type::List(inner) => {
                format!("List of {}", self.format_action_return_type(inner))
            }
            Type::Map(key, value) => format!(
                "Map of {} to {}",
                self.format_action_return_type(key),
                self.format_action_return_type(value)
            ),
            Type::Optional(inner) => {
                format!("Optional of {}", self.format_action_return_type(inner))
            }
            _ => self.format_type(type_val),
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn format_type(&self, type_val: &Type) -> String {
        match type_val {
            Type::Text => "Text".to_string(),
            Type::Number => "Number".to_string(),
            Type::Boolean => "Boolean".to_string(),
            Type::Nothing => "Nothing".to_string(),
            Type::Pattern => "Pattern".to_string(),
            Type::Date => "date".to_string(),
            Type::Time => "time".to_string(),
            Type::DateTime => "datetime".to_string(),
            Type::Binary => "Binary".to_string(),
            Type::Custom(name) => name.clone(),
            Type::List(inner) => format!("List of {}", self.format_type(inner)),
            Type::Map(key, value) => format!(
                "Map of {} to {}",
                self.format_type(key),
                self.format_type(value)
            ),
            Type::Function {
                parameters,
                return_type,
            } => {
                let params = parameters
                    .iter()
                    .map(|t| self.format_type(t))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("Function({}) -> {}", params, self.format_type(return_type))
            }
            Type::Container(name) => name.clone(),
            Type::ContainerInstance(name) => name.clone(),
            Type::Interface(name) => name.clone(),
            Type::Async(inner) => format!("Async {}", self.format_type(inner)),
            Type::Any => "Any".to_string(),
            Type::Optional(inner) => {
                format!("{} or Nothing", self.format_type(inner))
            }
            Type::Unknown => "Unknown".to_string(),
            Type::Error => "Error".to_string(),
        }
    }

    /// Build a unified patch with generic labels, or an empty string if unchanged.
    pub fn diff(&self, original: &str, fixed: &str) -> String {
        self.generate_diff(original, fixed)
    }

    /// Label a unified patch with the input path, preserving relative folders.
    /// Absolute or parent-traversing inputs use a basename so a displayed patch
    /// cannot direct a patch tool outside its working directory.
    pub fn diff_for_path(&self, path: &Path, original: &str, fixed: &str) -> String {
        use std::path::Component;

        let diff = self.generate_diff(original, fixed);
        if diff.is_empty() {
            return diff;
        }
        let relative = if path.components().any(|component| {
            matches!(
                component,
                Component::Prefix(_) | Component::RootDir | Component::ParentDir
            )
        }) {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "source.wfl".to_string())
        } else {
            path.components()
                .filter_map(|component| match component {
                    Component::Normal(name) => Some(name.to_string_lossy()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("/")
        };

        // Git's quoted path convention disambiguates spaces and control bytes
        // without changing Unicode names or platform-independent separators.
        /// Escape ambiguous patch labels using Git's C-style quoting convention.
        fn quote_path(path: &str) -> String {
            if !path
                .chars()
                .any(|ch| ch.is_ascii_control() || matches!(ch, ' ' | '"' | '\\'))
            {
                return path.to_string();
            }
            let mut quoted = String::from("\"");
            for ch in path.chars() {
                match ch {
                    '"' => quoted.push_str("\\\""),
                    '\\' => quoted.push_str("\\\\"),
                    '\t' => quoted.push_str("\\t"),
                    '\n' => quoted.push_str("\\n"),
                    '\r' => quoted.push_str("\\r"),
                    ch if ch.is_ascii_control() => {
                        quoted.push_str(&format!("\\{:03o}", ch as u32));
                    }
                    ch => quoted.push(ch),
                }
            }
            quoted.push('"');
            quoted
        }

        let original_path = quote_path(&format!("a/{relative}"));
        let fixed_path = quote_path(&format!("b/{relative}"));
        let hunks = diff.splitn(3, '\n').nth(2).unwrap_or_default();
        format!("--- {original_path}\n+++ {fixed_path}\n{hunks}")
    }

    /// Emit one complete linear-space hunk, including final-newline markers.
    pub fn generate_diff(&self, original: &str, fixed: &str) -> String {
        if original == fixed {
            return String::new();
        }
        // A single complete hunk is linear in source size and retains enough
        // context to apply the patch, including CRLF and missing final newlines.
        // Avoid an unbounded quadratic LCS table for large source files.
        let original_lines: Vec<_> = original.split_inclusive('\n').collect();
        let fixed_lines: Vec<_> = fixed.split_inclusive('\n').collect();
        let mut diff = format!(
            "--- a/source.wfl\n+++ b/source.wfl\n@@ -{},{} +{},{} @@\n",
            usize::from(!original_lines.is_empty()),
            original_lines.len(),
            usize::from(!fixed_lines.is_empty()),
            fixed_lines.len()
        );
        let mut append = |prefix: char, line: &str| {
            diff.push(prefix);
            diff.push_str(line);
            if !line.ends_with('\n') {
                diff.push_str("\n\\ No newline at end of file\n");
            }
        };
        for i in 0..original_lines.len().max(fixed_lines.len()) {
            match (original_lines.get(i), fixed_lines.get(i)) {
                (Some(old), Some(new)) if old == new => append(' ', old),
                (old, new) => {
                    if let Some(old) = old {
                        append('-', old);
                    }
                    if let Some(new) = new {
                        append('+', new);
                    }
                }
            }
        }
        diff
    }

    /// Apply the effective project configuration used by the linter as well.
    pub fn load_config(&mut self, dir: &Path) {
        let config = crate::config::load_config(dir);
        self.indent_size = config.indent_size;
        self.max_line_length = config.max_line_length;
        self.snake_case_variables = config.snake_case_variables;
        self.trailing_whitespace = config.trailing_whitespace;
        self.consistent_keyword_case = config.consistent_keyword_case;
    }
}

#[cfg(test)]
mod tests;

impl Default for CodeFixer {
    fn default() -> Self {
        Self::new()
    }
}
