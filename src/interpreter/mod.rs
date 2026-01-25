#![allow(clippy::await_holding_refcell_ref)]
mod assertion_helpers;
pub mod bounded_buffer;
pub mod command_sanitizer;
pub mod control_flow;
pub mod environment;
pub mod error;
pub mod expr;
pub mod io_client;
#[cfg(test)]
mod memory_tests;
pub mod stmt;
pub mod test_results;
#[cfg(test)]
mod tests;
pub mod value;
pub mod web;

use self::control_flow::ControlFlow;
#[allow(unused_imports)]
use self::expr::containers::ContainerExpressionEvaluator;
use self::expr::core::CoreExpressionEvaluator;
use self::expr::io::IoExpressionEvaluator;
use self::expr::patterns::PatternExpressionEvaluator;
use self::expr::time::TimeExpressionEvaluator;
use self::expr::web::WebExpressionEvaluator;
use self::io_client::IoClient;
pub(crate) use self::stmt::containers::ContainerExecutor;
pub(crate) use self::stmt::control_flow::ControlFlowExecutor;
pub(crate) use self::stmt::definitions::DefinitionsExecutor;
pub(crate) use self::stmt::io::IoExecutor;
pub(crate) use self::stmt::loops::LoopExecutor;
pub(crate) use self::stmt::processes::ProcessExecutor;
pub(crate) use self::stmt::testing::TestExecutor;
pub(crate) use self::stmt::variables::VariableExecutor;
pub(crate) use self::stmt::web::WebExecutor;
pub use self::test_results::{TestFailure, TestResults};
use self::web::{
    PendingResponseSender, ServerError, WflHttpRequest, WflHttpResponse, WflWebServer,
};

use self::environment::Environment;
use self::error::{ErrorKind, RuntimeError};
use self::value::{
    ContainerDefinitionValue, ContainerEventValue, ContainerInstanceValue, ContainerMethodValue,
    EventHandler, FunctionValue, InterfaceDefinitionValue, Value,
};
use crate::builtins::get_function_arity;
use crate::config::WflConfig;
use crate::debug_report::CallFrame;
#[cfg(debug_assertions)]
use crate::exec_block_enter;
#[cfg(debug_assertions)]
use crate::exec_block_exit;
#[cfg(debug_assertions)]
use crate::exec_control_flow;
use crate::exec_trace;
#[cfg(debug_assertions)]
use crate::exec_var_assign;
#[cfg(debug_assertions)]
use crate::exec_var_declare;
#[cfg(debug_assertions)]
use crate::logging::IndentGuard;
use crate::parser::ast::{Assertion, Expression, FileOpenMode, Literal, Program, Statement};
use crate::pattern::CompiledPattern;
use crate::stdlib;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Write};
use std::net::IpAddr;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};

// Type alias for complex pending response type
// type PendingResponseSender = Arc<tokio::sync::Mutex<Option<oneshot::Sender<WflHttpResponse>>>>;
use uuid;
use warp::Filter;

/// RAII guard that ensures module loading context is restored on scope exit.
/// Automatically pops loading_stack and restores current_source_file when dropped.
struct ModuleLoadGuard<'a> {
    interpreter: &'a Interpreter,
    previous_source: Option<PathBuf>,
    should_restore: bool,
}

impl<'a> ModuleLoadGuard<'a> {
    fn new(
        interpreter: &'a Interpreter,
        module_path: PathBuf,
        previous_source: Option<PathBuf>,
    ) -> Self {
        interpreter
            .loading_stack
            .borrow_mut()
            .push(module_path.clone());
        *interpreter.current_source_file.borrow_mut() = Some(module_path);

        Self {
            interpreter,
            previous_source,
            should_restore: true,
        }
    }

    /// Get the current loading chain before cleanup (for error reporting)
    fn get_chain(&self) -> Vec<String> {
        self.interpreter
            .loading_stack
            .borrow()
            .iter()
            .map(|p| p.display().to_string())
            .collect()
    }
}

impl<'a> Drop for ModuleLoadGuard<'a> {
    fn drop(&mut self) {
        if self.should_restore {
            *self.interpreter.current_source_file.borrow_mut() = self.previous_source.clone();
            self.interpreter.loading_stack.borrow_mut().pop();
        }
    }
}

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
        Statement::WriteContentStatement { .. } => "WriteContentStatement".to_string(),
        Statement::CloseFileStatement { .. } => "CloseFileStatement".to_string(),
        Statement::CreateDirectoryStatement { .. } => "CreateDirectoryStatement".to_string(),
        Statement::CreateFileStatement { .. } => "CreateFileStatement".to_string(),
        Statement::DeleteFileStatement { .. } => "DeleteFileStatement".to_string(),
        Statement::DeleteDirectoryStatement { .. } => "DeleteDirectoryStatement".to_string(),
        Statement::LoadModuleStatement { path, .. } => {
            format!("LoadModuleStatement from '{:?}'", path)
        }
        Statement::ExecuteCommandStatement { variable_name, .. } => {
            if let Some(var) = variable_name {
                format!("ExecuteCommandStatement '{var}'")
            } else {
                "ExecuteCommandStatement".to_string()
            }
        }
        Statement::SpawnProcessStatement { variable_name, .. } => {
            format!("SpawnProcessStatement '{variable_name}'")
        }
        Statement::ReadProcessOutputStatement { variable_name, .. } => {
            format!("ReadProcessOutputStatement '{variable_name}'")
        }
        Statement::KillProcessStatement { .. } => "KillProcessStatement".to_string(),
        Statement::WaitForProcessStatement { variable_name, .. } => {
            if let Some(var) = variable_name {
                format!("WaitForProcessStatement '{var}'")
            } else {
                "WaitForProcessStatement".to_string()
            }
        }
        Statement::WaitForStatement { .. } => "WaitForStatement".to_string(),
        Statement::WaitForDurationStatement { .. } => "WaitForDurationStatement".to_string(),
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
        Statement::RegisterSignalHandlerStatement {
            signal_type,
            handler_name,
            ..
        } => {
            format!(
                "RegisterSignalHandlerStatement '{}' -> '{}'",
                signal_type, handler_name
            )
        }
        Statement::StopAcceptingConnectionsStatement { .. } => {
            "StopAcceptingConnectionsStatement".to_string()
        }
        Statement::CloseServerStatement { .. } => "CloseServerStatement".to_string(),
        // Test framework statements
        Statement::DescribeBlock { description, .. } => {
            format!("DescribeBlock '{description}'")
        }
        Statement::TestBlock { description, .. } => {
            format!("TestBlock '{description}'")
        }
        Statement::ExpectStatement { .. } => "ExpectStatement".to_string(),
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
        Expression::HeaderAccess { header_name, .. } => format!("HeaderAccess '{header_name}'"),
        Expression::CurrentTimeMilliseconds { .. } => "CurrentTimeMilliseconds".to_string(),
        Expression::CurrentTimeFormatted { format, .. } => {
            format!("CurrentTimeFormatted '{format}'")
        }
        Expression::ProcessRunning { .. } => "ProcessRunning".to_string(),
    }
}

// use self::value::FutureValue;

pub struct Interpreter {
    pub(crate) global_env: Rc<RefCell<Environment>>,
    pub(crate) current_count: RefCell<Option<f64>>,
    pub(crate) in_count_loop: RefCell<bool>,
    pub(crate) in_main_loop: RefCell<bool>, // Track if we're in a main loop (disables timeout)
    pub(crate) started: Instant,
    pub(crate) max_duration: Duration,
    pub(crate) call_stack: RefCell<Vec<CallFrame>>,
    #[allow(dead_code)]
    pub(crate) io_client: Rc<IoClient>,
    pub(crate) step_mode: bool, // Controls single-step execution mode
    pub(crate) script_args: Vec<String>, // Command-line arguments passed to the script
    pub(crate) web_servers: RefCell<HashMap<String, WflWebServer>>, // Web servers by name
    pub(crate) pending_responses: RefCell<HashMap<String, PendingResponseSender>>, // Pending response senders by request ID
    #[allow(dead_code)] // Used for future security features
    pub(crate) config: Arc<WflConfig>, // Configuration for security and other settings
    pub(crate) current_source_file: RefCell<Option<PathBuf>>, // Currently executing source file (for path resolution)
    pub(crate) loading_stack: RefCell<Vec<PathBuf>>, // Stack of currently loading files (for circular dependency detection)
    // Test execution state
    pub(crate) test_mode: RefCell<bool>,
    pub(crate) test_results: RefCell<TestResults>,
    pub(crate) current_describe_stack: RefCell<Vec<String>>,
    pub(crate) current_test_name: RefCell<Option<String>>,
}

