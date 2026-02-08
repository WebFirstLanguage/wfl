//! JavaScript Code Generator
//!
//! This module contains the JavaScript code generator that transforms
//! WFL AST nodes into JavaScript code.

use crate::parser::ast::{
    Anchor, Argument, CharClass, ErrorType, Expression, FileOpenMode, Literal, Operator, Parameter,
    PatternExpression, Program, Quantifier, Statement, UnaryOperator, WriteMode,
};

use super::runtime::get_runtime;
use super::{TranspileError, TranspileResult, TranspileWarning, TranspilerConfig};

/// JavaScript transpiler that converts WFL AST to JavaScript code
pub struct JavaScriptTranspiler {
    config: TranspilerConfig,
    indent_level: usize,
    warnings: Vec<TranspileWarning>,
    /// Track whether we're in an async context
    in_async: bool,
}

impl JavaScriptTranspiler {
    /// Create a new JavaScript transpiler with the given configuration
    pub fn new(config: TranspilerConfig) -> Self {
        Self {
            config,
            indent_level: 0,
            warnings: Vec::new(),
            in_async: false,
        }
    }

    /// Get the current indentation string
    fn indent(&self) -> String {
        self.config.indent.repeat(self.indent_level)
    }

    /// Increase indentation level
    fn push_indent(&mut self) {
        self.indent_level += 1;
    }

    /// Decrease indentation level
    fn pop_indent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    /// Add a warning
    fn warn(&mut self, message: impl Into<String>, line: usize, column: usize) {
        self.warnings.push(TranspileWarning {
            message: message.into(),
            line,
            column,
        });
    }

    /// Transpile a WFL program to JavaScript
    pub fn transpile(mut self, program: &Program) -> Result<TranspileResult, TranspileError> {
        let mut output = String::new();

        // Add runtime if configured
        if self.config.include_runtime {
            output.push_str(get_runtime(self.config.target));
            output.push_str("\n\n");
        }

        // Wrap in IIFE if not using ES modules
        if !self.config.es_modules {
            output.push_str("(function() {\n");
            output.push_str("'use strict';\n\n");
            self.push_indent();
        }

        // First pass: collect all action definitions to hoist them
        let (actions, other_stmts): (Vec<_>, Vec<_>) = program
            .statements
            .iter()
            .partition(|s| matches!(s, Statement::ActionDefinition { .. }));

        // Generate action definitions first (hoisted)
        for stmt in &actions {
            let code = self.transpile_statement(stmt)?;
            output.push_str(&code);
            output.push('\n');
        }

        // Generate other statements
        for stmt in &other_stmts {
            let code = self.transpile_statement(stmt)?;
            output.push_str(&code);
            output.push('\n');
        }

        // Check for main action and call it
        let main_action = actions
            .iter()
            .find(|s| matches!(s, Statement::ActionDefinition { name, .. } if name == "main"));

        if let Some(Statement::ActionDefinition { body, .. }) = main_action {
            let is_main_async = self.contains_async(body);
            output.push_str(&self.indent());
            output.push_str("// Entry point\n");
            output.push_str(&self.indent());
            if is_main_async {
                output.push_str("(async () => { await main(); })();\n");
            } else {
                output.push_str("main();\n");
            }
        }

        // Close IIFE
        if !self.config.es_modules {
            self.pop_indent();
            output.push_str("})();\n");
        }

        Ok(TranspileResult {
            code: output,
            warnings: self.warnings,
        })
    }

