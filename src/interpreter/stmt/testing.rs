use std::cell::RefCell;
use std::rc::Rc;

use crate::interpreter::Interpreter;
use crate::interpreter::control_flow::ControlFlow;
use crate::interpreter::environment::Environment;
use crate::interpreter::error::{ErrorKind, RuntimeError};
use crate::interpreter::test_results::TestResult;
use crate::interpreter::value::Value;
use crate::parser::ast::{Assertion, Expression, Statement};

pub trait TestExecutor {
    async fn execute_describe_block(
        &self,
        description: &str,
        setup: Option<&[Statement]>,
        teardown: Option<&[Statement]>,
        tests: &[Statement],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_test_block(
        &self,
        description: &str,
        body: &[Statement],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_expect(
        &self,
        subject: &Expression,
        assertion: &Assertion,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;
}

impl TestExecutor for Interpreter {
    async fn execute_describe_block(
        &self,
        description: &str,
        setup: Option<&[Statement]>,
        teardown: Option<&[Statement]>,
        tests: &[Statement],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // Create a new environment for this block
        let block_env = Environment::new_child_env(&env);

        // Push current suite context
        self.current_describe_stack
            .borrow_mut()
            .push(description.to_string());

        // Execute tests
        for test in tests {
            // Run setup if present (before each test/describe)
            if let Some(setup_stmts) = setup {
                for stmt in setup_stmts {
                    self.execute_statement(stmt, Rc::clone(&block_env)).await?;
                }
            }

            // Run the test/describe
            self.execute_statement(test, Rc::clone(&block_env)).await?;

            // Run teardown if present (after each test/describe)
            if let Some(teardown_stmts) = teardown {
                for stmt in teardown_stmts {
                    self.execute_statement(stmt, Rc::clone(&block_env)).await?;
                }
            }
        }

        // Pop suite context
        self.current_describe_stack.borrow_mut().pop();

        Ok((Value::Null, ControlFlow::None))
    }

    async fn execute_test_block(
        &self,
        description: &str,
        body: &[Statement],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // Create a new environment for this test
        let test_env = Environment::new_child_env(&env);

        let start_time = std::time::Instant::now();
        let mut error = None;

        // Execute test body
        for stmt in body {
            if let Err(e) = self.execute_statement(stmt, Rc::clone(&test_env)).await {
                error = Some(e);
                break;
            }
        }

        let duration = start_time.elapsed();

        // Record result
        if let Some(e) = error {
            let context = self.current_describe_stack.borrow().clone();
            self.test_results.borrow_mut().add_result(TestResult::fail(
                description,
                e.message,
                duration,
                context,
                line,
                column,
            ));
            // We don't propagate the error in test mode, we just record it
            // Unless it's a fatal error? For now, we swallow it to let other tests run.
        } else {
            self.test_results
                .borrow_mut()
                .add_result(TestResult::pass(description, duration));
        }

        Ok((Value::Null, ControlFlow::None))
    }

    async fn execute_expect(
        &self,
        subject: &Expression,
        assertion: &Assertion,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let subject_val = self.evaluate_expression(subject, Rc::clone(&env)).await?;

        let result = match assertion {
            Assertion::Equal(expected) | Assertion::Be(expected) => {
                let expected_val = self.evaluate_expression(expected, Rc::clone(&env)).await?;
                if subject_val == expected_val {
                    Ok(())
                } else {
                    Err(format!(
                        "Expected {:?}, but got {:?}",
                        expected_val, subject_val
                    ))
                }
            }
            Assertion::GreaterThan(expected) => {
                let expected_val = self.evaluate_expression(expected, Rc::clone(&env)).await?;
                match (&subject_val, &expected_val) {
                    (Value::Number(a), Value::Number(b)) => {
                        if a > b {
                            Ok(())
                        } else {
                            Err(format!("Expected {:?} > {:?}", a, b))
                        }
                    }
                    _ => Err("GreaterThan requires numbers".to_string()),
                }
            }
            Assertion::LessThan(expected) => {
                let expected_val = self.evaluate_expression(expected, Rc::clone(&env)).await?;
                match (&subject_val, &expected_val) {
                    (Value::Number(a), Value::Number(b)) => {
                        if a < b {
                            Ok(())
                        } else {
                            Err(format!("Expected {:?} < {:?}", a, b))
                        }
                    }
                    _ => Err("LessThan requires numbers".to_string()),
                }
            }
            Assertion::BeYes => {
                if subject_val.is_truthy() {
                    Ok(())
                } else {
                    Err(format!("Expected {:?} to be yes (truthy)", subject_val))
                }
            }
            Assertion::BeNo => {
                if !subject_val.is_truthy() {
                    Ok(())
                } else {
                    Err(format!("Expected {:?} to be no (falsy)", subject_val))
                }
            }
            Assertion::Exist => match subject_val {
                Value::Nothing => Err("Expected value to exist, but got nothing".to_string()),
                _ => Ok(()),
            },
            Assertion::Contain(expected) => {
                let expected_val = self.evaluate_expression(expected, Rc::clone(&env)).await?;
                match &subject_val {
                    Value::List(list) => {
                        let list = list.borrow();
                        if list.contains(&expected_val) {
                            Ok(())
                        } else {
                            Err(format!("Expected list to contain {:?}", expected_val))
                        }
                    }
                    Value::Text(text) => match expected_val {
                        Value::Text(sub) => {
                            if text.contains(sub.as_ref()) {
                                Ok(())
                            } else {
                                Err(format!("Expected text to contain {:?}", sub))
                            }
                        }
                        _ => Err("Contain on text requires text argument".to_string()),
                    },
                    _ => Err("Contain requires list or text subject".to_string()),
                }
            }
            Assertion::BeEmpty => match &subject_val {
                Value::List(list) => {
                    if list.borrow().is_empty() {
                        Ok(())
                    } else {
                        Err(format!(
                            "Expected list to be empty, got size {}",
                            list.borrow().len()
                        ))
                    }
                }
                Value::Text(text) => {
                    if text.is_empty() {
                        Ok(())
                    } else {
                        Err(format!("Expected text to be empty, got '{}'", text))
                    }
                }
                _ => Err("BeEmpty requires list or text".to_string()),
            },
            Assertion::HaveLength(expected) => {
                let expected_val = self.evaluate_expression(expected, Rc::clone(&env)).await?;
                let length = match &subject_val {
                    Value::List(list) => list.borrow().len(),
                    Value::Text(text) => text.len(),
                    _ => {
                        return Err(RuntimeError::new(
                            "HaveLength requires list or text".to_string(),
                            line,
                            column,
                        ));
                    }
                };

                match expected_val {
                    Value::Number(n) => {
                        if length == n as usize {
                            Ok(())
                        } else {
                            Err(format!("Expected length {}, got {}", n, length))
                        }
                    }
                    _ => Err("HaveLength requires number".to_string()),
                }
            }
            Assertion::BeOfType(type_name) => {
                let actual_type = subject_val.type_name();
                if actual_type == *type_name {
                    Ok(())
                } else {
                    Err(format!("Expected type {}, got {}", type_name, actual_type))
                }
            }
        };

        match result {
            Ok(_) => Ok((Value::Null, ControlFlow::None)),
            Err(msg) => Err(RuntimeError::new(msg, line, column)),
        }
    }
}