// Process handle for managing subprocess state - removed, using one from io_client.rs

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    pub fn new() -> Self {
        Self::with_config(Arc::new(WflConfig::default()))
    }

    pub fn with_config(config: Arc<WflConfig>) -> Self {
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
            max_duration: Duration::from_secs(config.timeout_seconds),
            call_stack: RefCell::new(Vec::new()),
            io_client: Rc::new(IoClient::new(Arc::clone(&config))),
            step_mode: false,                          // Default to non-step mode
            script_args: Vec::new(),                   // Initialize empty, will be set later
            web_servers: RefCell::new(HashMap::new()), // Initialize empty web servers map
            pending_responses: RefCell::new(HashMap::new()), // Initialize empty pending responses map
            config,
            current_source_file: RefCell::new(None), // No source file initially
            loading_stack: RefCell::new(Vec::new()), // Empty loading stack
            // Test execution state
            test_mode: RefCell::new(false),
            test_results: RefCell::new(TestResults::default()),
            current_describe_stack: RefCell::new(Vec::new()),
            current_test_name: RefCell::new(None),
        }
    }

    pub fn with_timeout(seconds: u64) -> Self {
        let config = WflConfig {
            timeout_seconds: if seconds > 300 { 300 } else { seconds },
            ..Default::default()
        };
        Self::with_config(Arc::new(config))
    }

    pub fn set_step_mode(&mut self, step_mode: bool) {
        self.step_mode = step_mode;
    }

    pub fn set_script_args(&mut self, args: Vec<String>) {
        self.script_args = args;
    }

    pub fn set_source_file(&mut self, path: PathBuf) {
        *self.current_source_file.borrow_mut() = Some(path);
    }

    /// Enable or disable test mode
    pub fn set_test_mode(&self, enabled: bool) {
        *self.test_mode.borrow_mut() = enabled;
    }

    /// Get test results after running in test mode
    pub fn get_test_results(&self) -> TestResults {
        self.test_results.borrow().clone()
    }

    /// Extract variables from the environment for module analyzer
    /// Returns a HashMap of variable names to inferred types
    fn extract_parent_variables(
        env: &Rc<RefCell<Environment>>,
    ) -> HashMap<String, crate::parser::ast::Type> {
        let mut vars = HashMap::new();
        let env_borrowed = env.borrow();

        for (name, value) in &env_borrowed.values {
            let inferred_type = Self::infer_type_from_value(value);
            vars.insert(name.clone(), inferred_type);
        }

        // Also extract from parent scopes
        if let Some(parent_weak) = &env_borrowed.parent
            && let Some(parent_rc) = parent_weak.upgrade()
        {
            drop(env_borrowed); // Release borrow before recursive call
            let parent_vars = Self::extract_parent_variables(&parent_rc);
            // Parent variables are added first, can be shadowed by current scope
            for (name, ty) in parent_vars {
                vars.entry(name).or_insert(ty);
            }
        }

        vars
    }

    /// Infer AST Type from runtime Value
    fn infer_type_from_value(value: &Value) -> crate::parser::ast::Type {
        use crate::parser::ast::Type;

        match value {
            Value::Number(_) => Type::Number,
            Value::Text(_) => Type::Text,
            Value::Bool(_) => Type::Boolean,
            Value::List(_) => Type::List(Box::new(Type::Unknown)),
            Value::Object(_) => Type::Map(Box::new(Type::Text), Box::new(Type::Unknown)),
            Value::Function(_) => Type::Function {
                parameters: vec![],
                return_type: Box::new(Type::Unknown),
            },
            Value::Pattern(_) => Type::Pattern,
            Value::ContainerDefinition(def) => Type::Container(def.name.clone()),
            Value::ContainerInstance(inst) => {
                Type::ContainerInstance(inst.borrow().container_type.clone())
            }
            Value::Null | Value::Nothing => Type::Nothing,
            _ => Type::Unknown,
        }
    }

    async fn resolve_module_path(
        &self,
        relative_path: &str,
        line: usize,
        column: usize,
    ) -> Result<PathBuf, RuntimeError> {
        // Extract and clone the Option<PathBuf> to avoid holding the borrow across await
        let opt_path = self.current_source_file.borrow().as_ref().cloned();

        let resolved = if let Some(source_path) = opt_path {
            let base_dir = source_path.parent().ok_or_else(|| {
                RuntimeError::new(
                    "Cannot determine parent directory of current file".to_string(),
                    line,
                    column,
                )
            })?;
            base_dir.join(relative_path)
        } else {
            let cwd = std::env::current_dir().map_err(|e| {
                RuntimeError::new(
                    format!("Cannot determine current directory: {}", e),
                    line,
                    column,
                )
            })?;
            cwd.join(relative_path)
        };

        // Canonicalize to handle . and .. and detect duplicates
        let canonical = tokio::fs::canonicalize(&resolved).await.map_err(|e| {
            RuntimeError::new(
                format!("Cannot resolve module path '{}': {}", relative_path, e),
                line,
                column,
            )
        })?;

        Ok(canonical)
    }

    fn check_circular_dependency(
        &self,
        path: &PathBuf,
        line: usize,
        column: usize,
    ) -> Result<(), RuntimeError> {
        let stack = self.loading_stack.borrow();

        if stack.contains(path) {
            let mut chain: Vec<String> = stack.iter().map(|p| p.display().to_string()).collect();
            chain.push(path.display().to_string());

            return Err(RuntimeError::new(
                format!("Circular dependency detected: {}", chain.join(" → ")),
                line,
                column,
            ));
        }

        Ok(())
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

    pub(crate) async fn execute_statement(
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
            Statement::WriteContentStatement { line, column, .. } => (*line, *column),
            Statement::CloseFileStatement { line, column, .. } => (*line, *column),
            Statement::CreateDirectoryStatement { line, column, .. } => (*line, *column),
            Statement::CreateFileStatement { line, column, .. } => (*line, *column),
            Statement::DeleteFileStatement { line, column, .. } => (*line, *column),
            Statement::DeleteDirectoryStatement { line, column, .. } => (*line, *column),
            Statement::LoadModuleStatement { line, column, .. } => (*line, *column),
            Statement::WaitForStatement { line, column, .. } => (*line, *column),
            Statement::WaitForDurationStatement { line, column, .. } => (*line, *column),
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
            Statement::RegisterSignalHandlerStatement { line, column, .. } => (*line, *column),
            Statement::StopAcceptingConnectionsStatement { line, column, .. } => (*line, *column),
            Statement::CloseServerStatement { line, column, .. } => (*line, *column),
            Statement::ExecuteCommandStatement { line, column, .. } => (*line, *column),
            Statement::SpawnProcessStatement { line, column, .. } => (*line, *column),
            Statement::ReadProcessOutputStatement { line, column, .. } => (*line, *column),
            Statement::KillProcessStatement { line, column, .. } => (*line, *column),
            Statement::WaitForProcessStatement { line, column, .. } => (*line, *column),
            // Test framework statements
            Statement::DescribeBlock { line, column, .. } => (*line, *column),
            Statement::TestBlock { line, column, .. } => (*line, *column),
            Statement::ExpectStatement { line, column, .. } => (*line, *column),
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
                variable_name,
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
                    // Allow up to 10001 iterations to accommodate loops that need exactly 10000
                    // (e.g., "count from 1 to 10000" requires 10000 iterations)
                    10001
                };
                let mut iterations = 0;

                *self.in_count_loop.borrow_mut() = true;

                // Determine the variable name to use - custom name or default "count"
                let loop_var_name = variable_name.as_deref().unwrap_or("count");

                while should_continue(count, end_num) && iterations < max_iterations {
                    self.check_time()?;

                    *self.current_count.borrow_mut() = Some(count);

                    // Create a new scope for each iteration
                    let loop_env = Environment::new_child_env(&env);

                    // Make the loop variable available in the loop environment
                    // Use custom variable name if provided, otherwise default to "count"
                    let _ = loop_env
                        .borrow_mut()
                        .define(loop_var_name, Value::Number(count));

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

                    // Create a new scope for each iteration to properly isolate variables
                    let loop_env = Environment::new_child_env(&env);
                    let result = self.execute_block(body, Rc::clone(&loop_env)).await?;
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

                    // Create a new scope for each iteration to properly isolate variables
                    let loop_env = Environment::new_child_env(&env);
                    let result = self.execute_block(body, Rc::clone(&loop_env)).await?;
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
            Statement::WriteContentStatement {
                content,
                target,
                line,
                column,
            } => {
                let content_value = self.evaluate_expression(content, Rc::clone(&env)).await?;
                let target_value = self.evaluate_expression(target, Rc::clone(&env)).await?;

                let target_str = match &target_value {
                    Value::Text(s) => s.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected string for file handle, got {target_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                let content_str = format!("{content_value}");

                // Check if target is a file handle (starts with "file") or a file path
                if target_str.starts_with("file") {
                    // This is a file handle, use append_file to respect the file's open mode
                    match self.io_client.append_file(&target_str, &content_str).await {
                        Ok(_) => Ok((Value::Null, ControlFlow::None)),
                        Err(e) => Err(RuntimeError::new(e, *line, *column)),
                    }
                } else {
                    // This is a file path, use write_file (overwrite mode)
                    match self.io_client.write_file(&target_str, &content_str).await {
                        Ok(_) => Ok((Value::Null, ControlFlow::None)),
                        Err(e) => Err(RuntimeError::new(e, *line, *column)),
                    }
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

            Statement::LoadModuleStatement {
                path, line, column, ..
            } => {
                // 1. Evaluate path expression to string
                let path_value = self.evaluate_expression(path, Rc::clone(&env)).await?;
                let path_str: String = match &path_value {
                    Value::Text(s) => s.to_string(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Module path must be a string, got {path_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                // 2. Resolve absolute path
                let resolved_path = self.resolve_module_path(&path_str, *line, *column).await?;

                // 3. Check circular dependencies
                self.check_circular_dependency(&resolved_path, *line, *column)?;

                // 4. Read file content
                let content = tokio::fs::read_to_string(&resolved_path)
                    .await
                    .map_err(|e| {
                        RuntimeError::new(
                            format!("Cannot load module '{}': {}", path_str, e),
                            *line,
                            *column,
                        )
                    })?;

                // 6. Parse module
                use crate::lexer::lex_wfl_with_positions;
                use crate::parser::Parser;

                let tokens = lex_wfl_with_positions(&content);
                let mut parser = Parser::new(&tokens);
                let program = parser.parse().map_err(|errors| {
                    // Use the parse error's position from the module file, not the load site
                    let first_error = errors.first();
                    let (error_line, error_column) =
                        first_error.map(|e| (e.line, e.column)).unwrap_or((1, 1));
                    RuntimeError::new(
                        format!(
                            "Parse error in module '{}': {}",
                            resolved_path.display(),
                            first_error.map(|e| e.message.as_str()).unwrap_or("unknown")
                        ),
                        error_line,
                        error_column,
                    )
                })?;

                // 7. Analyze semantics
                use crate::analyzer::Analyzer;

                // Extract parent variables from current environment for module analyzer
                let parent_vars = Self::extract_parent_variables(&env);
                let mut analyzer = Analyzer::with_parent_variables(parent_vars);
                if let Err(errors) = analyzer.analyze(&program) {
                    // Use the semantic error's position from the module file, not the load site
                    let first_error = errors.first();
                    let (error_line, error_column) =
                        first_error.map(|e| (e.line, e.column)).unwrap_or((1, 1));
                    return Err(RuntimeError::new(
                        format!(
                            "Semantic error in module '{}': {}",
                            resolved_path.display(),
                            first_error.map(|e| e.to_string()).unwrap_or_default()
                        ),
                        error_line,
                        error_column,
                    ));
                }

                // 8. Type check
                use crate::typechecker::TypeChecker;

                // Use the analyzer with parent scope for type checking
                let mut tc = TypeChecker::with_analyzer(analyzer);
                if let Err(type_errors) = tc.check_types(&program) {
                    // Use the type error's position from the module file, not the load site
                    let first_error = type_errors.first();
                    let (error_line, error_column) =
                        first_error.map(|e| (e.line, e.column)).unwrap_or((1, 1));
                    return Err(RuntimeError::new(
                        format!(
                            "Type error in module '{}': {}",
                            resolved_path.display(),
                            first_error.map(|e| e.to_string()).unwrap_or_default()
                        ),
                        error_line,
                        error_column,
                    ));
                }

                // 8. Create isolated child environment
                // This prevents mutations of containers (lists/objects) from affecting parent scope
                use crate::interpreter::environment::Environment;
                let module_env = Environment::new_isolated_child_env(&env);

                // 9. Create guard to ensure context restoration on scope exit
                let previous_source = self.current_source_file.borrow().clone();
                let _guard = ModuleLoadGuard::new(self, resolved_path.clone(), previous_source);

                // 10. Execute module in child scope
                let result = self.execute_block(&program.statements, module_env).await;

                // Note: Context automatically restored when _guard drops at end of scope

                // 11. Handle result
                match result {
                    Ok((_, ControlFlow::None)) => Ok((Value::Null, ControlFlow::None)),
                    Ok((_, ControlFlow::Return(_))) => Err(RuntimeError::new(
                        "Cannot use 'return' in module scope".to_string(),
                        *line,
                        *column,
                    )),
                    Ok((_, ControlFlow::Break)) => Err(RuntimeError::new(
                        "Cannot use 'break' in module scope".to_string(),
                        *line,
                        *column,
                    )),
                    Ok((_, ControlFlow::Continue)) => Err(RuntimeError::new(
                        "Cannot use 'continue' in module scope".to_string(),
                        *line,
                        *column,
                    )),
                    Ok((_, ControlFlow::Exit)) => Err(RuntimeError::new(
                        "Cannot use 'exit' in module scope".to_string(),
                        *line,
                        *column,
                    )),
                    Err(e) => {
                        // Capture chain BEFORE guard drops (while current module is still on stack)
                        let chain = _guard.get_chain();
                        if chain.len() > 1 {
                            // Only show chain if there are multiple modules
                            // Preserve the original error kind and use the original error's coordinates
                            Err(RuntimeError::with_kind(
                                format!(
                                    "Error in module chain {}: {}",
                                    chain.join(" → "),
                                    e.message
                                ),
                                e.line,
                                e.column,
                                e.kind,
                            ))
                        } else {
                            Err(e)
                        }
                    }
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
            Statement::WaitForDurationStatement {
                duration,
                unit,
                line,
                column,
            } => {
                let duration_value = self.evaluate_expression(duration, Rc::clone(&env)).await?;
                let duration_ms = match &duration_value {
                    Value::Number(n) => match unit.as_str() {
                        "milliseconds" => *n as u64,
                        "seconds" => (*n * 1000.0) as u64,
                        _ => {
                            return Err(RuntimeError::new(
                                format!("Unsupported time unit: {}", unit),
                                *line,
                                *column,
                            ));
                        }
                    },
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Expected number for duration, got {duration_value:?}"),
                            *line,
                            *column,
                        ));
                    }
                };

                tokio::time::sleep(std::time::Duration::from_millis(duration_ms)).await;
                Ok((Value::Null, ControlFlow::None))
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
                                crate::parser::ast::ErrorType::ProcessNotFound => {
                                    err.kind == ErrorKind::ProcessNotFound
                                }
                                crate::parser::ast::ErrorType::ProcessSpawnFailed => {
                                    err.kind == ErrorKind::ProcessSpawnFailed
                                }
                                crate::parser::ast::ErrorType::ProcessKillFailed => {
                                    err.kind == ErrorKind::ProcessKillFailed
                                }
                                crate::parser::ast::ErrorType::CommandNotFound => {
                                    err.kind == ErrorKind::CommandNotFound
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
                let mut _last_value = Value::Null;

                loop {
                    self.check_time()?;

                    let condition_value = self
                        .evaluate_expression(condition, Rc::clone(&loop_env))
                        .await?;

                    if !condition_value.is_truthy() {
                        break;
                    }

                    let result = self.execute_block(body, Rc::clone(&loop_env)).await?;
                    _last_value = result.0;

                    match result.1 {
                        ControlFlow::Break => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Breaking out of repeat-while loop");
                            break;
                        }
                        ControlFlow::Continue => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Continuing repeat-while loop");
                            continue;
                        }
                        ControlFlow::Exit => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Exiting from repeat-while loop");
                            return Ok((_last_value, ControlFlow::Exit));
                        }
                        ControlFlow::Return(val) => {
                            #[cfg(debug_assertions)]
                            exec_trace!("Returning from repeat-while loop");
                            return Ok((val.clone(), ControlFlow::Return(val)));
                        }
                        ControlFlow::None => {}
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

                // Create request/response channels
                let (request_sender, request_receiver) =
                    mpsc::unbounded_channel::<WflHttpRequest>();
                let request_receiver = Arc::new(tokio::sync::Mutex::new(request_receiver));

                // Create warp routes that handle all HTTP methods and paths
                // Note: Body size validation is performed manually in the handler below
                // to allow GET requests without Content-Length headers
                let request_sender_clone = request_sender.clone();
                let routes = warp::any()
                    .and(warp::method())
                    .and(warp::path::full())
                    .and(warp::header::headers_cloned())
                    .and(warp::body::bytes())
                    .and(warp::addr::remote())
                    .and_then(
                        move |method: warp::http::Method,
                              path: warp::path::FullPath,
                              headers: warp::http::HeaderMap,
                              body: bytes::Bytes,
                              remote_addr: Option<std::net::SocketAddr>| {
                            let sender = request_sender_clone.clone();
                            async move {
                                // DoS PROTECTION: Enforce 1MB body size limit
                                // This maintains the security requirement from web_server_body_limit_test.wfl
                                const MAX_BODY_SIZE: usize = 1_048_576; // 1MB
                                if body.len() > MAX_BODY_SIZE {
                                    return Err(warp::reject::custom(ServerError(format!(
                                        "Request body too large: {} bytes (limit: {} bytes)",
                                        body.len(),
                                        MAX_BODY_SIZE
                                    ))));
                                }

                                // Generate unique request ID
                                let request_id = uuid::Uuid::new_v4().to_string();

                                // Extract client IP
                                let client_ip = remote_addr
                                    .map(|addr| addr.ip().to_string())
                                    .unwrap_or_else(|| "unknown".to_string());

                                // Convert headers to HashMap
                                let mut header_map = HashMap::new();
                                for (name, value) in headers.iter() {
                                    if let Ok(value_str) = value.to_str() {
                                        header_map.insert(name.to_string(), value_str.to_string());
                                    }
                                }

                                // Convert body to string
                                let body_str = String::from_utf8_lossy(&body).to_string();

                                // Create response channel
                                let (response_sender, response_receiver) =
                                    oneshot::channel::<WflHttpResponse>();

                                // Create WFL request
                                let wfl_request = WflHttpRequest {
                                    id: request_id,
                                    method: method.to_string(),
                                    path: path.as_str().to_string(),
                                    client_ip,
                                    body: body_str,
                                    headers: header_map,
                                    response_sender: Arc::new(tokio::sync::Mutex::new(Some(
                                        response_sender,
                                    ))),
                                };

                                // Send request to WFL interpreter
                                if sender.send(wfl_request).is_err() {
                                    return Err(warp::reject::custom(ServerError(
                                        "Request channel closed".to_string(),
                                    )));
                                }

                                // Wait for response
                                match response_receiver.await {
                                    Ok(response) => {
                                        let status_code =
                                            warp::http::StatusCode::from_u16(response.status)
                                                .unwrap_or(warp::http::StatusCode::OK);

                                        // Convert content to bytes for accurate Content-Length calculation
                                        // HTTP Content-Length must match exact byte count of body
                                        let content_bytes = response.content.into_bytes();
                                        let content_length = content_bytes.len();

                                        let mut reply_builder = warp::http::Response::builder()
                                            .status(status_code)
                                            .header("Content-Type", response.content_type)
                                            .header("Content-Length", content_length);

                                        // Add additional headers
                                        for (name, value) in response.headers {
                                            reply_builder = reply_builder.header(name, value);
                                        }

                                        match reply_builder.body(content_bytes) {
                                            Ok(response) => Ok(response),
                                            Err(_) => Err(warp::reject::custom(ServerError(
                                                "Failed to build response".to_string(),
                                            ))),
                                        }
                                    }
                                    Err(_) => Err(warp::reject::custom(ServerError(
                                        "Response channel closed".to_string(),
                                    ))),
                                }
                            }
                        },
                    );

                // Parse the bind address from config
                let bind_addr: IpAddr = match self.config.web_server_bind_address.parse() {
                    Ok(addr) => addr,
                    Err(_) => {
                        return Err(RuntimeError::new(
                            format!(
                                "Invalid web_server_bind_address in config: '{}'. Expected a valid IP address (e.g., '127.0.0.1' or '0.0.0.0')",
                                self.config.web_server_bind_address
                            ),
                            *line,
                            *column,
                        ));
                    }
                };

                // Start the server
                let server_task = warp::serve(routes).try_bind_ephemeral((bind_addr, port_num));

                match server_task {
                    Ok((addr, server)) => {
                        // Spawn the server in the background
                        let server_handle = tokio::spawn(server);

                        // Create WFL web server object
                        let wfl_server = WflWebServer {
                            request_receiver: request_receiver.clone(),
                            request_sender: request_sender.clone(),
                            server_handle: Some(server_handle),
                        };

                        // Store the server in the interpreter
                        self.web_servers
                            .borrow_mut()
                            .insert(server_name.clone(), wfl_server);

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
                timeout,
                line,
                column,
            } => {
                // Look up the server by name
                let server_name = match self.evaluate_expression(server, Rc::clone(&env)).await? {
                    Value::Text(name) => {
                        // Extract server name from "WebServer::host:port" format
                        let name_str = name.as_ref();
                        if name_str.starts_with("WebServer::") {
                            // Find the server by matching the exact server value
                            let web_servers = self.web_servers.borrow();

                            // Search through all servers to find which one has this exact value
                            let mut found_server = None;
                            for (server_name, _) in web_servers.iter() {
                                // Get the stored value for this server name
                                if let Some(Value::Text(stored_text)) =
                                    env.borrow().get(server_name)
                                    && stored_text.as_ref() == name_str
                                {
                                    // Found the matching server
                                    found_server = Some(server_name.clone());
                                    break;
                                }
                            }

                            // Return the found server or use first server as fallback
                            if let Some(server_name) = found_server {
                                server_name
                            } else if let Some((found_name, _)) = web_servers.iter().next() {
                                found_name.clone()
                            } else {
                                return Err(RuntimeError::new(
                                    "No web servers found".to_string(),
                                    *line,
                                    *column,
                                ));
                            }
                        } else {
                            name_str.to_string()
                        }
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            "Expected server name as text".to_string(),
                            *line,
                            *column,
                        ));
                    }
                };

                // Get the server's request receiver
                let request_receiver = {
                    let web_servers = self.web_servers.borrow();
                    if let Some(server) = web_servers.get(&server_name) {
                        server.request_receiver.clone()
                    } else {
                        return Err(RuntimeError::new(
                            format!("Web server '{}' not found", server_name),
                            *line,
                            *column,
                        ));
                    }
                };

                // Wait for a request to come in (with optional timeout)
                let request = {
                    let mut receiver = request_receiver.lock().await;

                    // Evaluate timeout if provided
                    let timeout_duration = if let Some(timeout_expr) = timeout {
                        let timeout_val = self
                            .evaluate_expression(timeout_expr, Rc::clone(&env))
                            .await?;
                        match timeout_val {
                            Value::Number(ms) if ms > 0.0 => {
                                Some(std::time::Duration::from_millis(ms as u64))
                            }
                            _ => {
                                return Err(RuntimeError::new(
                                    "Timeout must be a positive number (milliseconds)".to_string(),
                                    *line,
                                    *column,
                                ));
                            }
                        }
                    } else {
                        None
                    };

                    // Wait for request with or without timeout
                    if let Some(duration) = timeout_duration {
                        match tokio::time::timeout(duration, receiver.recv()).await {
                            Ok(Some(req)) => req,
                            Ok(None) => {
                                return Err(RuntimeError::new(
                                    "Request channel closed".to_string(),
                                    *line,
                                    *column,
                                ));
                            }
                            Err(_) => {
                                return Err(RuntimeError::new(
                                    format!(
                                        "Timeout waiting for request ({} ms)",
                                        duration.as_millis()
                                    ),
                                    *line,
                                    *column,
                                ));
                            }
                        }
                    } else {
                        // No timeout - wait indefinitely
                        match receiver.recv().await {
                            Some(req) => req,
                            None => {
                                return Err(RuntimeError::new(
                                    "Request channel closed".to_string(),
                                    *line,
                                    *column,
                                ));
                            }
                        }
                    }
                };

                // Store the request in a global map for RespondStatement to access
                {
                    let mut pending_responses = self.pending_responses.borrow_mut();
                    pending_responses.insert(request.id.clone(), request.response_sender);
                }

                // Define individual variables for request properties (more natural for WFL)
                let mut env_mut = env.borrow_mut();

                // Define the main request variable (for use in respond statements)
                let mut request_properties = HashMap::new();
                request_properties.insert(
                    "_response_sender".to_string(),
                    Value::Text(Rc::from(request.id.clone())),
                );
                let request_object = Value::Object(Rc::new(RefCell::new(request_properties)));

                match env_mut.define(request_name, request_object) {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                }

                // Define individual request property variables
                match env_mut.define("method", Value::Text(Rc::from(request.method.clone()))) {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                }

                match env_mut.define("path", Value::Text(Rc::from(request.path.clone()))) {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                }

                match env_mut.define(
                    "client_ip",
                    Value::Text(Rc::from(request.client_ip.clone())),
                ) {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                }

                match env_mut.define("body", Value::Text(Rc::from(request.body.clone()))) {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                }

                // Convert headers to WFL object and define as headers variable
                let mut headers_map = HashMap::new();
                for (key, value) in request.headers.iter() {
                    headers_map.insert(key.clone(), Value::Text(Rc::from(value.clone())));
                }
                let headers_object = Value::Object(Rc::new(RefCell::new(headers_map)));

                match env_mut.define("headers", headers_object) {
                    Ok(_) => {}
                    Err(msg) => return Err(RuntimeError::new(msg, *line, *column)),
                }

                drop(env_mut); // Release the borrow

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::RespondStatement {
                request,
                content,
                status,
                content_type,
                line,
                column,
            } => {
                // Get the request object
                let request_val = self.evaluate_expression(request, Rc::clone(&env)).await?;
                let request_id = match &request_val {
                    Value::Object(obj) => {
                        let obj_ref = obj.borrow();
                        match obj_ref.get("_response_sender") {
                            Some(Value::Text(id)) => id.as_ref().to_string(),
                            _ => {
                                return Err(RuntimeError::new(
                                    "Request object missing response sender ID".to_string(),
                                    *line,
                                    *column,
                                ));
                            }
                        }
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            "Expected request object".to_string(),
                            *line,
                            *column,
                        ));
                    }
                };

                // Evaluate response content
                let content_val = self.evaluate_expression(content, Rc::clone(&env)).await?;
                let content_str = match &content_val {
                    Value::Text(text) => text.as_ref().to_string(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => format!("{:?}", content_val),
                };

                // Evaluate status code (optional)
                let status_code = if let Some(status_expr) = status {
                    let status_val = self
                        .evaluate_expression(status_expr, Rc::clone(&env))
                        .await?;
                    match &status_val {
                        Value::Number(n) => *n as u16,
                        _ => {
                            return Err(RuntimeError::new(
                                "Status code must be a number".to_string(),
                                *line,
                                *column,
                            ));
                        }
                    }
                } else {
                    200 // Default to 200 OK
                };

                // Evaluate content type (optional)
                let content_type_str = if let Some(ct_expr) = content_type {
                    let ct_val = self.evaluate_expression(ct_expr, Rc::clone(&env)).await?;
                    match &ct_val {
                        Value::Text(text) => text.as_ref().to_string(),
                        _ => {
                            return Err(RuntimeError::new(
                                "Content type must be text".to_string(),
                                *line,
                                *column,
                            ));
                        }
                    }
                } else {
                    "text/plain".to_string() // Default content type
                };

                // Create response
                let response = WflHttpResponse {
                    content: content_str,
                    status: status_code,
                    content_type: content_type_str,
                    headers: HashMap::new(), // TODO: Add support for custom headers
                };

                // Send response
                let response_sender = {
                    let mut pending = self.pending_responses.borrow_mut();
                    pending.remove(&request_id)
                };

                if let Some(sender_arc) = response_sender {
                    let mut sender_opt = sender_arc.lock().await;
                    if let Some(sender) = sender_opt.take() {
                        if sender.send(response).is_err() {
                            return Err(RuntimeError::new(
                                "Failed to send response - client may have disconnected"
                                    .to_string(),
                                *line,
                                *column,
                            ));
                        }
                    } else {
                        return Err(RuntimeError::new(
                            "Response already sent for this request".to_string(),
                            *line,
                            *column,
                        ));
                    }
                } else {
                    return Err(RuntimeError::new(
                        "Request ID not found - response may have already been sent".to_string(),
                        *line,
                        *column,
                    ));
                }

                Ok((Value::Null, ControlFlow::None))
            }
            // Graceful shutdown and signal handling statements
            Statement::RegisterSignalHandlerStatement {
                signal_type,
                handler_name,
                line,
                column,
            } => {
                // For now, just store the signal handler registration
                // In a full implementation, this would set up actual signal handlers
                let signal_handler_key = format!("signal_handler_{}", signal_type);

                env.borrow_mut()
                    .define(
                        &signal_handler_key,
                        Value::Text(Rc::from(handler_name.clone())),
                    )
                    .map_err(|e| RuntimeError::new(e, *line, *column))?;

                // TODO: Implement actual signal handling with tokio::signal
                // For now, we'll simulate this in the graceful shutdown test

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::StopAcceptingConnectionsStatement {
                server,
                line,
                column,
            } => {
                let server_val = self.evaluate_expression(server, Rc::clone(&env)).await?;
                let server_name = match &server_val {
                    Value::Text(name) => {
                        let name_str = name.as_ref();
                        if name_str.starts_with("WebServer::") {
                            // Find the original server name in our web_servers map
                            let web_servers = self.web_servers.borrow();
                            if let Some((found_name, _)) = web_servers.iter().next() {
                                found_name.clone()
                            } else {
                                return Err(RuntimeError::new(
                                    "No web servers found".to_string(),
                                    *line,
                                    *column,
                                ));
                            }
                        } else {
                            name_str.to_string()
                        }
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            "Expected server name as text".to_string(),
                            *line,
                            *column,
                        ));
                    }
                };

                // Mark server as no longer accepting connections
                // In a full implementation, this would stop the warp server from accepting new connections
                // For now, we'll just set a flag
                env.borrow_mut()
                    .define(
                        &format!("{}_accepting_connections", server_name),
                        Value::Bool(false),
                    )
                    .map_err(|e| RuntimeError::new(e, *line, *column))?;

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::CloseServerStatement {
                server,
                line,
                column,
            } => {
                let server_val = self.evaluate_expression(server, Rc::clone(&env)).await?;
                let server_name = match &server_val {
                    Value::Text(name) => {
                        let name_str = name.as_ref();
                        if name_str.starts_with("WebServer::") {
                            // Find the server name that corresponds to this WebServer value
                            let web_servers = self.web_servers.borrow();

                            // Search through all servers to find which one has this exact value
                            let mut found_server = None;
                            for (server_name, _) in web_servers.iter() {
                                // Check if this server name's variable has the matching value
                                if let Some(Value::Text(stored_text)) =
                                    env.borrow().get(server_name)
                                    && stored_text.as_ref() == name_str
                                {
                                    found_server = Some(server_name.clone());
                                    break;
                                }
                            }

                            // Return the found server or use first server as fallback
                            if let Some(server_name) = found_server {
                                server_name
                            } else if let Some((found_name, _)) = web_servers.iter().next() {
                                found_name.clone()
                            } else {
                                return Err(RuntimeError::new(
                                    "No web servers found".to_string(),
                                    *line,
                                    *column,
                                ));
                            }
                        } else {
                            name_str.to_string()
                        }
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            "Expected server name as text".to_string(),
                            *line,
                            *column,
                        ));
                    }
                };

                // Close the server
                let mut web_servers = self.web_servers.borrow_mut();
                if let Some(wfl_server) = web_servers.remove(&server_name) {
                    // Graceful shutdown: Give in-flight responses time to complete transmission
                    // before forcefully aborting the server task
                    if let Some(handle) = wfl_server.server_handle {
                        // Allow 50ms for pending HTTP responses to be transmitted
                        // This prevents race condition where abort() closes the TCP connection
                        // before response bytes reach the client, causing IncompleteMessage errors
                        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                        handle.abort();
                    }
                } else {
                    return Err(RuntimeError::new(
                        format!("Server '{}' not found", server_name),
                        *line,
                        *column,
                    ));
                }

                Ok((Value::Null, ControlFlow::None))
            }
            // Subprocess statements
            Statement::ExecuteCommandStatement {
                command,
                arguments,
                variable_name,
                use_shell,
                line,
                column,
            } => {
                // Evaluate command expression
                let cmd_val = self.evaluate_expression(command, Rc::clone(&env)).await?;
                let cmd_str = match &cmd_val {
                    Value::Text(text) => text.as_ref(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Command must be text, got {}", cmd_val.type_name()),
                            *line,
                            *column,
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
                                *line,
                                *column,
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
                    .execute_command(cmd_str, &args_refs, *use_shell, *line, *column)
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
                        RuntimeError::with_kind(e, *line, *column, kind)
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
                        .map_err(|e| RuntimeError::new(e, *line, *column))?;
                }

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::SpawnProcessStatement {
                command,
                arguments,
                variable_name,
                use_shell,
                line,
                column,
            } => {
                // Evaluate command expression
                let cmd_val = self.evaluate_expression(command, Rc::clone(&env)).await?;
                let cmd_str = match &cmd_val {
                    Value::Text(text) => text.as_ref(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Command must be text, got {}", cmd_val.type_name()),
                            *line,
                            *column,
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
                                *line,
                                *column,
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
                    .spawn_process(cmd_str, &args_refs, *use_shell, *line, *column)
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
                        RuntimeError::with_kind(e, *line, *column, kind)
                    })?;

                // Store process ID in variable
                env.borrow_mut()
                    .define(variable_name, Value::Text(Rc::from(process_id.as_str())))
                    .map_err(|e| RuntimeError::new(e, *line, *column))?;

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::ReadProcessOutputStatement {
                process_id,
                variable_name,
                line,
                column,
            } => {
                // Evaluate process ID expression
                let proc_val = self
                    .evaluate_expression(process_id, Rc::clone(&env))
                    .await?;
                let proc_id = match &proc_val {
                    Value::Text(text) => text.as_ref(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Process ID must be text, got {}", proc_val.type_name()),
                            *line,
                            *column,
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
                        RuntimeError::with_kind(e, *line, *column, kind)
                    })?;

                // Store output in variable
                env.borrow_mut()
                    .define(variable_name, Value::Text(Rc::from(output.as_str())))
                    .map_err(|e| RuntimeError::new(e, *line, *column))?;

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::KillProcessStatement {
                process_id,
                line,
                column,
            } => {
                // Evaluate process ID expression
                let proc_val = self
                    .evaluate_expression(process_id, Rc::clone(&env))
                    .await?;
                let proc_id = match &proc_val {
                    Value::Text(text) => text.as_ref(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Process ID must be text, got {}", proc_val.type_name()),
                            *line,
                            *column,
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
                    RuntimeError::with_kind(e, *line, *column, kind)
                })?;

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::WaitForProcessStatement {
                process_id,
                variable_name,
                line,
                column,
            } => {
                // Evaluate process ID expression
                let proc_val = self
                    .evaluate_expression(process_id, Rc::clone(&env))
                    .await?;
                let proc_id = match &proc_val {
                    Value::Text(text) => text.as_ref(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Process ID must be text, got {}", proc_val.type_name()),
                            *line,
                            *column,
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
                        RuntimeError::with_kind(e, *line, *column, kind)
                    })?;

                // Store exit code in variable if provided
                if let Some(var_name) = variable_name {
                    env.borrow_mut()
                        .define(var_name, Value::Number(exit_code as f64))
                        .map_err(|e| RuntimeError::new(e, *line, *column))?;
                }

                Ok((Value::Null, ControlFlow::None))
            }
            // Test framework statements
            Statement::DescribeBlock {
                description,
                setup,
                teardown,
                tests,
                line,
                column,
            } => {
                if !*self.test_mode.borrow() {
                    return Err(RuntimeError::new(
                        "describe blocks can only be used in test mode (run with --test flag)"
                            .to_string(),
                        *line,
                        *column,
                    ));
                }

                // Push describe context
                self.current_describe_stack
                    .borrow_mut()
                    .push(description.clone());

                // Create describe-level environment for setup/teardown sharing
                // This allows tests to access setup variables while remaining isolated from each other
                let describe_env = Environment::new_child_env(&env);

                // Run setup if present (runs in describe environment)
                if let Some(setup_stmts) = setup {
                    for stmt in setup_stmts {
                        Box::pin(self._execute_statement(stmt, describe_env.clone())).await?;
                    }
                }

                // Execute all tests (each gets a child of describe_env for isolation)
                for test in tests {
                    Box::pin(self._execute_statement(test, describe_env.clone())).await?;
                }

                // Run teardown if present (runs in describe environment)
                if let Some(teardown_stmts) = teardown {
                    for stmt in teardown_stmts {
                        Box::pin(self._execute_statement(stmt, describe_env.clone())).await?;
                    }
                }

                // Pop describe context
                self.current_describe_stack.borrow_mut().pop();

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::TestBlock {
                description,
                body,
                line,
                column,
            } => {
                if !*self.test_mode.borrow() {
                    return Err(RuntimeError::new(
                        "test blocks can only be used in test mode (run with --test flag)"
                            .to_string(),
                        *line,
                        *column,
                    ));
                }

                // Set current test name for failure tracking
                *self.current_test_name.borrow_mut() = Some(description.clone());

                // Increment test count
                self.test_results.borrow_mut().total_tests += 1;

                // Create isolated environment for test (child of describe env)
                // Using isolated mode prevents tests from mutating setup variables,
                // ensuring each test gets a fresh copy for true isolation
                let test_env = Environment::new_isolated_child_env(&env);

                // Execute test body and catch assertion failures
                let mut test_passed = true;

                for stmt in body {
                    match Box::pin(self._execute_statement(stmt, test_env.clone())).await {
                        Ok(_) => {}
                        Err(e) => {
                            test_passed = false;

                            // Only record failure if not already recorded by expect statement
                            // Check if this is an assertion failure (which has already been recorded)
                            let error_msg = e.to_string();
                            if !error_msg.starts_with("Assertion failed:") {
                                // This is a non-assertion error (e.g., runtime error in test code)
                                let context = self.current_describe_stack.borrow().clone();
                                let failure = TestFailure {
                                    describe_context: context,
                                    test_name: description.clone(),
                                    assertion_message: error_msg,
                                    duration: Duration::from_millis(0), // Placeholder
                                    line: *line,
                                    column: *column,
                                };
                                self.test_results.borrow_mut().failures.push(failure);
                            }

                            // Don't propagate the error - continue running other tests
                            break;
                        }
                    }
                }

                if test_passed {
                    self.test_results.borrow_mut().passed_tests += 1;
                }

                // Clear current test name
                *self.current_test_name.borrow_mut() = None;

                Ok((Value::Null, ControlFlow::None))
            }
            Statement::ExpectStatement {
                subject,
                assertion,
                line,
                column,
            } => {
                if !*self.test_mode.borrow() {
                    return Err(RuntimeError::new(
                        "expect statements can only be used in test mode (run with --test flag)"
                            .to_string(),
                        *line,
                        *column,
                    ));
                }

                // Evaluate subject expression
                let subject_value = self.evaluate_expression(subject, env.clone()).await?;

                // Check assertion
                let (passed, expected_value) = self
                    .check_assertion(&subject_value, assertion, env.clone())
                    .await?;

                if !passed {
                    // Record failure with proper test name tracking
                    let message = self.create_assertion_message_with_values(
                        assertion,
                        &subject_value,
                        expected_value.as_ref(),
                    );
                    let context = self.current_describe_stack.borrow().clone();
                    let test_name = self
                        .current_test_name
                        .borrow()
                        .clone()
                        .unwrap_or_else(|| "unknown test".to_string());

                    let failure = TestFailure {
                        describe_context: context,
                        test_name,
                        assertion_message: message.clone(),
                        duration: Duration::from_millis(0), // Placeholder
                        line: *line,
                        column: *column,
                    };

                    self.test_results.borrow_mut().failures.push(failure);
                    self.test_results.borrow_mut().failed_tests += 1;

                    return Err(RuntimeError::new(
                        format!("Assertion failed: {message}"),
                        *line,
                        *column,
                    ));
                }

                Ok((Value::Null, ControlFlow::None))
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

    pub(crate) async fn execute_block(
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

    // Helper to evaluate literals directly without Box::pin allocation
    fn evaluate_literal_direct(
        &self,
        literal: &Literal,
        line: usize,
        column: usize,
    ) -> Result<Option<Value>, RuntimeError> {
        match literal {
            Literal::String(s) => Ok(Some(Value::Text(Rc::from(s.as_str())))),
            Literal::Integer(i) => Ok(Some(Value::Number(*i as f64))),
            Literal::Float(f) => Ok(Some(Value::Number(*f))),
            Literal::Boolean(b) => Ok(Some(Value::Bool(*b))),
            Literal::Nothing => Ok(Some(Value::Null)),
            // Pattern literals might error, so we can handle them here
            Literal::Pattern(_ir_string) => Err(RuntimeError::new(
                "Pattern literals not yet supported in new pattern system".to_string(),
                line,
                column,
            )),
            // List requires recursion, so fall through to boxed implementation
            Literal::List(_) => Ok(None),
        }
    }

    // Helper for variable auto-call logic
    fn handle_variable_auto_call(
        &self,
        value: Value,
        line: usize,
        column: usize,
    ) -> Result<Option<Value>, RuntimeError> {
        match &value {
            Value::NativeFunction(func_name, native_fn) => {
                if get_function_arity(func_name) == 0 {
                    // Native functions are synchronous
                    native_fn(vec![])
                        .map(Some)
                        .map_err(|e| RuntimeError::new(format!("{}", e), line, column))
                } else {
                    Ok(Some(value))
                }
            }
            Value::Function(func) => {
                if func.params.is_empty() {
                    // User functions are async -> return None to signal fallback to async
                    Ok(None)
                } else {
                    Ok(Some(value))
                }
            }
            _ => Ok(Some(value)),
        }
    }

    // Helper for fast variable lookup to avoid Box::pin allocation
    // Returns Ok(Some(value)) if handled synchronously
    // Returns Ok(None) if async handling (user function call) is needed
    // Returns Err(...) if runtime error
    fn try_evaluate_variable_sync(
        &self,
        name: &str,
        env: &Rc<RefCell<Environment>>,
        line: usize,
        column: usize,
    ) -> Result<Option<Value>, RuntimeError> {
        // Handle special count variable inside count loops
        if name == "count" && *self.in_count_loop.borrow() {
            if let Some(count_value) = *self.current_count.borrow() {
                return Ok(Some(Value::Number(count_value)));
            }
            return Err(RuntimeError::new(
                "Internal error: count variable accessed in count loop but no current count set"
                    .to_string(),
                line,
                column,
            ));
        }

        // Try normal variable lookup first
        if let Some(value) = env.borrow().get(name) {
            self.handle_variable_auto_call(value, line, column)
        } else if name == "count" {
            Err(RuntimeError::new(
                "Variable 'count' can only be used inside count loops. Use 'count from X to Y:' to create a count loop.".to_string(),
                line,
                column,
            ))
        } else {
            Err(RuntimeError::new(
                format!("Undefined variable '{name}'"),
                line,
                column,
            ))
        }
    }

    pub(crate) async fn evaluate_expression(
        &self,
        expr: &Expression,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        #[cfg(debug_assertions)]
        exec_trace!("Evaluating expression: {}", expr_type(expr));

        // OPTIMIZATION: Handle simple literals directly to avoid Box::pin allocation
        // This significantly improves performance for tight loops with literals
        if let Expression::Literal(literal, line, column) = expr
            && let Some(value) = self.evaluate_literal_direct(literal, *line, *column)?
        {
            return Ok(value);
        }

        // OPTIMIZATION: Handle simple variable lookups directly to avoid Box::pin allocation
        if let Expression::Variable(name, line, column) = expr {
            match self.try_evaluate_variable_sync(name, &env, *line, *column) {
                Ok(Some(val)) => return Ok(val),
                Ok(None) => {
                    // Fallthrough to boxed execution which handles user function calls (async)
                }
                Err(e) => return Err(e),
            }
        }

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
            Expression::StaticMemberAccess {
                container,
                member,
                line,
                column,
            } => {
                self.evaluate_static_member_access(
                    container,
                    member,
                    *line,
                    *column,
                    Rc::clone(&env),
                )
                .await
            }

            Expression::MethodCall {
                object,
                method,
                arguments,
                line,
                column,
            } => {
                let object_val = self.evaluate_expression(object, Rc::clone(&env)).await?;
                self.evaluate_method_call(
                    &object_val,
                    method,
                    arguments,
                    *line,
                    *column,
                    Rc::clone(&env),
                )
                .await
            }
            Expression::PropertyAccess {
                object,
                property,
                line,
                column,
            } => {
                let object_val = self.evaluate_expression(object, Rc::clone(&env)).await?;
                self.evaluate_property_access(
                    &object_val,
                    property,
                    *line,
                    *column,
                    Rc::clone(&env),
                )
                .await
            }
            Expression::AwaitExpression {
                expression,
                line: _,
                column: _,
            } => self.evaluate_expression(expression, Rc::clone(&env)).await,
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
                        Value::Function(func) => {
                            if func.params.is_empty() {
                                // Auto-call zero-argument user-defined functions
                                self.call_function(func, vec![], *line, *column).await
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
                self.evaluate_binary_operation(
                    left,
                    operator,
                    right,
                    *line,
                    *column,
                    Rc::clone(&env),
                )
                .await
            }

            Expression::UnaryOperation {
                operator,
                expression,
                line,
                column,
            } => {
                self.evaluate_unary_operation(operator, expression, *line, *column, Rc::clone(&env))
                    .await
            }

            Expression::FunctionCall {
                function,
                arguments,
                line,
                column,
            } => {
                self.evaluate_function_call(function, arguments, *line, *column, Rc::clone(&env))
                    .await
            }

            Expression::ActionCall {
                name,
                arguments,
                line,
                column,
            } => {
                self.evaluate_action_call(name, arguments, *line, *column, Rc::clone(&env))
                    .await
            }

            Expression::MemberAccess {
                object,
                property,
                line,
                column,
            } => {
                self.evaluate_member_access(object, property, *line, *column, Rc::clone(&env))
                    .await
            }

            Expression::IndexAccess {
                collection,
                index,
                line,
                column,
            } => {
                self.evaluate_index_access(collection, index, *line, *column, Rc::clone(&env))
                    .await
            }

            Expression::Concatenation {
                left,
                right,
                line,
                column,
            } => {
                self.evaluate_concatenation(left, right, *line, *column, Rc::clone(&env))
                    .await
            }

            Expression::PatternMatch {
                text,
                pattern,
                line,
                column,
            } => {
                self.evaluate_pattern_match(text, pattern, *line, *column, Rc::clone(&env))
                    .await
            }

            Expression::PatternFind {
                text,
                pattern,
                line,
                column,
            } => {
                self.evaluate_pattern_find(text, pattern, *line, *column, Rc::clone(&env))
                    .await
            }

            Expression::PatternReplace {
                text,
                pattern,
                replacement,
                line,
                column,
            } => {
                self.evaluate_pattern_replace(
                    text,
                    pattern,
                    replacement,
                    *line,
                    *column,
                    Rc::clone(&env),
                )
                .await
            }

            Expression::PatternSplit {
                text,
                pattern,
                line,
                column,
            } => {
                self.evaluate_pattern_split(text, pattern, *line, *column, Rc::clone(&env))
                    .await
            }
            Expression::StringSplit {
                text,
                delimiter,
                line,
                column,
            } => {
                self.evaluate_string_split(text, delimiter, *line, *column, Rc::clone(&env))
                    .await
            }
            Expression::FileExists { path, line, column } => {
                self.evaluate_file_exists(path, *line, *column, Rc::clone(&env))
                    .await
            }
            Expression::DirectoryExists { path, line, column } => {
                self.evaluate_directory_exists(path, *line, *column, Rc::clone(&env))
                    .await
            }
            Expression::ListFiles { path, line, column } => {
                self.evaluate_list_files(path, *line, *column, Rc::clone(&env))
                    .await
            }
            Expression::ReadContent {
                file_handle,
                line,
                column,
            } => {
                self.evaluate_read_content(file_handle, *line, *column, Rc::clone(&env))
                    .await
            }
            Expression::ListFilesRecursive {
                path,
                extensions,
                line,
                column,
            } => {
                self.evaluate_list_files_recursive(
                    path,
                    extensions.as_ref(),
                    *line,
                    *column,
                    Rc::clone(&env),
                )
                .await
            }
            Expression::ListFilesFiltered {
                path,
                extensions,
                line,
                column,
            } => {
                self.evaluate_list_files_filtered(path, extensions, *line, *column, Rc::clone(&env))
                    .await
            }
            Expression::HeaderAccess {
                header_name,
                request,
                line,
                column,
            } => {
                self.evaluate_header_access(header_name, request, *line, *column, Rc::clone(&env))
                    .await
            }
            Expression::CurrentTimeMilliseconds { line, column } => {
                self.evaluate_current_time_milliseconds(*line, *column, Rc::clone(&env))
            }
            Expression::CurrentTimeFormatted {
                format,
                line,
                column,
            } => self.evaluate_current_time_formatted(format, *line, *column, Rc::clone(&env)),
            Expression::ProcessRunning {
                process_id,
                line,
                column,
            } => {
                self.evaluate_process_running(process_id, *line, *column, Rc::clone(&env))
                    .await
            }
        };
        self.assert_invariants();
        result
    }

    pub(crate) async fn call_function(
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

    // Helper method to create container instance with inheritance
    #[allow(clippy::only_used_in_recursion)]
    pub(crate) fn create_container_instance_with_inheritance(
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
}

#[cfg(test)]
mod process_tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_simple_command() {
        let config = Arc::new(WflConfig::default());
        let client = IoClient::new(config);

        // Use safe argument-based execution (no shell)
        let result = client
            .execute_command("echo", &["hello"], false, 0, 0)
            .await;

        assert!(result.is_ok(), "Failed to execute command");
        let (stdout, stderr, exit_code) = result.unwrap();
        assert!(stdout.contains("hello"), "Output should contain 'hello'");
        assert_eq!(exit_code, 0, "Exit code should be 0 for successful command");
        assert!(
            stderr.is_empty() || stderr.trim().is_empty(),
            "Stderr should be empty"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_spawn_and_kill_process() {
        let config = Arc::new(WflConfig::default());
        let client = IoClient::new(config);

        // Unix-specific test using sleep command
        let proc_id = client
            .spawn_process("sleep", &["10"], false, 0, 0)
            .await
            .expect("Failed to spawn process");

        // Check that process is running
        assert!(
            client.is_process_running(&proc_id).await,
            "Process should be running"
        );

        // Kill the process
        client
            .kill_process(&proc_id)
            .await
            .expect("Failed to kill process");

        // Give it time to terminate
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Process should no longer be running
        assert!(
            !client.is_process_running(&proc_id).await,
            "Process should not be running after kill"
        );
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn test_spawn_and_kill_process() {
        let config = Arc::new(WflConfig::default());
        let client = IoClient::new(config);

        // Windows-specific test using timeout command
        let proc_id = client
            .spawn_process("timeout", &["10"], false, 0, 0)
            .await
            .expect("Failed to spawn process");

        // Check that process is running
        assert!(
            client.is_process_running(&proc_id).await,
            "Process should be running"
        );

        // Kill the process
        client
            .kill_process(&proc_id)
            .await
            .expect("Failed to kill process");

        // Give it time to terminate
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Process should no longer be running
        assert!(
            !client.is_process_running(&proc_id).await,
            "Process should not be running after kill"
        );
    }

    #[tokio::test]
    async fn test_capture_process_output() {
        let config = Arc::new(WflConfig::default());
        let client = IoClient::new(config);

        // Use shell command that works cross-platform (no args = shell execution)
        let proc_id = client
            .spawn_process("echo", &["test output"], false, 0, 0)
            .await
            .expect("Failed to spawn process");

        // Give process time to complete and output to be captured
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let output = client
            .read_process_output(&proc_id)
            .await
            .expect("Failed to read process output");

        assert!(
            output.contains("test output"),
            "Output should contain 'test output'"
        );
    }

    #[tokio::test]
    async fn test_wait_for_process_completion() {
        let config = Arc::new(WflConfig::default());
        let client = IoClient::new(config);

        // Use shell command that works cross-platform (no args = shell execution)
        let proc_id = client
            .spawn_process("echo", &["done"], false, 0, 0)
            .await
            .expect("Failed to spawn process");

        let exit_code = client
            .wait_for_process(&proc_id)
            .await
            .expect("Failed to wait for process");

        assert_eq!(exit_code, 0, "Process should exit with code 0");
    }

    #[tokio::test]
    async fn test_command_not_found() {
        let config = Arc::new(WflConfig::default());
        let client = IoClient::new(config);

        // With shell execution, the shell runs successfully but reports command not found
        // So we check for non-zero exit code or error in stderr
        let result = client
            .execute_command("nonexistent_command_xyz_123", &[], false, 0, 0)
            .await;

        // Shell execution succeeds, but command fails
        if let Ok((_stdout, stderr, exit_code)) = result {
            // Either non-zero exit code or error message in stderr
            assert!(
                exit_code != 0 || stderr.contains("not found") || stderr.contains("not recognized"),
                "Should indicate command failure - exit_code: {}, stderr: {}",
                exit_code,
                stderr
            );
        } else {
            // Or direct execution might fail
            assert!(result.is_err(), "Should fail when command doesn't exist");
        }
    }

    #[tokio::test]
    async fn test_invalid_process_id() {
        let config = Arc::new(WflConfig::default());
        let client = IoClient::new(config);

        // Test invalid process ID handling
        let result = client.read_process_output("invalid_proc_id").await;

        assert!(result.is_err(), "Should fail for invalid process ID");
        let err = result.unwrap_err();
        assert!(
            err.contains("Invalid process ID"),
            "Error should indicate invalid process ID: {}",
            err
        );
    }
}