    /// Transpile a statement to JavaScript
    fn transpile_statement(&mut self, stmt: &Statement) -> Result<String, TranspileError> {
        match stmt {
            Statement::VariableDeclaration {
                name,
                value,
                is_constant,
                ..
            } => {
                let keyword = if *is_constant { "const" } else { "let" };
                let val = self.transpile_expression(value)?;
                Ok(format!(
                    "{}{} {} = {};\n",
                    self.indent(),
                    keyword,
                    self.sanitize_name(name),
                    val
                ))
            }

            Statement::Assignment { name, value, .. } => {
                let val = self.transpile_expression(value)?;
                Ok(format!(
                    "{}{} = {};\n",
                    self.indent(),
                    self.sanitize_name(name),
                    val
                ))
            }

            Statement::IfStatement {
                condition,
                then_block,
                else_block,
                ..
            } => {
                let cond = self.transpile_expression(condition)?;
                let mut result = format!("{}if ({}) {{\n", self.indent(), cond);
                self.push_indent();
                for s in then_block {
                    result.push_str(&self.transpile_statement(s)?);
                }
                self.pop_indent();
                if let Some(else_stmts) = else_block {
                    result.push_str(&format!("{}}} else {{\n", self.indent()));
                    self.push_indent();
                    for s in else_stmts {
                        result.push_str(&self.transpile_statement(s)?);
                    }
                    self.pop_indent();
                }
                result.push_str(&format!("{}}}\n", self.indent()));
                Ok(result)
            }

            Statement::SingleLineIf {
                condition,
                then_stmt,
                else_stmt,
                ..
            } => {
                let cond = self.transpile_expression(condition)?;
                let then_code = self.transpile_statement(then_stmt)?;
                let mut result =
                    format!("{}if ({}) {}", self.indent(), cond, then_code.trim_start());
                if let Some(else_s) = else_stmt {
                    let else_code = self.transpile_statement(else_s)?;
                    result = format!(
                        "{}if ({}) {{ {} }} else {{ {} }}\n",
                        self.indent(),
                        cond,
                        then_code.trim(),
                        else_code.trim()
                    );
                }
                Ok(result)
            }

            Statement::ForEachLoop {
                item_name,
                collection,
                reversed,
                body,
                ..
            } => {
                let coll = self.transpile_expression(collection)?;
                let iter_var = self.sanitize_name(item_name);
                let mut result = if *reversed {
                    format!(
                        "{}for (const {} of [...{}].reverse()) {{\n",
                        self.indent(),
                        iter_var,
                        coll
                    )
                } else {
                    format!("{}for (const {} of {}) {{\n", self.indent(), iter_var, coll)
                };
                self.push_indent();
                for s in body {
                    result.push_str(&self.transpile_statement(s)?);
                }
                self.pop_indent();
                result.push_str(&format!("{}}}\n", self.indent()));
                Ok(result)
            }

            Statement::CountLoop {
                start,
                end,
                step,
                downward,
                variable_name,
                body,
                ..
            } => {
                let start_val = self.transpile_expression(start)?;
                let end_val = self.transpile_expression(end)?;
                let var_name = variable_name.as_deref().unwrap_or("count");
                let var_name = self.sanitize_name(var_name);
                let step_val = step
                    .as_ref()
                    .map(|s| self.transpile_expression(s))
                    .transpose()?
                    .unwrap_or_else(|| "1".to_string());

                let (compare_op, update_op) = if *downward {
                    (">=", "-=")
                } else {
                    ("<=", "+=")
                };

                let mut result = format!(
                    "{}for (let {} = {}; {} {} {}; {} {} {}) {{\n",
                    self.indent(),
                    var_name,
                    start_val,
                    var_name,
                    compare_op,
                    end_val,
                    var_name,
                    update_op,
                    step_val
                );
                self.push_indent();
                for s in body {
                    result.push_str(&self.transpile_statement(s)?);
                }
                self.pop_indent();
                result.push_str(&format!("{}}}\n", self.indent()));
                Ok(result)
            }

            Statement::WhileLoop {
                condition, body, ..
            } => {
                let cond = self.transpile_expression(condition)?;
                let mut result = format!("{}while ({}) {{\n", self.indent(), cond);
                self.push_indent();
                for s in body {
                    result.push_str(&self.transpile_statement(s)?);
                }
                self.pop_indent();
                result.push_str(&format!("{}}}\n", self.indent()));
                Ok(result)
            }

            Statement::RepeatWhileLoop {
                condition, body, ..
            } => {
                let cond = self.transpile_expression(condition)?;
                let mut result = format!("{}do {{\n", self.indent());
                self.push_indent();
                for s in body {
                    result.push_str(&self.transpile_statement(s)?);
                }
                self.pop_indent();
                result.push_str(&format!("{}}} while ({});\n", self.indent(), cond));
                Ok(result)
            }

            Statement::RepeatUntilLoop {
                condition, body, ..
            } => {
                let cond = self.transpile_expression(condition)?;
                let mut result = format!("{}do {{\n", self.indent());
                self.push_indent();
                for s in body {
                    result.push_str(&self.transpile_statement(s)?);
                }
                self.pop_indent();
                result.push_str(&format!("{}}} while (!({}));\n", self.indent(), cond));
                Ok(result)
            }

            Statement::ForeverLoop { body, .. } => {
                let mut result = format!("{}while (true) {{\n", self.indent());
                self.push_indent();
                for s in body {
                    result.push_str(&self.transpile_statement(s)?);
                }
                self.pop_indent();
                result.push_str(&format!("{}}}\n", self.indent()));
                Ok(result)
            }

            Statement::MainLoop { body, .. } => {
                // Main loop is essentially a forever loop
                let mut result = format!("{}while (true) {{\n", self.indent());
                self.push_indent();
                for s in body {
                    result.push_str(&self.transpile_statement(s)?);
                }
                self.pop_indent();
                result.push_str(&format!("{}}}\n", self.indent()));
                Ok(result)
            }

            Statement::DisplayStatement { value, .. } => {
                let val = self.transpile_expression(value)?;
                Ok(format!("{}WFL.display({});\n", self.indent(), val))
            }

            Statement::ActionDefinition {
                name,
                parameters,
                body,
                ..
            } => {
                // Check if the body contains any async operations
                let is_async = self.contains_async(body);
                let old_async = self.in_async;
                self.in_async = is_async;

                let params = parameters
                    .iter()
                    .map(|p| self.transpile_parameter(p))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");

                let async_keyword = if is_async { "async " } else { "" };
                let mut result = format!(
                    "{}{}function {}({}) {{\n",
                    self.indent(),
                    async_keyword,
                    self.sanitize_name(name),
                    params
                );
                self.push_indent();
                for s in body {
                    result.push_str(&self.transpile_statement(s)?);
                }
                self.pop_indent();
                result.push_str(&format!("{}}}\n", self.indent()));

                self.in_async = old_async;
                Ok(result)
            }

            Statement::ReturnStatement { value, .. } => {
                if let Some(val) = value {
                    let v = self.transpile_expression(val)?;
                    Ok(format!("{}return {};\n", self.indent(), v))
                } else {
                    Ok(format!("{}return;\n", self.indent()))
                }
            }

            Statement::ExpressionStatement { expression, .. } => {
                let expr = self.transpile_expression(expression)?;
                Ok(format!("{}{};\n", self.indent(), expr))
            }

            Statement::BreakStatement { .. } => Ok(format!("{}break;\n", self.indent())),

            Statement::ContinueStatement { .. } => Ok(format!("{}continue;\n", self.indent())),

            Statement::ExitStatement { .. } => Ok(format!("{}WFL.exit(0);\n", self.indent())),

            Statement::CreateListStatement {
                name,
                initial_values,
                ..
            } => {
                let values = initial_values
                    .iter()
                    .map(|e| self.transpile_expression(e))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                Ok(format!(
                    "{}let {} = [{}];\n",
                    self.indent(),
                    self.sanitize_name(name),
                    values
                ))
            }

            Statement::PushStatement { list, value, .. } => {
                let list_expr = self.transpile_expression(list)?;
                let val = self.transpile_expression(value)?;
                Ok(format!("{}{}.push({});\n", self.indent(), list_expr, val))
            }

            Statement::AddToListStatement {
                value, list_name, ..
            } => {
                let val = self.transpile_expression(value)?;
                Ok(format!(
                    "{}{}.push({});\n",
                    self.indent(),
                    self.sanitize_name(list_name),
                    val
                ))
            }

            Statement::RemoveFromListStatement {
                value, list_name, ..
            } => {
                let val = self.transpile_expression(value)?;
                Ok(format!(
                    "{}WFL.list.remove({}, {});\n",
                    self.indent(),
                    self.sanitize_name(list_name),
                    val
                ))
            }

            Statement::ClearListStatement { list_name, .. } => Ok(format!(
                "{}{}.length = 0;\n",
                self.indent(),
                self.sanitize_name(list_name)
            )),

            Statement::MapCreation { name, entries, .. } => {
                let mut result =
                    format!("{}let {} = {{\n", self.indent(), self.sanitize_name(name));
                self.push_indent();
                for (key, value) in entries {
                    let val = self.transpile_expression(value)?;
                    result.push_str(&format!(
                        "{}{}: {},\n",
                        self.indent(),
                        self.escape_key(key),
                        val
                    ));
                }
                self.pop_indent();
                result.push_str(&format!("{}}};\n", self.indent()));
                Ok(result)
            }

            Statement::ReadFileStatement {
                path,
                variable_name,
                ..
            } => {
                let path_expr = self.transpile_expression(path)?;
                Ok(format!(
                    "{}let {} = WFL.file.read({});\n",
                    self.indent(),
                    self.sanitize_name(variable_name),
                    path_expr
                ))
            }

            Statement::WriteFileStatement {
                file,
                content,
                mode,
                ..
            } => {
                let file_expr = self.transpile_expression(file)?;
                let content_expr = self.transpile_expression(content)?;
                let func = match mode {
                    WriteMode::Overwrite => "write",
                    WriteMode::Append => "append",
                };
                Ok(format!(
                    "{}WFL.file.{}({}, {});\n",
                    self.indent(),
                    func,
                    file_expr,
                    content_expr
                ))
            }

            Statement::CreateFileStatement { path, content, .. } => {
                let path_expr = self.transpile_expression(path)?;
                let content_expr = self.transpile_expression(content)?;
                Ok(format!(
                    "{}WFL.file.write({}, {});\n",
                    self.indent(),
                    path_expr,
                    content_expr
                ))
            }

            Statement::DeleteFileStatement { path, .. } => {
                let path_expr = self.transpile_expression(path)?;
                Ok(format!(
                    "{}WFL.file.delete({});\n",
                    self.indent(),
                    path_expr
                ))
            }

            Statement::CreateDirectoryStatement { path, .. } => {
                let path_expr = self.transpile_expression(path)?;
                Ok(format!(
                    "{}WFL.directory.create({});\n",
                    self.indent(),
                    path_expr
                ))
            }

            Statement::DeleteDirectoryStatement { path, .. } => {
                let path_expr = self.transpile_expression(path)?;
                Ok(format!(
                    "{}WFL.directory.delete({});\n",
                    self.indent(),
                    path_expr
                ))
            }

            Statement::OpenFileStatement {
                path,
                variable_name,
                mode,
                ..
            } => {
                let path_expr = self.transpile_expression(path)?;
                let mode_str = match mode {
                    FileOpenMode::Read => "r",
                    FileOpenMode::Write => "w",
                    FileOpenMode::Append => "a",
                    FileOpenMode::ReadBinary => "rb",
                    FileOpenMode::WriteBinary => "wb",
                };
                Ok(format!(
                    "{}let {} = {{ path: {}, mode: '{}', content: null }};\n",
                    self.indent(),
                    self.sanitize_name(variable_name),
                    path_expr,
                    mode_str
                ))
            }

            Statement::WriteToStatement { content, file, .. } => {
                let content_expr = self.transpile_expression(content)?;
                let file_expr = self.transpile_expression(file)?;
                Ok(format!(
                    "{}WFL.file.write({}.path, {});\n",
                    self.indent(),
                    file_expr,
                    content_expr
                ))
            }

            Statement::CloseFileStatement { .. } => {
                // JavaScript doesn't need explicit file closing for sync operations
                Ok(format!("{}// File closed (no-op in JS)\n", self.indent()))
            }

            Statement::WriteContentStatement {
                content, target, ..
            } => {
                let content_expr = self.transpile_expression(content)?;
                let target_expr = self.transpile_expression(target)?;
                Ok(format!(
                    "{}WFL.file.write({}, {});\n",
                    self.indent(),
                    target_expr,
                    content_expr
                ))
            }

            Statement::WriteBinaryStatement {
                content, target, ..
            } => {
                let content_expr = self.transpile_expression(content)?;
                let target_expr = self.transpile_expression(target)?;
                Ok(format!(
                    "{}WFL.file.writeBinary({}.path, {});\n",
                    self.indent(),
                    target_expr,
                    content_expr
                ))
            }

            Statement::ExecuteCommandStatement {
                command,
                arguments,
                variable_name,
                ..
            } => {
                let cmd = self.transpile_expression(command)?;
                let args = arguments
                    .as_ref()
                    .map(|a| self.transpile_expression(a))
                    .transpose()?;
                let exec_call = if let Some(a) = args {
                    format!("WFL.process.execute({}, {})", cmd, a)
                } else {
                    format!("WFL.process.execute({})", cmd)
                };
                if let Some(var) = variable_name {
                    Ok(format!(
                        "{}let {} = {};\n",
                        self.indent(),
                        self.sanitize_name(var),
                        exec_call
                    ))
                } else {
                    Ok(format!("{}{};\n", self.indent(), exec_call))
                }
            }

            Statement::SpawnProcessStatement {
                command,
                arguments,
                variable_name,
                ..
            } => {
                let cmd = self.transpile_expression(command)?;
                let args = arguments
                    .as_ref()
                    .map(|a| self.transpile_expression(a))
                    .transpose()?;
                let spawn_call = if let Some(a) = args {
                    format!("WFL.process.spawn({}, {})", cmd, a)
                } else {
                    format!("WFL.process.spawn({})", cmd)
                };
                Ok(format!(
                    "{}let {} = {};\n",
                    self.indent(),
                    self.sanitize_name(variable_name),
                    spawn_call
                ))
            }

            Statement::KillProcessStatement { process_id, .. } => {
                let pid = self.transpile_expression(process_id)?;
                Ok(format!("{}WFL.process.kill({});\n", self.indent(), pid))
            }

            Statement::ReadProcessOutputStatement {
                process_id,
                variable_name,
                ..
            } => {
                let pid = self.transpile_expression(process_id)?;
                Ok(format!(
                    "{}let {} = {}.stdout;\n",
                    self.indent(),
                    self.sanitize_name(variable_name),
                    pid
                ))
            }

            Statement::WaitForProcessStatement {
                process_id,
                variable_name,
                ..
            } => {
                let pid = self.transpile_expression(process_id)?;
                if let Some(var) = variable_name {
                    Ok(format!(
                        "{}let {} = await {}.exitCode;\n",
                        self.indent(),
                        self.sanitize_name(var),
                        pid
                    ))
                } else {
                    Ok(format!("{}await {}.exitCode;\n", self.indent(), pid))
                }
            }

            Statement::WaitForStatement { inner, .. } => {
                // For variable declarations, we need to await the expression part,
                // not the entire statement
                match inner.as_ref() {
                    Statement::VariableDeclaration {
                        name,
                        value,
                        is_constant,
                        ..
                    } => {
                        let js_name = self.sanitize_name(name);
                        let awaited_value = format!("await {}", self.transpile_expression(value)?);
                        let keyword = if *is_constant { "const" } else { "let" };
                        Ok(format!(
                            "{}{} {} = {};\n",
                            self.indent(),
                            keyword,
                            js_name,
                            awaited_value
                        ))
                    }
                    Statement::Assignment { name, value, .. } => {
                        let js_name = self.sanitize_name(name);
                        let awaited_value = format!("await {}", self.transpile_expression(value)?);
                        Ok(format!(
                            "{}{} = {};\n",
                            self.indent(),
                            js_name,
                            awaited_value
                        ))
                    }
                    _ => {
                        // For other statement types, wrap the result in await
                        let inner_code = self.transpile_statement(inner)?;
                        let trimmed = inner_code.trim();
                        if let Some(expr) = trimmed.strip_suffix(';') {
                            Ok(format!("{}await {};\n", self.indent(), expr.trim_start()))
                        } else {
                            Ok(format!("{}await {};\n", self.indent(), trimmed))
                        }
                    }
                }
            }

            Statement::WaitForDurationStatement { duration, unit, .. } => {
                let dur = self.transpile_expression(duration)?;
                let ms = match unit.as_str() {
                    "milliseconds" | "ms" => dur,
                    "seconds" | "s" => format!("({}) * 1000", dur),
                    "minutes" | "m" => format!("({}) * 60000", dur),
                    "hours" | "h" => format!("({}) * 3600000", dur),
                    _ => dur,
                };
                Ok(format!("{}await WFL.sleep({});\n", self.indent(), ms))
            }

            Statement::HttpGetStatement {
                url, variable_name, ..
            } => {
                let url_expr = self.transpile_expression(url)?;
                Ok(format!(
                    "{}let {} = await WFL.http.get({});\n",
                    self.indent(),
                    self.sanitize_name(variable_name),
                    url_expr
                ))
            }

            Statement::HttpPostStatement {
                url,
                data,
                variable_name,
                ..
            } => {
                let url_expr = self.transpile_expression(url)?;
                let data_expr = self.transpile_expression(data)?;
                Ok(format!(
                    "{}let {} = await WFL.http.post({}, {});\n",
                    self.indent(),
                    self.sanitize_name(variable_name),
                    url_expr,
                    data_expr
                ))
            }

            Statement::TryStatement {
                body,
                when_clauses,
                otherwise_block,
                ..
            } => {
                let mut result = format!("{}try {{\n", self.indent());
                self.push_indent();
                for s in body {
                    result.push_str(&self.transpile_statement(s)?);
                }
                self.pop_indent();
                result.push_str(&format!("{}}} catch (_wfl_error) {{\n", self.indent()));
                self.push_indent();

                // Generate when clauses as if-else chain
                let mut first = true;
                for clause in when_clauses {
                    let error_check = match clause.error_type {
                        ErrorType::General => "true".to_string(),
                        ErrorType::FileNotFound => "_wfl_error.code === 'ENOENT'".to_string(),
                        ErrorType::PermissionDenied => "_wfl_error.code === 'EACCES'".to_string(),
                        _ => "true".to_string(),
                    };

                    let keyword = if first { "if" } else { "else if" };
                    first = false;
                    result.push_str(&format!(
                        "{}{} ({}) {{\n",
                        self.indent(),
                        keyword,
                        error_check
                    ));
                    self.push_indent();
                    // Bind the error name
                    result.push_str(&format!(
                        "{}let {} = _wfl_error;\n",
                        self.indent(),
                        self.sanitize_name(&clause.error_name)
                    ));
                    for s in &clause.body {
                        result.push_str(&self.transpile_statement(s)?);
                    }
                    self.pop_indent();
                    result.push_str(&format!("{}}}", self.indent()));
                }

                if let Some(otherwise) = otherwise_block {
                    if first {
                        // No when clauses, just execute otherwise
                        for s in otherwise {
                            result.push_str(&self.transpile_statement(s)?);
                        }
                    } else {
                        result.push_str(" else {\n");
                        self.push_indent();
                        for s in otherwise {
                            result.push_str(&self.transpile_statement(s)?);
                        }
                        self.pop_indent();
                        result.push_str(&format!("{}}}", self.indent()));
                    }
                }
                result.push('\n');

                self.pop_indent();
                result.push_str(&format!("{}}}\n", self.indent()));
                Ok(result)
            }

            Statement::ContainerDefinition {
                name,
                extends,
                properties,
                methods,
                static_properties,
                static_methods,
                ..
            } => {
                let extends_clause = extends
                    .as_ref()
                    .map(|e| format!(" extends {}", e))
                    .unwrap_or_default();

                let mut result = format!(
                    "{}class {}{} {{\n",
                    self.indent(),
                    self.sanitize_name(name),
                    extends_clause
                );
                self.push_indent();

                // Constructor
                result.push_str(&format!("{}constructor() {{\n", self.indent()));
                self.push_indent();
                if extends.is_some() {
                    result.push_str(&format!("{}super();\n", self.indent()));
                }
                result.push_str(&format!("{}this._wfl_container = true;\n", self.indent()));
                result.push_str(&format!("{}this._wfl_type = '{}';\n", self.indent(), name));

                // Initialize properties
                for prop in properties {
                    let default_val = prop
                        .default_value
                        .as_ref()
                        .map(|v| self.transpile_expression(v))
                        .transpose()?
                        .unwrap_or_else(|| "null".to_string());
                    result.push_str(&format!(
                        "{}this.{} = {};\n",
                        self.indent(),
                        self.sanitize_name(&prop.name),
                        default_val
                    ));
                }

                self.pop_indent();
                result.push_str(&format!("{}}}\n", self.indent()));

                // Static properties
                for prop in static_properties {
                    let default_val = prop
                        .default_value
                        .as_ref()
                        .map(|v| self.transpile_expression(v))
                        .transpose()?
                        .unwrap_or_else(|| "null".to_string());
                    result.push_str(&format!(
                        "{}static {} = {};\n",
                        self.indent(),
                        self.sanitize_name(&prop.name),
                        default_val
                    ));
                }

                // Methods
                for method in methods {
                    if let Statement::ActionDefinition {
                        name: method_name,
                        parameters,
                        body,
                        ..
                    } = method
                    {
                        let is_async = self.contains_async(body);
                        let async_keyword = if is_async { "async " } else { "" };
                        let params = parameters
                            .iter()
                            .map(|p| self.transpile_parameter(p))
                            .collect::<Result<Vec<_>, _>>()?
                            .join(", ");

                        result.push_str(&format!(
                            "{}{}{}({}) {{\n",
                            self.indent(),
                            async_keyword,
                            self.sanitize_name(method_name),
                            params
                        ));
                        self.push_indent();
                        for s in body {
                            result.push_str(&self.transpile_statement(s)?);
                        }
                        self.pop_indent();
                        result.push_str(&format!("{}}}\n", self.indent()));
                    }
                }

                // Static methods
                for method in static_methods {
                    if let Statement::ActionDefinition {
                        name: method_name,
                        parameters,
                        body,
                        ..
                    } = method
                    {
                        let is_async = self.contains_async(body);
                        let async_keyword = if is_async { "async " } else { "" };
                        let params = parameters
                            .iter()
                            .map(|p| self.transpile_parameter(p))
                            .collect::<Result<Vec<_>, _>>()?
                            .join(", ");

                        result.push_str(&format!(
                            "{}static {}{}({}) {{\n",
                            self.indent(),
                            async_keyword,
                            self.sanitize_name(method_name),
                            params
                        ));
                        self.push_indent();
                        for s in body {
                            result.push_str(&self.transpile_statement(s)?);
                        }
                        self.pop_indent();
                        result.push_str(&format!("{}}}\n", self.indent()));
                    }
                }

                self.pop_indent();
                result.push_str(&format!("{}}}\n", self.indent()));
                Ok(result)
            }

            Statement::ContainerInstantiation {
                container_type,
                instance_name,
                property_initializers,
                ..
            } => {
                let mut result = format!(
                    "{}let {} = new {}();\n",
                    self.indent(),
                    self.sanitize_name(instance_name),
                    self.sanitize_name(container_type)
                );
                for init in property_initializers {
                    let val = self.transpile_expression(&init.value)?;
                    result.push_str(&format!(
                        "{}{}.{} = {};\n",
                        self.indent(),
                        self.sanitize_name(instance_name),
                        self.sanitize_name(&init.name),
                        val
                    ));
                }
                Ok(result)
            }

            Statement::InterfaceDefinition {
                name, line, column, ..
            } => {
                // Interfaces don't exist in JavaScript, emit a comment
                self.warn(
                    format!("Interface '{}' has no JavaScript equivalent, skipped", name),
                    *line,
                    *column,
                );
                Ok(format!(
                    "{}// Interface {} (no JS equivalent)\n",
                    self.indent(),
                    name
                ))
            }

            Statement::PatternDefinition { name, pattern, .. } => {
                let pattern_regex = self.transpile_pattern_expression(pattern)?;
                Ok(format!(
                    "{}const {} = new WFL.Pattern({});\n",
                    self.indent(),
                    self.sanitize_name(name),
                    pattern_regex
                ))
            }

            Statement::CreateDateStatement { name, value, .. } => {
                let date_expr = if let Some(v) = value {
                    let val = self.transpile_expression(v)?;
                    format!("new Date({})", val)
                } else {
                    "WFL.time.today()".to_string()
                };
                Ok(format!(
                    "{}let {} = {};\n",
                    self.indent(),
                    self.sanitize_name(name),
                    date_expr
                ))
            }

            Statement::CreateTimeStatement { name, value, .. } => {
                let time_expr = if let Some(v) = value {
                    let val = self.transpile_expression(v)?;
                    format!("new Date({})", val)
                } else {
                    "new Date()".to_string()
                };
                Ok(format!(
                    "{}let {} = {};\n",
                    self.indent(),
                    self.sanitize_name(name),
                    time_expr
                ))
            }

            Statement::EventDefinition {
                name, line, column, ..
            } => {
                self.warn(
                    format!("Event '{}' definition not fully supported in JS", name),
                    *line,
                    *column,
                );
                Ok(format!("{}// Event: {}\n", self.indent(), name))
            }

            Statement::EventTrigger {
                name, arguments, ..
            } => {
                let args = arguments
                    .iter()
                    .map(|a| self.transpile_argument(a))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                Ok(format!(
                    "{}this.dispatchEvent(new CustomEvent('{}', {{ detail: [{}] }}));\n",
                    self.indent(),
                    name,
                    args
                ))
            }

            Statement::EventHandler {
                event_source,
                event_name,
                handler_body,
                ..
            } => {
                let source = self.transpile_expression(event_source)?;
                let mut result = format!(
                    "{}{}.addEventListener('{}', (event) => {{\n",
                    self.indent(),
                    source,
                    event_name
                );
                self.push_indent();
                for s in handler_body {
                    result.push_str(&self.transpile_statement(s)?);
                }
                self.pop_indent();
                result.push_str(&format!("{}}});\n", self.indent()));
                Ok(result)
            }

            Statement::ParentMethodCall {
                method_name,
                arguments,
                ..
            } => {
                let args = arguments
                    .iter()
                    .map(|a| self.transpile_argument(a))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                Ok(format!(
                    "{}super.{}({});\n",
                    self.indent(),
                    self.sanitize_name(method_name),
                    args
                ))
            }

            Statement::LoadModuleStatement {
                path,
                alias,
                line,
                column,
                ..
            } => {
                self.warn(
                    "Module loading requires bundler support in JavaScript",
                    *line,
                    *column,
                );
                let path_expr = self.transpile_expression(path)?;
                if let Some(a) = alias {
                    Ok(format!(
                        "{}const {} = require({});\n",
                        self.indent(),
                        self.sanitize_name(a),
                        path_expr
                    ))
                } else {
                    Ok(format!("{}require({});\n", self.indent(), path_expr))
                }
            }

            Statement::ListenStatement {
                port,
                server_name,
                line,
                column,
                ..
            } => {
                self.warn(
                    "Server functionality has limitations: WFL's web server statements are transpiled to basic Node.js http setup. Features like middleware, routing, and request handling require manual implementation in JS",
                    *line,
                    *column,
                );
                let port_expr = self.transpile_expression(port)?;
                Ok(format!(
                    "{}// Note: Basic server setup only - implement request handlers manually\n{}const {} = require('http').createServer(); {}.listen({});\n",
                    self.indent(),
                    self.indent(),
                    self.sanitize_name(server_name),
                    self.sanitize_name(server_name),
                    port_expr
                ))
            }

            Statement::WaitForRequestStatement { line, column, .. } => {
                self.warn(
                    "WaitForRequest cannot be directly transpiled: WFL's synchronous request waiting model doesn't map to JavaScript's event-driven model. Use server.on('request', callback) pattern instead",
                    *line,
                    *column,
                );
                Ok(format!(
                    "{}// TODO: WaitForRequest - implement using server.on('request', (req, res) => {{ ... }}) pattern\n",
                    self.indent()
                ))
            }

            Statement::RespondStatement {
                content,
                status,
                content_type,
                line,
                column,
                ..
            } => {
                self.warn("Respond requires request context in JS", *line, *column);
                let content_expr = self.transpile_expression(content)?;
                let status_expr = status
                    .as_ref()
                    .map(|s| self.transpile_expression(s))
                    .transpose()?
                    .unwrap_or_else(|| "200".to_string());
                let ct_expr = content_type
                    .as_ref()
                    .map(|c| self.transpile_expression(c))
                    .transpose()?
                    .unwrap_or_else(|| "'text/html'".to_string());
                Ok(format!(
                    "{}// response.writeHead({}, {{ 'Content-Type': {} }}); response.end({});\n",
                    self.indent(),
                    status_expr,
                    ct_expr,
                    content_expr
                ))
            }

            Statement::RegisterSignalHandlerStatement {
                signal_type,
                handler_name,
                ..
            } => {
                let signal = match signal_type.as_str() {
                    "SIGINT" => "SIGINT",
                    "SIGTERM" => "SIGTERM",
                    _ => signal_type,
                };
                Ok(format!(
                    "{}process.on('{}', {});\n",
                    self.indent(),
                    signal,
                    self.sanitize_name(handler_name)
                ))
            }

            Statement::StopAcceptingConnectionsStatement { server, .. } => {
                let server_expr = self.transpile_expression(server)?;
                Ok(format!("{}{}.close();\n", self.indent(), server_expr))
            }

            Statement::CloseServerStatement { server, .. } => {
                let server_expr = self.transpile_expression(server)?;
                Ok(format!("{}{}.close();\n", self.indent(), server_expr))
            }

            // Testing statements - transpile to JavaScript test framework equivalents
            Statement::DescribeBlock {
                description,
                setup,
                teardown,
                tests,
                ..
            } => {
                let mut result = format!(
                    "{}describe({}, function() {{\n",
                    self.indent(),
                    self.escape_string(description)
                );
                self.push_indent();
                // Transpile setup (beforeEach)
                if let Some(setup_stmts) = setup {
                    result.push_str(&format!("{}beforeEach(function() {{\n", self.indent()));
                    self.push_indent();
                    for stmt in setup_stmts {
                        result.push_str(&self.transpile_statement(stmt)?);
                    }
                    self.pop_indent();
                    result.push_str(&format!("{}}});\n", self.indent()));
                }
                // Transpile teardown (afterEach)
                if let Some(teardown_stmts) = teardown {
                    result.push_str(&format!("{}afterEach(function() {{\n", self.indent()));
                    self.push_indent();
                    for stmt in teardown_stmts {
                        result.push_str(&self.transpile_statement(stmt)?);
                    }
                    self.pop_indent();
                    result.push_str(&format!("{}}});\n", self.indent()));
                }
                // Transpile tests
                for stmt in tests {
                    result.push_str(&self.transpile_statement(stmt)?);
                }
                self.pop_indent();
                result.push_str(&format!("{}}});\n", self.indent()));
                Ok(result)
            }

            Statement::TestBlock {
                description, body, ..
            } => {
                let mut result = format!(
                    "{}it({}, function() {{\n",
                    self.indent(),
                    self.escape_string(description)
                );
                self.push_indent();
                for stmt in body {
                    result.push_str(&self.transpile_statement(stmt)?);
                }
                self.pop_indent();
                result.push_str(&format!("{}}});\n", self.indent()));
                Ok(result)
            }

            Statement::ExpectStatement {
                subject, assertion, ..
            } => {
                use crate::parser::ast::Assertion;
                let subject_expr = self.transpile_expression(subject)?;
                match assertion {
                    Assertion::Equal(expected) => {
                        let expected_expr = self.transpile_expression(expected)?;
                        Ok(format!(
                            "{}expect({}).toEqual({});\n",
                            self.indent(),
                            subject_expr,
                            expected_expr
                        ))
                    }
                    Assertion::Be(expected) => {
                        let expected_expr = self.transpile_expression(expected)?;
                        Ok(format!(
                            "{}expect({}).toBe({});\n",
                            self.indent(),
                            subject_expr,
                            expected_expr
                        ))
                    }
                    Assertion::GreaterThan(expected) => {
                        let expected_expr = self.transpile_expression(expected)?;
                        Ok(format!(
                            "{}expect({}).toBeGreaterThan({});\n",
                            self.indent(),
                            subject_expr,
                            expected_expr
                        ))
                    }
                    Assertion::LessThan(expected) => {
                        let expected_expr = self.transpile_expression(expected)?;
                        Ok(format!(
                            "{}expect({}).toBeLessThan({});\n",
                            self.indent(),
                            subject_expr,
                            expected_expr
                        ))
                    }
                    Assertion::BeYes => Ok(format!(
                        "{}expect({}).toBeTruthy();\n",
                        self.indent(),
                        subject_expr
                    )),
                    Assertion::BeNo => Ok(format!(
                        "{}expect({}).toBeFalsy();\n",
                        self.indent(),
                        subject_expr
                    )),
                    Assertion::Exist => Ok(format!(
                        "{}expect({}).toBeDefined();\n",
                        self.indent(),
                        subject_expr
                    )),
                    Assertion::Contain(expected) => {
                        let expected_expr = self.transpile_expression(expected)?;
                        Ok(format!(
                            "{}expect({}).toContain({});\n",
                            self.indent(),
                            subject_expr,
                            expected_expr
                        ))
                    }
                    Assertion::BeEmpty => Ok(format!(
                        "{}expect({}).toHaveLength(0);\n",
                        self.indent(),
                        subject_expr
                    )),
                    Assertion::HaveLength(expected) => {
                        let expected_expr = self.transpile_expression(expected)?;
                        Ok(format!(
                            "{}expect({}).toHaveLength({});\n",
                            self.indent(),
                            subject_expr,
                            expected_expr
                        ))
                    }
                    Assertion::BeOfType(type_name) => Ok(format!(
                        "{}expect(typeof {}).toBe({});\n",
                        self.indent(),
                        subject_expr,
                        self.escape_string(type_name)
                    )),
                }
            }

            Statement::IncludeStatement { line, column, .. } => {
                // Include statements are not supported in JavaScript transpilation
                Err(TranspileError {
                    message: "Include statements are not supported in JavaScript transpilation. Use 'load module' instead.".to_string(),
                    line: *line,
                    column: *column,
                })
            }

            Statement::ExportStatement { line, column, .. } => {
                // Export statements are not supported in JavaScript transpilation
                Err(TranspileError {
                    message: "Export statements are not supported in JavaScript transpilation. They are for module system management only.".to_string(),
                    line: *line,
                    column: *column,
                })
            }
        }
    }

