use std::cell::RefCell;
use std::rc::Rc;

use crate::interpreter::Interpreter;
use crate::interpreter::control_flow::ControlFlow;
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::{ContainerEventValue, FunctionValue, Value};
use crate::parser::ast::{Parameter, PatternExpression, Statement, Type};

#[allow(async_fn_in_trait)]
pub trait DefinitionsExecutor {
    #[allow(clippy::too_many_arguments)]
    async fn execute_function_definition(
        &self,
        name: &str,
        parameters: &[Parameter],
        body: &[Statement],
        return_type: Option<&Type>,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_pattern_definition(
        &self,
        name: &str,
        pattern: &PatternExpression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    #[allow(clippy::too_many_arguments)]
    async fn execute_action_definition(
        &self,
        name: &str,
        parameters: &[Parameter],
        body: &[Statement],
        return_type: Option<&Type>,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_event_definition(
        &self,
        name: &str,
        parameters: &[Parameter],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;
}

impl DefinitionsExecutor for Interpreter {
    async fn execute_function_definition(
        &self,
        name: &str,
        parameters: &[Parameter],
        body: &[Statement],
        _return_type: Option<&Type>,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // Convert Parameter to String for FunctionValue
        // FunctionValue currently stores params as Vec<String>, losing type info for now
        let param_names: Vec<String> = parameters.iter().map(|p| p.name.clone()).collect();

        let function = FunctionValue {
            name: Some(name.to_string()),
            params: param_names,
            body: body.to_vec(),
            env: Rc::downgrade(&env),
            line,
            column,
        };

        let val = Value::Function(Rc::new(function));

        match env.borrow_mut().define(name, val) {
            Ok(_) => Ok((Value::Null, ControlFlow::None)),
            Err(msg) => Err(RuntimeError::new(msg, line, column)),
        }
    }

    async fn execute_pattern_definition(
        &self,
        name: &str,
        pattern: &PatternExpression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // We need to compile the pattern expression into a CompiledPattern
        // But PatternExpression is AST, CompiledPattern is runtime.
        // The AST `PatternExpression` needs to be compiled.

        use crate::pattern::CompiledPattern;

        // We need to compile it here.
        // CompiledPattern::compile_with_env(pattern, &env.borrow())
        let env_borrow = env.borrow();
        match CompiledPattern::compile_with_env(pattern, &env_borrow) {
            Ok(compiled) => {
                let pattern_val = Value::Pattern(Rc::new(compiled));
                match env.borrow_mut().define(name, pattern_val) {
                    Ok(_) => Ok((Value::Null, ControlFlow::None)),
                    Err(msg) => Err(RuntimeError::new(msg, line, column)),
                }
            }
            Err(e) => Err(RuntimeError::new(
                format!("Failed to compile pattern: {}", e),
                line,
                column,
            )),
        }
    }

    async fn execute_action_definition(
        &self,
        name: &str,
        parameters: &[Parameter],
        body: &[Statement],
        return_type: Option<&Type>,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // Reuse function logic
        self.execute_function_definition(name, parameters, body, return_type, line, column, env)
            .await
    }

    async fn execute_event_definition(
        &self,
        name: &str,
        parameters: &[Parameter],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let param_names: Vec<String> = parameters.iter().map(|p| p.name.clone()).collect();

        let event_def = ContainerEventValue {
            name: name.to_string(),
            params: param_names,
            handlers: Vec::new(),
            line,
            column,
        };

        let val = Value::ContainerEvent(Rc::new(event_def));

        match env.borrow_mut().define(name, val) {
            Ok(_) => Ok((Value::Null, ControlFlow::None)),
            Err(msg) => Err(RuntimeError::new(msg, line, column)),
        }
    }
}
