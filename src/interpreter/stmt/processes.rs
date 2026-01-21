use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::interpreter::Interpreter;
use crate::interpreter::control_flow::ControlFlow;
use crate::interpreter::environment::Environment;
use crate::interpreter::error::{ErrorKind, RuntimeError};
use crate::interpreter::value::Value;
use crate::parser::ast::Expression;

pub trait ProcessExecutor {
    async fn execute_command(
        &self,
        command: &Expression,
        arguments: Option<&Expression>,
        variable_name: Option<&String>,
        use_shell: bool,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_spawn_process(
        &self,
        command: &Expression,
        arguments: Option<&Expression>,
        variable_name: &str,
        use_shell: bool,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_read_process_output(
        &self,
        process_id: &Expression,
        variable_name: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_kill_process(
        &self,
        process_id: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_wait_for_process(
        &self,
        process_id: &Expression,
        variable_name: Option<&String>,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;
}

impl ProcessExecutor for Interpreter {
    async fn execute_command(
        &self,
        command: &Expression,
        arguments: Option<&Expression>,
        variable_name: Option<&String>,
        use_shell: bool,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // Evaluate command expression
        let cmd_val = self.evaluate_expression(command, Rc::clone(&env)).await?;
        let cmd_str = match &cmd_val {
            Value::Text(text) => text.as_ref(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Command must be text, got {}", cmd_val.type_name()),
                    line,
                    column,
                ));
            }
        };

        // Evaluate arguments if provided
        let args_vec: Vec<String> = if let Some(args_expr) = arguments {
            let args_val = self.evaluate_expression(args_expr, Rc::clone(&env)).await?;
            match &args_val {
                Value::List(list) => {
                    let list_ref = list.borrow();
                    list_ref
                        .iter()
                        .map(|v| match v {
                            Value::Text(t) => Ok(t.as_ref().to_string()),
                            _ => Ok(v.to_string()),
                        })
                        .collect::<Result<Vec<_>, RuntimeError>>()?
                }
                Value::Text(text) => vec![text.as_ref().to_string()],
                _ => {
                    return Err(RuntimeError::new(
                        format!(
                            "Arguments must be a list or text, got {}",
                            args_val.type_name()
                        ),
                        line,
                        column,
                    ));
                }
            }
        } else {
            Vec::new()
        };

        // Execute command
        let args_refs: Vec<&str> = args_vec.iter().map(|s| s.as_str()).collect();
        let (stdout, stderr, exit_code) = self
            .io_client
            .execute_command(cmd_str, &args_refs, use_shell, line, column)
            .await
            .map_err(|e| {
                // Determine error kind based on error message
                let kind = if e.contains("program not found")
                    || e.contains("cannot find")
                    || e.contains("not recognized")
                {
                    ErrorKind::CommandNotFound
                } else if e.contains("spawn") {
                    ErrorKind::ProcessSpawnFailed
                } else {
                    ErrorKind::General
                };
                RuntimeError::with_kind(e, line, column, kind)
            })?;

        // Build result object
        let mut result_map = HashMap::new();
        result_map.insert("output".to_string(), Value::Text(Rc::from(stdout.as_str())));
        result_map.insert("error".to_string(), Value::Text(Rc::from(stderr.as_str())));
        result_map.insert("exit_code".to_string(), Value::Number(exit_code as f64));
        result_map.insert("success".to_string(), Value::Bool(exit_code == 0));

        let result_obj = Value::Object(Rc::new(RefCell::new(result_map)));

        // Store result if variable name provided
        if let Some(var_name) = variable_name {
            env.borrow_mut()
                .define(var_name, result_obj)
                .map_err(|e| RuntimeError::new(e, line, column))?;
        }

        Ok((Value::Null, ControlFlow::None))
    }

    async fn execute_spawn_process(
        &self,
        command: &Expression,
        arguments: Option<&Expression>,
        variable_name: &str,
        use_shell: bool,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // Evaluate command expression
        let cmd_val = self.evaluate_expression(command, Rc::clone(&env)).await?;
        let cmd_str = match &cmd_val {
            Value::Text(text) => text.as_ref(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Command must be text, got {}", cmd_val.type_name()),
                    line,
                    column,
                ));
            }
        };

        // Evaluate arguments if provided
        let args_vec: Vec<String> = if let Some(args_expr) = arguments {
            let args_val = self.evaluate_expression(args_expr, Rc::clone(&env)).await?;
            match &args_val {
                Value::List(list) => {
                    let list_ref = list.borrow();
                    list_ref
                        .iter()
                        .map(|v| match v {
                            Value::Text(t) => Ok(t.as_ref().to_string()),
                            _ => Ok(v.to_string()),
                        })
                        .collect::<Result<Vec<_>, RuntimeError>>()?
                }
                Value::Text(text) => vec![text.as_ref().to_string()],
                _ => {
                    return Err(RuntimeError::new(
                        format!(
                            "Arguments must be a list or text, got {}",
                            args_val.type_name()
                        ),
                        line,
                        column,
                    ));
                }
            }
        } else {
            Vec::new()
        };

        // Spawn process
        let args_refs: Vec<&str> = args_vec.iter().map(|s| s.as_str()).collect();
        let process_id = self
            .io_client
            .spawn_process(cmd_str, &args_refs, use_shell, line, column)
            .await
            .map_err(|e| {
                let kind = if e.contains("program not found")
                    || e.contains("cannot find")
                    || e.contains("not recognized")
                {
                    ErrorKind::CommandNotFound
                } else {
                    ErrorKind::ProcessSpawnFailed
                };
                RuntimeError::with_kind(e, line, column, kind)
            })?;

        // Store process ID in variable
        env.borrow_mut()
            .define(variable_name, Value::Text(Rc::from(process_id.as_str())))
            .map_err(|e| RuntimeError::new(e, line, column))?;

        Ok((Value::Null, ControlFlow::None))
    }

    async fn execute_read_process_output(
        &self,
        process_id: &Expression,
        variable_name: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // Evaluate process ID expression
        let proc_val = self
            .evaluate_expression(process_id, Rc::clone(&env))
            .await?;
        let proc_id = match &proc_val {
            Value::Text(text) => text.as_ref(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Process ID must be text, got {}", proc_val.type_name()),
                    line,
                    column,
                ));
            }
        };