    /// Transpile an expression to JavaScript
    fn transpile_expression(&mut self, expr: &Expression) -> Result<String, TranspileError> {
        match expr {
            Expression::Literal(lit, _, _) => self.transpile_literal(lit),

            Expression::Variable(name, _, _) => Ok(self.sanitize_name(name)),

            Expression::BinaryOperation {
                left,
                operator,
                right,
                ..
            } => {
                let l = self.transpile_expression(left)?;
                let r = self.transpile_expression(right)?;
                let op = self.transpile_operator(operator);
                // Handle special case for Contains
                if *operator == Operator::Contains {
                    Ok(format!("({}).includes({})", l, r))
                } else {
                    Ok(format!("({} {} {})", l, op, r))
                }
            }

            Expression::UnaryOperation {
                operator,
                expression,
                ..
            } => {
                let expr = self.transpile_expression(expression)?;
                let op = match operator {
                    UnaryOperator::Not => "!",
                    UnaryOperator::Minus => "-",
                };
                Ok(format!("({}{})", op, expr))
            }

            Expression::FunctionCall {
                function,
                arguments,
                ..
            } => {
                let func = self.transpile_expression(function)?;
                let args = arguments
                    .iter()
                    .map(|a| self.transpile_argument(a))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                Ok(format!("{}({})", func, args))
            }

            Expression::ActionCall {
                name, arguments, ..
            } => {
                let args = arguments
                    .iter()
                    .map(|a| self.transpile_argument(a))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                Ok(format!("{}({})", self.sanitize_name(name), args))
            }

            Expression::MemberAccess {
                object, property, ..
            } => {
                let obj = self.transpile_expression(object)?;
                // Handle built-in properties
                let prop = match property.as_str() {
                    "length" => "length",
                    _ => property,
                };
                Ok(format!("{}.{}", obj, self.sanitize_name(prop)))
            }

            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                let obj = self.transpile_expression(object)?;
                let args = arguments
                    .iter()
                    .map(|a| self.transpile_argument(a))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                Ok(format!("{}.{}({})", obj, self.sanitize_name(method), args))
            }

            Expression::PropertyAccess {
                object, property, ..
            } => {
                let obj = self.transpile_expression(object)?;
                Ok(format!("{}.{}", obj, self.sanitize_name(property)))
            }

            Expression::IndexAccess {
                collection, index, ..
            } => {
                let coll = self.transpile_expression(collection)?;
                let idx = self.transpile_expression(index)?;
                Ok(format!("{}[{}]", coll, idx))
            }

            Expression::Concatenation { left, right, .. } => {
                let l = self.transpile_expression(left)?;
                let r = self.transpile_expression(right)?;
                Ok(format!("(String({}) + String({}))", l, r))
            }

            Expression::PatternMatch { text, pattern, .. } => {
                let t = self.transpile_expression(text)?;
                let p = self.transpile_expression(pattern)?;
                Ok(format!("{}.match({})", p, t))
            }

            Expression::PatternFind { text, pattern, .. } => {
                let t = self.transpile_expression(text)?;
                let p = self.transpile_expression(pattern)?;
                Ok(format!("{}.find({})", p, t))
            }

            Expression::PatternReplace {
                text,
                pattern,
                replacement,
                ..
            } => {
                let t = self.transpile_expression(text)?;
                let p = self.transpile_expression(pattern)?;
                let r = self.transpile_expression(replacement)?;
                Ok(format!("{}.replace({}, {})", p, t, r))
            }

            Expression::PatternSplit { text, pattern, .. } => {
                let t = self.transpile_expression(text)?;
                let p = self.transpile_expression(pattern)?;
                Ok(format!("{}.split({})", p, t))
            }

            Expression::StringSplit {
                text, delimiter, ..
            } => {
                let t = self.transpile_expression(text)?;
                let d = self.transpile_expression(delimiter)?;
                Ok(format!("({}).split({})", t, d))
            }

            Expression::AwaitExpression { expression, .. } => {
                let expr = self.transpile_expression(expression)?;
                Ok(format!("(await {})", expr))
            }

            Expression::StaticMemberAccess {
                container, member, ..
            } => Ok(format!(
                "{}.{}",
                self.sanitize_name(container),
                self.sanitize_name(member)
            )),

            Expression::HeaderAccess {
                header_name,
                request,
                ..
            } => {
                let req = self.transpile_expression(request)?;
                Ok(format!("{}.headers['{}']", req, header_name.to_lowercase()))
            }

            Expression::CurrentTimeMilliseconds { .. } => Ok("Date.now()".to_string()),

            Expression::CurrentTimeFormatted { format, .. } => {
                Ok(format!("WFL.time.format(new Date(), '{}')", format))
            }

            Expression::FileExists { path, .. } => {
                let p = self.transpile_expression(path)?;
                Ok(format!("WFL.file.exists({})", p))
            }

            Expression::DirectoryExists { path, .. } => {
                let p = self.transpile_expression(path)?;
                Ok(format!("WFL.directory.exists({})", p))
            }

            Expression::ListFiles { path, .. } => {
                let p = self.transpile_expression(path)?;
                Ok(format!("WFL.directory.list({})", p))
            }

            Expression::ListFilesRecursive {
                path, extensions, ..
            } => {
                let p = self.transpile_expression(path)?;
                let ext = extensions
                    .as_ref()
                    .map(|exts| {
                        exts.iter()
                            .map(|e| self.transpile_expression(e))
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()?
                    .map(|v| format!("[{}]", v.join(", ")))
                    .unwrap_or_else(|| "[]".to_string());
                Ok(format!("WFL.directory.listRecursive({}, {})", p, ext))
            }

            Expression::ListFilesFiltered {
                path, extensions, ..
            } => {
                let p = self.transpile_expression(path)?;
                let ext = extensions
                    .iter()
                    .map(|e| self.transpile_expression(e))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                Ok(format!("WFL.directory.listRecursive({}, [{}])", p, ext))
            }

            Expression::ReadContent { file_handle, .. } => {
                let fh = self.transpile_expression(file_handle)?;
                Ok(format!("WFL.file.read({}.path)", fh))
            }

            Expression::ReadBinaryContent { file_handle, .. } => {
                let fh = self.transpile_expression(file_handle)?;
                Ok(format!("WFL.file.readBinary({}.path)", fh))
            }

            Expression::ReadBinaryN {
                file_handle, count, ..
            } => {
                let fh = self.transpile_expression(file_handle)?;
                let n = self.transpile_expression(count)?;
                Ok(format!("WFL.file.readBinaryN({}.path, {})", fh, n))
            }

            Expression::FileSizeOf { file_handle, .. } => {
                let fh = self.transpile_expression(file_handle)?;
                Ok(format!("WFL.file.size({}.path)", fh))
            }

            Expression::ProcessRunning { process_id, .. } => {
                let pid = self.transpile_expression(process_id)?;
                Ok(format!("WFL.process.isRunning({})", pid))
            }
        }
    }

    /// Transpile a literal to JavaScript
    fn transpile_literal(&self, lit: &Literal) -> Result<String, TranspileError> {
        match lit {
            Literal::String(s) => Ok(format!("\"{}\"", self.escape_string(s))),
            Literal::Integer(i) => Ok(i.to_string()),
            Literal::Float(f) => Ok(f.to_string()),
            Literal::Boolean(b) => Ok(if *b { "true" } else { "false" }.to_string()),
            Literal::Nothing => Ok("null".to_string()),
            Literal::Pattern(p) => Ok(format!("new WFL.Pattern({})", self.pattern_to_regex(p))),
            Literal::List(items) => {
                let elements = items
                    .iter()
                    .map(|e| self.clone().transpile_expression(e))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                Ok(format!("[{}]", elements))
            }
        }
    }

    /// Transpile an operator to JavaScript
    fn transpile_operator(&self, op: &Operator) -> &'static str {
        match op {
            Operator::Plus => "+",
            Operator::Minus => "-",
            Operator::Multiply => "*",
            Operator::Divide => "/",
            Operator::Modulo => "%",
            Operator::Equals => "===",
            Operator::NotEquals => "!==",
            Operator::GreaterThan => ">",
            Operator::LessThan => "<",
            Operator::GreaterThanOrEqual => ">=",
            Operator::LessThanOrEqual => "<=",
            Operator::And => "&&",
            Operator::Or => "||",
            Operator::Contains => "includes", // Handled specially in transpile_expression
        }
    }

