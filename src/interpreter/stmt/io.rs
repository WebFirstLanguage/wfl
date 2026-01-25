use std::cell::RefCell;
use std::rc::Rc;

use crate::interpreter::Interpreter;
use crate::interpreter::control_flow::ControlFlow;
use crate::interpreter::environment::Environment;
use crate::interpreter::error::{ErrorKind, RuntimeError};
use crate::interpreter::value::Value;
use crate::parser::ast::{Expression, FileOpenMode, Literal, WriteMode};

pub trait IoExecutor {
    async fn execute_open_file(
        &self,
        path: &Expression,
        variable_name: &str,
        mode: FileOpenMode,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_read_file(
        &self,
        path: &Expression,
        variable_name: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_write_file(
        &self,
        file: &Expression,
        content: &Expression,
        mode: WriteMode,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_close_file(
        &self,
        file: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_create_directory(
        &self,
        path: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_create_file(
        &self,
        path: &Expression,
        content: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_delete_file(
        &self,
        path: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_write_to(
        &self,
        content: &Expression,
        file: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_write_content(
        &self,
        content: &Expression,
        target: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_delete_directory(
        &self,
        path: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;
}

impl IoExecutor for Interpreter {
    async fn execute_open_file(
        &self,
        path: &Expression,
        variable_name: &str,
        mode: FileOpenMode,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
        let path_str = match &path_value {
            Value::Text(s) => s.clone(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Expected string for file path, got {path_value:?}"),
                    line,
                    column,
                ));
            }
        };

        // Use the appropriate file open mode
        match self
            .io_client
            .open_file_with_mode(&path_str, mode.clone())
            .await
        {
            Ok(handle) => {
                match env
                    .borrow_mut()
                    .define(variable_name, Value::Text(handle.into()))
                {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(msg) => Err(RuntimeError::new(msg, line, column)),
                }
            }
            Err(e) => Err(e),
        }
    }

    async fn execute_read_file(
        &self,
        path: &Expression,
        variable_name: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
        let path_str = match &path_value {
            Value::Text(s) => s.clone(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Expected string for file path or handle, got {path_value:?}"),
                    line,
                    column,
                ));
            }
        };

        let is_file_path = matches!(path, Expression::Literal(Literal::String(_), _, _));

        if is_file_path {
            match self.io_client.open_file(&path_str).await {
                Ok(handle) => match self.io_client.read_file(&handle).await {
                    Ok(content) => {
                        match env
                            .borrow_mut()
                            .define(variable_name, Value::Text(content.into()))
                        {
                            Ok(_) => {
                                let _ = self.io_client.close_file(&handle).await;
                                Ok((Value::Null, ControlFlow::None))
                            }
                            Err(msg) => {
                                let _ = self.io_client.close_file(&handle).await;
                                Err(RuntimeError::new(msg, line, column))
                            }
                        }
                    }
                    Err(e) => {
                        let _ = self.io_client.close_file(&handle).await;
                        Err(RuntimeError::new(e, line, column))
                    }
                },
                Err(e) => Err(RuntimeError::new(e, line, column)),
            }
        } else {
            match self.io_client.read_file(&path_str).await {
                Ok(content) => {
                    match env
                        .borrow_mut()
                        .define(variable_name, Value::Text(content.into()))
                    {
                        Ok(_) => Ok((Value::Null, ControlFlow::None)),
                        Err(msg) => Err(RuntimeError::new(msg, line, column)),
                    }
                }
                Err(e) => Err(RuntimeError::new(e, line, column)),
            }
        }
    }

    async fn execute_write_file(
        &self,
        file: &Expression,
        content: &Expression,
        mode: WriteMode,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let file_value = self.evaluate_expression(file, Rc::clone(&env)).await?;
        let content_value = self.evaluate_expression(content, Rc::clone(&env)).await?;

        let file_str = match &file_value {
            Value::Text(s) => s.clone(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Expected string for file handle, got {file_value:?}"),
                    line,
                    column,
                ));
            }
        };

        let content_str = match &content_value {
            Value::Text(s) => s.clone(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Expected string for file content, got {content_value:?}"),
                    line,
                    column,
                ));
            }
        };

        match mode {
            crate::parser::ast::WriteMode::Append => {
                match self.io_client.append_file(&file_str, &content_str).await {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(e) => Err(RuntimeError::new(e, line, column)),
                }
            }
            crate::parser::ast::WriteMode::Overwrite => {
                match self.io_client.write_file(&file_str, &content_str).await {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(e) => Err(RuntimeError::new(e, line, column)),
                }
            }
        }
    }

    async fn execute_close_file(
        &self,
        file: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let file_value = self.evaluate_expression(file, Rc::clone(&env)).await?;

        let file_str = match &file_value {
            Value::Text(s) => s.clone(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Expected string for file handle, got {file_value:?}"),
                    line,
                    column,
                ));
            }
        };

        match self.io_client.close_file(&file_str).await {
            Ok(_) => Ok((Value::Null, ControlFlow::None)),
            Err(e) => Err(RuntimeError::new(e, line, column)),
        }
    }

