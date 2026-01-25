use std::cell::RefCell;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::interpreter::Interpreter;
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;

#[allow(async_fn_in_trait)]
pub trait TimeExpressionEvaluator {
    fn evaluate_current_time_milliseconds(
        &self,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;

    fn evaluate_current_time_formatted(
        &self,
        format: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;
}

impl TimeExpressionEvaluator for Interpreter {
    fn evaluate_current_time_milliseconds(
        &self,
        line: usize,
        column: usize,
        _env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(n) => Ok(Value::Number(n.as_millis() as f64)),
            Err(e) => Err(RuntimeError::new(
                format!("Failed to get system time: {}", e),
                line,
                column,
            )),
        }
    }

    fn evaluate_current_time_formatted(
        &self,
        format: &str,
        line: usize,
        column: usize,
        _env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        let now = chrono::Local::now();
        match std::panic::catch_unwind(|| now.format(format).to_string()) {
            Ok(formatted) => Ok(Value::Text(Rc::from(formatted))),
            Err(_) => Err(RuntimeError::new(
                format!("Invalid date format string: '{}'", format),
                line,
                column,
            )),
        }
    }
}