    /// Transpile a parameter to JavaScript
    fn transpile_parameter(&self, param: &Parameter) -> Result<String, TranspileError> {
        let name = self.sanitize_name(&param.name);
        if let Some(default) = &param.default_value {
            // Clone self to transpile the expression (since we need mutable access)
            let mut cloned = self.clone();
            let default_val = cloned
                .transpile_expression(default)
                .map_err(|e| TranspileError {
                    message: format!(
                        "Failed to transpile default value for parameter '{}': {}",
                        name, e
                    ),
                    line: e.line,
                    column: e.column,
                })?;
            Ok(format!("{} = {}", name, default_val))
        } else {
            Ok(name)
        }
    }

    /// Transpile an argument to JavaScript
    fn transpile_argument(&mut self, arg: &Argument) -> Result<String, TranspileError> {
        self.transpile_expression(&arg.value)
    }

    /// Transpile a pattern expression to a JavaScript regex string
    fn transpile_pattern_expression(
        &self,
        pattern: &PatternExpression,
    ) -> Result<String, TranspileError> {
        let regex = self.pattern_expr_to_regex(pattern)?;
        Ok(format!("/{}/", regex))
    }

    /// Convert a WFL pattern expression to a regex string
    #[allow(clippy::only_used_in_recursion)]
    fn pattern_expr_to_regex(&self, pattern: &PatternExpression) -> Result<String, TranspileError> {
        match pattern {
            PatternExpression::Literal(s) => Ok(regex_escape(s)),
            PatternExpression::CharacterClass(class) => Ok(match class {
                CharClass::Digit => r"\d".to_string(),
                CharClass::Letter => r"[a-zA-Z]".to_string(),
                CharClass::Whitespace => r"\s".to_string(),
                CharClass::Any => ".".to_string(),
                CharClass::UnicodeCategory(cat) => format!(r"\p{{{}}}", cat),
                CharClass::UnicodeScript(script) => format!(r"\p{{Script={}}}", script),
                CharClass::UnicodeProperty(prop) => format!(r"\p{{{}}}", prop),
            }),
            PatternExpression::Quantified {
                pattern,
                quantifier,
            } => {
                let inner = self.pattern_expr_to_regex(pattern)?;
                let quant = match quantifier {
                    Quantifier::Optional => "?",
                    Quantifier::ZeroOrMore => "*",
                    Quantifier::OneOrMore => "+",
                    Quantifier::Exactly(n) => return Ok(format!("(?:{}){{{}}}", inner, n)),
                    Quantifier::Between(min, max) => {
                        return Ok(format!("(?:{}){{{},{}}}", inner, min, max));
                    }
                    Quantifier::AtLeast(n) => return Ok(format!("(?:{}){{{},}}", inner, n)),
                    Quantifier::AtMost(n) => return Ok(format!("(?:{}){{0,{}}}", inner, n)),
                };
                Ok(format!("(?:{}){}", inner, quant))
            }
            PatternExpression::Sequence(patterns) => {
                let parts: Result<Vec<_>, _> = patterns
                    .iter()
                    .map(|p| self.pattern_expr_to_regex(p))
                    .collect();
                Ok(parts?.join(""))
            }
            PatternExpression::Alternative(patterns) => {
                let parts: Result<Vec<_>, _> = patterns
                    .iter()
                    .map(|p| self.pattern_expr_to_regex(p))
                    .collect();
                Ok(format!("(?:{})", parts?.join("|")))
            }
            PatternExpression::Capture { name, pattern } => {
                let inner = self.pattern_expr_to_regex(pattern)?;
                Ok(format!("(?<{}>{})", name, inner))
            }
            PatternExpression::Backreference(name) => Ok(format!(r"\k<{}>", name)),
            PatternExpression::Anchor(anchor) => Ok(match anchor {
                Anchor::StartOfText => "^".to_string(),
                Anchor::EndOfText => "$".to_string(),
            }),
            PatternExpression::Lookahead(pattern) => {
                let inner = self.pattern_expr_to_regex(pattern)?;
                Ok(format!("(?={})", inner))
            }
            PatternExpression::NegativeLookahead(pattern) => {
                let inner = self.pattern_expr_to_regex(pattern)?;
                Ok(format!("(?!{})", inner))
            }
            PatternExpression::Lookbehind(pattern) => {
                let inner = self.pattern_expr_to_regex(pattern)?;
                Ok(format!("(?<={})", inner))
            }
            PatternExpression::NegativeLookbehind(pattern) => {
                let inner = self.pattern_expr_to_regex(pattern)?;
                Ok(format!("(?<!{})", inner))
            }
            PatternExpression::ListReference(name) => {
                // This would need runtime support to work properly
                Ok(format!("(?:${{{}}})", name))
            }
        }
    }

