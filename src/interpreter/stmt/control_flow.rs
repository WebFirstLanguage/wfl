use std::cell::RefCell;
use std::rc::Rc;

use crate::interpreter::Interpreter;
use crate::interpreter::control_flow::ControlFlow;
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
#[cfg(debug_assertions)]
use crate::logging::IndentGuard;
use crate::parser::ast::{Expression, Statement};

#[allow(async_fn_in_trait)]
pub trait ControlFlowExecutor {
    async fn execute_if(
        &self,
        condition: &Expression,
        then_block: &[Statement],
        else_block: Option<&Vec<Statement>>,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_single_line_if(
        &self,
        condition: &Expression,
        then_stmt: &Statement,
        else_stmt: Option<&Statement>,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_return(
        &self,
        value: Option<&Expression>,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_break(&self) -> Result<(Value, ControlFlow), RuntimeError>;
    async fn execute_continue(&self) -> Result<(Value, ControlFlow), RuntimeError>;
    async fn execute_exit(&self) -> Result<(Value, ControlFlow), RuntimeError>;
}

impl ControlFlowExecutor for Interpreter {
    async fn execute_if(
        &self,
        condition: &Expression,
        then_block: &[Statement],
        else_block: Option<&Vec<Statement>>,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let condition_value = self.evaluate_expression(condition, Rc::clone(&env)).await?;
        #[cfg(debug_assertions)]
        crate::exec_control_flow!("if condition", condition_value.is_truthy());

        if condition_value.is_truthy() {
            #[cfg(debug_assertions)]
            let _guard = IndentGuard::new();
            #[cfg(debug_assertions)]
            crate::exec_block_enter!("if branch");
            let result = self.execute_block(then_block, Rc::clone(&env)).await;
            #[cfg(debug_assertions)]
            crate::exec_block_exit!("if branch");
            result
        } else if let Some(else_stmts) = else_block {
            #[cfg(debug_assertions)]
            let _guard = IndentGuard::new();
            #[cfg(debug_assertions)]
            crate::exec_block_enter!("else branch");
            let result = self.execute_block(else_stmts, Rc::clone(&env)).await;
            #[cfg(debug_assertions)]
            crate::exec_block_exit!("else branch");
            result
        } else {
            Ok((Value::Null, ControlFlow::None))
        }
    }

    async fn execute_single_line_if(
        &self,
        condition: &Expression,
        then_stmt: &Statement,
        else_stmt: Option<&Statement>,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let condition_value = self.evaluate_expression(condition, Rc::clone(&env)).await?;

        if condition_value.is_truthy() {
            self.execute_statement(then_stmt, Rc::clone(&env)).await
        } else if let Some(else_stmt) = else_stmt {
            self.execute_statement(else_stmt, Rc::clone(&env)).await
        } else {
            Ok((Value::Null, ControlFlow::None))
        }
    }

    async fn execute_return(
        &self,
        value: Option<&Expression>,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        #[cfg(debug_assertions)]
        crate::exec_trace!("Executing return statement");

        if let Some(expr) = value {
            let result = self.evaluate_expression(expr, Rc::clone(&env)).await?;
            Ok((result.clone(), ControlFlow::Return(result)))
        } else {
            Ok((Value::Null, ControlFlow::Return(Value::Null)))
        }
    }

    async fn execute_break(&self) -> Result<(Value, ControlFlow), RuntimeError> {
        #[cfg(debug_assertions)]
        crate::exec_trace!("Executing break statement");
        Ok((Value::Null, ControlFlow::Break))
    }

    async fn execute_continue(&self) -> Result<(Value, ControlFlow), RuntimeError> {
        #[cfg(debug_assertions)]
        crate::exec_trace!("Executing continue statement");
        Ok((Value::Null, ControlFlow::Continue))
    }

    async fn execute_exit(&self) -> Result<(Value, ControlFlow), RuntimeError> {
        #[cfg(debug_assertions)]
        crate::exec_trace!("Executing exit statement");
        Ok((Value::Null, ControlFlow::Exit))
    }
}