    async fn execute_create_directory(
        &self,
        path: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
        let path_str = match &path_value {
            Value::Text(s) => s.clone(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Expected string for directory path, got {path_value:?}"),
                    line,
                    column,
                ));
            }
        };

        match self.io_client.create_directory(&path_str).await {
            Ok(_) => Ok((Value::Null, ControlFlow::None)),
            Err(e) => Err(RuntimeError::new(e, line, column)),
        }
    }

    async fn execute_create_file(
        &self,
        path: &Expression,
        content: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
        let content_value = self.evaluate_expression(content, Rc::clone(&env)).await?;

        let path_str = match &path_value {
            Value::Text(s) => s.clone(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Expected string for file path, got {path_value:?}"),
                    line,
                    column,
                ));
            }
        };

        let content_str = format!("{content_value}");

        match self.io_client.create_file(&path_str, &content_str).await {
            Ok(_) => Ok((Value::Null, ControlFlow::None)),
            Err(e) => Err(RuntimeError::new(e, line, column)),
        }
    }

    async fn execute_delete_file(
        &self,
        path: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
        let path_str = match &path_value {
            Value::Text(s) => s.clone(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Expected string for file path, got {path_value:?}"),
                    line,
                    column,
                ));
            }
        };

        match self.io_client.delete_file(&path_str).await {
            Ok(_) => Ok((Value::Null, ControlFlow::None)),
            Err(e) => Err(RuntimeError::new(e, line, column)),
        }
    }

    async fn execute_write_to(
        &self,
        content: &Expression,
        file: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let content_value = self.evaluate_expression(content, Rc::clone(&env)).await?;
        let file_value = self.evaluate_expression(file, Rc::clone(&env)).await?;

        let file_str = match &file_value {
            Value::Text(s) => s.clone(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Expected string for file handle, got {file_value:?}"),
                    line,
                    column,
                ));
            }
        };

        let content_str = format!("{content_value}");

        match self.io_client.write_file(&file_str, &content_str).await {
            Ok(_) => Ok((Value::Null, ControlFlow::None)),
            Err(e) => Err(RuntimeError::new(e, line, column)),
        }
    }

    async fn execute_write_content(
        &self,
        content: &Expression,
        target: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let content_value = self.evaluate_expression(content, Rc::clone(&env)).await?;
        let target_value = self.evaluate_expression(target, Rc::clone(&env)).await?;

        let target_str = match &target_value {
            Value::Text(s) => s.clone(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Expected string for file handle, got {target_value:?}"),
                    line,
                    column,
                ));
            }
        };

        let content_str = format!("{content_value}");

        // Check if target is a file handle (starts with "file") or a file path
        if target_str.starts_with("file") {
            // This is a file handle, use append_file to respect the file's open mode
            match self.io_client.append_file(&target_str, &content_str).await {
                Ok(_) => Ok((Value::Null, ControlFlow::None)),
                Err(e) => Err(RuntimeError::new(e, line, column)),
            }
        } else {
            // This is a file path, use write_file (overwrite mode)
            match self.io_client.write_file(&target_str, &content_str).await {
                Ok(_) => Ok((Value::Null, ControlFlow::None)),
                Err(e) => Err(RuntimeError::new(e, line, column)),
            }
        }
    }

    async fn execute_delete_directory(
        &self,
        path: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
        let path_str = match &path_value {
            Value::Text(s) => s.clone(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Expected string for directory path, got {path_value:?}"),
                    line,
                    column,
                ));
            }
        };

        match self.io_client.delete_directory(&path_str).await {
            Ok(_) => Ok((Value::Null, ControlFlow::None)),
            Err(e) => Err(RuntimeError::new(e, line, column)),
        }
    }
}
