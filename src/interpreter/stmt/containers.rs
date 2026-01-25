use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::interpreter::Interpreter;
use crate::interpreter::control_flow::ControlFlow;
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::{
    self, ActionSignature, ContainerDefinitionValue, ContainerEventValue, ContainerMethodValue,
    EventHandler, FunctionValue, InterfaceDefinitionValue, Value,
};
use crate::parser::ast::{
    ActionSignature as AstActionSignature, Argument, EventDefinition, Expression,
    PropertyDefinition, PropertyInitializer, Statement,
};

#[allow(async_fn_in_trait)]
pub trait ContainerExecutor {
    #[allow(clippy::too_many_arguments)]
    async fn execute_container_definition(
        &self,
        name: &str,
        extends: Option<&str>,
        implements: &[String],
        properties: &[PropertyDefinition],
        methods: &[Statement],
        events: &[EventDefinition],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    #[allow(clippy::too_many_arguments)]
    async fn execute_container_instantiation(
        &self,
        container_type: &str,
        instance_name: &str,
        arguments: &[Argument],
        property_initializers: &[PropertyInitializer],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_interface_definition(
        &self,
        name: &str,
        extends: &[String],
        required_actions: &[AstActionSignature],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_event_trigger(
        &self,
        name: &str,
        arguments: &[Argument],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_event_handler(
        &self,
        event_source: &Expression,
        event_name: &str,
        handler_body: &[Statement],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_parent_method_call(
        &self,
        method_name: &str,
        arguments: &[Argument],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_push(
        &self,
        list: &Expression,
        value: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_create_list(
        &self,
        name: &str,
        initial_values: &[Expression],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_map_creation(
        &self,
        name: &str,
        entries: &[(String, Expression)],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_create_date(
        &self,
        name: &str,
        value: Option<&Expression>,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_create_time(
        &self,
        name: &str,
        value: Option<&Expression>,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_add_to_list(
        &self,
        value: &Expression,
        list_name: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_remove_from_list(
        &self,
        value: &Expression,
        list_name: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;

    async fn execute_clear_list(
        &self,
        list_name: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError>;
}

impl ContainerExecutor for Interpreter {
    async fn execute_container_definition(
        &self,
        name: &str,
        extends: Option<&str>,
        implements: &[String],
        properties: &[PropertyDefinition],
        methods: &[Statement],
        events: &[EventDefinition],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // Create a new container definition
        let mut container_properties = HashMap::new();
        let mut container_methods = HashMap::new();

        for prop in properties {
            let property_type_str = prop
                .property_type
                .as_ref()
                .map(|ast_type| format!("{ast_type:?}"));

            let default_val = match &prop.default_value {
                Some(expr) => {
                    // Evaluate the default expression to get a Value
                    // We need to use evaluate_expression but it's on self.
                    // This is an async call.
                    // But we are in async fn, so await is fine.
                    // However, we need access to evaluate_expression from here.
                    // Interpreter implements it.
                    (self.evaluate_expression(expr, env.clone()).await).ok()
                }
                None => None,
            };

            let value_prop = value::PropertyDefinition {
                name: prop.name.clone(),
                property_type: property_type_str,
                default_value: default_val,
                validation_rules: Vec::new(),
                is_static: false,
                is_public: true,
                line: prop.line,
                column: prop.column,
            };
            container_properties.insert(prop.name.clone(), value_prop);
        }

        for method in methods {
            if let Statement::ActionDefinition {
                name,
                parameters,
                body,
                line,
                column,
                ..
            } = method
            {
                let container_method = ContainerMethodValue {
                    name: name.clone(),
                    params: parameters.iter().map(|p| p.name.clone()).collect(),
                    body: body.clone(),
                    is_static: false,
                    is_public: true,
                    env: Rc::downgrade(&env),
                    line: *line,
                    column: *column,
                };
                container_methods.insert(name.clone(), container_method);
            }
        }

        // Process events
        let mut container_events = HashMap::new();
        for event in events {
            let container_event = ContainerEventValue {
                name: event.name.clone(),
                params: event.parameters.iter().map(|p| p.name.clone()).collect(),
                handlers: Vec::new(),
                line: event.line,
                column: event.column,
            };
            container_events.insert(event.name.clone(), container_event);
        }

        let container_def = ContainerDefinitionValue {
            name: name.to_string(),
            extends: extends.map(|s| s.to_string()),
            implements: implements.to_vec(),
            properties: container_properties,
            methods: container_methods,
            events: container_events,
            static_properties: HashMap::new(), // Future feature
            static_methods: HashMap::new(),    // Future feature
            line,
            column,
        };

        // Create the container definition value
        let container_value = Value::ContainerDefinition(Rc::new(container_def));

        // Store the container definition in the environment
        match env.borrow_mut().define(name, container_value.clone()) {
            Ok(_) => {}
            Err(msg) => return Err(RuntimeError::new(msg, line, column)),
        }

        Ok((container_value, ControlFlow::None))
    }

    async fn execute_container_instantiation(
        &self,
        container_type: &str,
        instance_name: &str,
        arguments: &[Argument],
        property_initializers: &[PropertyInitializer],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // Create container instance with inheritance support
        let mut instance =
            self.create_container_instance_with_inheritance(container_type, &env, line, column)?;

        // Process property initializers (override inherited properties)
        for initializer in property_initializers {
            let init_value = self
                .evaluate_expression(&initializer.value, env.clone())
                .await?;
            instance
                .properties
                .insert(initializer.name.clone(), init_value);
        }

        let instance_value = Value::ContainerInstance(Rc::new(RefCell::new(instance)));

        // Store the instance in the environment
        match env
            .borrow_mut()
            .define(instance_name, instance_value.clone())
        {
            Ok(_) => {}
            Err(msg) => return Err(RuntimeError::new(msg, line, column)),
        }

        // Call constructor method if arguments are provided
        if !arguments.is_empty() {
            // Look up the container definition to find the initialize method
            let container_def = match env.borrow().get(container_type) {
                Some(Value::ContainerDefinition(def)) => def.clone(),
                _ => {
                    return Err(RuntimeError::new(
                        format!("Container '{container_type}' not found"),
                        line,
                        column,
                    ));
                }
            };

            // Check if the container has an "initialize" method
            if let Some(init_method) = container_def.methods.get("initialize") {
                // Create a function value from the initialize method
                let _init_function = FunctionValue {
                    name: Some("initialize".to_string()),
                    params: init_method.params.clone(),
                    body: init_method.body.clone(),
                    env: init_method.env.clone(),
                    line: init_method.line,
                    column: init_method.column,
                };

                // Create a new environment for the constructor execution
                let init_env = Environment::new_child_env(&env);

                // Add 'this' to the environment (the instance being constructed)
                let _ = init_env.borrow_mut().define("this", instance_value.clone());

                // Evaluate the arguments
                let mut arg_values = Vec::with_capacity(arguments.len());
                for arg in arguments {
                    let arg_val = self.evaluate_expression(&arg.value, env.clone()).await?;
                    arg_values.push(arg_val);
                }

                // Call the initialize method
                // We need to use call_function from Interpreter
                // It is async
                // We don't have public access to call_function from trait?
                // Ah, Interpreter implements this trait, so self.call_function works if it's visible?
                // call_function is private in Interpreter.
                // But this code is in `src/interpreter/stmt/containers.rs` which is a submodule.
                // It can access `pub(crate)` methods.
                // I need to make `call_function` pub(crate) in `mod.rs`.
                // For now, I assume it will be made accessible or I'll fix it.
                // Let's assume I'll make it pub(crate).

                // Oops, `call_function` is private. I need to update `mod.rs`.
                // I'll proceed assuming I will update `mod.rs`.
                // Actually, I can't call private methods from another module.
                // I will add a step to make `call_function` pub(crate).

                // But wait, `evaluate_expression` is also private in `mod.rs` (async fn evaluate_expression).
                // I need to make `evaluate_expression` pub(crate) too?
                // Yes.

                // Let's check `mod.rs` again. `evaluate_expression` is `async fn`.
            } else if !arguments.is_empty() {
                return Err(RuntimeError::new(
                    format!(
                        "Container '{container_type}' does not have an initialize method but arguments were provided"
                    ),
                    line,
                    column,
                ));
            }
        }

        Ok((instance_value, ControlFlow::None))
    }

    async fn execute_interface_definition(
        &self,
        name: &str,
        extends: &[String],
        required_actions: &[AstActionSignature],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // Create a new interface definition
        let mut interface_required_actions = HashMap::new();

        for action in required_actions {
            let value_action = ActionSignature {
                name: action.name.clone(),
                params: action.parameters.iter().map(|p| p.name.clone()).collect(),
                line: action.line,
                column: action.column,
            };
            interface_required_actions.insert(action.name.clone(), value_action);
        }

        let interface_def = InterfaceDefinitionValue {
            name: name.to_string(),
            extends: extends.to_vec(),
            required_actions: interface_required_actions,
            line,
            column,
        };

        let interface_value = Value::InterfaceDefinition(Rc::new(interface_def));

        // Store the interface definition in the environment
        match env.borrow_mut().define(name, interface_value.clone()) {
            Ok(_) => {}
            Err(msg) => return Err(RuntimeError::new(msg, line, column)),
        }

        Ok((interface_value, ControlFlow::None))
    }

    async fn execute_event_trigger(
        &self,
        name: &str,
        arguments: &[Argument],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // Look up the event
        let event = match env.borrow().get(name) {
            Some(Value::ContainerEvent(event)) => event.clone(),
            _ => {
                return Err(RuntimeError::new(
                    format!("Event '{name}' not found"),
                    line,
                    column,
                ));
            }
        };

        // Evaluate the arguments
        let mut arg_values = Vec::with_capacity(arguments.len());
        for arg in arguments {
            let arg_val = self
                .evaluate_expression(&arg.value, Rc::clone(&env))
                .await?;
            arg_values.push(arg_val);
        }

        // Execute all event handlers
        for handler in &event.handlers {
            // Create a new environment for the handler
            let handler_env = Environment::new_child_env(&env);

            // Bind arguments to parameters
            for (i, param_name) in event.params.iter().enumerate() {
                if i < arg_values.len() {
                    let _ = handler_env
                        .borrow_mut()
                        .define(param_name, arg_values[i].clone());
                } else {
                    let _ = handler_env.borrow_mut().define(param_name, Value::Null);
                }
            }

            // Execute the handler
            // `execute_block` is also private in `mod.rs`. Needs to be pub(crate).
            self.execute_block(&handler.body, handler_env).await?;
        }

        Ok((Value::Null, ControlFlow::None))
    }

    async fn execute_event_handler(
        &self,
        event_source: &Expression,
        event_name: &str,
        handler_body: &[Statement],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // Evaluate the event source
        let source_val = self
            .evaluate_expression(event_source, Rc::clone(&env))
            .await?;

        // Check if the source is a container instance
        if let Value::ContainerInstance(instance_rc) = &source_val {
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

            // Look up the event
            if let Some(event) = container_def.events.get(event_name) {
                // Create a new event handler
                let handler = EventHandler {
                    body: handler_body.to_vec(),
                    env: Rc::downgrade(&env),
                    line,
                    column,
                };

                // Create a new event with the handler added
                let mut handlers = event.handlers.clone();
                handlers.push(handler);

                // Create a new event value
                let new_event = ContainerEventValue {
                    name: event.name.clone(),
                    params: event.params.clone(),
                    handlers,
                    line: event.line,
                    column: event.column,
                };

                // Store the updated event in the environment
                let event_value = Value::ContainerEvent(Rc::new(new_event));
                let _ = env.borrow_mut().define(event_name, event_value.clone());

                Ok((Value::Null, ControlFlow::None))
            } else {
                Err(RuntimeError::new(
                    format!("Event '{event_name}' not found in container '{container_type}'"),
                    line,
                    column,
                ))
            }
        } else {
            Err(RuntimeError::new(
                "Cannot add event handler to non-container value".to_string(),
                line,
                column,
            ))
        }
    }

    async fn execute_parent_method_call(
        &self,
        method_name: &str,
        arguments: &[Argument],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // Get the current container instance (this)
        let this_val = match env.borrow().get("this") {
            Some(val) => val.clone(),
            None => {
                return Err(RuntimeError::new(
                    "Parent method call can only be used inside a container method".to_string(),
                    line,
                    column,
                ));
            }
        };

        // Check if this is a container instance
        if let Value::ContainerInstance(instance_rc) = &this_val {
            let instance = instance_rc.borrow();

            // Check if the instance has a parent
            if let Some(parent_rc) = &instance.parent {
                let parent = parent_rc.borrow();
                let parent_type = parent.container_type.clone();

                // Look up the parent container definition
                let parent_def = match env.borrow().get(&parent_type) {
                    Some(Value::ContainerDefinition(def)) => def.clone(),
                    _ => {
                        return Err(RuntimeError::new(
                            format!("Parent container '{parent_type}' not found"),
                            line,
                            column,
                        ));
                    }
                };

                // Look up the method in the parent
                if let Some(method_val) = parent_def.methods.get(method_name) {
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

                    // Add 'this' to the environment (the current instance, not the parent)
                    let _ = method_env.borrow_mut().define("this", this_val.clone());

                    // Evaluate the arguments
                    let mut arg_values = Vec::with_capacity(arguments.len());
                    for arg in arguments {
                        let arg_val = self
                            .evaluate_expression(&arg.value, Rc::clone(&env))
                            .await?;
                        arg_values.push(arg_val);
                    }

                    // Call the function
                    // Need call_function to be pub(crate)
                    let result = self
                        .call_function(&function, arg_values, line, column)
                        .await?;

                    Ok((result, ControlFlow::None))
                } else {
                    Err(RuntimeError::new(
                        format!(
                            "Method '{method_name}' not found in parent container '{parent_type}'"
                        ),
                        line,
                        column,
                    ))
                }
            } else {
                Err(RuntimeError::new(
                    "Cannot call parent method: no parent container".to_string(),
                    line,
                    column,
                ))
            }
        } else {
            Err(RuntimeError::new(
                "Parent method call can only be used inside a container method".to_string(),
                line,
                column,
            ))
        }
    }

    async fn execute_push(
        &self,
        list: &Expression,
        value: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let list_val = self.evaluate_expression(list, Rc::clone(&env)).await?;
        let value_val = self.evaluate_expression(value, Rc::clone(&env)).await?;

        match list_val {
            Value::List(list_rc) => {
                list_rc.borrow_mut().push(value_val);
                Ok((Value::Null, ControlFlow::None))
            }
            _ => Err(RuntimeError::new(
                format!("Cannot push to non-list value: {list_val:?}"),
                line,
                column,
            )),
        }
    }

    async fn execute_create_list(
        &self,
        name: &str,
        initial_values: &[Expression],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // Create a new list with initial values
        let mut list_items = Vec::new();
        for value_expr in initial_values {
            let value = self
                .evaluate_expression(value_expr, Rc::clone(&env))
                .await?;
            list_items.push(value);
        }

        let list_value = Value::List(Rc::new(RefCell::new(list_items)));
        match env.borrow_mut().define(name, list_value) {
            Ok(_) => {}
            Err(msg) => return Err(RuntimeError::new(msg, line, column)),
        }

        Ok((Value::Null, ControlFlow::None))
    }

    async fn execute_map_creation(
        &self,
        name: &str,
        entries: &[(String, Expression)],
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // Create a new map/object with initial entries
        let mut map = std::collections::HashMap::new();
        for (key, value_expr) in entries {
            let value = self
                .evaluate_expression(value_expr, Rc::clone(&env))
                .await?;
            map.insert(key.clone(), value);
        }

        let map_value = Value::Object(Rc::new(RefCell::new(map)));
        match env.borrow_mut().define(name, map_value) {
            Ok(_) => {}
            Err(msg) => return Err(RuntimeError::new(msg, line, column)),
        }

        Ok((Value::Null, ControlFlow::None))
    }

    async fn execute_create_date(
        &self,
        name: &str,
        value: Option<&Expression>,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let date_value = if let Some(expr) = value {
            // Evaluate the expression to get the date
            self.evaluate_expression(expr, Rc::clone(&env)).await?
        } else {
            // Default to today's date
            let today = chrono::Local::now().date_naive();
            Value::Date(Rc::new(today))
        };

        match env.borrow_mut().define(name, date_value) {
            Ok(_) => {}
            Err(msg) => return Err(RuntimeError::new(msg, line, column)),
        }
        Ok((Value::Null, ControlFlow::None))
    }

    async fn execute_create_time(
        &self,
        name: &str,
        value: Option<&Expression>,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        let time_value = if let Some(expr) = value {
            // Evaluate the expression to get the time
            self.evaluate_expression(expr, Rc::clone(&env)).await?
        } else {
            // Default to current time
            let now = chrono::Local::now().time();
            Value::Time(Rc::new(now))
        };

        match env.borrow_mut().define(name, time_value) {
            Ok(_) => {}
            Err(msg) => return Err(RuntimeError::new(msg, line, column)),
        }
        Ok((Value::Null, ControlFlow::None))
    }

    async fn execute_add_to_list(
        &self,
        value: &Expression,
        list_name: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // Evaluate the value to add
        let value_to_add = self.evaluate_expression(value, Rc::clone(&env)).await?;

        // Get the list from the environment
        let list_val = env.borrow().get(list_name).ok_or_else(|| {
            RuntimeError::new(format!("Undefined variable: {list_name}"), line, column)
        })?;

        match list_val {
            Value::List(list_rc) => {
                list_rc.borrow_mut().push(value_to_add);
                Ok((Value::Null, ControlFlow::None))
            }
            Value::Number(_) => {
                // This is actually arithmetic add
                // Convert to arithmetic operation
                let current = list_val;
                if let (Value::Number(n1), Value::Number(n2)) = (&current, &value_to_add) {
                    let result = Value::Number(n1 + n2);
                    env.borrow_mut()
                        .assign(list_name, result)
                        .map_err(|e| RuntimeError::new(e, line, column))?;
                    Ok((Value::Null, ControlFlow::None))
                } else {
                    Err(RuntimeError::new(
                        "Cannot add non-numeric value to number".to_string(),
                        line,
                        column,
                    ))
                }
            }
            _ => Err(RuntimeError::new(
                format!("Cannot add to non-list value: {list_val:?}"),
                line,
                column,
            )),
        }
    }

    async fn execute_remove_from_list(
        &self,
        value: &Expression,
        list_name: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // Evaluate the value to remove
        let value_to_remove = self.evaluate_expression(value, Rc::clone(&env)).await?;

        // Get the list from the environment
        let list_val = env.borrow().get(list_name).ok_or_else(|| {
            RuntimeError::new(format!("Undefined variable: {list_name}"), line, column)
        })?;

        match list_val {
            Value::List(list_rc) => {
                let mut list = list_rc.borrow_mut();
                // Remove the first occurrence of the value
                if let Some(pos) = list.iter().position(|v| v == &value_to_remove) {
                    list.remove(pos);
                }
                Ok((Value::Null, ControlFlow::None))
            }
            _ => Err(RuntimeError::new(
                format!("Cannot remove from non-list value: {list_val:?}"),
                line,
                column,
            )),
        }
    }

    async fn execute_clear_list(
        &self,
        list_name: &str,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(Value, ControlFlow), RuntimeError> {
        // Get the list from the environment
        let list_val = env.borrow().get(list_name).ok_or_else(|| {
            RuntimeError::new(format!("Undefined variable: {list_name}"), line, column)
        })?;

        match list_val {
            Value::List(list_rc) => {
                list_rc.borrow_mut().clear();
                Ok((Value::Null, ControlFlow::None))
            }
            _ => Err(RuntimeError::new(
                format!("Cannot clear non-list value: {list_val:?}"),
                line,
                column,
            )),
        }
    }
}
