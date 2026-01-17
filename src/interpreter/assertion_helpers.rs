//! Assertion helper methods for the test framework

use super::*;

impl Interpreter {
    /// Check if an assertion passes
    pub(super) async fn check_assertion(
        &self,
        subject: &Value,
        assertion: &Assertion,
        env: Rc<RefCell<Environment>>,
    ) -> Result<bool, RuntimeError> {
        match assertion {
            Assertion::Equal(expected_expr) | Assertion::Be(expected_expr) => {
                let expected = self.evaluate_expression(expected_expr, env).await?;
                Ok(values_equal(subject, &expected))
            }
            Assertion::GreaterThan(expected_expr) => {
                let expected = self.evaluate_expression(expected_expr, env).await?;
                match (subject, &expected) {
                    (Value::Number(a), Value::Number(b)) => Ok(a > b),
                    _ => Ok(false),
                }
            }
            Assertion::LessThan(expected_expr) => {
                let expected = self.evaluate_expression(expected_expr, env).await?;
                match (subject, &expected) {
                    (Value::Number(a), Value::Number(b)) => Ok(a < b),
                    _ => Ok(false),
                }
            }
            Assertion::BeYes => Ok(is_truthy(subject)),
            Assertion::BeNo => Ok(!is_truthy(subject)),
            Assertion::Exist => Ok(!matches!(subject, Value::Null)),
            Assertion::Contain(item_expr) => {
                let item = self.evaluate_expression(item_expr, env).await?;
                match subject {
                    Value::List(list) => {
                        let list_ref = list.borrow();
                        Ok(list_ref.iter().any(|v| values_equal(v, &item)))
                    }
                    Value::Text(text) => {
                        if let Value::Text(search) = &item {
                            Ok(text.contains(search.as_ref()))
                        } else {
                            Ok(false)
                        }
                    }
                    _ => Ok(false),
                }
            }
            Assertion::BeEmpty => match subject {
                Value::List(list) => Ok(list.borrow().is_empty()),
                Value::Text(text) => Ok(text.is_empty()),
                _ => Ok(false),
            },
            Assertion::HaveLength(expected_expr) => {
                let expected = self.evaluate_expression(expected_expr, env).await?;
                if let Value::Number(expected_len) = expected {
                    let actual_len = match subject {
                        Value::List(list) => list.borrow().len() as f64,
                        Value::Text(text) => text.len() as f64,
                        _ => return Ok(false),
                    };
                    Ok((actual_len - expected_len).abs() < f64::EPSILON)
                } else {
                    Ok(false)
                }
            }
            Assertion::BeOfType(type_name) => {
                let actual_type = subject.type_name();
                Ok(actual_type.eq_ignore_ascii_case(type_name))
            }
        }
    }

    /// Create a helpful assertion failure message
    pub(super) fn create_assertion_message(&self, assertion: &Assertion, subject: &Value) -> String {
        match assertion {
            Assertion::Equal(expr) | Assertion::Be(expr) => {
                format!("Expected value to equal {:?}, but got {:?}", expr, subject)
            }
            Assertion::GreaterThan(expr) => {
                format!("Expected {:?} to be greater than {:?}", subject, expr)
            }
            Assertion::LessThan(expr) => {
                format!("Expected {:?} to be less than {:?}", subject, expr)
            }
            Assertion::BeYes => {
                format!("Expected {:?} to be truthy", subject)
            }
            Assertion::BeNo => {
                format!("Expected {:?} to be falsy", subject)
            }
            Assertion::Exist => {
                format!("Expected value to exist, but got {:?}", subject)
            }
            Assertion::Contain(expr) => {
                format!("Expected {:?} to contain {:?}", subject, expr)
            }
            Assertion::BeEmpty => {
                format!("Expected {:?} to be empty", subject)
            }
            Assertion::HaveLength(expr) => {
                format!("Expected {:?} to have length {:?}", subject, expr)
            }
            Assertion::BeOfType(type_name) => {
                format!(
                    "Expected type {}, but got {}",
                    type_name,
                    subject.type_name()
                )
            }
        }
    }
}

/// Helper function to check if two values are equal
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => (a - b).abs() < f64::EPSILON,
        (Value::Text(a), Value::Text(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Null, Value::Null) => true,
        (Value::List(a), Value::List(b)) => {
            let a_ref = a.borrow();
            let b_ref = b.borrow();
            if a_ref.len() != b_ref.len() {
                return false;
            }
            a_ref.iter().zip(b_ref.iter()).all(|(x, y)| values_equal(x, y))
        }
        _ => false,
    }
}

/// Helper function to check if a value is truthy
fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Bool(b) => *b,
        Value::Null => false,
        Value::Number(n) => *n != 0.0,
        Value::Text(s) => !s.is_empty(),
        Value::List(l) => !l.borrow().is_empty(),
        _ => true,
    }
}
