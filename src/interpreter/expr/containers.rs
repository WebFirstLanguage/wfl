use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::{FunctionValue, Value};
use crate::interpreter::Interpreter;
use crate::parser::ast::Argument;

pub trait ContainerExpressionEvaluator {
    async fn evaluate_static_member_access(
        &self,
        container: &str,
        member: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;

    async fn evaluate_method_call(
        &self,
        object: &Value,
        method: &str,
        arguments: &[Argument],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;

    async fn evaluate_property_access(
        &self,
        object: &Value,
        property: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;
}

impl ContainerExpressionEvaluator for Interpreter {
    async fn evaluate_static_member_access(
        &self,
        container: &str,
        member: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        // Look up the container definition
        match env.borrow().get(container) {
            Some(Value::ContainerDefinition(def)) => {
                // Look up static property
                // In ContainerDefinitionValue, static_properties is HashMap<String, Value>
                if let Some(val) = def.static_properties.get(member) {
                    Ok(val.clone())
                } else {
                    Err(RuntimeError::new(
                        format!("Static member '{member}' not found in container '{container}'"),
                        line,
                        column,
                    ))
                }
            }
            Some(v) => Err(RuntimeError::new(
                format!(
                    "'{container}' is not a container definition (got {})",
                    v.type_name()
                ),
                line,
                column,
            )),
            None => Err(RuntimeError::new(
                format!("Container '{container}' not found"),
                line,
                column,
            )),
        }
    }

    async fn evaluate_method_call(
        &self,
        object: &Value,
        method: &str,
        arguments: &[Argument],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        match object {
            Value::ContainerInstance(instance_rc) => {
                let instance = instance_rc.borrow();
                let container_type = instance.container_type.clone();

                // Look up the container definition
                let container_def = match env.borrow().get(&container_type) {
                    Some(Value::ContainerDefinition(def)) => def.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Container '{container_type}' not found"),
                            line,
                            column,
                        ));
                    }
                };

                // Look up the method in the container definition (or parents)
                let mut current_def = Some(container_def);
                let mut method_found = None;

                while let Some(def) = current_def {
                    if let Some(method_val) = def.methods.get(method) {
                        method_found = Some(method_val.clone());
                        break;
                    }

                    // Check parent
                    if let Some(parent_type) = &def.extends {
                        match env.borrow().get(parent_type) {
                            Some(Value::ContainerDefinition(parent_def)) => {
                                current_def = Some(parent_def.clone());
                            }
                            _ => current_def = None,
                        }
                    } else {
                        current_def = None;
                    }
                }

                if let Some(method_val) = method_found {
                    // Create a function value from the method
                    let function = FunctionValue {
                        name: Some(method_val.name.clone()),
                        params: method_val.params.clone(),
                        body: method_val.body.clone(),
                        env: method_val.env.clone(),
                        line: method_val.line,
                        column: method_val.column,
                    };

                    // Create a new environment for the method execution
                    let method_env = Environment::new_child_env(&env);

                    // Add 'this' to the environment
                    let _ = method_env
                        .borrow_mut()
                        .define("this", Value::ContainerInstance(Rc::clone(instance_rc)));

                    // Evaluate the arguments
                    let mut arg_values = Vec::with_capacity(arguments.len());
                    for arg in arguments {
                        let arg_val = self.evaluate_expression(&arg.value, Rc::clone(&env)).await?;
                        arg_values.push(arg_val);
                    }

                    // Call the function
                    self.call_function(&function, arg_values, line, column)
                        .await
                } else {
                    Err(RuntimeError::new(
                        format!(
                            "Method '{method}' not found in container '{container_type}'"
                        ),
                        line,
                        column,
                    ))
                }
            }
            Value::List(list) => {
                // List methods
                match method {
                    "size" | "length" | "count" => Ok(Value::Number(list.borrow().len() as f64)),
                    "contains" => {
                        if arguments.len() != 1 {
                            return Err(RuntimeError::new(
                                format!(
                                    "Method 'contains' expects 1 argument, got {}",
                                    arguments.len()
                                ),
                                line,
                                column,
                            ));
                        }
                        let arg_val = self
                            .evaluate_expression(&arguments[0].value, Rc::clone(&env))
                            .await?;
                        let list_ref = list.borrow();
                        Ok(Value::Bool(list_ref.contains(&arg_val)))
                    }
                    "isEmpty" => Ok(Value::Bool(list.borrow().is_empty())),
                    "get" => {
                        if arguments.len() != 1 {
                            return Err(RuntimeError::new(
                                format!(
                                    "Method 'get' expects 1 argument, got {}",
                                    arguments.len()
                                ),
                                line,
                                column,
                            ));
                        }
                        let index_val = self
                            .evaluate_expression(&arguments[0].value, Rc::clone(&env))
                            .await?;
                        match index_val {
                            Value::Number(n) => {
                                let index = n as usize;
                                let list_ref = list.borrow();
                                if index < list_ref.len() {
                                    Ok(list_ref[index].clone())
                                } else {
                                    Err(RuntimeError::new(
                                        format!(
                                            "Index out of bounds: {} (size: {})",
                                            index,
                                            list_ref.len()
                                        ),
                                        line,
                                        column,
                                    ))
                                }
                            }
                            _ => Err(RuntimeError::new(
                                "Index must be a number".to_string(),
                                line,
                                column,
                            )),
                        }
                    }
                    _ => Err(RuntimeError::new(
                        format!("Method '{method}' not found on list"),
                        line,
                        column,
                    )),
                }
            }
            Value::Object(map) => {
                // Map methods
                match method {
                    "size" | "length" | "count" => Ok(Value::Number(map.borrow().len() as f64)),
                    "keys" => {
                        let keys: Vec<Value> = map
                            .borrow()
                            .keys()
                            .map(|k| Value::Text(Rc::from(k.as_str())))
                            .collect();
                        Ok(Value::List(Rc::new(RefCell::new(keys))))
                    }
                    "values" => {
                        let values: Vec<Value> = map.borrow().values().cloned().collect();
                        Ok(Value::List(Rc::new(RefCell::new(values))))
                    }
                    "containsKey" => {
                        if arguments.len() != 1 {
                            return Err(RuntimeError::new(
                                format!(
                                    "Method 'containsKey' expects 1 argument, got {}",
                                    arguments.len()
                                ),
                                line,
                                column,
                            ));
                        }
                        let key_val = self
                            .evaluate_expression(&arguments[0].value, Rc::clone(&env))
                            .await?;
                        match key_val {
                            Value::Text(key) => {
                                Ok(Value::Bool(map.borrow().contains_key(key.as_ref())))
                            }
                            _ => Err(RuntimeError::new(
                                "Key must be a string".to_string(),
                                line,
                                column,
                            )),
                        }
                    }
                    _ => Err(RuntimeError::new(
                        format!("Method '{method}' not found on object"),
                        line,
                        column,
                    )),
                }
            }
            Value::Text(text) => {
                // String methods
                match method {
                    "length" | "size" | "count" => Ok(Value::Number(text.len() as f64)),
                    "contains" => {
                        if arguments.len() != 1 {
                            return Err(RuntimeError::new(
                                format!(
                                    "Method 'contains' expects 1 argument, got {}",
                                    arguments.len()
                                ),
                                line,
                                column,
                            ));
                        }
                        let arg_val = self
                            .evaluate_expression(&arguments[0].value, Rc::clone(&env))
                            .await?;
                        match arg_val {
                            Value::Text(sub) => Ok(Value::Bool(text.contains(sub.as_ref()))),
                            _ => Err(RuntimeError::new(
                                "Argument to contains must be a string".to_string(),
                                line,
                                column,
                            )),
                        }
                    }
                    "startsWith" => {
                        if arguments.len() != 1 {
                            return Err(RuntimeError::new(
                                format!(
                                    "Method 'startsWith' expects 1 argument, got {}",
                                    arguments.len()
                                ),
                                line,
                                column,
                            ));
                        }
                        let arg_val = self
                            .evaluate_expression(&arguments[0].value, Rc::clone(&env))
                            .await?;
                        match arg_val {
                            Value::Text(sub) => Ok(Value::Bool(text.starts_with(sub.as_ref()))),
                            _ => Err(RuntimeError::new(
                                "Argument to startsWith must be a string".to_string(),
                                line,
                                column,
                            )),
                        }
                    }
                    "endsWith" => {
                        if arguments.len() != 1 {
                            return Err(RuntimeError::new(
                                format!(
                                    "Method 'endsWith' expects 1 argument, got {}",
                                    arguments.len()
                                ),
                                line,
                                column,
                            ));
                        }
                        let arg_val = self
                            .evaluate_expression(&arguments[0].value, Rc::clone(&env))
                            .await?;
                        match arg_val {
                            Value::Text(sub) => Ok(Value::Bool(text.ends_with(sub.as_ref()))),
                            _ => Err(RuntimeError::new(
                                "Argument to endsWith must be a string".to_string(),
                                line,
                                column,
                            )),
                        }
                    }
                    "toUpper" | "toUpperCase" => Ok(Value::Text(Rc::from(text.to_uppercase()))),
                    "toLower" | "toLowerCase" => Ok(Value::Text(Rc::from(text.to_lowercase()))),
                    "trim" => Ok(Value::Text(Rc::from(text.trim()))),
                    "substring" => {
                        // substring(start, length) or substring(start)
                        if arguments.len() < 1 || arguments.len() > 2 {
                             return Err(RuntimeError::new(
                                format!("Method 'substring' expects 1 or 2 arguments, got {}", arguments.len()),
                                line,
                                column
                            ));
                        }

                        let start_val = self.evaluate_expression(&arguments[0].value, Rc::clone(&env)).await?;
                        let start = match start_val {
                            Value::Number(n) => n as usize,
                            _ => return Err(RuntimeError::new("Start index must be a number".to_string(), line, column)),
                        };

                        let len = if arguments.len() == 2 {
                            let len_val = self.evaluate_expression(&arguments[1].value, Rc::clone(&env)).await?;
                            match len_val {
                                Value::Number(n) => Some(n as usize),
                                _ => return Err(RuntimeError::new("Length must be a number".to_string(), line, column)),
                            }
                        } else {
                            None
                        };

                        // Handle unicode characters correctly
                        let chars: Vec<char> = text.chars().collect();
                        if start >= chars.len() {
                            Ok(Value::Text(Rc::from("")))
                        } else {
                            let end = if let Some(l) = len {
                                std::cmp::min(start + l, chars.len())
                            } else {
                                chars.len()
                            };
                            let substring: String = chars[start..end].iter().collect();
                            Ok(Value::Text(Rc::from(substring)))
                        }
                    }
                    _ => Err(RuntimeError::new(
                        format!("Method '{method}' not found on string"),
                        line,
                        column,
                    )),
                }
            }
            _ => Err(RuntimeError::new(
                format!("Cannot call method '{method}' on {}", object.type_name()),
                line,
                column,
            )),
        }
    }

