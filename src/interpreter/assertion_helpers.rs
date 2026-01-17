//! Assertion helper methods for the test framework

use super::*;

impl Interpreter {
    /// Check if an assertion passes and return the expected value for error messages
    pub(super) async fn check_assertion(
        &self,
        subject: &Value,
        assertion: &Assertion,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(bool, Option<Value>), RuntimeError> {
        match assertion {
            Assertion::Equal(expected_expr) | Assertion::Be(expected_expr) => {
                let expected = self.evaluate_expression(expected_expr, env).await?;
                Ok((values_equal(subject, &expected), Some(expected)))
            }
            Assertion::GreaterThan(expected_expr) => {
                let expected = self.evaluate_expression(expected_expr, env).await?;
                let result = match (subject, &expected) {
                    (Value::Number(a), Value::Number(b)) => a > b,
                    _ => false,
                };
                Ok((result, Some(expected)))
            }
            Assertion::LessThan(expected_expr) => {
                let expected = self.evaluate_expression(expected_expr, env).await?;
                let result = match (subject, &expected) {
                    (Value::Number(a), Value::Number(b)) => a < b,
                    _ => false,
                };
                Ok((result, Some(expected)))
            }
            Assertion::BeYes => Ok((is_truthy(subject), None)),
            Assertion::BeNo => Ok((!is_truthy(subject), None)),
            Assertion::Exist => Ok((!matches!(subject, Value::Null | Value::Nothing), None)),
            Assertion::Contain(item_expr) => {
                let item = self.evaluate_expression(item_expr, env).await?;
                let result = match subject {
                    Value::List(list) => {
                        let list_ref = list.borrow();
                        list_ref.iter().any(|v| values_equal(v, &item))
                    }
                    Value::Text(text) => {
                        if let Value::Text(search) = &item {
                            text.contains(search.as_ref())
                        } else {
                            false
                        }
                    }
                    _ => false,
                };
                Ok((result, Some(item)))
            }
            Assertion::BeEmpty => {
                let result = match subject {
                    Value::List(list) => list.borrow().is_empty(),
                    Value::Text(text) => text.is_empty(),
                    _ => false,
                };
                Ok((result, None))
            }
            Assertion::HaveLength(expected_expr) => {
                let expected = self.evaluate_expression(expected_expr, env).await?;
                if let Value::Number(expected_len) = expected {
                    let actual_len = match subject {
                        Value::List(list) => list.borrow().len() as f64,
                        Value::Text(text) => text.chars().count() as f64, // Use character count for text length
                        _ => return Ok((false, Some(Value::Number(expected_len)))),
                    };
                    Ok((
                        (actual_len - expected_len).abs() < f64::EPSILON,
                        Some(Value::Number(expected_len)),
                    ))
                } else {
                    Ok((false, Some(expected)))
                }
            }
            Assertion::BeOfType(type_name) => {
                let actual_type = subject.type_name();
                Ok((actual_type.eq_ignore_ascii_case(type_name), None))
            }
        }
    }

    /// Create a helpful assertion failure message with actual values
    pub(super) fn create_assertion_message_with_values(
        &self,
        assertion: &Assertion,
        subject: &Value,
        expected_value: Option<&Value>,
    ) -> String {
        match assertion {
            Assertion::Equal(_) | Assertion::Be(_) => {
                if let Some(expected) = expected_value {
                    format!(
                        "Expected {} to equal {}",
                        self.format_value_for_message(subject),
                        self.format_value_for_message(expected)
                    )
                } else {
                    format!(
                        "Expected value to equal expected value, but got {}",
                        self.format_value_for_message(subject)
                    )
                }
            }
            Assertion::GreaterThan(_) => {
                if let Some(expected) = expected_value {
                    format!(
                        "Expected {} to be greater than {}, but it was not",
                        self.format_value_for_message(subject),
                        self.format_value_for_message(expected)
                    )
                } else {
                    format!(
                        "Expected {} to be greater than expected value",
                        self.format_value_for_message(subject)
                    )
                }
            }
            Assertion::LessThan(_) => {
                if let Some(expected) = expected_value {
                    format!(
                        "Expected {} to be less than {}, but it was not",
                        self.format_value_for_message(subject),
                        self.format_value_for_message(expected)
                    )
                } else {
                    format!(
                        "Expected {} to be less than expected value",
                        self.format_value_for_message(subject)
                    )
                }
            }
            Assertion::BeYes => {
                format!(
                    "Expected {} to be truthy, but it was falsy",
                    self.format_value_for_message(subject)
                )
            }
            Assertion::BeNo => {
                format!(
                    "Expected {} to be falsy, but it was truthy",
                    self.format_value_for_message(subject)
                )
            }
            Assertion::Exist => {
                format!(
                    "Expected value to exist, but got {}",
                    self.format_value_for_message(subject)
                )
            }
            Assertion::Contain(_) => {
                if let Some(item) = expected_value {
                    format!(
                        "Expected {} to contain {}, but it did not",
                        self.format_value_for_message(subject),
                        self.format_value_for_message(item)
                    )
                } else {
                    format!(
                        "Expected {} to contain expected item",
                        self.format_value_for_message(subject)
                    )
                }
            }
            Assertion::BeEmpty => {
                let actual_length = match subject {
                    Value::Text(s) => Some(s.chars().count()),
                    Value::List(list) => Some(list.borrow().len()),
                    _ => None,
                };
                match actual_length {
                    Some(len) => format!(
                        "Expected {} to be empty, but it has {} item{}",
                        self.format_value_for_message(subject),
                        len,
                        if len == 1 { "" } else { "s" }
                    ),
                    None => format!(
                        "Expected {} to be empty, but it is not applicable for this type",
                        self.format_value_for_message(subject)
                    ),
                }
            }
            Assertion::HaveLength(_) => {
                if let Some(Value::Number(expected_len)) = expected_value {
                    let actual_length = match subject {
                        Value::Text(s) => Some(s.chars().count()),
                        Value::List(list) => Some(list.borrow().len()),
                        _ => None,
                    };
                    match actual_length {
                        Some(len) => format!(
                            "Expected {} to have length {}, but its length is {}",
                            self.format_value_for_message(subject),
                            *expected_len as usize,
                            len
                        ),
                        None => format!(
                            "Expected {} to have length {}, but length is not applicable for this type",
                            self.format_value_for_message(subject),
                            *expected_len as usize
                        ),
                    }
                } else {
                    format!(
                        "Expected {} to have expected length",
                        self.format_value_for_message(subject)
                    )
                }
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

    /// Format a value for display in error messages
    fn format_value_for_message(&self, value: &Value) -> String {
        match value {
            Value::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Value::Text(s) => format!("\"{}\"", s),
            Value::Bool(b) => format!("{}", if *b { "yes" } else { "no" }),
            Value::List(list) => {
                let items = list.borrow();
                if items.is_empty() {
                    "empty list".to_string()
                } else {
                    format!(
                        "list with {} item{}",
                        items.len(),
                        if items.len() == 1 { "" } else { "s" }
                    )
                }
            }
            Value::Null => "null".to_string(),
            Value::Nothing => "nothing".to_string(),
            _ => format!("{}", value.type_name()),
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
        (Value::Nothing, Value::Nothing) => true,
        (Value::List(a), Value::List(b)) => {
            let a_ref = a.borrow();
            let b_ref = b.borrow();
            if a_ref.len() != b_ref.len() {
                return false;
            }
            a_ref
                .iter()
                .zip(b_ref.iter())
                .all(|(x, y)| values_equal(x, y))
        }
        _ => false,
    }
}

/// Helper function to check if a value is truthy
fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Bool(b) => *b,
        Value::Null => false,
        Value::Nothing => false,
        Value::Number(n) => *n != 0.0,
        Value::Text(s) => !s.is_empty(),
        Value::List(l) => !l.borrow().is_empty(),
        _ => true,
    }
}
