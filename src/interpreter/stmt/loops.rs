use std::cell::RefCell;
use std::rc::Rc;

use crate::interpreter::Interpreter;
use crate::interpreter::control_flow::ControlFlow;
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use crate::parser::ast::{Expression, Statement};

pub trait LoopExecutor {
    async fn execute_count_loop(
        &self,
        start: &Expression,
        end: &Expression,
        step: Option<&Expression>,
        downward: bool,
        variable_name: Option<&String>,
        body: &[Statement],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_foreach_loop(
        &self,
        item_name: &str,
        collection: &Expression,
        reversed: bool,
        body: &[Statement],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_while_loop(
        &self,
        condition: &Expression,
        body: &[Statement],
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_repeat_until_loop(
        &self,
        condition: &Expression,
        body: &[Statement],
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_repeat_while_loop(
        &self,
        condition: &Expression,
        body: &[Statement],
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_forever_loop(
        &self,
        body: &[Statement],
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_main_loop(
        &self,
        body: &[Statement],
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;
}

impl LoopExecutor for Interpreter {
    async fn execute_count_loop(
        &self,
        start: &Expression,
        end: &Expression,
        step: Option<&Expression>,
        downward: bool,
        variable_name: Option<&String>,
        body: &[Statement],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
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
                    line,
                    column,
                ));
            }
        };

        let step_num = if let Some(step_expr) = step {
            match self.evaluate_expression(step_expr, Rc::clone(&env)).await? {
                Value::Number(n) => n,
                _ => {
                    return Err(RuntimeError::new(
                        "Count loop step must be a number".to_string(),
                        line,
                        column,
                    ));
                }
            }
        } else {
            1.0
        };

        let mut count = start_num;

        let should_continue: Box<dyn Fn(f64, f64) -> bool> = if downward {
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
        let loop_var_name = variable_name.map(|s| s.as_str()).unwrap_or("count");

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
                        crate::exec_trace!("Breaking out of count loop");
                        break;
                    }
                    ControlFlow::Continue => {
                        #[cfg(debug_assertions)]
                        crate::exec_trace!("Continuing count loop");
                    }
                    ControlFlow::Exit => {
                        #[cfg(debug_assertions)]
                        crate::exec_trace!("Exiting from count loop");
                        *self.current_count.borrow_mut() = previous_count;
                        *self.in_count_loop.borrow_mut() = was_in_count_loop;
                        return Ok((Value::Null, ControlFlow::Exit));
                    }
                    ControlFlow::Return(val) => {
                        #[cfg(debug_assertions)]
                        crate::exec_trace!("Returning from count loop with value: {:?}", val);
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

            if downward {
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
                line,
                column,
            ));
        }

        Ok((Value::Null, ControlFlow::None))
    }

    async fn execute_foreach_loop(
        &self,
        item_name: &str,
        collection: &Expression,
        reversed: bool,
        body: &[Statement],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let collection_val = self
            .evaluate_expression(collection, Rc::clone(&env))
            .await?;

        match collection_val {
            Value::List(list_rc) => {
                let items: Vec<Value> = {
                    let list = list_rc.borrow();
                    let indices: Vec<usize> = if reversed {
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
                        Err(msg) => return Err(RuntimeError::new(msg, line, column)),
                    }
                    let result = self.execute_block(body, Rc::clone(&loop_env)).await?;

                    match result.1 {
                        ControlFlow::Break => {
                            #[cfg(debug_assertions)]
                            crate::exec_trace!("Breaking out of foreach loop");
                            break;
                        }
                        ControlFlow::Continue => {
                            #[cfg(debug_assertions)]
                            crate::exec_trace!("Continuing foreach loop");
                            continue;
                        }
                        ControlFlow::Exit => {
                            #[cfg(debug_assertions)]
                            crate::exec_trace!("Exiting from foreach loop");
                            return Ok((Value::Null, ControlFlow::Exit));
                        }
                        ControlFlow::Return(val) => {
                            #[cfg(debug_assertions)]
                            crate::exec_trace!("Returning from foreach loop with value: {:?}", val);
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
                        Err(msg) => return Err(RuntimeError::new(msg, line, column)),
                    }
                    let result = self.execute_block(body, Rc::clone(&loop_env)).await?;

                    match result.1 {
                        ControlFlow::Break => {
                            #[cfg(debug_assertions)]
                            crate::exec_trace!("Breaking out of foreach loop (object)");
                            break;
                        }
                        ControlFlow::Continue => {
                            #[cfg(debug_assertions)]
                            crate::exec_trace!("Continuing foreach loop (object)");
                            continue;
                        }
                        ControlFlow::Exit => {
                            #[cfg(debug_assertions)]
                            crate::exec_trace!("Exiting from foreach loop (object)");
                            return Ok((Value::Null, ControlFlow::Exit));
                        }
                        ControlFlow::Return(val) => {
                            #[cfg(debug_assertions)]
                            crate::exec_trace!("Returning from foreach loop with value: {:?}", val);
                            return Ok((val.clone(), ControlFlow::Return(val)));
                        }
                        ControlFlow::None => {}
                    }
                }
            }
            _ => {
                return Err(RuntimeError::new(
                    format!("Cannot iterate over {}", collection_val.type_name()),
                    line,
                    column,
                ));
            }
        }

        Ok((Value::Null, ControlFlow::None))
    }

    async fn execute_while_loop(
        &self,
        condition: &Expression,
        body: &[Statement],
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
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
                    crate::exec_trace!("Breaking out of while loop");
                    break;
                }
                ControlFlow::Continue => {
                    #[cfg(debug_assertions)]
                    crate::exec_trace!("Continuing while loop");
                    continue;
                }
                ControlFlow::Exit => {
                    #[cfg(debug_assertions)]
                    crate::exec_trace!("Exiting from while loop");
                    return Ok((_last_value, ControlFlow::Exit));
                }
                ControlFlow::Return(val) => {
                    #[cfg(debug_assertions)]
                    crate::exec_trace!("Returning from while loop with value: {:?}", val);
                    return Ok((val.clone(), ControlFlow::Return(val)));
                }
                ControlFlow::None => {}
            }
        }

        Ok((_last_value, ControlFlow::None))
    }

