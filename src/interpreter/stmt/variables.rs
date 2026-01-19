use std::cell::RefCell;
use std::rc::Rc;

use crate::interpreter::control_flow::ControlFlow;
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use crate::interpreter::Interpreter;
use crate::parser::ast::Expression;

pub trait VariableExecutor {
    async fn execute_variable_declaration(
        &self,
        name: &str,
        value: &Expression,
        is_constant: bool,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_assignment(
        &self,
        name: &str,
        value: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;
}

impl VariableExecutor for Interpreter {
    async fn execute_variable_declaration(
        &self,
        name: &str,
        value: &Expression,
        is_constant: bool,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let mut evaluated_value = self.evaluate_expression(value, Rc::clone(&env)).await?;

        if let Value::Text(text) = &evaluated_value
            && text.as_ref() == "[]"
        {
            evaluated_value = Value::List(Rc::new(RefCell::new(Vec::new())));
        }

        #[cfg(debug_assertions)]
        crate::exec_var_declare!(name, &evaluated_value);

        let result = if is_constant {
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

    async fn execute_assignment(
        &self,
        name: &str,
        value: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let value = self.evaluate_expression(value, Rc::clone(&env)).await?;
        #[cfg(debug_assertions)]
        crate::exec_var_assign!(name, &value);
        match env.borrow_mut().assign(name, value.clone()) {
            Ok(_) => Ok((Value::Null, ControlFlow::None)),
            Err(msg) => Err(RuntimeError::new(msg, line, column)),
        }
    }
}
