use std::cell::RefCell;
use std::rc::Rc;

use crate::interpreter::Interpreter;
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use crate::parser::ast::Expression;

pub trait WebExpressionEvaluator {
    async fn evaluate_header_access(
        &self,
        header_name: &str,
        request: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;
}

impl WebExpressionEvaluator for Interpreter {
    async fn evaluate_header_access(
        &self,
        header_name: &str,
        request: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        let request_val = self.evaluate_expression(request, Rc::clone(&env)).await?;

        // Expect an object (the request object)
        if let Value::Object(obj) = &request_val {
            let obj_ref = obj.borrow();

            // Check if it has a 'headers' property that is also an object
            if let Some(Value::Object(headers_obj)) = obj_ref.get("headers") {
                let headers = headers_obj.borrow();
                // Case-insensitive lookup would be better, but map is case-sensitive
                // For now, assume headers are normalized or exact match
                // We could iterate if we want case insensitivity
                if let Some(val) = headers.get(header_name) {
                    return Ok(val.clone());
                } else {
                    // Try lowercase
                    if let Some(val) = headers.get(&header_name.to_lowercase()) {
                        return Ok(val.clone());
                    }
                    return Ok(Value::Null); // Header not found returns null/nothing
                }
            }
        }

        Err(RuntimeError::new(
            "Expected request object for header access".to_string(),
            line,
            column,
        ))
    }
}