    async fn evaluate_property_access(
        &self,
        object: &Value,
        property: &str,
        line: usize,
        column: usize,
        _env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        match object {
            Value::ContainerInstance(instance_rc) => {
                let instance = instance_rc.borrow();

                // Look up property in the instance
                if let Some(val) = instance.properties.get(property) {
                    Ok(val.clone())
                } else {
                    Err(RuntimeError::new(
                        format!(
                            "Property '{property}' not found in container instance"
                        ),
                        line,
                        column,
                    ))
                }
            }
            Value::Object(map_rc) => {
                let map = map_rc.borrow();
                if let Some(val) = map.get(property) {
                    Ok(val.clone())
                } else {
                    Ok(Value::Null) // Return Null for missing properties on maps/objects
                }
            }
            Value::List(list_rc) => {
                match property {
                     "length" | "size" | "count" => Ok(Value::Number(list_rc.borrow().len() as f64)),
                     _ => Err(RuntimeError::new(
                        format!("Property '{property}' not found on list"),
                        line,
                        column,
                    )),
                }
            }
            Value::Text(text) => {
                 match property {
                     "length" | "size" | "count" => Ok(Value::Number(text.len() as f64)),
                     _ => Err(RuntimeError::new(
                        format!("Property '{property}' not found on string"),
                        line,
                        column,
                    )),
                }
            }
            _ => Err(RuntimeError::new(
                format!("Cannot access property '{property}' on {}", object.type_name()),
                line,
                column,
            )),
        }
    }
}
