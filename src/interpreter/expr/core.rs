use std::cell::RefCell;
use std::rc::Rc;

use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use crate::interpreter::Interpreter;
use crate::parser::ast::{Argument, Expression, Literal, Operator, UnaryOperator};

pub trait CoreExpressionEvaluator {
    async fn evaluate_binary_operation(
        &self,
        left: &Expression,
        operator: &Operator,
        right: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;

    async fn evaluate_unary_operation(
        &self,
        operator: &UnaryOperator,
        expression: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;

    async fn evaluate_function_call(
        &self,
        function: &Expression,
        arguments: &[Argument],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;

    async fn evaluate_action_call(
        &self,
        name: &str,
        arguments: &[Argument],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;

    async fn evaluate_member_access(
        &self,
        object: &Expression,
        property: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;

    async fn evaluate_index_access(
        &self,
        collection: &Expression,
        index: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;

    async fn evaluate_concatenation(
        &self,
        left: &Expression,
        right: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;

    // Helpers
    fn add(&self, left: Value, right: Value, line: usize, column: usize) -> Result<Value, RuntimeError>;
    fn subtract(&self, left: Value, right: Value, line: usize, column: usize) -> Result<Value, RuntimeError>;
    fn multiply(&self, left: Value, right: Value, line: usize, column: usize) -> Result<Value, RuntimeError>;
    fn divide(&self, left: Value, right: Value, line: usize, column: usize) -> Result<Value, RuntimeError>;
    fn modulo(&self, left: Value, right: Value, line: usize, column: usize) -> Result<Value, RuntimeError>;
    fn is_equal(&self, left: &Value, right: &Value) -> bool;
    fn greater_than(&self, left: Value, right: Value, line: usize, column: usize) -> Result<Value, RuntimeError>;
    fn less_than(&self, left: Value, right: Value, line: usize, column: usize) -> Result<Value, RuntimeError>;
    fn greater_than_equal(&self, left: Value, right: Value, line: usize, column: usize) -> Result<Value, RuntimeError>;
    fn less_than_equal(&self, left: Value, right: Value, line: usize, column: usize) -> Result<Value, RuntimeError>;
    fn contains(&self, left: Value, right: Value, line: usize, column: usize) -> Result<Value, RuntimeError>;
}

impl CoreExpressionEvaluator for Interpreter {
    async fn evaluate_binary_operation(
        &self,
        left: &Expression,
        operator: &Operator,
        right: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        let left_val = self.evaluate_expression(left, Rc::clone(&env)).await?;
        let right_val = self.evaluate_expression(right, Rc::clone(&env)).await?;

        match operator {
            Operator::Plus => self.add(left_val, right_val, line, column),
            Operator::Minus => self.subtract(left_val, right_val, line, column),
            Operator::Multiply => self.multiply(left_val, right_val, line, column),
            Operator::Divide => self.divide(left_val, right_val, line, column),
            Operator::Modulo => self.modulo(left_val, right_val, line, column),
            Operator::Equals => Ok(Value::Bool(self.is_equal(&left_val, &right_val))),
            Operator::NotEquals => Ok(Value::Bool(!self.is_equal(&left_val, &right_val))),
            Operator::GreaterThan => self.greater_than(left_val, right_val, line, column),
            Operator::LessThan => self.less_than(left_val, right_val, line, column),
            Operator::GreaterThanOrEqual => {
                self.greater_than_equal(left_val, right_val, line, column)
            }
            Operator::LessThanOrEqual => {
                self.less_than_equal(left_val, right_val, line, column)
            }
            Operator::And => Ok(Value::Bool(left_val.is_truthy() && right_val.is_truthy())),
            Operator::Or => Ok(Value::Bool(left_val.is_truthy() || right_val.is_truthy())),
            Operator::Contains => self.contains(left_val, right_val, line, column),
        }
    }

    async fn evaluate_unary_operation(
        &self,
        operator: &UnaryOperator,
        expression: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        let value = self.evaluate_expression(expression, Rc::clone(&env)).await?;

        match operator {
            UnaryOperator::Not => Ok(Value::Bool(!value.is_truthy())),
            UnaryOperator::Minus => match value {
                Value::Number(n) => Ok(Value::Number(-n)),
                _ => Err(RuntimeError::new(
                    format!("Cannot negate {}", value.type_name()),
                    line,
                    column,
                )),
            },
        }
    }

    async fn evaluate_function_call(
        &self,
        function: &Expression,
        arguments: &[Argument],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        let function_val = self.evaluate_expression(function, Rc::clone(&env)).await?;

        let mut arg_values = Vec::new();
        for arg in arguments {
            arg_values.push(
                self.evaluate_expression(&arg.value, Rc::clone(&env))
                    .await?,
            );
        }

        match function_val {
            Value::Function(func) => {
                self.call_function(&func, arg_values, line, column).await
            }
            Value::NativeFunction(_, native_fn) => {
                native_fn(arg_values.clone()).map_err(|e| {
                    RuntimeError::new(
                        format!("Error in native function: {e}"),
                        line,
                        column,
                    )
                })
            }
            _ => Err(RuntimeError::new(
                format!("Cannot call {}", function_val.type_name()),
                line,
                column,
            )),
        }
    }

    async fn evaluate_action_call(
        &self,
        name: &str,
        arguments: &[Argument],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        let function_val = env.borrow().get(name).ok_or_else(|| {
            RuntimeError::new(format!("Undefined action '{name}'"), line, column)
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

                self.call_function(&func, arg_values, line, column).await
            }
            _ => Err(RuntimeError::new(
                format!("'{name}' is not callable"),
                line,
                column,
            )),
        }
    }

    async fn evaluate_member_access(
        &self,
        object: &Expression,
        property: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        let object_val = self.evaluate_expression(object, Rc::clone(&env)).await?;

        match object_val {
            Value::Object(obj_rc) => {
                let obj = obj_rc.borrow();
                if let Some(value) = obj.get(property) {
                    Ok(value.clone())
                } else {
                    Err(RuntimeError::new(
                        format!("Object has no property '{property}'"),
                        line,
                        column,
                    ))
                }
            }
            _ => Err(RuntimeError::new(
                format!("Cannot access property of {}", object_val.type_name()),
                line,
                column,
            )),
        }
    }

    async fn evaluate_index_access(
        &self,
        collection: &Expression,
        index: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
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
                        line,
                        column,
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
                        line,
                        column,
                    ))
                }
            }
            (collection, index) => Err(RuntimeError::new(
                format!(
                    "Cannot index {} with {}",
                    collection.type_name(),
                    index.type_name()
                ),
                line,
                column,
            )),
        }
    }

    async fn evaluate_concatenation(
        &self,
        left: &Expression,
        right: &Expression,
        _line: usize,
        _column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        let left_val = self.evaluate_expression(left, Rc::clone(&env)).await?;
        let right_val = self.evaluate_expression(right, Rc::clone(&env)).await?;

        let result = format!("{left_val}{right_val}");
        Ok(Value::Text(Rc::from(result.as_str())))
    }

    // Helper implementations
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
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => {
                if b == 0.0 {
                    Err(RuntimeError::new(
                        "Division by zero".to_string(),
                        line,
                        column,
                    ))
                } else {
                    let result = a / b;
                    if !result.is_finite() {
                        return Err(RuntimeError::new(
                            format!("Division resulted in invalid number: {result}"),
                            line,
                            column,
                        ));
                    }
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

    fn modulo(
        &self,
        left: Value,
        right: Value,
        line: usize,
        column: usize,
    ) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => {
                if b == 0.0 {
                    Err(RuntimeError::new(
                        "Modulo by zero".to_string(),
                        line,
                        column,
                    ))
                } else {
                    let result = a % b;
                    if !result.is_finite() {
                        return Err(RuntimeError::new(
                            format!("Modulo resulted in invalid number: {result}"),
                            line,
                            column,
                        ));
                    }
                    Ok(Value::Number(result))
                }
            }
            (a, b) => Err(RuntimeError::new(
                format!(
                    "Cannot compute modulo of {} by {}",
                    a.type_name(),
                    b.type_name()
                ),
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
}