    async fn execute_repeat_until_loop(
        &self,
        condition: &Expression,
        body: &[Statement],
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let mut _last_value = Value::Null;

        loop {
            self.check_time()?;
            let result = self.execute_block(body, Rc::clone(&env)).await?;
            _last_value = result.0;

            match result.1 {
                ControlFlow::Break => {
                    #[cfg(debug_assertions)]
                    crate::exec_trace!("Breaking out of repeat-until loop");
                    break;
                }
                ControlFlow::Continue => {
                    #[cfg(debug_assertions)]
                    crate::exec_trace!("Continuing repeat-until loop");
                }
                ControlFlow::Exit => {
                    #[cfg(debug_assertions)]
                    crate::exec_trace!("Exiting from repeat-until loop");
                    return Ok((_last_value, ControlFlow::Exit));
                }
                ControlFlow::Return(val) => {
                    #[cfg(debug_assertions)]
                    crate::exec_trace!("Returning from repeat-until loop with value: {:?}", val);
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

    async fn execute_repeat_while_loop(
        &self,
        condition: &Expression,
        body: &[Statement],
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
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
                    crate::exec_trace!("Breaking out of repeat-while loop");
                    break;
                }
                ControlFlow::Continue => {
                    #[cfg(debug_assertions)]
                    crate::exec_trace!("Continuing repeat-while loop");
                    continue;
                }
                ControlFlow::Exit => {
                    #[cfg(debug_assertions)]
                    crate::exec_trace!("Exiting from repeat-while loop");
                    return Ok((_last_value, ControlFlow::Exit));
                }
                ControlFlow::Return(val) => {
                    #[cfg(debug_assertions)]
                    crate::exec_trace!("Returning from repeat-while loop");
                    return Ok((val.clone(), ControlFlow::Return(val)));
                }
                ControlFlow::None => {}
            }
        }

        Ok((Value::Null, ControlFlow::None))
    }

    async fn execute_forever_loop(
        &self,
        body: &[Statement],
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        #[cfg(debug_assertions)]
        crate::exec_trace!("Executing forever loop");

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
                    crate::exec_trace!("Breaking out of forever loop");
                    break;
                }
                ControlFlow::Continue => {
                    #[cfg(debug_assertions)]
                    crate::exec_trace!("Continuing forever loop");
                    continue;
                }
                ControlFlow::Exit => {
                    #[cfg(debug_assertions)]
                    crate::exec_trace!("Exiting from forever loop");
                    return Ok((_last_value, ControlFlow::Exit));
                }
                ControlFlow::Return(val) => {
                    #[cfg(debug_assertions)]
                    crate::exec_trace!("Returning from forever loop with value: {:?}", val);
                    return Ok((val.clone(), ControlFlow::Return(val)));
                }
                ControlFlow::None => {}
            }
        }

        Ok((_last_value, ControlFlow::None))
    }

    async fn execute_main_loop(
        &self,
        body: &[Statement],
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        #[cfg(debug_assertions)]
        crate::exec_trace!("Executing main loop (timeout disabled)");

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
                    crate::exec_trace!("Breaking out of main loop");
                    break;
                }
                ControlFlow::Continue => {
                    #[cfg(debug_assertions)]
                    crate::exec_trace!("Continuing main loop");
                    continue;
                }
                ControlFlow::Exit => {
                    #[cfg(debug_assertions)]
                    crate::exec_trace!("Exiting from main loop");
                    *self.in_main_loop.borrow_mut() = false;
                    return Ok((_last_value, ControlFlow::Exit));
                }
                ControlFlow::Return(val) => {
                    #[cfg(debug_assertions)]
                    crate::exec_trace!("Returning from main loop with value: {:?}", val);
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
}