        // Read process output
        let output = self
            .io_client
            .read_process_output(proc_id)
            .await
            .map_err(|e| {
                let kind = if e.contains("Invalid process ID") {
                    ErrorKind::ProcessNotFound
                } else {
                    ErrorKind::General
                };
                RuntimeError::with_kind(e, line, column, kind)
            })?;

        // Store output in variable
        env.borrow_mut()
            .define(variable_name, Value::Text(Rc::from(output.as_str())))
            .map_err(|e| RuntimeError::new(e, line, column))?;

        Ok((Value::Null, ControlFlow::None))
    }

    async fn execute_kill_process(
        &self,
        process_id: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // Evaluate process ID expression
        let proc_val = self
            .evaluate_expression(process_id, Rc::clone(&env))
            .await?;
        let proc_id = match &proc_val {
            Value::Text(text) => text.as_ref(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Process ID must be text, got {}", proc_val.type_name()),
                    line,
                    column,
                ));
            }
        };

        // Kill process
        self.io_client.kill_process(proc_id).await.map_err(|e| {
            let kind = if e.contains("Invalid process ID") {
                ErrorKind::ProcessNotFound
            } else {
                ErrorKind::ProcessKillFailed
            };
            RuntimeError::with_kind(e, line, column, kind)
        })?;

        Ok((Value::Null, ControlFlow::None))
    }

    async fn execute_wait_for_process(
        &self,
        process_id: &Expression,
        variable_name: Option<&String>,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // Evaluate process ID expression
        let proc_val = self
            .evaluate_expression(process_id, Rc::clone(&env))
            .await?;
        let proc_id = match &proc_val {
            Value::Text(text) => text.as_ref(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Process ID must be text, got {}", proc_val.type_name()),
                    line,
                    column,
                ));
            }
        };

        // Wait for process to complete
        let exit_code = self
            .io_client
            .wait_for_process(proc_id)
            .await
            .map_err(|e| {
                let kind = if e.contains("Invalid process ID") {
                    ErrorKind::ProcessNotFound
                } else {
                    ErrorKind::General
                };
                RuntimeError::with_kind(e, line, column, kind)
            })?;

        // Store exit code in variable if provided
        if let Some(var_name) = variable_name {
            env.borrow_mut()
                .define(var_name, Value::Number(exit_code as f64))
                .map_err(|e| RuntimeError::new(e, line, column))?;
        }

        Ok((Value::Null, ControlFlow::None))
    }
}
