// Memory leak tests specifically targeting reference cycles in closures
#[cfg(test)]
mod tests {
    use super::super::environment::Environment;
    use super::super::value::{FunctionValue, Value};
    use crate::parser::ast::{Statement, Type};
    use std::rc::Rc;

    #[test]
    fn test_function_weak_reference() {
        // Test that functions hold weak references to their defining environment
        // This is the key fix for the memory leak in log_message

        // Create a global environment
        let global_env = Environment::new_global();

        {
            // Create a function definition using Weak reference to environment
            let function = FunctionValue {
                name: Some("test_function".to_string()),
                params: vec!["param1".to_string()],
                body: vec![],
                env: Rc::downgrade(&global_env), // Weak reference!
                line: 1,
                column: 1,
            };

            // Store in environment
            let function_value = Value::Function(Rc::new(function));
            global_env
                .borrow_mut()
                .define("test_function", function_value);

            // Check strong count before inner scope exit
            assert_eq!(
                Rc::strong_count(&global_env),
                1,
                "Environment should have only one strong reference"
            );
        }

        // After function creation scope is exited:
        // 1. If we used Rc instead of Weak, count would be 2+ (leak)
        // 2. With Weak, count should remain 1 (no leak)
        assert_eq!(
            Rc::strong_count(&global_env),
            1,
            "Environment should still have only one strong reference after function scope exit"
        );

        // Look up the function and verify it still works
        let function_value = global_env.borrow().get("test_function").unwrap();
        if let Value::Function(func) = &function_value {
            // Verify the weak reference can be upgraded
            assert!(
                func.env.upgrade().is_some(),
                "Function's environment weak reference should upgrade successfully"
            );
        } else {
            panic!("Expected function value");
        }
    }

    #[test]
    fn test_action_definition_memory_usage() {
        // Test specifically mimicking the action definition from test.wfl

        // Create global environment
        let global_env = Environment::new_global();

        // Define a test action similar to log_message
        let parameters = vec![crate::parser::ast::Parameter {
            name: "message_text".to_string(),
            param_type: Some(Type::Text),
            default_value: None,
            line: 0,
            column: 0,
        }];

        let body = vec![Statement::DisplayStatement {
            value: crate::parser::ast::Expression::Variable("message_text".to_string(), 1, 1),
            line: 1,
            column: 1,
        }];

        let action_def = Statement::ActionDefinition {
            name: "log_message".to_string(),
            parameters,
            body,
            return_type: None,
            line: 1,
            column: 1,
        };

        // Store initial reference count
        let initial_count = Rc::strong_count(&global_env);

        // Create a child environment and execute the action definition
        let child_env = Environment::new_child_env(&global_env);

        // Manually implement what happens in execute_statement for ActionDefinition
        if let Statement::ActionDefinition {
            name,
            parameters,
            body,
            ..
        } = &action_def
        {
            let param_names: Vec<String> = parameters.iter().map(|p| p.name.clone()).collect();

            let function = FunctionValue {
                name: Some(name.clone()),
                params: param_names,
                body: body.clone(),
                env: Rc::downgrade(&child_env), // Using Weak reference
                line: 1,
                column: 1,
            };

            let function_value = Value::Function(Rc::new(function));
            child_env.borrow_mut().define(name, function_value);
        }

        // Ensure child_env is properly linked to its parent
        assert!(child_env.borrow().parent.is_some());

        // Drop the child environment
        drop(child_env);

        // Verify global_env reference count didn't increase
        let final_count = Rc::strong_count(&global_env);
        assert_eq!(
            initial_count, final_count,
            "Reference count before ({}) and after ({}) should be the same",
            initial_count, final_count
        );
    }
}