    /// Convert a simple pattern string to a regex
    fn pattern_to_regex(&self, pattern: &str) -> String {
        // For simple patterns, just escape regex special characters
        format!("\"{}\"", regex_escape(pattern))
    }

    /// Check if a list of statements contains any async operations
    fn contains_async(&self, stmts: &[Statement]) -> bool {
        for stmt in stmts {
            if self.stmt_is_async(stmt) {
                return true;
            }
        }
        false
    }

    /// Check if a statement is or contains async operations
    fn stmt_is_async(&self, stmt: &Statement) -> bool {
        match stmt {
            Statement::WaitForStatement { .. }
            | Statement::WaitForDurationStatement { .. }
            | Statement::HttpGetStatement { .. }
            | Statement::HttpPostStatement { .. }
            | Statement::WaitForProcessStatement { .. }
            | Statement::WaitForRequestStatement { .. } => true,

            Statement::IfStatement {
                then_block,
                else_block,
                ..
            } => {
                self.contains_async(then_block)
                    || else_block
                        .as_ref()
                        .map(|b| self.contains_async(b))
                        .unwrap_or(false)
            }

            Statement::ForEachLoop { body, .. }
            | Statement::CountLoop { body, .. }
            | Statement::WhileLoop { body, .. }
            | Statement::RepeatWhileLoop { body, .. }
            | Statement::RepeatUntilLoop { body, .. }
            | Statement::ForeverLoop { body, .. }
            | Statement::MainLoop { body, .. } => self.contains_async(body),

            Statement::TryStatement {
                body,
                when_clauses,
                otherwise_block,
                ..
            } => {
                self.contains_async(body)
                    || when_clauses.iter().any(|c| self.contains_async(&c.body))
                    || otherwise_block
                        .as_ref()
                        .map(|b| self.contains_async(b))
                        .unwrap_or(false)
            }

            Statement::ExpressionStatement { expression, .. } => self.expr_is_async(expression),

            _ => false,
        }
    }

