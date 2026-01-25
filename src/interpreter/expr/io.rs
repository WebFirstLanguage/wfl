use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::interpreter::Interpreter;
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use crate::parser::ast::{Expression, Literal};

#[allow(async_fn_in_trait)]
pub trait IoExpressionEvaluator {
    async fn evaluate_file_exists(
        &self,
        path: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;

    async fn evaluate_directory_exists(
        &self,
        path: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;

    async fn evaluate_list_files(
        &self,
        path: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;

    async fn evaluate_read_content(
        &self,
        file_handle: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;

    async fn evaluate_list_files_recursive(
        &self,
        path: &Expression,
        extensions: Option<&Vec<Expression>>,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;

    async fn evaluate_list_files_filtered(
        &self,
        path: &Expression,
        extensions: &[Expression],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;

    async fn evaluate_process_running(
        &self,
        process_id: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;
}

impl IoExpressionEvaluator for Interpreter {
    async fn evaluate_file_exists(
        &self,
        path: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        let path_val = self.evaluate_expression(path, Rc::clone(&env)).await?;
        let path_str = match &path_val {
            Value::Text(s) => s.as_ref(),
            _ => {
                return Err(RuntimeError::new(
                    "Path must be a string".to_string(),
                    line,
                    column,
                ));
            }
        };

        let exists = tokio::fs::try_exists(path_str).await.unwrap_or(false);
        if exists {
            let is_file = tokio::fs::metadata(path_str)
                .await
                .map(|m| m.is_file())
                .unwrap_or(false);
            Ok(Value::Bool(is_file))
        } else {
            Ok(Value::Bool(false))
        }
    }

    async fn evaluate_directory_exists(
        &self,
        path: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        let path_val = self.evaluate_expression(path, Rc::clone(&env)).await?;
        let path_str = match &path_val {
            Value::Text(s) => s.as_ref(),
            _ => {
                return Err(RuntimeError::new(
                    "Path must be a string".to_string(),
                    line,
                    column,
                ));
            }
        };

        let exists = tokio::fs::try_exists(path_str).await.unwrap_or(false);
        if exists {
            let is_dir = tokio::fs::metadata(path_str)
                .await
                .map(|m| m.is_dir())
                .unwrap_or(false);
            Ok(Value::Bool(is_dir))
        } else {
            Ok(Value::Bool(false))
        }
    }

    async fn evaluate_list_files(
        &self,
        path: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        let path_val = self.evaluate_expression(path, Rc::clone(&env)).await?;
        let path_str = match &path_val {
            Value::Text(s) => s.as_ref(),
            _ => {
                return Err(RuntimeError::new(
                    "Path must be a string".to_string(),
                    line,
                    column,
                ));
            }
        };

        match tokio::fs::read_dir(path_str).await {
            Ok(mut entries) => {
                let mut files = Vec::new();
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if let Ok(file_name) = entry.file_name().into_string() {
                        files.push(Value::Text(Rc::from(file_name)));
                    }
                }
                files.sort_by(|a, b| {
                    if let (Value::Text(a_str), Value::Text(b_str)) = (a, b) {
                        a_str.cmp(b_str)
                    } else {
                        std::cmp::Ordering::Equal
                    }
                });
                Ok(Value::List(Rc::new(RefCell::new(files))))
            }
            Err(e) => Err(RuntimeError::new(
                format!("Failed to list files: {}", e),
                line,
                column,
            )),
        }
    }

    async fn evaluate_read_content(
        &self,
        file_handle: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        let handle_val = self
            .evaluate_expression(file_handle, Rc::clone(&env))
            .await?;
        let handle_str = match &handle_val {
            Value::Text(s) => s.as_ref(),
            _ => {
                return Err(RuntimeError::new(
                    "File handle must be a string".to_string(),
                    line,
                    column,
                ));
            }
        };

        // Determine if this is a file handle ID or a file path
        let is_file_path = matches!(file_handle, Expression::Literal(Literal::String(_), _, _));

        let content = if is_file_path {
            // Treat as file path
            match self.io_client.open_file(handle_str).await {
                Ok(handle) => match self.io_client.read_file(&handle).await {
                    Ok(content) => {
                        let _ = self.io_client.close_file(&handle).await;
                        content
                    }
                    Err(e) => {
                        let _ = self.io_client.close_file(&handle).await;
                        return Err(RuntimeError::new(e, line, column));
                    }
                },
                Err(e) => return Err(RuntimeError::new(e, line, column)),
            }
        } else {
            // Treat as handle ID
            match self.io_client.read_file(handle_str).await {
                Ok(content) => content,
                Err(e) => return Err(RuntimeError::new(e, line, column)),
            }
        };

        Ok(Value::Text(Rc::from(content)))
    }

    async fn evaluate_list_files_recursive(
        &self,
        path: &Expression,
        extensions: Option<&Vec<Expression>>,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        let path_val = self.evaluate_expression(path, Rc::clone(&env)).await?;
        let path_str = match &path_val {
            Value::Text(s) => s.as_ref().to_string(),
            _ => {
                return Err(RuntimeError::new(
                    "Path must be a string".to_string(),
                    line,
                    column,
                ));
            }
        };

        let mut ext_strings = Vec::new();
        if let Some(exts) = extensions {
            for ext_expr in exts {
                let ext_val = self.evaluate_expression(ext_expr, Rc::clone(&env)).await?;
                if let Value::Text(s) = ext_val {
                    ext_strings.push(s.as_ref().to_string());
                }
            }
        }

        let files = self
            .list_files_recursive_helper(
                &path_str,
                if ext_strings.is_empty() {
                    None
                } else {
                    Some(&ext_strings)
                },
            )
            .await
            .map_err(|e| RuntimeError::new(e, line, column))?;

        let value_files: Vec<Value> = files
            .into_iter()
            .map(|s| Value::Text(Rc::from(s)))
            .collect();

        Ok(Value::List(Rc::new(RefCell::new(value_files))))
    }

    async fn evaluate_list_files_filtered(
        &self,
        path: &Expression,
        extensions: &[Expression],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        let path_val = self.evaluate_expression(path, Rc::clone(&env)).await?;
        let path_str = match &path_val {
            Value::Text(s) => s.as_ref().to_string(),
            _ => {
                return Err(RuntimeError::new(
                    "Path must be a string".to_string(),
                    line,
                    column,
                ));
            }
        };

        let mut ext_strings = Vec::new();
        for ext_expr in extensions {
            let ext_val = self.evaluate_expression(ext_expr, Rc::clone(&env)).await?;
            if let Value::Text(s) = ext_val {
                ext_strings.push(s.as_ref().to_string());
            }
        }

        // Use read_dir and filter manually since it's shallow
        match tokio::fs::read_dir(&path_str).await {
            Ok(mut entries) => {
                let mut files = Vec::new();
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if let Ok(file_name) = entry.file_name().into_string() {
                        let path = std::path::Path::new(&file_name);
                        if let Some(ext) = path.extension()
                            && let Some(ext_str) = ext.to_str()
                            && ext_strings.iter().any(|e| e == ext_str)
                        {
                            files.push(Value::Text(Rc::from(file_name)));
                        }
                    }
                }
                files.sort_by(|a, b| {
                    if let (Value::Text(a_str), Value::Text(b_str)) = (a, b) {
                        a_str.cmp(b_str)
                    } else {
                        std::cmp::Ordering::Equal
                    }
                });
                Ok(Value::List(Rc::new(RefCell::new(files))))
            }
            Err(e) => Err(RuntimeError::new(
                format!("Failed to list files: {}", e),
                line,
                column,
            )),
        }
    }

    async fn evaluate_process_running(
        &self,
        process_id: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        let pid_val = self
            .evaluate_expression(process_id, Rc::clone(&env))
            .await?;
        let pid_str = match &pid_val {
            Value::Text(s) => s.as_ref(),
            _ => {
                return Err(RuntimeError::new(
                    "Process ID must be a string".to_string(),
                    line,
                    column,
                ));
            }
        };

        Ok(Value::Bool(
            self.io_client.is_process_running(pid_str).await,
        ))
    }
}

// Helper methods for Interpreter
impl Interpreter {
    async fn list_files_recursive_helper(
        &self,
        root_path: &str,
        extensions: Option<&Vec<String>>,
    ) -> Result<Vec<String>, String> {
        let mut files = Vec::new();
        let mut dirs = vec![PathBuf::from(root_path)];

        while let Some(current_dir) = dirs.pop() {
            match tokio::fs::read_dir(&current_dir).await {
                Ok(mut entries) => {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let entry_path = entry.path();
                        if entry_path.is_dir() {
                            dirs.push(entry_path);
                        } else if let Some(exts) = extensions {
                            if let Some(ext) = entry_path.extension()
                                && let Some(ext_str) = ext.to_str()
                                && exts.iter().any(|e| e == ext_str)
                            {
                                if let Ok(rel_path) =
                                    entry_path.strip_prefix(std::path::Path::new(root_path))
                                {
                                    files.push(rel_path.to_string_lossy().into_owned());
                                } else {
                                    files.push(entry_path.to_string_lossy().into_owned());
                                }
                            }
                        } else if let Ok(rel_path) =
                            entry_path.strip_prefix(std::path::Path::new(root_path))
                        {
                            files.push(rel_path.to_string_lossy().into_owned());
                        } else {
                            files.push(entry_path.to_string_lossy().into_owned());
                        }
                    }
                }
                Err(e) => return Err(format!("Failed to read directory: {}", e)),
            }
        }

        Ok(files)
    }
}
