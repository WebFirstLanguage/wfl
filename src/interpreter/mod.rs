#![allow(clippy::await_holding_refcell_ref)]
pub mod control_flow;
pub mod environment;
pub mod error;
#[cfg(test)]
mod memory_tests;
#[cfg(test)]
mod tests;
pub mod value;

use self::control_flow::ControlFlow;

use self::environment::Environment;
use self::error::{ErrorKind, RuntimeError};
use self::value::{
    ContainerDefinitionValue, ContainerEventValue, ContainerInstanceValue, ContainerMethodValue,
    EventHandler, FunctionValue, InterfaceDefinitionValue, Value,
};
use crate::builtins::get_function_arity;
use crate::debug_report::CallFrame;
#[cfg(debug_assertions)]
use crate::exec_block_enter;
#[cfg(debug_assertions)]
use crate::exec_block_exit;
#[cfg(debug_assertions)]
use crate::exec_control_flow;
#[cfg(debug_assertions)]
use crate::exec_function_call;
#[cfg(debug_assertions)]
use crate::exec_function_return;
use crate::exec_trace;
#[cfg(debug_assertions)]
use crate::exec_var_assign;
#[cfg(debug_assertions)]
use crate::exec_var_declare;
#[cfg(debug_assertions)]
use crate::logging::IndentGuard;
use crate::parser::ast::{
    Expression, FileOpenMode, Literal, Operator, Program, Statement, UnaryOperator,
};
use crate::pattern::CompiledPattern;
use crate::stdlib;
use std::cell::RefCell;
use std::io::{self, Write};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};
use warp::Filter;

// Helper functions for execution logging
#[cfg(debug_assertions)]
fn stmt_type(stmt: &Statement) -> String {
    match stmt {
        Statement::VariableDeclaration { name, .. } => format!("VariableDeclaration '{name}'"),
        Statement::Assignment { name, .. } => format!("Assignment to '{name}'"),
        Statement::IfStatement { .. } => "IfStatement".to_string(),
        Statement::SingleLineIf { .. } => "SingleLineIf".to_string(),
        Statement::DisplayStatement { .. } => "DisplayStatement".to_string(),
        Statement::ActionDefinition { name, .. } => format!("ActionDefinition '{name}'"),
        Statement::ReturnStatement { .. } => "ReturnStatement".to_string(),
        Statement::ExpressionStatement { .. } => "ExpressionStatement".to_string(),
        Statement::CountLoop { .. } => "CountLoop".to_string(),
        Statement::ForEachLoop { item_name, .. } => format!("ForEachLoop '{item_name}'"),
        Statement::WhileLoop { .. } => "WhileLoop".to_string(),
        Statement::RepeatUntilLoop { .. } => "RepeatUntilLoop".to_string(),
        Statement::RepeatWhileLoop { .. } => "RepeatWhileLoop".to_string(),
        Statement::ForeverLoop { .. } => "ForeverLoop".to_string(),
        Statement::MainLoop { .. } => "MainLoop".to_string(),
        Statement::BreakStatement { .. } => "BreakStatement".to_string(),
        Statement::ContinueStatement { .. } => "ContinueStatement".to_string(),
        Statement::ExitStatement { .. } => "ExitStatement".to_string(),
        Statement::OpenFileStatement { variable_name, .. } => {
            format!("OpenFileStatement '{variable_name}'")
        }
        Statement::ReadFileStatement { variable_name, .. } => {
            format!("ReadFileStatement '{variable_name}'")
        }
        Statement::WriteFileStatement { .. } => "WriteFileStatement".to_string(),
        Statement::WriteToStatement { .. } => "WriteToStatement".to_string(),
        Statement::CloseFileStatement { .. } => "CloseFileStatement".to_string(),
        Statement::CreateDirectoryStatement { .. } => "CreateDirectoryStatement".to_string(),
        Statement::CreateFileStatement { .. } => "CreateFileStatement".to_string(),
        Statement::DeleteFileStatement { .. } => "DeleteFileStatement".to_string(),
        Statement::DeleteDirectoryStatement { .. } => "DeleteDirectoryStatement".to_string(),
        Statement::WaitForStatement { .. } => "WaitForStatement".to_string(),
        Statement::TryStatement { .. } => "TryStatement".to_string(),
        Statement::HttpGetStatement { variable_name, .. } => {
            format!("HttpGetStatement '{variable_name}'")
        }
        Statement::HttpPostStatement { variable_name, .. } => {
            format!("HttpPostStatement '{variable_name}'")
        }
        Statement::PushStatement { .. } => "PushStatement to list".to_string(),
        Statement::CreateListStatement { name, .. } => format!("CreateListStatement '{name}'"),
        Statement::MapCreation { name, .. } => format!("MapCreation '{name}'"),
        Statement::CreateDateStatement { name, .. } => format!("CreateDateStatement '{name}'"),
        Statement::CreateTimeStatement { name, .. } => format!("CreateTimeStatement '{name}'"),
        Statement::AddToListStatement { list_name, .. } => {
            format!("AddToListStatement to '{list_name}'")
        }
        Statement::RemoveFromListStatement { list_name, .. } => {
            format!("RemoveFromListStatement from '{list_name}'")
        }
        Statement::ClearListStatement { list_name, .. } => {
            format!("ClearListStatement '{list_name}'")
        }
        // Container-related statements
        Statement::ContainerDefinition { name, .. } => format!("ContainerDefinition '{name}'"),
        Statement::ContainerInstantiation {
            container_type,
            instance_name,
            ..
        } => format!("ContainerInstantiation '{container_type}' as '{instance_name}'"),
        Statement::InterfaceDefinition { name, .. } => format!("InterfaceDefinition '{name}'"),
        Statement::EventDefinition { name, .. } => format!("EventDefinition '{name}'"),
        Statement::EventTrigger { name, .. } => format!("EventTrigger '{name}'"),
        Statement::EventHandler { event_name, .. } => format!("EventHandler '{event_name}'"),
        Statement::ParentMethodCall { method_name, .. } => {
            format!("ParentMethodCall '{method_name}'")
        }
        Statement::PatternDefinition { name, .. } => {
            format!("PatternDefinition '{name}'")
        }
        Statement::ListenStatement { server_name, .. } => {
            format!("ListenStatement '{server_name}'")
        }
        Statement::WaitForRequestStatement { request_name, .. } => {
            format!("WaitForRequestStatement '{request_name}'")
        }
        Statement::RespondStatement { .. } => "RespondStatement".to_string(),
    }
}

#[cfg(debug_assertions)]
fn expr_type(expr: &Expression) -> String {
    match expr {
        Expression::Literal(lit, ..) => match lit {
            Literal::String(s) => format!("StringLiteral \"{s}\""),
            Literal::Integer(i) => format!("IntegerLiteral {i}"),
            Literal::Float(f) => format!("FloatLiteral {f}"),
            Literal::Boolean(b) => format!("BooleanLiteral {b}"),
            Literal::Nothing => "NullLiteral".to_string(),
            Literal::Pattern(p) => format!("PatternLiteral \"{p}\""),
            Literal::List(_) => "ListLiteral".to_string(),
        },
        Expression::Variable(name, ..) => format!("Variable '{name}'"),
        Expression::BinaryOperation { operator, .. } => format!("BinaryOperation '{operator:?}'"),
        Expression::UnaryOperation { operator, .. } => format!("UnaryOperation '{operator:?}'"),
        Expression::FunctionCall { function, .. } => match function.as_ref() {
            Expression::Variable(name, ..) => format!("FunctionCall '{name}'"),
            _ => "FunctionCall".to_string(),
        },
        Expression::ActionCall { name, .. } => format!("ActionCall '{name}'"),
        Expression::MemberAccess { property, .. } => format!("MemberAccess '{property}'"),
        Expression::IndexAccess { .. } => "IndexAccess".to_string(),
        Expression::Concatenation { .. } => "Concatenation".to_string(),
        Expression::PatternMatch { .. } => "PatternMatch".to_string(),
        Expression::PatternFind { .. } => "PatternFind".to_string(),
        Expression::PatternReplace { .. } => "PatternReplace".to_string(),
        Expression::PatternSplit { .. } => "PatternSplit".to_string(),
        Expression::StringSplit { .. } => "StringSplit".to_string(),
        Expression::AwaitExpression { .. } => "AwaitExpression".to_string(),
        // Container-related expressions
        Expression::StaticMemberAccess {
            container, member, ..
        } => format!("StaticMemberAccess '{container}' member '{member}'"),
        Expression::MethodCall { method, .. } => format!("MethodCall '{method}'"),
        Expression::PropertyAccess { property, .. } => format!("PropertyAccess '{property}'"),
        Expression::FileExists { .. } => "FileExists".to_string(),
        Expression::DirectoryExists { .. } => "DirectoryExists".to_string(),
        Expression::ListFiles { .. } => "ListFiles".to_string(),
        Expression::ReadContent { .. } => "ReadContent".to_string(),
        Expression::ListFilesRecursive { .. } => "ListFilesRecursive".to_string(),
        Expression::ListFilesFiltered { .. } => "ListFilesFiltered".to_string(),
    }
}

use std::collections::HashMap;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncSeekExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
// use self::value::FutureValue;

pub struct Interpreter {
    global_env: Rc<RefCell<Environment>>,
    current_count: RefCell<Option<f64>>,
    in_count_loop: RefCell<bool>,
    in_main_loop: RefCell<bool>, // Track if we're in a main loop (disables timeout)
    started: Instant,
    max_duration: Duration,
    call_stack: RefCell<Vec<CallFrame>>,
    #[allow(dead_code)]
    io_client: Rc<IoClient>,
    step_mode: bool,          // Controls single-step execution mode
    script_args: Vec<String>, // Command-line arguments passed to the script
}

#[allow(dead_code)]
pub struct IoClient {
    http_client: reqwest::Client,
    file_handles: Mutex<HashMap<String, (PathBuf, tokio::fs::File)>>,
    next_file_id: Mutex<usize>,
}