    /// Check if an expression contains async operations
    fn expr_is_async(&self, expr: &Expression) -> bool {
        matches!(expr, Expression::AwaitExpression { .. })
    }

    /// Sanitize a WFL identifier to be a valid JavaScript identifier
    fn sanitize_name(&self, name: &str) -> String {
        // Handle reserved JavaScript keywords
        let reserved = [
            "break",
            "case",
            "catch",
            "continue",
            "debugger",
            "default",
            "delete",
            "do",
            "else",
            "finally",
            "for",
            "function",
            "if",
            "in",
            "instanceof",
            "new",
            "return",
            "switch",
            "this",
            "throw",
            "try",
            "typeof",
            "var",
            "void",
            "while",
            "with",
            "class",
            "const",
            "enum",
            "export",
            "extends",
            "import",
            "super",
            "implements",
            "interface",
            "let",
            "package",
            "private",
            "protected",
            "public",
            "static",
            "yield",
            "await",
            "async",
        ];

        let mut result = name.to_string();

        // Replace spaces and dashes with underscores
        result = result.replace([' ', '-'], "_");

        // If starts with a digit, prefix with underscore
        if result
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
        {
            result = format!("_{}", result);
        }

        // If it's a reserved word, prefix with underscore
        if reserved.contains(&result.as_str()) {
            result = format!("_{}", result);
        }

        result
    }

    /// Escape a string for JavaScript
    fn escape_string(&self, s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    /// Escape a key for JavaScript object literal
    fn escape_key(&self, key: &str) -> String {
        // Check if key is a valid identifier
        let is_valid_identifier = key
            .chars()
            .next()
            .map(|c| c.is_alphabetic() || c == '_')
            .unwrap_or(false)
            && key.chars().all(|c| c.is_alphanumeric() || c == '_');

        if is_valid_identifier {
            key.to_string()
        } else {
            format!("\"{}\"", self.escape_string(key))
        }
    }
}

impl Clone for JavaScriptTranspiler {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            indent_level: self.indent_level,
            warnings: Vec::new(), // Don't clone warnings
            in_async: self.in_async,
        }
    }
}

/// Escape special regex characters in a string
fn regex_escape(s: &str) -> String {
    let special_chars = [
        '.', '*', '+', '?', '^', '$', '{', '}', '[', ']', '(', ')', '|', '\\',
    ];
    let mut result = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        if special_chars.contains(&c) {
            result.push('\\');
        }
        result.push(c);
    }
    result
}