impl IoClient {
    fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
            file_handles: Mutex::new(HashMap::new()),
            next_file_id: Mutex::new(1),
        }
    }

    #[allow(dead_code)]
    async fn http_get(&self, url: &str) -> Result<String, String> {
        match self.http_client.get(url).send().await {
            Ok(response) => match response.text().await {
                Ok(text) => Ok(text),
                Err(e) => Err(format!("Failed to read response body: {e}")),
            },
            Err(e) => Err(format!("Failed to send HTTP GET request: {e}")),
        }
    }

    #[allow(dead_code)]
    async fn http_post(&self, url: &str, data: &str) -> Result<String, String> {
        match self
            .http_client
            .post(url)
            .body(data.to_string())
            .send()
            .await
        {
            Ok(response) => match response.text().await {
                Ok(text) => Ok(text),
                Err(e) => Err(format!("Failed to read response body: {e}")),
            },
            Err(e) => Err(format!("Failed to send HTTP POST request: {e}")),
        }
    }

    #[allow(dead_code)]
    async fn open_file(&self, path: &str) -> Result<String, String> {
        let handle_id = {
            let mut next_id = self.next_file_id.lock().await;
            let id = format!("file{}", *next_id);
            *next_id += 1;
            id
        };

        let path_buf = PathBuf::from(path);

        match tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false) // Explicitly preserve file content on open
            .open(path)
            .await
        {
            Ok(file) => {
                let mut file_handles = self.file_handles.lock().await;

                // Check if the file is already open, but don't error - just use a new handle
                file_handles.insert(handle_id.clone(), (path_buf, file));
                Ok(handle_id)
            }
            Err(e) => Err(format!("Failed to open file {path}: {e}")),
        }
    }

    #[allow(dead_code)]
    async fn open_file_with_mode(
        &self,
        path: &str,
        mode: FileOpenMode,
    ) -> Result<String, RuntimeError> {
        let handle_id = {
            let mut next_id = self.next_file_id.lock().await;
            let id = format!("file{}", *next_id);
            *next_id += 1;
            id
        };

        let path_buf = PathBuf::from(path);

        let mut options = tokio::fs::OpenOptions::new();
        match mode {
            FileOpenMode::Read => {
                options.read(true).write(false).create(false);
            }
            FileOpenMode::Write => {
                options.read(false).write(true).create(true).truncate(true);
            }
            FileOpenMode::Append => {
                options.read(false).write(true).create(true).append(true);
            }
        }

        match options.open(path).await {
            Ok(file) => {
                let mut file_handles = self.file_handles.lock().await;
                file_handles.insert(handle_id.clone(), (path_buf, file));
                Ok(handle_id)
            }
            Err(e) => {
                let error_kind = match e.kind() {
                    std::io::ErrorKind::NotFound => ErrorKind::FileNotFound,
                    std::io::ErrorKind::PermissionDenied => ErrorKind::PermissionDenied,
                    _ => ErrorKind::General,
                };
                Err(RuntimeError::with_kind(
                    format!("Failed to open file {path}: {e}"),
                    0,
                    0,
                    error_kind,
                ))
            }
        }
    }

    #[allow(dead_code)]
    async fn read_file(&self, handle_id: &str) -> Result<String, String> {
        let mut file_handles = self.file_handles.lock().await;

        if !file_handles.contains_key(handle_id) {
            drop(file_handles);

            match self.open_file(handle_id).await {
                Ok(new_handle) => {
                    // Now read from the new handle - use Box::pin to handle recursion in async fn
                    let future = Box::pin(self.read_file(&new_handle));
                    let result = future.await;
                    let _ = self.close_file(&new_handle).await;
                    return result;
                }
                Err(e) => return Err(format!("Invalid file handle or path: {handle_id}: {e}")),
            }
        }

        let mut file_clone = match file_handles.get_mut(handle_id).unwrap().1.try_clone().await {
            Ok(clone) => clone,
            Err(e) => return Err(format!("Failed to clone file handle: {e}")),
        };

        drop(file_handles);

        let mut contents = String::new();
        match AsyncReadExt::read_to_string(&mut file_clone, &mut contents).await {
            Ok(_) => Ok(contents),
            Err(e) => Err(format!("Failed to read file: {e}")),
        }
    }

    #[allow(dead_code)]
    async fn write_file(&self, handle_id: &str, content: &str) -> Result<(), String> {
        let mut file_handles = self.file_handles.lock().await;

        if !file_handles.contains_key(handle_id) {
            drop(file_handles);

            match self.open_file(handle_id).await {
                Ok(new_handle) => {
                    // Now write to the new handle - use Box::pin to handle recursion in async fn
                    let future = Box::pin(self.write_file(&new_handle, content));
                    let result = future.await;
                    let _ = self.close_file(&new_handle).await;
                    return result;
                }
                Err(e) => return Err(format!("Invalid file handle or path: {handle_id}: {e}")),
            }
        }

        let mut file_clone = match file_handles.get_mut(handle_id).unwrap().1.try_clone().await {
            Ok(clone) => clone,
            Err(e) => return Err(format!("Failed to clone file handle: {e}")),
        };

        drop(file_handles);

        match AsyncSeekExt::seek(&mut file_clone, std::io::SeekFrom::Start(0)).await {
            Ok(_) => match file_clone.set_len(0).await {
                Ok(_) => {
                    match AsyncWriteExt::write_all(&mut file_clone, content.as_bytes()).await {
                        Ok(_) => Ok(()),
                        Err(e) => Err(format!("Failed to write to file: {e}")),
                    }
                }
                Err(e) => Err(format!("Failed to truncate file: {e}")),
            },
            Err(e) => Err(format!("Failed to seek in file: {e}")),
        }
    }

    /// Improved file append operation - directly appends content without reading the whole file first
    /// This is much more memory efficient, especially for large log files
    #[allow(dead_code)]
    async fn close_file(&self, handle_id: &str) -> Result<(), String> {
        let mut file_handles = self.file_handles.lock().await;

        if !file_handles.contains_key(handle_id) {
            return Ok(());
        }

        file_handles.remove(handle_id);
        Ok(())
    }

    #[allow(dead_code)]
    async fn append_file(&self, handle_id: &str, content: &str) -> Result<(), String> {
        let mut file_handles = self.file_handles.lock().await;

        let (_, file) = match file_handles.get_mut(handle_id) {
            Some(entry) => entry,
            None => return Err(format!("Invalid file handle: {handle_id}")),
        };

        match AsyncSeekExt::seek(file, std::io::SeekFrom::End(0)).await {
            Ok(_) => match AsyncWriteExt::write_all(file, content.as_bytes()).await {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Failed to append to file: {e}")),
            },
            Err(e) => Err(format!("Failed to seek to end of file: {e}")),
        }
    }

    #[allow(dead_code)]
    async fn create_directory(&self, path: &str) -> Result<(), String> {
        match tokio::fs::create_dir_all(path).await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to create directory: {e}")),
        }
    }

    #[allow(dead_code)]
    async fn create_file(&self, path: &str, content: &str) -> Result<(), String> {
        match tokio::fs::write(path, content).await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to create file: {e}")),
        }
    }

    #[allow(dead_code)]
    async fn delete_file(&self, path: &str) -> Result<(), String> {
        match tokio::fs::remove_file(path).await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to delete file: {e}")),
        }
    }

    #[allow(dead_code)]
    async fn delete_directory(&self, path: &str) -> Result<(), String> {
        match tokio::fs::remove_dir_all(path).await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to delete directory: {e}")),
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    pub fn new() -> Self {
        let global_env = Environment::new_global();

        {
            let mut env = global_env.borrow_mut();
            let _ = env.define(
                "display",
                Value::NativeFunction("display", Self::native_display),
            );

            stdlib::register_stdlib(&mut env);
        }

        Interpreter {
            global_env,
            current_count: RefCell::new(None),
            in_count_loop: RefCell::new(false),
            in_main_loop: RefCell::new(false),
            started: Instant::now(),
            max_duration: Duration::from_secs(u64::MAX), // Effectively no timeout by default
            call_stack: RefCell::new(Vec::new()),
            io_client: Rc::new(IoClient::new()),
            step_mode: false,        // Default to non-step mode
            script_args: Vec::new(), // Initialize empty, will be set later
        }
    }

    pub fn with_timeout(seconds: u64) -> Self {
        let mut interpreter = Self::new();
        interpreter.started = Instant::now();
        interpreter.max_duration = Duration::from_secs(if seconds > 300 { 300 } else { seconds });
        interpreter
    }

    pub fn set_step_mode(&mut self, step_mode: bool) {
        self.step_mode = step_mode;
    }

    pub fn set_script_args(&mut self, args: Vec<String>) {
        self.script_args = args;
    }

    fn dump_state(
        &self,
        stmt: &Statement,
        line: usize,
        _column: usize,
        env_before: &HashMap<String, Value>,
    ) {
        println!("Line {}: {}", line, Self::get_statement_text(stmt));

        let current_env = self.global_env.borrow();
        let mut changes = Vec::new();

        for (name, value) in current_env.values.iter() {
            if let Some(old_value) = env_before.get(name) {
                if !value.eq(old_value) {
                    changes.push(format!("{name} = {old_value} -> {value}"));
                }
            } else {
                changes.push(format!("{name} = {value}"));
            }
        }

        if !changes.is_empty() {
            println!("Variables changed:");
            for change in changes {
                println!("  {change}");
            }
        }

        let call_stack = self.get_call_stack();
        if !call_stack.is_empty() {
            println!("Call stack:");
            for frame in &call_stack {
                println!("  {} (line {})", frame.func_name, frame.call_line);
            }
        }
    }

    fn get_statement_text(stmt: &Statement) -> String {
        format!("{stmt:?}")
    }

    pub fn prompt_continue(&self) -> bool {
        loop {
            print!("continue (y/n)? ");
            if let Err(e) = io::stdout().flush() {
                eprintln!("Error flushing stdout: {e}");
            }

            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(_) => {
                    let input = input.trim().to_lowercase();
                    match input.as_str() {
                        "y" => return true,
                        "n" => return false,
                        _ => {
                            println!("Invalid input. Please enter 'y' or 'n'.");
                            continue;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading input: {e}");
                    return false;
                }
            }
        }
    }

    pub fn get_call_stack(&self) -> Vec<CallFrame> {
        self.call_stack.borrow().clone()
    }

    pub fn clear_call_stack(&self) {
        self.call_stack.borrow_mut().clear();
    }
    pub fn global_env(&self) -> &Rc<RefCell<Environment>> {
        &self.global_env
    }

    fn check_time(&self) -> Result<(), RuntimeError> {
        // Skip timeout check if we're in a main loop
        if *self.in_main_loop.borrow() {
            return Ok(());
        }

        if self.started.elapsed() > self.max_duration {
            if *self.in_count_loop.borrow() {
                *self.in_count_loop.borrow_mut() = false;
                *self.current_count.borrow_mut() = None;
            }

            // Force all resources to be released
            self.call_stack.borrow_mut().clear();

            // Terminate with a timeout error
            Err(RuntimeError::with_kind(
                format!(
                    "Execution exceeded timeout ({}s)",
                    self.max_duration.as_secs()
                ),
                0,
                0,
                ErrorKind::Timeout,
            ))
        } else {
            Ok(())
        }
    }

    fn assert_invariants(&self) {
        debug_assert_eq!(
            *self.in_count_loop.borrow(),
            self.current_count.borrow().is_some()
        );

        debug_assert!(self.call_stack.borrow().len() < 10_000);
    }

    fn native_display(args: Vec<Value>) -> Result<Value, RuntimeError> {
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                print!(" ");
            }
            print!("{arg}");
        }
        println!();
        Ok(Value::Null)
    }

    pub async fn interpret(&mut self, program: &Program) -> Result<Value, Vec<RuntimeError>> {
        self.assert_invariants();
        self.call_stack.borrow_mut().clear();

        // Set up script arguments in the global environment
        {
            let mut env = self.global_env.borrow_mut();

            // Create args list with all arguments
            let args_list: Vec<Value> = self
                .script_args
                .iter()
                .map(|arg| Value::Text(Rc::from(arg.as_str())))
                .collect();
            let _ = env.define("args", Value::List(Rc::new(RefCell::new(args_list))));

            // Parse and set up flags (arguments starting with - or --)
            let mut flags = HashMap::new();
            let mut positional_args = Vec::new();
            let mut i = 0;

            while i < self.script_args.len() {
                let arg = &self.script_args[i];
                if arg.starts_with("--") {
                    let flag_name = arg.trim_start_matches("--");
                    // Check if next argument is a value for this flag
                    if i + 1 < self.script_args.len() && !self.script_args[i + 1].starts_with("-") {
                        flags.insert(
                            flag_name.to_string(),
                            Value::Text(Rc::from(self.script_args[i + 1].as_str())),
                        );
                        i += 2;
                    } else {
                        flags.insert(flag_name.to_string(), Value::Bool(true));
                        i += 1;
                    }
                } else if arg.starts_with("-") && arg.len() > 1 {
                    // Handle short flags like -f
                    let flag_name = arg.trim_start_matches("-");
                    // Check if next argument is a value for this flag
                    if i + 1 < self.script_args.len() && !self.script_args[i + 1].starts_with("-") {
                        flags.insert(
                            flag_name.to_string(),
                            Value::Text(Rc::from(self.script_args[i + 1].as_str())),
                        );
                        i += 2;
                    } else {
                        flags.insert(flag_name.to_string(), Value::Bool(true));
                        i += 1;
                    }
                } else {
                    positional_args.push(Value::Text(Rc::from(arg.as_str())));
                    i += 1;
                }
            }

            // Convert flags HashMap to Value
            let mut flags_map = HashMap::new();
            for (key, value) in flags {
                flags_map.insert(key, value);
            }

            // Store positional arguments
            let _ = env.define(
                "positional_args",
                Value::List(Rc::new(RefCell::new(positional_args.clone()))),
            );

            // Store argument count
            let _ = env.define("arg_count", Value::Number(self.script_args.len() as f64));

            // Store program name (first argument or empty string)
            let program_name = if self.script_args.is_empty() {
                "wfl".to_string()
            } else {
                // Extract just the filename from the path
                std::path::Path::new(&self.script_args[0])
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            };
            let _ = env.define("program_name", Value::Text(Rc::from(program_name)));

            // Store current directory
            let current_dir = std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let _ = env.define("current_directory", Value::Text(Rc::from(current_dir)));

            // Store flags as individual variables with flag_ prefix
            for (key, value) in flags_map {
                let _ = env.define(&format!("flag_{key}"), value);
            }
        }

        // Use exec_trace for execution logs instead of println
        if !self.step_mode {
            exec_trace!(
                "Starting script execution with {} statements...",
                program.statements.len()
            );
        }
        exec_trace!("=== Starting program execution ===");

        let mut last_value = Value::Null;
        let mut errors = Vec::new();

        #[allow(unused_variables)]
        for (i, statement) in program.statements.iter().enumerate() {
            if !self.step_mode {
                exec_trace!(
                    "Executing statement {}/{}...",
                    i + 1,
                    program.statements.len()
                );
            }
            exec_trace!("Executing statement {}/{}", i + 1, program.statements.len());

            if let Err(err) = self.check_time() {
                if !self.step_mode {
                    exec_trace!(
                        "Timeout reached at statement {}/{}",
                        i + 1,
                        program.statements.len()
                    );
                }
                errors.push(err);
                return Err(errors);
            }

            match self
                .execute_statement(statement, Rc::clone(&self.global_env))
                .await
            {
                Ok((value, control_flow)) => {
                    last_value = value;
                    if !self.step_mode {
                        exec_trace!(
                            "Statement {}/{} completed successfully",
                            i + 1,
                            program.statements.len()
                        );
                    }

                    match control_flow {
                        ControlFlow::Break | ControlFlow::Continue | ControlFlow::Exit => {
                            exec_trace!("Warning: {:?} at top level ignored", control_flow);
                        }
                        ControlFlow::Return(val) => {
                            exec_trace!("Return at top level with value: {:?}", val);
                            last_value = val;
                            break;
                        }
                        ControlFlow::None => {}
                    }
                }
                Err(err) => {
                    if !self.step_mode {
                        exec_trace!(
                            "Error at statement {}/{}: {:?}",
                            i + 1,
                            program.statements.len(),
                            err
                        );
                    }
                    errors.push(err);
                    break; // Stop on first runtime error
                }
            }
        }

        if errors.is_empty() {
            let main_func_opt = {
                if let Some(Value::Function(main_func)) = self.global_env.borrow().get("main") {
                    Some(main_func.clone())
                } else {
                    None
                }
            };

            if let Some(main_func) = main_func_opt {
                exec_trace!("Calling main function");
                match self.call_function(&main_func, vec![], 0, 0).await {
                    Ok(value) => {
                        exec_trace!("Main function returned: {:?}", value);
                        last_value = value
                    }
                    Err(err) => {
                        exec_trace!("Main function failed: {}", err);
                        errors.push(err);
                    }
                }
            }

            self.assert_invariants();
            Ok(last_value)
        } else {
            self.assert_invariants();
            Err(errors)
        }
    }

    async fn execute_statement(
        &self,
        stmt: &Statement,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        #[cfg(debug_assertions)]
        exec_trace!("Executing statement: {}", stmt_type(stmt));
        Box::pin(self._execute_statement(stmt, env)).await
    }

    async fn _execute_statement(
        &self,
        stmt: &Statement,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        self.check_time()?;

        let env_before = if self.step_mode {
            self.global_env.borrow().values.clone()
        } else {
            HashMap::new()
        };

        let (line, column) = match stmt {
            Statement::VariableDeclaration { line, column, .. } => (*line, *column),
            Statement::Assignment { line, column, .. } => (*line, *column),
            Statement::IfStatement { line, column, .. } => (*line, *column),
            Statement::SingleLineIf { line, column, .. } => (*line, *column),
            Statement::DisplayStatement { line, column, .. } => (*line, *column),
            Statement::ActionDefinition { line, column, .. } => (*line, *column),
            Statement::ReturnStatement { line, column, .. } => (*line, *column),
            Statement::ExpressionStatement { line, column, .. } => (*line, *column),
            Statement::CountLoop { line, column, .. } => (*line, *column),
            Statement::ForEachLoop { line, column, .. } => (*line, *column),
            Statement::WhileLoop { line, column, .. } => (*line, *column),
            Statement::RepeatUntilLoop { line, column, .. } => (*line, *column),
            Statement::RepeatWhileLoop { line, column, .. } => (*line, *column),
            Statement::ForeverLoop { line, column, .. } => (*line, *column),
            Statement::MainLoop { line, column, .. } => (*line, *column),
            Statement::BreakStatement { line, column, .. } => (*line, *column),
            Statement::ContinueStatement { line, column, .. } => (*line, *column),
            Statement::ExitStatement { line, column, .. } => (*line, *column),
            Statement::OpenFileStatement { line, column, .. } => (*line, *column),
            Statement::ReadFileStatement { line, column, .. } => (*line, *column),
            Statement::WriteFileStatement { line, column, .. } => (*line, *column),
            Statement::WriteToStatement { line, column, .. } => (*line, *column),
            Statement::CloseFileStatement { line, column, .. } => (*line, *column),
            Statement::CreateDirectoryStatement { line, column, .. } => (*line, *column),
            Statement::CreateFileStatement { line, column, .. } => (*line, *column),
            Statement::DeleteFileStatement { line, column, .. } => (*line, *column),
            Statement::DeleteDirectoryStatement { line, column, .. } => (*line, *column),
            Statement::WaitForStatement { line, column, .. } => (*line, *column),
            Statement::TryStatement { line, column, .. } => (*line, *column),
            Statement::HttpGetStatement { line, column, .. } => (*line, *column),
            Statement::HttpPostStatement { line, column, .. } => (*line, *column),
            Statement::PushStatement { line, column, .. } => (*line, *column),
            Statement::CreateListStatement { line, column, .. } => (*line, *column),
            Statement::MapCreation { line, column, .. } => (*line, *column),
            Statement::CreateDateStatement { line, column, .. } => (*line, *column),
            Statement::CreateTimeStatement { line, column, .. } => (*line, *column),
            Statement::AddToListStatement { line, column, .. } => (*line, *column),
            Statement::RemoveFromListStatement { line, column, .. } => (*line, *column),
            Statement::ClearListStatement { line, column, .. } => (*line, *column),
            // Container-related statements
            Statement::ContainerDefinition { line, column, .. } => (*line, *column),
            Statement::ContainerInstantiation { line, column, .. } => (*line, *column),
            Statement::InterfaceDefinition { line, column, .. } => (*line, *column),
            Statement::EventDefinition { line, column, .. } => (*line, *column),
            Statement::EventTrigger { line, column, .. } => (*line, *column),
            Statement::EventHandler { line, column, .. } => (*line, *column),
            Statement::ParentMethodCall { line, column, .. } => (*line, *column),
            Statement::PatternDefinition { line, column, .. } => (*line, *column),
            Statement::ListenStatement { line, column, .. } => (*line, *column),
            Statement::WaitForRequestStatement { line, column, .. } => (*line, *column),
            Statement::RespondStatement { line, column, .. } => (*line, *column),
        };

        let result = match stmt {
            Statement::VariableDeclaration {
                name,
                value,
                is_constant,
                line: _line,
                column: _column,
            } => {
                let mut evaluated_value = self.evaluate_expression(value, Rc::clone(&env)).await?;

                if let Value::Text(text) = &evaluated_value
                    && text.as_ref() == "[]"
                {
                    evaluated_value = Value::List(Rc::new(RefCell::new(Vec::new())));
                }

                #[cfg(debug_assertions)]
                exec_var_declare!(name, &evaluated_value);

                let result = if *is_constant {
                    env.borrow_mut()
                        .define_constant(name, evaluated_value.clone())
                } else {
                    // Check if this variable already exists in the current environment
                    // This handles container property assignment in methods
                    if env.borrow().get(name).is_some() {
                        // Variable exists, use assignment instead of definition
                        env.borrow_mut().assign(name, evaluated_value.clone())
                    } else {
                        // Variable doesn't exist, use normal definition
                        env.borrow_mut().define(name, evaluated_value.clone())
                    }
                };

                match result {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(msg) => Err(RuntimeError::new(msg, line, column)),
                }
            }

            Statement::Assignment {
                name,
                value,
                line,
                column,
            } => {
                let value = self.evaluate_expression(value, Rc::clone(&env)).await?;
                #[cfg(debug_assertions)]
                exec_var_assign!(name, &value);
                match env.borrow_mut().assign(name, value.clone()) {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(msg) => Err(RuntimeError::new(msg, *line, *column)),
                }
            }

            Statement::IfStatement {
                condition,
                then_block,
                else_block,
                line: _line,
                column: _column,
            } => {
                let condition_value = self.evaluate_expression(condition, Rc::clone(&env)).await?;
                #[cfg(debug_assertions)]
                exec_control_flow!("if condition", condition_value.is_truthy());

                if condition_value.is_truthy() {
                    #[cfg(debug_assertions)]
                    let _guard = IndentGuard::new();
                    #[cfg(debug_assertions)]
                    exec_block_enter!("if branch");
                    let result = self.execute_block(then_block, Rc::clone(&env)).await;
                    #[cfg(debug_assertions)]
                    exec_block_exit!("if branch");
                    result
                } else if let Some(else_stmts) = else_block {
                    #[cfg(debug_assertions)]
                    let _guard = IndentGuard::new();
                    #[cfg(debug_assertions)]
                    exec_block_enter!("else branch");
                    let result = self.execute_block(else_stmts, Rc::clone(&env)).await;
                    #[cfg(debug_assertions)]
                    exec_block_exit!("else branch");
                    result
                } else {
                    Ok((Value::Null, ControlFlow::None))
                }
            }

            Statement::SingleLineIf {
                condition,
                then_stmt,
                else_stmt,
                line: _line,
                column: _column,
            } => {
                let condition_value = self.evaluate_expression(condition, Rc::clone(&env)).await?;

                if condition_value.is_truthy() {
                    self.execute_statement(then_stmt, Rc::clone(&env)).await
                } else if let Some(else_stmt) = else_stmt {
                    self.execute_statement(else_stmt, Rc::clone(&env)).await
                } else {
                    Ok((Value::Null, ControlFlow::None))
                }
            }

            Statement::DisplayStatement {
                value,
                line: _line,
                column: _column,
            } => {
                let value = self.evaluate_expression(value, Rc::clone(&env)).await?;
                println!("{value}");
                Ok((Value::Null, ControlFlow::None))
            }

            Statement::ActionDefinition {
                name,
                parameters,
                body,
                return_type: _return_type,
                line,
                column,
            } => {
                let param_names: Vec<String> = parameters.iter().map(|p| p.name.clone()).collect();

                let function = FunctionValue {
                    name: Some(name.clone()),
                    params: param_names,
                    body: body.clone(),
                    env: Rc::downgrade(&env),
                    line: *line,
                    column: *column,
                };

                let function_value = Value::Function(Rc::new(function));
                match env.borrow_mut().define(name, function_value.clone()) {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                }

                Ok((function_value, ControlFlow::None))
            }

            Statement::ReturnStatement {
                value,
                line: _line,
                column: _column,
            } => {
                #[cfg(debug_assertions)]
                exec_trace!("Executing return statement");

                if let Some(expr) = value {
                    let result = self.evaluate_expression(expr, Rc::clone(&env)).await?;
                    Ok((result.clone(), ControlFlow::Return(result)))
                } else {
                    Ok((Value::Null, ControlFlow::Return(Value::Null)))
                }
            }

            Statement::ExpressionStatement {
                expression,
                line: _line,
                column: _column,
            } => {
                // Check if this is a bare action call (just the action name without parentheses)
                if let Expression::Variable(name, var_line, var_column) = expression {
                    // Check if the variable refers to an action
                    if let Some(Value::Function(func)) = env.borrow().get(name) {
                        // It's an action, so execute it as a call with no arguments
                        #[cfg(debug_assertions)]
                        exec_trace!("Executing bare action call: {}", name);
                        return self
                            .call_function(&func, vec![], *var_line, *var_column)
                            .await
                            .map(|value| (value, ControlFlow::None));
                    }
                }

                // Regular expression evaluation
                let value = self
                    .evaluate_expression(expression, Rc::clone(&env))
                    .await?;
                Ok((value, ControlFlow::None))
            }

            Statement::CountLoop {
                start,
                end,
                step,
                downward,
                body,
                line,
                column,
            } => {
                // === CRITICAL FIX: Reset count loop state before starting ===
                let previous_count = *self.current_count.borrow();
                let was_in_count_loop = *self.in_count_loop.borrow();

                // Force reset state to prevent inheriting stale values
                *self.current_count.borrow_mut() = None;
                *self.in_count_loop.borrow_mut() = false;

                crate::exec_trace!("Count loop: resetting state before evaluation");

                let start_val = self.evaluate_expression(start, Rc::clone(&env)).await?;
                let end_val = self.evaluate_expression(end, Rc::clone(&env)).await?;

                let (start_num, end_num) = match (start_val, end_val) {
                    (Value::Number(s), Value::Number(e)) => (s, e),
                    _ => {
                        return Err(RuntimeError::new(
                            "Count loop requires numeric start and end values".to_string(),
                            *line,
                            *column,
                        ));
                    }
                };

                let step_num = if let Some(step_expr) = step {
                    match self.evaluate_expression(step_expr, Rc::clone(&env)).await? {
                        Value::Number(n) => n,
                        _ => {
                            return Err(RuntimeError::new(
                                "Count loop step must be a number".to_string(),
                                *line,
                                *column,
                            ));
                        }
                    }
                } else {
                    1.0
                };

                let mut count = start_num;

                let should_continue: Box<dyn Fn(f64, f64) -> bool> = if *downward {
                    Box::new(|count, end_num| count >= end_num)
                } else {
                    Box::new(|count, end_num| count <= end_num)
                };

                let max_iterations = if end_num > 1000000.0 {
                    u64::MAX // Effectively no limit for large end values, rely on timeout instead
                } else {
                    10000 // Reasonable limit for normal loops
                };
                let mut iterations = 0;

                *self.in_count_loop.borrow_mut() = true;

                while should_continue(count, end_num) && iterations < max_iterations {
                    self.check_time()?;

                    *self.current_count.borrow_mut() = Some(count);

                    // Create a new scope for each iteration
                    let loop_env = Environment::new_child_env(&env);

                    // Also make count available as a regular variable in the loop environment
                    // This ensures consistency and allows for nested count loops to work properly
                    let _ = loop_env.borrow_mut().define("count", Value::Number(count));

                    let result = self.execute_block(body, Rc::clone(&loop_env)).await;

                    match result {
                        Ok((_, control_flow)) => match control_flow {
                            ControlFlow::Break => {
                                #[cfg(debug_assertions)]
                                exec_trace!("Breaking out of count loop");
                                break;
                            }
                            ControlFlow::Continue => {
                                #[cfg(debug_assertions)]
                                exec_trace!("Continuing count loop");
                            }
                            ControlFlow::Exit => {
                                #[cfg(debug_assertions)]
                                exec_trace!("Exiting from count loop");
                                *self.current_count.borrow_mut() = previous_count;
                                *self.in_count_loop.borrow_mut() = was_in_count_loop;
                                return Ok((Value::Null, ControlFlow::Exit));
                            }
                            ControlFlow::Return(val) => {
                                #[cfg(debug_assertions)]
                                exec_trace!("Returning from count loop with value: {:?}", val);
                                *self.current_count.borrow_mut() = previous_count;
                                *self.in_count_loop.borrow_mut() = was_in_count_loop;
                                return Ok((val.clone(), ControlFlow::Return(val)));
                            }
                            ControlFlow::None => {}
                        },
                        Err(e) => {
                            *self.current_count.borrow_mut() = previous_count;
                            *self.in_count_loop.borrow_mut() = was_in_count_loop;
                            return Err(e);
                        }
                    }

                    if *downward {
                        count -= step_num;
                    } else {
                        count += step_num;
                    }

                    iterations += 1;
                }

                *self.current_count.borrow_mut() = previous_count;
                *self.in_count_loop.borrow_mut() = was_in_count_loop;

                if iterations >= max_iterations {
                    return Err(RuntimeError::new(
                        format!("Count loop exceeded maximum iterations ({max_iterations})"),
                        *line,
                        *column,
                    ));
                }

                Ok((Value::Null, ControlFlow::None))
            }

            Statement::ForEachLoop {
                item_name,
                collection,
                reversed,
                body,
                line,
                column,
            } => {
                let collection_val = self
                    .evaluate_expression(collection, Rc::clone(&env))
                    .await?;

                match collection_val {
                    Value::List(list_rc) => {
                        let items: Vec<Value> = {
                            let list = list_rc.borrow();
                            let indices: Vec<usize> = if *reversed {
                                (0..list.len()).rev().collect()
                            } else {
                                (0..list.len()).collect()
                            };
                            indices.iter().map(|&i| list[i].clone()).collect()
                        };

                        for item in items {
                            // Create a new scope for each iteration
                            let loop_env = Environment::new_child_env(&env);
                            match loop_env.borrow_mut().define(item_name, item) {
                                Ok(_) => {}
                                Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                            }
                            let result = self.execute_block(body, Rc::clone(&loop_env)).await?;

                            match result.1 {
                                ControlFlow::Break => {
                                    #[cfg(debug_assertions)]
                                    exec_trace!("Breaking out of foreach loop");
                                    break;
                                }
                                ControlFlow::Continue => {
                                    #[cfg(debug_assertions)]
                                    exec_trace!("Continuing foreach loop");
                                    continue;
                                }
                                ControlFlow::Exit => {
                                    #[cfg(debug_assertions)]
                                    exec_trace!("Exiting from foreach loop");
                                    return Ok((Value::Null, ControlFlow::Exit));
                                }
                                ControlFlow::Return(val) => {
                                    #[cfg(debug_assertions)]
                                    exec_trace!(
                                        "Returning from foreach loop with value: {:?}",
                                        val
                                    );
                                    return Ok((val.clone(), ControlFlow::Return(val)));
                                }
                                ControlFlow::None => {}
                            }
                        }
                    }
                    Value::Object(obj_rc) => {
                        let items: Vec<(String, Value)> = {
                            let obj = obj_rc.borrow();
                            obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                        };

                        for (_, value) in items {
                            // Create a new scope for each iteration
                            let loop_env = Environment::new_child_env(&env);
                            match loop_env.borrow_mut().define(item_name, value) {
                                Ok(_) => {}
                                Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                            }
                            let result = self.execute_block(body, Rc::clone(&loop_env)).await?;

                            match result.1 {
                                ControlFlow::Break => {
                                    #[cfg(debug_assertions)]
                                    exec_trace!("Breaking out of foreach loop (object)");
                                    break;
                                }
                                ControlFlow::Continue => {
                                    #[cfg(debug_assertions)]
                                    exec_trace!("Continuing foreach loop (object)");
                                    continue;
                                }
                                ControlFlow::Exit => {
                                    #[cfg(debug_assertions)]
                                    exec_trace!("Exiting from foreach loop (object)");
                                    return Ok((Value::Null, ControlFlow::Exit));
                                }
                                ControlFlow::Return(val) => {
                                    #[cfg(debug_assertions)]
                                    exec_trace!(
                                        "Returning from foreach loop with value: {:?}",
                                        val
                                    );
                                    return Ok((val.clone(), ControlFlow::Return(val)));
                                }
                                ControlFlow::None => {}
                            }
                        }
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Cannot iterate over {}", collection_val.type_name()),
                            *line,
                            *column,
                        ));
                    }
                }

                Ok((Value::Null, ControlFlow::None))
            }

            Statement::WhileLoop {
                condition,
                body,
                line: _line,
                column: _column,
            } => {
                let mut _last_value = Value::Null;

                while self
                    .evaluate_expression(condition, Rc::clone(&env))
                    .await?
                    .is_truthy()
                {
                    self.check_time()?;
                    let result = self.execute_block(body, Rc::clone(&env)).await?;
                    _last_value = result.0;

                    match result.1 {
                        ControlFlow::Break => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Breaking out of while loop");
                            break;
                        }
                        ControlFlow::Continue => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Continuing while loop");
                            continue;
                        }
                        ControlFlow::Exit => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Exiting from while loop");
                            return Ok((_last_value, ControlFlow::Exit));
                        }
                        ControlFlow::Return(val) => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Returning from while loop with value: {:?}", val);
                            return Ok((val.clone(), ControlFlow::Return(val)));
                        }
                        ControlFlow::None => {}
                    }
                }

                Ok((_last_value, ControlFlow::None))
            }

            Statement::RepeatUntilLoop {
                condition,
                body,
                line: _line,
                column: _column,
            } => {
                let mut _last_value = Value::Null;

                loop {
                    self.check_time()?;
                    let result = self.execute_block(body, Rc::clone(&env)).await?;
                    _last_value = result.0;

                    match result.1 {
                        ControlFlow::Break => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Breaking out of repeat-until loop");
                            break;
                        }
                        ControlFlow::Continue => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Continuing repeat-until loop");
                        }
                        ControlFlow::Exit => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Exiting from repeat-until loop");
                            return Ok((_last_value, ControlFlow::Exit));
                        }
                        ControlFlow::Return(val) => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Returning from repeat-until loop with value: {:?}", val);
                            return Ok((val.clone(), ControlFlow::Return(val)));
                        }
                        ControlFlow::None => {}
                    }

                    if self
                        .evaluate_expression(condition, Rc::clone(&env))
                        .await?
                        .is_truthy()
                    {
                        break;
                    }
                }

                Ok((_last_value, ControlFlow::None))
            }

            Statement::ForeverLoop {
                body,
                line: _line,
                column: _column,
            } => {
                #[cfg(debug_assertions)]
                exec_trace!("Executing forever loop");

                let mut _last_value = Value::Null;
                loop {
                    self.check_time()?;
                    let result = self.execute_block(body, Rc::clone(&env)).await?;
                    _last_value = result.0;

                    match result.1 {
                        ControlFlow::Break => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Breaking out of forever loop");
                            break;
                        }
                        ControlFlow::Continue => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Continuing forever loop");
                            continue;
                        }
                        ControlFlow::Exit => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Exiting from forever loop");
                            return Ok((_last_value, ControlFlow::Exit));
                        }
                        ControlFlow::Return(val) => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Returning from forever loop with value: {:?}", val);
                            return Ok((val.clone(), ControlFlow::Return(val)));
                        }
                        ControlFlow::None => {}
                    }
                }

                Ok((_last_value, ControlFlow::None))
            }

            Statement::MainLoop {
                body,
                line: _line,
                column: _column,
            } => {
                #[cfg(debug_assertions)]
                exec_trace!("Executing main loop (timeout disabled)");

                // Set the main loop flag to disable timeout
                *self.in_main_loop.borrow_mut() = true;

                let mut _last_value = Value::Null;
                loop {
                    // Note: check_time() will skip timeout check when in_main_loop is true
                    self.check_time()?;
                    let result = self.execute_block(body, Rc::clone(&env)).await?;
                    _last_value = result.0;

                    match result.1 {
                        ControlFlow::Break => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Breaking out of main loop");
                            break;
                        }
                        ControlFlow::Continue => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Continuing main loop");
                            continue;
                        }
                        ControlFlow::Exit => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Exiting from main loop");
                            *self.in_main_loop.borrow_mut() = false;
                            return Ok((_last_value, ControlFlow::Exit));
                        }
                        ControlFlow::Return(val) => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Returning from main loop with value: {:?}", val);
                            *self.in_main_loop.borrow_mut() = false;
                            return Ok((val.clone(), ControlFlow::Return(val)));
                        }
                        ControlFlow::None => {}
                    }
                }

                // Reset the main loop flag when exiting normally
                *self.in_main_loop.borrow_mut() = false;

                Ok((_last_value, ControlFlow::None))
            }

            Statement::BreakStatement { .. } => {
                #[cfg(debug_assertions)]
                exec_trace!("Executing break statement");
                Ok((Value::Null, ControlFlow::Break))
            }

            Statement::ContinueStatement { .. } => {
                #[cfg(debug_assertions)]
                exec_trace!("Executing continue statement");
                Ok((Value::Null, ControlFlow::Continue))
            }

            Statement::ExitStatement { .. } => {
                #[cfg(debug_assertions)]
                exec_trace!("Executing exit statement");
                Ok((Value::Null, ControlFlow::Exit))
            }

            Statement::OpenFileStatement {
                path,
                variable_name,
                mode,
                line,
                column,
            } => {
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str = match &path_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for file path, got {path_value:?}"),
                            *line,
                            *column,
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
                            Err(msg) => Err(RuntimeError::new(msg, *line, *column)),
                        }
                    }
                    Err(e) => Err(e),
                }
            }
            Statement::ReadFileStatement {
                path,
                variable_name,
                line,
                column,
            } => {
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str = match &path_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for file path or handle, got {path_value:?}"),
                            *line,
                            *column,
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
                                        Err(RuntimeError::new(msg, *line, *column))
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = self.io_client.close_file(&handle).await;
                                Err(RuntimeError::new(e, *line, *column))
                            }
                        },
                        Err(e) => Err(RuntimeError::new(e, *line, *column)),
                    }
                } else {
                    match self.io_client.read_file(&path_str).await {
                        Ok(content) => {
                            match env
                                .borrow_mut()
                                .define(variable_name, Value::Text(content.into()))
                            {
                                Ok(_) => Ok((Value::Null, ControlFlow::None)),
                                Err(msg) => Err(RuntimeError::new(msg, *line, *column)),
                            }
                        }
                        Err(e) => Err(RuntimeError::new(e, *line, *column)),
                    }
                }
            }
            Statement::WriteFileStatement {
                file,
                content,
                mode,
                line,
                column,
            } => {
                let file_value = self.evaluate_expression(file, Rc::clone(&env)).await?;
                let content_value = self.evaluate_expression(content, Rc::clone(&env)).await?;

                let file_str = match &file_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for file handle, got {file_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                let content_str = match &content_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for file content, got {content_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                match mode {
                    crate::parser::ast::WriteMode::Append => {
                        match self.io_client.append_file(&file_str, &content_str).await {
                            Ok(_) => Ok((Value::Null, ControlFlow::None)),
                            Err(e) => Err(RuntimeError::new(e, *line, *column)),
                        }
                    }
                    crate::parser::ast::WriteMode::Overwrite => {
                        match self.io_client.write_file(&file_str, &content_str).await {
                            Ok(_) => Ok((Value::Null, ControlFlow::None)),
                            Err(e) => Err(RuntimeError::new(e, *line, *column)),
                        }
                    }
                }
            }
            Statement::CloseFileStatement { file, line, column } => {
                let file_value = self.evaluate_expression(file, Rc::clone(&env)).await?;

                let file_str = match &file_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for file handle, got {file_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                match self.io_client.close_file(&file_str).await {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(e) => Err(RuntimeError::new(e, *line, *column)),
                }
            }
            Statement::CreateDirectoryStatement { path, line, column } => {
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str = match &path_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for directory path, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                match self.io_client.create_directory(&path_str).await {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(e) => Err(RuntimeError::new(e, *line, *column)),
                }
            }
            Statement::CreateFileStatement {
                path,
                content,
                line,
                column,
            } => {
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let content_value = self.evaluate_expression(content, Rc::clone(&env)).await?;

                let path_str = match &path_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for file path, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                let content_str = format!("{content_value}");

                match self.io_client.create_file(&path_str, &content_str).await {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(e) => Err(RuntimeError::new(e, *line, *column)),
                }
            }
            Statement::DeleteFileStatement { path, line, column } => {
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str = match &path_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for file path, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                match self.io_client.delete_file(&path_str).await {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(e) => Err(RuntimeError::new(e, *line, *column)),
                }
            }
            Statement::WriteToStatement {
                content,
                file,
                line,
                column,
            } => {
                let content_value = self.evaluate_expression(content, Rc::clone(&env)).await?;
                let file_value = self.evaluate_expression(file, Rc::clone(&env)).await?;

                let file_str = match &file_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for file handle, got {file_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                let content_str = format!("{content_value}");

                match self.io_client.write_file(&file_str, &content_str).await {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(e) => Err(RuntimeError::new(e, *line, *column)),
                }
            }
            Statement::DeleteDirectoryStatement { path, line, column } => {
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str = match &path_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for directory path, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                match self.io_client.delete_directory(&path_str).await {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(e) => Err(RuntimeError::new(e, *line, *column)),
                }
            }
            Statement::WaitForStatement {
                inner,
                line: _line,
                column: _column,
            } => {
                match inner.as_ref() {
                    Statement::ExpressionStatement {
                        expression: Expression::Variable(var_name, _, _),
                        line: _,
                        column: _,
                    } => {
                        let max_attempts = 1000; // Prevent infinite waiting
                        for _ in 0..max_attempts {
                            if let Some(value) = env.borrow().get(var_name)
                                && !matches!(value, Value::Null)
                            {
                                return Ok((Value::Null, ControlFlow::None));
                            }

                            tokio::time::sleep(std::time::Duration::from_millis(10)).await;

                            self.check_time()?;
                        }

                        Err(RuntimeError::new(
                            format!("Timeout waiting for variable '{var_name}'"),
                            0,
                            0,
                        ))
                    }
                    Statement::WriteFileStatement {
                        file,
                        content,
                        mode,
                        line,
                        column,
                    } => {
                        let file_value = self.evaluate_expression(file, Rc::clone(&env)).await?;
                        let content_value =
                            self.evaluate_expression(content, Rc::clone(&env)).await?;

                        let file_str = match &file_value {
                            Value::Text(s) => s.clone(),
                            _ => {
                                return Err(RuntimeError::new(
                                    format!("Expected string for file handle, got {file_value:?}"),
                                    *line,
                                    *column,
                                ));
                            }
                        };

                        let content_str = match &content_value {
                            Value::Text(s) => s.clone(),
                            _ => {
                                return Err(RuntimeError::new(
                                    format!(
                                        "Expected string for file content, got {content_value:?}"
                                    ),
                                    *line,
                                    *column,
                                ));
                            }
                        };

                        exec_trace!("Writing to file: {}, content: {}", file_str, content_str);
                        match mode {
                            crate::parser::ast::WriteMode::Append => {
                                match self.io_client.append_file(&file_str, &content_str).await {
                                    Ok(_) => {
                                        exec_trace!("Successfully appended to file");
                                        Ok((Value::Null, ControlFlow::None))
                                    }
                                    Err(e) => {
                                        exec_trace!("Error appending to file: {}", e);
                                        Err(RuntimeError::new(e, *line, *column))
                                    }
                                }
                            }
                            crate::parser::ast::WriteMode::Overwrite => {
                                match self.io_client.write_file(&file_str, &content_str).await {
                                    Ok(_) => {
                                        exec_trace!("Successfully wrote to file");
                                        Ok((Value::Null, ControlFlow::None))
                                    }
                                    Err(e) => {
                                        exec_trace!("Error writing to file: {}", e);
                                        Err(RuntimeError::new(e, *line, *column))
                                    }
                                }
                            }
                        }
                    }
                    Statement::ReadFileStatement {
                        path,
                        variable_name,
                        line,
                        column,
                    } => {
                        exec_trace!("Executing wait for read file statement");
                        let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                        let path_str = match &path_value {
                            Value::Text(s) => s.clone(),
                            _ => {
                                return Err(RuntimeError::new(
                                    format!(
                                        "Expected string for file path or handle, got {path_value:?}"
                                    ),
                                    *line,
                                    *column,
                                ));
                            }
                        };

                        let is_file_path =
                            matches!(path, Expression::Literal(Literal::String(_), _, _));

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
                                                Err(RuntimeError::new(msg, *line, *column))
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let _ = self.io_client.close_file(&handle).await;
                                        Err(RuntimeError::new(e, *line, *column))
                                    }
                                },
                                Err(e) => Err(RuntimeError::new(e, *line, *column)),
                            }
                        } else {
                            match self.io_client.read_file(&path_str).await {
                                Ok(content) => {
                                    match env
                                        .borrow_mut()
                                        .define(variable_name, Value::Text(content.into()))
                                    {
                                        Ok(_) => Ok((Value::Null, ControlFlow::None)),
                                        Err(msg) => Err(RuntimeError::new(msg, *line, *column)),
                                    }
                                }
                                Err(e) => Err(RuntimeError::new(e, *line, *column)),
                            }
                        }
                    }
                    _ => self.execute_statement(inner, Rc::clone(&env)).await,
                }
            }
            Statement::TryStatement {
                body,
                when_clauses,
                otherwise_block,
                line: _line,
                column: _column,
            } => {
                let child_env = Environment::new_child_env(&env);

                match self.execute_block(body, Rc::clone(&child_env)).await {
                    Ok(val) => Ok(val), // Success path: just bubble result
                    Err(err) => {
                        // Find matching when clause based on error kind
                        let mut executed = false;
                        let mut result = Err(err.clone());

                        for when_clause in when_clauses {
                            let matches = match &when_clause.error_type {
                                crate::parser::ast::ErrorType::General => true, // General catches all errors
                                crate::parser::ast::ErrorType::FileNotFound => {
                                    err.kind == ErrorKind::FileNotFound
                                }
                                crate::parser::ast::ErrorType::PermissionDenied => {
                                    err.kind == ErrorKind::PermissionDenied
                                }
                            };

                            if matches {
                                let _ = child_env.borrow_mut().define(
                                    &when_clause.error_name,
                                    Value::Text(err.message.into()),
                                );

                                result = self
                                    .execute_block(&when_clause.body, Rc::clone(&child_env))
                                    .await;
                                executed = true;
                                break;
                            }
                        }

                        // If no when clause matched and there's an otherwise block
                        if !executed && otherwise_block.is_some() {
                            result = self
                                .execute_block(otherwise_block.as_ref().unwrap(), child_env)
                                .await;
                        }

                        result
                    }
                }
            }
            Statement::HttpGetStatement {
                url,
                variable_name,
                line,
                column,
            } => {
                let url_val = self.evaluate_expression(url, Rc::clone(&env)).await?;
                let url_str = match &url_val {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for URL, got {url_val:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                match self.io_client.http_get(&url_str).await {
                    Ok(body) => {
                        match env
                            .borrow_mut()
                            .define(variable_name, Value::Text(body.into()))
                        {
                            Ok(_) => Ok((Value::Null, ControlFlow::None)),
                            Err(msg) => Err(RuntimeError::new(msg, *line, *column)),
                        }
                    }
                    Err(e) => Err(RuntimeError::new(e, *line, *column)),
                }
            }
            Statement::HttpPostStatement {
                url,
                data,
                variable_name,
                line,
                column,
            } => {
                let url_val = self.evaluate_expression(url, Rc::clone(&env)).await?;
                let data_val = self.evaluate_expression(data, Rc::clone(&env)).await?;

                let url_str = match &url_val {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for URL, got {url_val:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                let data_str = match &data_val {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for data, got {data_val:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                match self.io_client.http_post(&url_str, &data_str).await {
                    Ok(body) => {
                        match env
                            .borrow_mut()
                            .define(variable_name, Value::Text(body.into()))
                        {
                            Ok(_) => Ok((Value::Null, ControlFlow::None)),
                            Err(msg) => Err(RuntimeError::new(msg, *line, *column)),
                        }
                    }
                    Err(e) => Err(RuntimeError::new(e, *line, *column)),
                }
            }
            Statement::RepeatWhileLoop {
                condition,
                body,
                line: _line,
                column: _column,
            } => {
                let loop_env = Environment::new_child_env(&env);

                loop {
                    self.check_time()?;

                    let condition_value = self
                        .evaluate_expression(condition, Rc::clone(&loop_env))
                        .await?;

                    if !condition_value.is_truthy() {
                        break;
                    }

                    match self.execute_block(body, Rc::clone(&loop_env)).await {
                        Ok(_) => {}
                        Err(e) => return Err(e),
                    }
                }

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::PushStatement {
                list,
                value,
                line,
                column,
            } => {
                let list_val = self.evaluate_expression(list, Rc::clone(&env)).await?;
                let value_val = self.evaluate_expression(value, Rc::clone(&env)).await?;

                match list_val {
                    Value::List(list_rc) => {
                        list_rc.borrow_mut().push(value_val);
                        Ok((Value::Null, ControlFlow::None))
                    }
                    _ => Err(RuntimeError::new(
                        format!("Cannot push to non-list value: {list_val:?}"),
                        *line,
                        *column,
                    )),
                }
            }
            Statement::CreateListStatement {
                name,
                initial_values,
                line,
                column,
            } => {
                // Create a new list with initial values
                let mut list_items = Vec::new();
                for value_expr in initial_values {
                    let value = self
                        .evaluate_expression(value_expr, Rc::clone(&env))
                        .await?;
                    list_items.push(value);
                }

                let list_value = Value::List(Rc::new(RefCell::new(list_items)));
                match env.borrow_mut().define(name, list_value) {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                }

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::MapCreation {
                name,
                entries,
                line,
                column,
            } => {
                // Create a new map/object with initial entries
                let mut map = std::collections::HashMap::new();
                for (key, value_expr) in entries {
                    let value = self
                        .evaluate_expression(value_expr, Rc::clone(&env))
                        .await?;
                    map.insert(key.clone(), value);
                }

                let map_value = Value::Object(Rc::new(RefCell::new(map)));
                match env.borrow_mut().define(name, map_value) {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                }

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::CreateDateStatement {
                name,
                value,
                line,
                column,
            } => {
                let date_value = if let Some(expr) = value {
                    // Evaluate the expression to get the date
                    self.evaluate_expression(expr, Rc::clone(&env)).await?
                } else {
                    // Default to today's date
                    let today = chrono::Local::now().date_naive();
                    Value::Date(Rc::new(today))
                };

                match env.borrow_mut().define(name, date_value) {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                }
                Ok((Value::Null, ControlFlow::None))
            }
            Statement::CreateTimeStatement {
                name,
                value,
                line,
                column,
            } => {
                let time_value = if let Some(expr) = value {
                    // Evaluate the expression to get the time
                    self.evaluate_expression(expr, Rc::clone(&env)).await?
                } else {
                    // Default to current time
                    let now = chrono::Local::now().time();
                    Value::Time(Rc::new(now))
                };

                match env.borrow_mut().define(name, time_value) {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                }
                Ok((Value::Null, ControlFlow::None))
            }
            Statement::AddToListStatement {
                value,
                list_name,
                line,
                column,
            } => {
                // Evaluate the value to add
                let value_to_add = self.evaluate_expression(value, Rc::clone(&env)).await?;

                // Get the list from the environment
                let list_val = env.borrow().get(list_name).ok_or_else(|| {
                    RuntimeError::new(format!("Undefined variable: {list_name}"), *line, *column)
                })?;

                match list_val {
                    Value::List(list_rc) => {
                        list_rc.borrow_mut().push(value_to_add);
                        Ok((Value::Null, ControlFlow::None))
                    }
                    Value::Number(_) => {
                        // This is actually arithmetic add
                        // Convert to arithmetic operation
                        let current = list_val;
                        if let (Value::Number(n1), Value::Number(n2)) = (&current, &value_to_add) {
                            let result = Value::Number(n1 + n2);
                            env.borrow_mut()
                                .assign(list_name, result)
                                .map_err(|e| RuntimeError::new(e, *line, *column))?;
                            Ok((Value::Null, ControlFlow::None))
                        } else {
                            Err(RuntimeError::new(
                                "Cannot add non-numeric value to number".to_string(),
                                *line,
                                *column,
                            ))
                        }
                    }
                    _ => Err(RuntimeError::new(
                        format!("Cannot add to non-list value: {list_val:?}"),
                        *line,
                        *column,
                    )),
                }
            }
            Statement::RemoveFromListStatement {
                value,
                list_name,
                line,
                column,
            } => {
                // Evaluate the value to remove
                let value_to_remove = self.evaluate_expression(value, Rc::clone(&env)).await?;

                // Get the list from the environment
                let list_val = env.borrow().get(list_name).ok_or_else(|| {
                    RuntimeError::new(format!("Undefined variable: {list_name}"), *line, *column)
                })?;

                match list_val {
                    Value::List(list_rc) => {
                        let mut list = list_rc.borrow_mut();
                        // Remove the first occurrence of the value
                        if let Some(pos) = list.iter().position(|v| v == &value_to_remove) {
                            list.remove(pos);
                        }
                        Ok((Value::Null, ControlFlow::None))
                    }
                    _ => Err(RuntimeError::new(
                        format!("Cannot remove from non-list value: {list_val:?}"),
                        *line,
                        *column,
                    )),
                }
            }
            Statement::ClearListStatement {
                list_name,
                line,
                column,
            } => {
                // Get the list from the environment
                let list_val = env.borrow().get(list_name).ok_or_else(|| {
                    RuntimeError::new(format!("Undefined variable: {list_name}"), *line, *column)
                })?;

                match list_val {
                    Value::List(list_rc) => {
                        list_rc.borrow_mut().clear();
                        Ok((Value::Null, ControlFlow::None))
                    }
                    _ => Err(RuntimeError::new(
                        format!("Cannot clear non-list value: {list_val:?}"),
                        *line,
                        *column,
                    )),
                }
            }
            // Container-related statements
            Statement::ContainerDefinition {
                name,
                extends,
                implements,
                properties,
                methods,
                events,
                static_properties: _static_properties,
                static_methods: _static_methods,
                line,
                column,
            } => {
                // Create a new container definition
                let mut container_properties = HashMap::new();
                let mut container_methods = HashMap::new();

                for prop in properties {
                    let property_type_str = prop
                        .property_type
                        .as_ref()
                        .map(|ast_type| format!("{ast_type:?}"));

                    let default_val = match &prop.default_value {
                        Some(expr) => {
                            // Evaluate the default expression to get a Value
                            (self._evaluate_expression(expr, env.clone()).await).ok()
                        }
                        None => None,
                    };

                    let value_prop = value::PropertyDefinition {
                        name: prop.name.clone(),
                        property_type: property_type_str,
                        default_value: default_val,
                        validation_rules: Vec::new(),
                        is_static: false,
                        is_public: true,
                        line: prop.line,
                        column: prop.column,
                    };
                    container_properties.insert(prop.name.clone(), value_prop);
                }

                for method in methods {
                    if let Statement::ActionDefinition {
                        name,
                        parameters,
                        body,
                        line,
                        column,
                        ..
                    } = method
                    {
                        let container_method = ContainerMethodValue {
                            name: name.clone(),
                            params: parameters.iter().map(|p| p.name.clone()).collect(),
                            body: body.clone(),
                            is_static: false,
                            is_public: true,
                            env: Rc::downgrade(&env),
                            line: *line,
                            column: *column,
                        };
                        container_methods.insert(name.clone(), container_method);
                    }
                }

                // Process events
                let mut container_events = HashMap::new();
                for event in events {
                    let container_event = ContainerEventValue {
                        name: event.name.clone(),
                        params: event.parameters.iter().map(|p| p.name.clone()).collect(),
                        handlers: Vec::new(),
                        line: event.line,
                        column: event.column,
                    };
                    container_events.insert(event.name.clone(), container_event);
                }

                let container_def = ContainerDefinitionValue {
                    name: name.clone(),
                    extends: extends.clone(),
                    implements: implements.clone(),
                    properties: container_properties,
                    methods: container_methods,
                    events: container_events,
                    static_properties: HashMap::new(), // Future feature
                    static_methods: HashMap::new(),    // Future feature
                    line: *line,
                    column: *column,
                };

                // Create the container definition value
                let container_value = Value::ContainerDefinition(Rc::new(container_def));

                // Store the container definition in the environment
                match env.borrow_mut().define(name, container_value.clone()) {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                }

                Ok((container_value, ControlFlow::None))
            }
            Statement::ContainerInstantiation {
                container_type,
                instance_name,
                arguments,
                property_initializers,
                line,
                column,
            } => {
                // Create container instance with inheritance support
                let mut instance = self.create_container_instance_with_inheritance(
                    container_type,
                    &env,
                    *line,
                    *column,
                )?;

                // Process property initializers (override inherited properties)
                for initializer in property_initializers {
                    let init_value = self
                        ._evaluate_expression(&initializer.value, env.clone())
                        .await?;
                    instance
                        .properties
                        .insert(initializer.name.clone(), init_value);
                }

                let instance_value = Value::ContainerInstance(Rc::new(RefCell::new(instance)));

                // Store the instance in the environment
                match env
                    .borrow_mut()
                    .define(instance_name, instance_value.clone())
                {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                }

                // Call constructor method if arguments are provided
                if !arguments.is_empty() {
                    // Look up the container definition to find the initialize method
                    let container_def = match env.borrow().get(container_type) {
                        Some(Value::ContainerDefinition(def)) => def.clone(),
                        _ => {
                            return Err(RuntimeError::new(
                                format!("Container '{container_type}' not found"),
                                *line,
                                *column,
                            ));
                        }
                    };

                    // Check if the container has an "initialize" method
                    if let Some(init_method) = container_def.methods.get("initialize") {
                        // Create a function value from the initialize method
                        let init_function = FunctionValue {
                            name: Some("initialize".to_string()),
                            params: init_method.params.clone(),
                            body: init_method.body.clone(),
                            env: init_method.env.clone(),
                            line: init_method.line,
                            column: init_method.column,
                        };

                        // Create a new environment for the constructor execution
                        let init_env = Environment::new_child_env(&env);

                        // Add 'this' to the environment (the instance being constructed)
                        let _ = init_env.borrow_mut().define("this", instance_value.clone());

                        // Evaluate the arguments
                        let mut arg_values = Vec::with_capacity(arguments.len());
                        for arg in arguments {
                            let arg_val = self.evaluate_expression(&arg.value, env.clone()).await?;
                            arg_values.push(arg_val);
                        }

                        // Call the initialize method
                        self.call_function(&init_function, arg_values, *line, *column)
                            .await?;
                    } else if !arguments.is_empty() {
                        return Err(RuntimeError::new(
                            format!(
                                "Container '{container_type}' does not have an initialize method but arguments were provided"
                            ),
                            *line,
                            *column,
                        ));
                    }
                }

                Ok((instance_value, ControlFlow::None))
            }
            Statement::InterfaceDefinition {
                name,
                extends,
                required_actions,
                line: _line,
                column: _column,
            } => {
                // Create a new interface definition
                let mut interface_required_actions = HashMap::new();

                for action in required_actions {
                    let value_action = value::ActionSignature {
                        name: action.name.clone(),
                        params: action.parameters.iter().map(|p| p.name.clone()).collect(),
                        line: action.line,
                        column: action.column,
                    };
                    interface_required_actions.insert(action.name.clone(), value_action);
                }

                let interface_def = InterfaceDefinitionValue {
                    name: name.clone(),
                    extends: extends.clone(),
                    required_actions: interface_required_actions,
                    line: *_line,
                    column: *_column,
                };

                let interface_value = Value::InterfaceDefinition(Rc::new(interface_def));

                // Store the interface definition in the environment
                match env.borrow_mut().define(name, interface_value.clone()) {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *_line, *_column)),
                }

                Ok((interface_value, ControlFlow::None))
            }
            Statement::EventDefinition {
                name,
                parameters,
                line: _line,
                column: _column,
            } => {
                // Create a new event definition
                let event_def = ContainerEventValue {
                    name: name.clone(),
                    params: parameters.iter().map(|p| p.name.clone()).collect(),
                    handlers: Vec::new(),
                    line: *_line,
                    column: *_column,
                };

                let event_value = Value::ContainerEvent(Rc::new(event_def));

                // Store the event definition in the environment
                match env.borrow_mut().define(name, event_value.clone()) {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *_line, *_column)),
                }

                Ok((event_value, ControlFlow::None))
            }
            Statement::EventTrigger {
                name,
                arguments,
                line: _line,
                column: _column,
            } => {
                // Look up the event
                let event = match env.borrow().get(name) {
                    Some(Value::ContainerEvent(event)) => event.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Event '{name}' not found"),
                            *_line,
                            *_column,
                        ));
                    }
                };

                // Evaluate the arguments
                let mut arg_values = Vec::with_capacity(arguments.len());
                for arg in arguments {
                    let arg_val = self
                        .evaluate_expression(&arg.value, Rc::clone(&env))
                        .await?;
                    arg_values.push(arg_val);
                }

                // Execute all event handlers
                for handler in &event.handlers {
                    // Create a new environment for the handler
                    let handler_env = Environment::new_child_env(&env);

                    // Bind arguments to parameters
                    for (i, param_name) in event.params.iter().enumerate() {
                        if i < arg_values.len() {
                            let _ = handler_env
                                .borrow_mut()
                                .define(param_name, arg_values[i].clone());
                        } else {
                            let _ = handler_env.borrow_mut().define(param_name, Value::Null);
                        }
                    }

                    // Execute the handler
                    self.execute_block(&handler.body, handler_env).await?;
                }

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::EventHandler {
                event_source,
                event_name,
                handler_body,
                line: _line,
                column: _column,
            } => {
                // Evaluate the event source
                let source_val = self
                    .evaluate_expression(event_source, Rc::clone(&env))
                    .await?;

                // Check if the source is a container instance
                if let Value::ContainerInstance(instance_rc) = &source_val {
                    let instance = instance_rc.borrow();
                    let container_type = instance.container_type.clone();

                    // Look up the container definition
                    let container_def = match env.borrow().get(&container_type) {
                        Some(Value::ContainerDefinition(def)) => def.clone(),
                        _ => {
                            return Err(RuntimeError::new(
                                format!("Container '{container_type}' not found"),
                                *_line,
                                *_column,
                            ));
                        }
                    };

                    // Look up the event
                    if let Some(event) = container_def.events.get(event_name) {
                        // Create a new event handler
                        let handler = EventHandler {
                            body: handler_body.clone(),
                            env: Rc::downgrade(&env),
                            line: *_line,
                            column: *_column,
                        };

                        // Create a new event with the handler added
                        let mut handlers = event.handlers.clone();
                        handlers.push(handler);

                        // Create a new event value
                        let new_event = ContainerEventValue {
                            name: event.name.clone(),
                            params: event.params.clone(),
                            handlers,
                            line: event.line,
                            column: event.column,
                        };

                        // Store the updated event in the environment
                        let event_value = Value::ContainerEvent(Rc::new(new_event));
                        let _ = env.borrow_mut().define(event_name, event_value.clone());

                        Ok((Value::Null, ControlFlow::None))
                    } else {
                        Err(RuntimeError::new(
                            format!(
                                "Event '{event_name}' not found in container '{container_type}'"
                            ),
                            *_line,
                            *_column,
                        ))
                    }
                } else {
                    Err(RuntimeError::new(
                        "Cannot add event handler to non-container value".to_string(),
                        *_line,
                        *_column,
                    ))
                }
            }
            Statement::ParentMethodCall {
                method_name,
                arguments,
                line,
                column,
            } => {
                // Get the current container instance (this)
                let this_val = match env.borrow().get("this") {
                    Some(val) => val.clone(),
                    None => {
                        return Err(RuntimeError::new(
                            "Parent method call can only be used inside a container method"
                                .to_string(),
                            *line,
                            *column,
                        ));
                    }
                };

                // Check if this is a container instance
                if let Value::ContainerInstance(instance_rc) = &this_val {
                    let instance = instance_rc.borrow();

                    // Check if the instance has a parent
                    if let Some(parent_rc) = &instance.parent {
                        let parent = parent_rc.borrow();
                        let parent_type = parent.container_type.clone();

                        // Look up the parent container definition
                        let parent_def = match env.borrow().get(&parent_type) {
                            Some(Value::ContainerDefinition(def)) => def.clone(),
                            _ => {
                                return Err(RuntimeError::new(
                                    format!("Parent container '{parent_type}' not found"),
                                    *line,
                                    *column,
                                ));
                            }
                        };

                        // Look up the method in the parent
                        if let Some(method_val) = parent_def.methods.get(method_name) {
                            // Create a function value from the method
                            let function = FunctionValue {
                                name: Some(method_val.name.clone()),
                                params: method_val.params.clone(),
                                body: method_val.body.clone(),
                                env: method_val.env.clone(),
                                line: method_val.line,
                                column: method_val.column,
                            };

                            // Create a new environment for the method execution
                            let method_env = Environment::new_child_env(&env);

                            // Add 'this' to the environment (the current instance, not the parent)
                            let _ = method_env.borrow_mut().define("this", this_val.clone());

                            // Evaluate the arguments
                            let mut arg_values = Vec::with_capacity(arguments.len());
                            for arg in arguments {
                                let arg_val = self
                                    .evaluate_expression(&arg.value, Rc::clone(&env))
                                    .await?;
                                arg_values.push(arg_val);
                            }

                            // Call the function
                            let result = self
                                .call_function(&function, arg_values, *line, *column)
                                .await?;

                            Ok((result, ControlFlow::None))
                        } else {
                            Err(RuntimeError::new(
                                format!(
                                    "Method '{method_name}' not found in parent container '{parent_type}'"
                                ),
                                *line,
                                *column,
                            ))
                        }
                    } else {
                        Err(RuntimeError::new(
                            "Cannot call parent method: no parent container".to_string(),
                            *line,
                            *column,
                        ))
                    }
                } else {
                    Err(RuntimeError::new(
                        "Parent method call can only be used inside a container method".to_string(),
                        *line,
                        *column,
                    ))
                }
            }
            Statement::PatternDefinition {
                name,
                pattern,
                line,
                column,
                ..
            } => {
                // Compile the pattern AST into bytecode with environment access for list references
                let compiled_pattern = {
                    let env_borrow = env.borrow();
                    CompiledPattern::compile_with_env(pattern, &env_borrow)
                };
                match compiled_pattern {
                    Ok(compiled_pattern) => {
                        // Store the compiled pattern in the environment
                        let pattern_value = Value::Pattern(Rc::new(compiled_pattern));
                        match env.borrow_mut().define(name, pattern_value.clone()) {
                            Ok(_) => {}
                            Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                        }
                        Ok((pattern_value, ControlFlow::None))
                    }
                    Err(compile_error) => Err(RuntimeError {
                        kind: ErrorKind::General,
                        message: format!("Failed to compile pattern '{name}': {compile_error}"),
                        line: *line,
                        column: *column,
                    }),
                }
            }
            Statement::ListenStatement {
                port,
                server_name,
                line,
                column,
            } => {
                let port_val = self.evaluate_expression(port, Rc::clone(&env)).await?;
                let port_num = match &port_val {
                    Value::Number(n) => *n as u16,
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected number for port, got {port_val:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                // Create a basic web server using warp
                let routes = warp::path::end().map(|| "Hello from WFL Web Server!");

                // Start the server in a background task
                let server_task =
                    warp::serve(routes).try_bind_ephemeral(([127, 0, 0, 1], port_num));

                match server_task {
                    Ok((addr, server)) => {
                        // Spawn the server in the background
                        tokio::spawn(server);

                        // Create a server value with the actual address
                        let server_value = Value::Text(Rc::from(format!(
                            "WebServer::{}:{}",
                            addr.ip(),
                            addr.port()
                        )));

                        println!("Server is listening on port {}", addr.port());

                        match env.borrow_mut().define(server_name, server_value) {
                            Ok(_) => Ok((Value::Null, ControlFlow::None)),
                            Err(msg) => Err(RuntimeError::new(msg, *line, *column)),
                        }
                    }
                    Err(e) => Err(RuntimeError::new(
                        format!("Failed to start web server on port {}: {}", port_num, e),
                        *line,
                        *column,
                    )),
                }
            }
            Statement::WaitForRequestStatement {
                server,
                request_name,
                line,
                column,
            } => {
                // For now, return an error indicating this is not yet implemented
                // TODO: Implement proper request handling with warp
                Err(RuntimeError::new(
                    "Web server request handling is not yet implemented - requires async request queue".to_string(),
                    *line,
                    *column,
                ))
            }
            Statement::RespondStatement {
                request,
                content,
                status,
                content_type,
                line,
                column,
            } => {
                // For now, return an error indicating this is not yet implemented
                // TODO: Implement proper response handling with warp
                Err(RuntimeError::new(
                    "Web server response handling is not yet implemented - requires async response channel".to_string(),
                    *line,
                    *column,
                ))
            }
        };

        if self.step_mode {
            self.dump_state(stmt, line, column, &env_before);
            if !self.prompt_continue() {
                std::process::exit(0);
            }
        }

        result
    }

    async fn execute_block(
        &self,
        statements: &[Statement],
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        Box::pin(self._execute_block(statements, env)).await
    }

    async fn _execute_block(
        &self,
        statements: &[Statement],
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        self.assert_invariants();
        let mut last_value = Value::Null;

        #[cfg(debug_assertions)]
        exec_trace!("Executing block of {} statements", statements.len());

        #[cfg(debug_assertions)]
        let _guard = IndentGuard::new();

        let mut control_flow = ControlFlow::None;

        for statement in statements {
            let result = self.execute_statement(statement, Rc::clone(&env)).await?;
            last_value = result.0;
            control_flow = result.1;

            if !matches!(control_flow, ControlFlow::None) {
                #[cfg(debug_assertions)]
                exec_trace!(
                    "Block execution interrupted by control flow: {:?}",
                    control_flow
                );
                break;
            }
        }

        self.assert_invariants();
        Ok((last_value, control_flow))
    }

    async fn evaluate_expression(
        &self,
        expr: &Expression,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        #[cfg(debug_assertions)]
        exec_trace!("Evaluating expression: {}", expr_type(expr));
        Box::pin(self._evaluate_expression(expr, env)).await
    }

    async fn _evaluate_expression(
        &self,
        expr: &Expression,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        self.assert_invariants();
        self.check_time()?;

        let result = match expr {
            // Container-related expressions
            &Expression::StaticMemberAccess {
                ref container,
                ref member,
                line,
                column,
            } => {
                // Look up the container definition
                let container_def = match env.borrow().get(container) {
                    Some(Value::ContainerDefinition(def)) => def.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Container '{container}' not found"),
                            line,
                            column,
                        ));
                    }
                };

                // Look up the static member
                if let Some(value) = container_def.static_properties.get(member) {
                    Ok(value.clone())
                } else if let Some(method) = container_def.static_methods.get(member) {
                    // Create a function value from the method
                    let function = FunctionValue {
                        name: Some(method.name.clone()),
                        params: method.params.clone(),
                        body: method.body.clone(),
                        env: method.env.clone(),
                        line: method.line,
                        column: method.column,
                    };

                    Ok(Value::Function(Rc::new(function)))
                } else {
                    Err(RuntimeError::new(
                        format!("Static member '{member}' not found in container '{container}'"),
                        line,
                        column,
                    ))
                }
            }

            &Expression::MethodCall {
                ref object,
                ref method,
                ref arguments,
                line,
                column,
            } => {
                // Evaluate the object
                let object_val = self.evaluate_expression(object, Rc::clone(&env)).await?;

                // Clone the object value to avoid borrow issues
                let object_val_clone = object_val.clone();

                // Check if the object is a container instance
                if let Value::ContainerInstance(instance_rc) = &object_val_clone {
                    let instance = instance_rc.borrow();
                    let container_type = instance.container_type.clone();

                    // Look up the container definition
                    let container_def = match env.borrow().get(&container_type) {
                        Some(Value::ContainerDefinition(def)) => def.clone(),
                        _ => {
                            return Err(RuntimeError::new(
                                format!("Container '{container_type}' not found"),
                                line,
                                column,
                            ));
                        }
                    };

                    // Look up the method (with inheritance support)
                    let mut found_method = container_def.methods.get(method).cloned();
                    let mut current_container_name = container_type.clone();

                    // If method not found, check parent containers
                    while found_method.is_none() {
                        if let Some(Value::ContainerDefinition(def)) =
                            env.borrow().get(&current_container_name)
                        {
                            if let Some(parent_name) = &def.extends {
                                current_container_name = parent_name.clone();
                                if let Some(Value::ContainerDefinition(parent_def)) =
                                    env.borrow().get(parent_name)
                                {
                                    found_method = parent_def.methods.get(method).cloned();
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    if let Some(method_val) = found_method {
                        // Create a function value from the method
                        let function = FunctionValue {
                            name: Some(method_val.name.clone()),
                            params: method_val.params.clone(),
                            body: method_val.body.clone(),
                            env: method_val.env.clone(),
                            line: method_val.line,
                            column: method_val.column,
                        };

                        // Create a new environment for the method execution
                        let method_env = Environment::new_child_env(&env);

                        // Add 'this' to the environment
                        let _ = method_env.borrow_mut().define("this", object_val.clone());

                        // Add container properties and events as accessible variables
                        if let Value::ContainerInstance(instance_rc) = &object_val_clone {
                            let instance = instance_rc.borrow();

                            // Add properties
                            for (prop_name, prop_value) in &instance.properties {
                                let _ = method_env
                                    .borrow_mut()
                                    .define(prop_name, prop_value.clone());
                            }

                            // Add events from the container definition
                            if let Some(Value::ContainerDefinition(container_def_rc)) =
                                env.borrow().get(&instance.container_type)
                            {
                                let container_def = container_def_rc.clone();
                                for (event_name, event_value) in &container_def.events {
                                    let _ = method_env.borrow_mut().define(
                                        event_name,
                                        Value::ContainerEvent(Rc::new(event_value.clone())),
                                    );
                                }
                            }
                        }

                        // Evaluate the arguments
                        let mut arg_values = Vec::with_capacity(arguments.len());
                        for arg in arguments {
                            let arg_val = self
                                .evaluate_expression(&arg.value, Rc::clone(&env))
                                .await?;
                            arg_values.push(arg_val);
                        }

                        // Create a modified function with the method environment
                        let method_function = FunctionValue {
                            name: function.name.clone(),
                            params: function.params.clone(),
                            body: function.body.clone(),
                            env: Rc::downgrade(&method_env),
                            line: function.line,
                            column: function.column,
                        };

                        // Call the function with the method environment
                        let result = self
                            .call_function(&method_function, arg_values, line, column)
                            .await?;

                        Ok(result)
                    } else {
                        Err(RuntimeError::new(
                            format!("Method '{method}' not found in container '{container_type}'"),
                            line,
                            column,
                        ))
                    }
                } else {
                    Err(RuntimeError::new(
                        format!("Cannot call method '{method}' on non-container value"),
                        line,
                        column,
                    ))
                }
            }
            &Expression::AwaitExpression {
                ref expression,
                line: _line,
                column: _column,
            } => {
                let value = self
                    .evaluate_expression(expression, Rc::clone(&env))
                    .await?;
                Ok(value)
            }
            Expression::Literal(literal, _line, _column) => match literal {
                Literal::String(s) => Ok(Value::Text(Rc::from(s.as_str()))),
                Literal::Integer(i) => Ok(Value::Number(*i as f64)),
                Literal::Float(f) => Ok(Value::Number(*f)),
                Literal::Boolean(b) => Ok(Value::Bool(*b)),
                Literal::Nothing => Ok(Value::Null),
                Literal::Pattern(_ir_string) => {
                    // TODO: Update to use new pattern system
                    Err(RuntimeError::new(
                        "Pattern literals not yet supported in new pattern system".to_string(),
                        *_line,
                        *_column,
                    ))
                }
                Literal::List(elements) => {
                    let mut list_values = Vec::new();
                    for element in elements {
                        // Use Box::pin to handle recursion in async fn
                        let future = Box::pin(self._evaluate_expression(element, Rc::clone(&env)));
                        let value = future.await?;
                        list_values.push(value);
                    }
                    Ok(Value::List(Rc::new(RefCell::new(list_values))))
                }
            },

            Expression::Variable(name, line, column) => {
                // Handle special count variable inside count loops
                if name == "count" && *self.in_count_loop.borrow() {
                    if let Some(count_value) = *self.current_count.borrow() {
                        return Ok(Value::Number(count_value));
                    }
                    // If we're in a count loop but don't have a current count, this is an error
                    return Err(RuntimeError::new(
                        "Internal error: count variable accessed in count loop but no current count set".to_string(),
                        *line,
                        *column,
                    ));
                }

                // Try normal variable lookup first (allows user-defined 'count' variables outside loops)
                if let Some(value) = env.borrow().get(name) {
                    // Check if this is a zero-argument native function that should be auto-called
                    match &value {
                        Value::NativeFunction(func_name, native_fn) => {
                            if get_function_arity(func_name) == 0 {
                                // Auto-call zero-argument functions when referenced as variables
                                native_fn(vec![]).map_err(|e| {
                                    RuntimeError::new(
                                        format!("Error in native function '{}': {}", func_name, e),
                                        *line,
                                        *column,
                                    )
                                })
                            } else {
                                // Return function object for functions with arguments
                                Ok(value)
                            }
                        }
                        _ => Ok(value),
                    }
                } else if name == "count" {
                    // If 'count' is not found and we're not in a count loop, provide helpful error
                    Err(RuntimeError::new(
                        "Variable 'count' can only be used inside count loops. Use 'count from X to Y:' to create a count loop.".to_string(),
                        *line,
                        *column,
                    ))
                } else {
                    Err(RuntimeError::new(
                        format!("Undefined variable '{name}'"),
                        *line,
                        *column,
                    ))
                }
            }

            Expression::BinaryOperation {
                left,
                operator,
                right,
                line,
                column,
            } => {
                // Use Box::pin to handle recursion in async fn
                let left_future = Box::pin(self.evaluate_expression(left, Rc::clone(&env)));
                let left_val = left_future.await?;

                let right_val = self.evaluate_expression(right, Rc::clone(&env)).await?;

                match operator {
                    Operator::Plus => self.add(left_val, right_val, *line, *column),
                    Operator::Minus => self.subtract(left_val, right_val, *line, *column),
                    Operator::Multiply => self.multiply(left_val, right_val, *line, *column),
                    Operator::Divide => self.divide(left_val, right_val, *line, *column),
                    Operator::Equals => Ok(Value::Bool(self.is_equal(&left_val, &right_val))),
                    Operator::NotEquals => Ok(Value::Bool(!self.is_equal(&left_val, &right_val))),
                    Operator::GreaterThan => self.greater_than(left_val, right_val, *line, *column),
                    Operator::LessThan => self.less_than(left_val, right_val, *line, *column),
                    Operator::GreaterThanOrEqual => {
                        self.greater_than_equal(left_val, right_val, *line, *column)
                    }
                    Operator::LessThanOrEqual => {
                        self.less_than_equal(left_val, right_val, *line, *column)
                    }
                    Operator::And => Ok(Value::Bool(left_val.is_truthy() && right_val.is_truthy())),
                    Operator::Or => Ok(Value::Bool(left_val.is_truthy() || right_val.is_truthy())),
                    Operator::Contains => self.contains(left_val, right_val, *line, *column),
                }
            }

            Expression::UnaryOperation {
                operator,
                expression,
                line,
                column,
            } => {
                let value = self
                    .evaluate_expression(expression, Rc::clone(&env))
                    .await?;

                match operator {
                    UnaryOperator::Not => Ok(Value::Bool(!value.is_truthy())),
                    UnaryOperator::Minus => match value {
                        Value::Number(n) => Ok(Value::Number(-n)),
                        _ => Err(RuntimeError::new(
                            format!("Cannot negate {}", value.type_name()),
                            *line,
                            *column,
                        )),
                    },
                }
            }

            Expression::FunctionCall {
                function,
                arguments,
                line,
                column,
            } => {
                let function_val = self.evaluate_expression(function, Rc::clone(&env)).await?;

                let mut arg_values = Vec::new();
                for arg in arguments {
                    arg_values.push(
                        self.evaluate_expression(&arg.value, Rc::clone(&env))
                            .await?,
                    );
                }

                #[cfg(debug_assertions)]
                let func_name = match &function_val {
                    Value::Function(f) => {
                        f.name.clone().unwrap_or_else(|| "<anonymous>".to_string())
                    }
                    _ => format!("{function_val:?}"),
                };

                #[cfg(debug_assertions)]
                exec_function_call!(&func_name, &arg_values);

                let result = match function_val {
                    Value::Function(func) => {
                        self.call_function(&func, arg_values, *line, *column).await
                    }
                    Value::NativeFunction(_, native_fn) => {
                        native_fn(arg_values.clone()).map_err(|e| {
                            RuntimeError::new(
                                format!("Error in native function: {e}"),
                                *line,
                                *column,
                            )
                        })
                    }
                    _ => Err(RuntimeError::new(
                        format!("Cannot call {}", function_val.type_name()),
                        *line,
                        *column,
                    )),
                };

                #[cfg(debug_assertions)]
                if let Ok(ref val) = result {
                    exec_function_return!(&func_name, val);
                }

                result
            }

            Expression::ActionCall {
                name,
                arguments,
                line,
                column,
            } => {
                let function_val = env.borrow().get(name).ok_or_else(|| {
                    RuntimeError::new(format!("Undefined action '{name}'"), *line, *column)
                })?;

                match function_val {
                    Value::Function(func) => {
                        let mut arg_values = Vec::new();
                        for arg in arguments.iter() {
                            arg_values.push(
                                self.evaluate_expression(&arg.value, Rc::clone(&env))
                                    .await?,
                            );
                        }

                        #[cfg(debug_assertions)]
                        let func_name = func
                            .name
                            .clone()
                            .unwrap_or_else(|| "<anonymous>".to_string());

                        #[cfg(debug_assertions)]
                        exec_function_call!(&func_name, &arg_values);

                        let result = self.call_function(&func, arg_values, *line, *column).await;

                        #[cfg(debug_assertions)]
                        if let Ok(ref val) = result {
                            exec_function_return!(&func_name, val);
                        }

                        result
                    }
                    _ => Err(RuntimeError::new(
                        format!("'{name}' is not callable"),
                        *line,
                        *column,
                    )),
                }
            }

            Expression::MemberAccess {
                object,
                property,
                line,
                column,
            } => {
                let object_val = self.evaluate_expression(object, Rc::clone(&env)).await?;

                match object_val {
                    Value::Object(obj_rc) => {
                        let obj = obj_rc.borrow();
                        if let Some(value) = obj.get(property) {
                            Ok(value.clone())
                        } else {
                            Err(RuntimeError::new(
                                format!("Object has no property '{property}'"),
                                *line,
                                *column,
                            ))
                        }
                    }
                    _ => Err(RuntimeError::new(
                        format!("Cannot access property of {}", object_val.type_name()),
                        *line,
                        *column,
                    )),
                }
            }

            Expression::IndexAccess {
                collection,
                index,
                line,
                column,
            } => {
                let collection_val = self
                    .evaluate_expression(collection, Rc::clone(&env))
                    .await?;
                let index_val = self.evaluate_expression(index, Rc::clone(&env)).await?;

                match (collection_val, index_val) {
                    (Value::List(list_rc), Value::Number(idx)) => {
                        let list = list_rc.borrow();
                        let idx = idx as usize;

                        if idx < list.len() {
                            Ok(list[idx].clone())
                        } else {
                            Err(RuntimeError::new(
                                format!(
                                    "Index {} out of bounds for list of length {}",
                                    idx,
                                    list.len()
                                ),
                                *line,
                                *column,
                            ))
                        }
                    }
                    (Value::Object(obj_rc), Value::Text(key)) => {
                        let obj = obj_rc.borrow();
                        let key_str = key.to_string();

                        if let Some(value) = obj.get(&key_str) {
                            Ok(value.clone())
                        } else {
                            Err(RuntimeError::new(
                                format!("Object has no key '{key_str}'"),
                                *line,
                                *column,
                            ))
                        }
                    }
                    (collection, index) => Err(RuntimeError::new(
                        format!(
                            "Cannot index {} with {}",
                            collection.type_name(),
                            index.type_name()
                        ),
                        *line,
                        *column,
                    )),
                }
            }

            Expression::Concatenation {
                left,
                right,
                line: _line,
                column: _column,
            } => {
                // Use Box::pin to handle recursion in async fn
                let left_future = Box::pin(self.evaluate_expression(left, Rc::clone(&env)));
                let left_val = left_future.await?;

                let right_val = self.evaluate_expression(right, Rc::clone(&env)).await?;

                let result = format!("{left_val}{right_val}");
                Ok(Value::Text(Rc::from(result.as_str())))
            }

            Expression::PatternMatch {
                text,
                pattern,
                line: _line,
                column: _column,
            } => {
                let text_val = self.evaluate_expression(text, Rc::clone(&env)).await?;
                let pattern_val = self.evaluate_expression(pattern, Rc::clone(&env)).await?;

                // Extract text string
                let text_str = match &text_val {
                    Value::Text(s) => s.as_ref(),
                    _ => {
                        return Err(RuntimeError::new(
                            "Pattern match requires text as first argument".to_string(),
                            *_line,
                            *_column,
                        ));
                    }
                };

                // Extract compiled pattern
                let compiled_pattern = match &pattern_val {
                    Value::Pattern(p) => p,
                    _ => {
                        return Err(RuntimeError::new(
                            "Pattern match requires pattern as second argument".to_string(),
                            *_line,
                            *_column,
                        ));
                    }
                };

                // Perform the match
                let matches = compiled_pattern.matches(text_str);
                Ok(Value::Bool(matches))
            }

            Expression::PatternFind {
                text,
                pattern,
                line: _line,
                column: _column,
            } => {
                let text_val = self.evaluate_expression(text, Rc::clone(&env)).await?;
                let pattern_val = self.evaluate_expression(pattern, Rc::clone(&env)).await?;

                // Extract text string
                let text_str = match &text_val {
                    Value::Text(s) => s.as_ref(),
                    _ => {
                        return Err(RuntimeError::new(
                            "Pattern find requires text as first argument".to_string(),
                            *_line,
                            *_column,
                        ));
                    }
                };

                // Extract compiled pattern
                let compiled_pattern = match &pattern_val {
                    Value::Pattern(p) => p,
                    _ => {
                        return Err(RuntimeError::new(
                            "Pattern find requires pattern as second argument".to_string(),
                            *_line,
                            *_column,
                        ));
                    }
                };

                // Find the first match
                match compiled_pattern.find(text_str) {
                    Some(match_result) => {
                        // Return an object with match information
                        let mut result_map = std::collections::HashMap::new();
                        result_map.insert(
                            "matched_text".to_string(),
                            Value::Text(Rc::from(match_result.matched_text.as_str())),
                        );
                        result_map.insert(
                            "start".to_string(),
                            Value::Number(match_result.start as f64),
                        );
                        result_map
                            .insert("end".to_string(), Value::Number(match_result.end as f64));

                        // Add captures if any
                        if !match_result.captures.is_empty() {
                            let mut captures_map = std::collections::HashMap::new();
                            for (name, value) in match_result.captures {
                                captures_map.insert(name, Value::Text(Rc::from(value.as_str())));
                            }
                            result_map.insert(
                                "captures".to_string(),
                                Value::Object(Rc::new(RefCell::new(captures_map))),
                            );
                        }

                        Ok(Value::Object(Rc::new(RefCell::new(result_map))))
                    }
                    None => Ok(Value::Null),
                }
            }

            Expression::PatternReplace {
                text,
                pattern,
                replacement,
                line: _line,
                column: _column,
            } => {
                let text_val = self.evaluate_expression(text, Rc::clone(&env)).await?;
                let pattern_val = self.evaluate_expression(pattern, Rc::clone(&env)).await?;
                let replacement_val = self
                    .evaluate_expression(replacement, Rc::clone(&env))
                    .await?;

                let args = vec![text_val, pattern_val, replacement_val]; // Note: text, pattern, then replacement
                crate::stdlib::pattern::native_pattern_replace(args, *_line, *_column)
            }

            Expression::PatternSplit {
                text,
                pattern,
                line: _line,
                column: _column,
            } => {
                let text_val = self.evaluate_expression(text, Rc::clone(&env)).await?;
                let pattern_val = self.evaluate_expression(pattern, Rc::clone(&env)).await?;

                let args = vec![text_val, pattern_val];
                crate::stdlib::pattern::native_pattern_split(args, *_line, *_column)
            }
            Expression::StringSplit {
                text,
                delimiter,
                line: _line,
                column: _column,
            } => {
                let text_val = self.evaluate_expression(text, Rc::clone(&env)).await?;
                let delimiter_val = self.evaluate_expression(delimiter, Rc::clone(&env)).await?;

                // Validate types
                if !matches!(text_val, Value::Text(_)) {
                    return Err(RuntimeError::new(
                        format!("Cannot split {} - expected text", text_val.type_name()),
                        *_line,
                        *_column,
                    ));
                }
                if !matches!(delimiter_val, Value::Text(_)) {
                    return Err(RuntimeError::new(
                        format!("Delimiter must be text - got {}", delimiter_val.type_name()),
                        *_line,
                        *_column,
                    ));
                }

                let args = vec![text_val, delimiter_val];
                crate::stdlib::text::native_string_split(args)
            }
            Expression::PropertyAccess {
                object,
                property,
                line,
                column,
            } => {
                let obj_value = self.evaluate_expression(object, Rc::clone(&env)).await?;
                match obj_value {
                    Value::ContainerInstance(instance) => {
                        let instance_ref = instance.borrow();
                        if let Some(prop_value) = instance_ref.properties.get(property) {
                            Ok(prop_value.clone())
                        } else {
                            Err(RuntimeError::new(
                                format!("Property '{property}' not found"),
                                *line,
                                *column,
                            ))
                        }
                    }
                    _ => Err(RuntimeError::new(
                        format!("Cannot access property '{property}' on non-container value"),
                        *line,
                        *column,
                    )),
                }
            }
            Expression::FileExists { path, line, column } => {
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str = match &path_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for file path, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                Ok(Value::Bool(tokio::fs::metadata(&*path_str).await.is_ok()))
            }
            Expression::DirectoryExists { path, line, column } => {
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str = match &path_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for directory path, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                match tokio::fs::metadata(&*path_str).await {
                    Ok(metadata) => Ok(Value::Bool(metadata.is_dir())),
                    Err(_) => Ok(Value::Bool(false)),
                }
            }
            Expression::ListFiles { path, line, column } => {
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str = match &path_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for directory path, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                match tokio::fs::read_dir(&*path_str).await {
                    Ok(mut entries) => {
                        let mut files = Vec::new();
                        while let Ok(Some(entry)) = entries.next_entry().await {
                            if let Ok(file_name) = entry.file_name().into_string() {
                                files.push(Value::Text(file_name.into()));
                            }
                        }
                        Ok(Value::List(Rc::new(RefCell::new(files))))
                    }
                    Err(e) => Err(RuntimeError::new(
                        format!("Failed to list files in directory: {e}"),
                        *line,
                        *column,
                    )),
                }
            }
            Expression::ReadContent {
                file_handle,
                line,
                column,
            } => {
                let handle_value = self
                    .evaluate_expression(file_handle, Rc::clone(&env))
                    .await?;
                let handle_str = match &handle_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for file handle, got {handle_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                match self.io_client.read_file(&handle_str).await {
                    Ok(content) => Ok(Value::Text(content.into())),
                    Err(e) => Err(RuntimeError::new(e, *line, *column)),
                }
            }
            Expression::ListFilesRecursive {
                path,
                extensions,
                line,
                column,
            } => {
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str = match &path_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for directory path, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                // Evaluate extensions if provided
                let ext_filters = if let Some(ext_exprs) = extensions {
                    let mut filters = Vec::new();
                    for ext_expr in ext_exprs {
                        let ext_value = self.evaluate_expression(ext_expr, Rc::clone(&env)).await?;
                        match &ext_value {
                            Value::Text(s) => filters.push(s.to_string()),
                            Value::List(list) => {
                                // If we get a list, extract all string values from it
                                let list_ref = list.borrow();
                                for item in list_ref.iter() {
                                    match item {
                                        Value::Text(s) => filters.push(s.to_string()),
                                        _ => {
                                            return Err(RuntimeError::new(
                                                format!(
                                                    "Expected string in extension list, got {item:?}"
                                                ),
                                                *line,
                                                *column,
                                            ));
                                        }
                                    }
                                }
                            }
                            _ => {
                                return Err(RuntimeError::new(
                                    format!(
                                        "Expected string or list for extension, got {ext_value:?}"
                                    ),
                                    *line,
                                    *column,
                                ));
                            }
                        }
                    }
                    Some(filters)
                } else {
                    None
                };

                // Perform recursive directory listing
                match self.list_files_recursive(&path_str, ext_filters).await {
                    Ok(files) => Ok(Value::List(Rc::new(RefCell::new(files)))),
                    Err(e) => Err(RuntimeError::new(
                        format!("Failed to list files recursively: {e}"),
                        *line,
                        *column,
                    )),
                }
            }
            Expression::ListFilesFiltered {
                path,
                extensions,
                line,
                column,
            } => {
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str = match &path_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for directory path, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                // Evaluate extensions
                let mut ext_filters = Vec::new();
                for ext_expr in extensions {
                    let ext_value = self.evaluate_expression(ext_expr, Rc::clone(&env)).await?;
                    match &ext_value {
                        Value::Text(s) => ext_filters.push(s.to_string()),
                        Value::List(list) => {
                            // If we get a list, extract all string values from it
                            let list_ref = list.borrow();
                            for item in list_ref.iter() {
                                match item {
                                    Value::Text(s) => ext_filters.push(s.to_string()),
                                    _ => {
                                        return Err(RuntimeError::new(
                                            format!(
                                                "Expected string in extension list, got {item:?}"
                                            ),
                                            *line,
                                            *column,
                                        ));
                                    }
                                }
                            }
                        }
                        _ => {
                            return Err(RuntimeError::new(
                                format!("Expected string or list for extension, got {ext_value:?}"),
                                *line,
                                *column,
                            ));
                        }
                    }
                }

                // List files with filtering
                match self.list_files_filtered(&path_str, ext_filters).await {
                    Ok(files) => Ok(Value::List(Rc::new(RefCell::new(files)))),
                    Err(e) => Err(RuntimeError::new(
                        format!("Failed to list files with filter: {e}"),
                        *line,
                        *column,
                    )),
                }
            }
        };
        self.assert_invariants();
        result
    }

    async fn call_function(
        &self,
        func: &FunctionValue,
        args: Vec<Value>,
        line: usize,
        column: usize,
    ) -> Result<Value, RuntimeError> {
        #[cfg(feature = "dhat-ad-hoc")]
        dhat::ad_hoc_event(1);

        #[cfg(debug_assertions)]
        let func_name = func
            .name
            .clone()
            .unwrap_or_else(|| "<anonymous>".to_string());

        if args.len() != func.params.len() {
            return Err(RuntimeError::new(
                format!(
                    "Expected {} arguments but got {}",
                    func.params.len(),
                    args.len()
                ),
                line,
                column,
            ));
        }

        let func_env = match func.env.upgrade() {
            Some(env) => {
                exec_trace!("call_function - Successfully upgraded function environment");
                env
            }
            None => {
                exec_trace!("call_function - Failed to upgrade function environment");
                return Err(RuntimeError::new(
                    "Environment no longer exists".to_string(),
                    line,
                    column,
                ));
            }
        };

        let call_env = Environment::new_child_env(&func_env);
        exec_trace!("call_function - Created child environment for function call");

        for (_i, (param, arg)) in func.params.iter().zip(args.clone()).enumerate() {
            exec_trace!(
                "call_function - Binding parameter {} '{}' to argument {:?}",
                _i,
                param,
                arg
            );

            #[cfg(debug_assertions)]
            exec_var_declare!(param, &arg);
            let _ = call_env.borrow_mut().define(param, arg.clone());
        }

        let frame = CallFrame::new(
            func.name
                .clone()
                .unwrap_or_else(|| "<anonymous>".to_string()),
            line,
            column,
        );
        self.call_stack.borrow_mut().push(frame);
        exec_trace!("call_function - Pushed frame to call stack");

        #[cfg(debug_assertions)]
        exec_block_enter!(format!("function {}", func_name));

        #[cfg(debug_assertions)]
        let _guard = IndentGuard::new();

        exec_trace!("call_function - Executing function body");
        let result = self.execute_block(&func.body, call_env.clone()).await;
        exec_trace!("call_function - Function execution result: {:?}", result);

        #[cfg(debug_assertions)]
        exec_block_exit!(format!("function {}", func_name));

        match result {
            Ok((value, control_flow)) => {
                self.call_stack.borrow_mut().pop();

                let return_value = match control_flow {
                    ControlFlow::Return(val) => {
                        exec_trace!(
                            "call_function - Function explicitly returned with value: {:?}",
                            val
                        );
                        val
                    }
                    _ => {
                        exec_trace!("call_function - Function completed with value: {:?}", value);
                        value
                    }
                };

                exec_trace!(
                    "call_function - Function returned successfully with value: {:?}",
                    return_value
                );
                Ok(return_value)
            }
            Err(err) => {
                exec_trace!(
                    "call_function - Function execution failed with error: {:?}",
                    err
                );
                if let Some(last_frame) = self.call_stack.borrow_mut().last_mut() {
                    last_frame.capture_locals(&call_env);
                }

                let error_with_stack = err.clone();

                self.call_stack.borrow_mut().pop();

                Err(error_with_stack)
            }
        }
    }

    fn add(
        &self,
        left: Value,
        right: Value,
        line: usize,
        column: usize,
    ) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
            (Value::Text(a), Value::Text(b)) => {
                let result = format!("{a}{b}");
                Ok(Value::Text(Rc::from(result.as_str())))
            }
            (Value::Text(a), b) => {
                let result = format!("{a}{b}");
                Ok(Value::Text(Rc::from(result.as_str())))
            }
            (a, Value::Text(b)) => {
                let result = format!("{a}{b}");
                Ok(Value::Text(Rc::from(result.as_str())))
            }
            (a, b) => Err(RuntimeError::new(
                format!("Cannot add {} and {}", a.type_name(), b.type_name()),
                line,
                column,
            )),
        }
    }

    fn subtract(
        &self,
        left: Value,
        right: Value,
        line: usize,
        column: usize,
    ) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a - b)),
            (a, b) => Err(RuntimeError::new(
                format!("Cannot subtract {} from {}", b.type_name(), a.type_name()),
                line,
                column,
            )),
        }
    }

    fn multiply(
        &self,
        left: Value,
        right: Value,
        line: usize,
        column: usize,
    ) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a * b)),
            (a, b) => Err(RuntimeError::new(
                format!("Cannot multiply {} and {}", a.type_name(), b.type_name()),
                line,
                column,
            )),
        }
    }

    fn divide(
        &self,
        left: Value,
        right: Value,
        line: usize,
        column: usize,
    ) -> Result<Value, RuntimeError> {
        #[cfg(feature = "dhat-ad-hoc")]
        dhat::ad_hoc_event(1); // Track division operations for memory profiling

        match (left, right) {
            (Value::Number(a), Value::Number(b)) => {
                if b == 0.0 {
                    Err(RuntimeError::new(
                        "Division by zero".to_string(),
                        line,
                        column,
                    ))
                } else {
                    // Calculate the result of the division operation
                    let result = a / b;

                    // Check if the result is valid (not NaN or infinite)
                    if !result.is_finite() {
                        return Err(RuntimeError::new(
                            format!("Division resulted in invalid number: {result}"),
                            line,
                            column,
                        ));
                    }

                    // Return the valid result as a Value::Number
                    Ok(Value::Number(result))
                }
            }
            (a, b) => Err(RuntimeError::new(
                format!("Cannot divide {} by {}", a.type_name(), b.type_name()),
                line,
                column,
            )),
        }
    }

    fn is_equal(&self, left: &Value, right: &Value) -> bool {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => (a - b).abs() < f64::EPSILON,
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Null, Value::Null) => true,
            _ => false,
        }
    }

    // Helper method to create container instance with inheritance
    #[allow(clippy::only_used_in_recursion)]
    fn create_container_instance_with_inheritance(
        &self,
        container_type: &str,
        env: &Rc<RefCell<Environment>>,
        line: usize,
        column: usize,
    ) -> Result<ContainerInstanceValue, RuntimeError> {
        // Look up the container definition
        let container_def = match env.borrow().get(container_type) {
            Some(Value::ContainerDefinition(def)) => def.clone(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Container '{container_type}' not found"),
                    line,
                    column,
                ));
            }
        };

        // Create parent instance if container extends another
        let parent_instance = if let Some(parent_type) = &container_def.extends {
            // Recursively create parent instance
            let parent =
                self.create_container_instance_with_inheritance(parent_type, env, line, column)?;
            Some(Rc::new(RefCell::new(parent)))
        } else {
            None
        };

        // Create instance with inherited properties
        let mut instance_properties = HashMap::new();

        // Copy properties from parent if exists
        if let Some(ref parent) = parent_instance {
            for (key, value) in &parent.borrow().properties {
                instance_properties.insert(key.clone(), value.clone());
            }
        }

        // Initialize properties with default values from container definition
        for (prop_name, prop_def) in &container_def.properties {
            if let Some(default_value) = &prop_def.default_value {
                instance_properties.insert(prop_name.clone(), default_value.clone());
            }
        }

        Ok(ContainerInstanceValue {
            container_type: container_type.to_string(),
            properties: instance_properties,
            parent: parent_instance,
            line,
            column,
        })
    }

    fn greater_than(
        &self,
        left: Value,
        right: Value,
        line: usize,
        column: usize,
    ) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a > b)),
            (Value::Text(a), Value::Text(b)) => Ok(Value::Bool(a > b)),
            (a, b) => Err(RuntimeError::new(
                format!(
                    "Cannot compare {} and {} with >",
                    a.type_name(),
                    b.type_name()
                ),
                line,
                column,
            )),
        }
    }

    fn less_than(
        &self,
        left: Value,
        right: Value,
        line: usize,
        column: usize,
    ) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a < b)),
            (Value::Text(a), Value::Text(b)) => Ok(Value::Bool(a < b)),
            (a, b) => Err(RuntimeError::new(
                format!(
                    "Cannot compare {} and {} with <",
                    a.type_name(),
                    b.type_name()
                ),
                line,
                column,
            )),
        }
    }

    fn greater_than_equal(
        &self,
        left: Value,
        right: Value,
        line: usize,
        column: usize,
    ) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a >= b)),
            (Value::Text(a), Value::Text(b)) => Ok(Value::Bool(a >= b)),
            (a, b) => Err(RuntimeError::new(
                format!(
                    "Cannot compare {} and {} with >=",
                    a.type_name(),
                    b.type_name()
                ),
                line,
                column,
            )),
        }
    }

    fn less_than_equal(
        &self,
        left: Value,
        right: Value,
        line: usize,
        column: usize,
    ) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a <= b)),
            (Value::Text(a), Value::Text(b)) => Ok(Value::Bool(a <= b)),
            (a, b) => Err(RuntimeError::new(
                format!(
                    "Cannot compare {} and {} with <=",
                    a.type_name(),
                    b.type_name()
                ),
                line,
                column,
            )),
        }
    }

    fn contains(
        &self,
        left: Value,
        right: Value,
        line: usize,
        column: usize,
    ) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::List(list_rc), item) => {
                let list = list_rc.borrow();
                for value in list.iter() {
                    if self.is_equal(value, &item) {
                        return Ok(Value::Bool(true));
                    }
                }
                Ok(Value::Bool(false))
            }
            (Value::Object(obj_rc), Value::Text(key)) => {
                let obj = obj_rc.borrow();
                Ok(Value::Bool(obj.contains_key(&key.to_string())))
            }
            (Value::Text(text), Value::Text(substring)) => {
                Ok(Value::Bool(text.contains(&*substring)))
            }
            (a, b) => Err(RuntimeError::new(
                format!(
                    "Cannot check if {} contains {}",
                    a.type_name(),
                    b.type_name()
                ),
                line,
                column,
            )),
        }
    }

    async fn list_files_recursive(
        &self,
        path: &str,
        extensions: Option<Vec<String>>,
    ) -> Result<Vec<Value>, std::io::Error> {
        use tokio::fs;

        let mut files = Vec::new();
        let mut dirs_to_process = vec![path.to_string()];

        while let Some(current_dir) = dirs_to_process.pop() {
            let mut entries = fs::read_dir(&current_dir).await?;

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let path_str = path.to_string_lossy().to_string();

                if path.is_dir() {
                    dirs_to_process.push(path_str);
                } else if path.is_file() {
                    // Check extension filter if provided
                    if let Some(ref exts) = extensions {
                        let file_ext = path
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| format!(".{ext}"));

                        if let Some(ext) = file_ext
                            && exts.iter().any(|e| e == &ext)
                        {
                            files.push(Value::Text(path_str.into()));
                        }
                    } else {
                        files.push(Value::Text(path_str.into()));
                    }
                }
            }
        }

        Ok(files)
    }

    async fn list_files_filtered(
        &self,
        path: &str,
        extensions: Vec<String>,
    ) -> Result<Vec<Value>, std::io::Error> {
        use tokio::fs;

        let mut files = Vec::new();
        let mut entries = fs::read_dir(path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_file() {
                let path_str = path.to_string_lossy().to_string();

                // Check extension filter
                let file_ext = path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| format!(".{ext}"));

                if let Some(ext) = file_ext
                    && extensions.iter().any(|e| e == &ext)
                {
                    files.push(Value::Text(path_str.into()));
                }
            }
        }

        Ok(files)
    }
}
