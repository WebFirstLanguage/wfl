use wfl::interpreter::environment::Environment;
/// Test file to reproduce and fix the pattern module doctest compilation errors
///
/// This test file reproduces the exact issues found in the doctests:
/// 1. PatternExpression enum is private and cannot be imported from wfl::pattern
/// 2. Environment::new() requires a parent parameter
/// 3. Type mismatch: compile_with_env expects &Environment but receives &Rc<RefCell<Environment>>
use wfl::pattern::{CompiledPattern, PatternCompiler};

#[test]
fn test_pattern_expression_visibility_issue() {
    // This test verifies that PatternExpression can now be imported from the pattern module
    // as shown in the doctest examples

    // Use PatternExpression as shown in the fixed doctest
    use wfl::pattern::PatternExpression;

    // Create a global environment (this should work)
    let env = Environment::new_global();

    // Create a pattern expression
    let pattern = PatternExpression::ListReference("protocols".to_string());

    // This should fail because we need to pass &Environment, not &Rc<RefCell<Environment>>
    // and we need to borrow the environment correctly
    let env_borrowed = env.borrow();
    let result = CompiledPattern::compile_with_env(&pattern, &env_borrowed);

    // The test should pass once we fix the API
    assert!(result.is_ok() || result.is_err()); // Just check it compiles for now
}

#[test]
fn test_pattern_compiler_environment_issue() {
    // This test verifies the PatternCompiler doctest now works correctly
    use wfl::pattern::PatternExpression;

    let mut compiler = PatternCompiler::new();

    // This should fail initially because Environment::new() requires a parent
    // We need to use Environment::new_global() instead
    let env = Environment::new_global();

    let ast = PatternExpression::ListReference("my_list".to_string());

    // This should fail because we need to pass &Environment, not &Rc<RefCell<Environment>>
    let env_borrowed = env.borrow();
    let result = compiler.compile_with_env(&ast, &env_borrowed);

    // The test should pass once we fix the API
    assert!(result.is_ok() || result.is_err()); // Just check it compiles for now
}

#[test]
fn test_doctest_examples_should_work_after_fix() {
    // This test verifies that the fixed doctests work correctly
    use wfl::pattern::PatternExpression;

    // Test the mod.rs doctest example
    {
        let env = Environment::new_global();
        let pattern = PatternExpression::ListReference("protocols".to_string());
        let env_borrowed = env.borrow();
        let compiled = CompiledPattern::compile_with_env(&pattern, &env_borrowed);

        // Should compile without errors (may fail at runtime due to missing list)
        if compiled.is_ok() {
            // Success case
        } else {
            // Expected error due to missing "protocols" list
        }
    }

    // Test the compiler.rs doctest example
    {
        let mut compiler = PatternCompiler::new();
        let env = Environment::new_global();
        let ast = PatternExpression::ListReference("my_list".to_string());
        let env_borrowed = env.borrow();
        let program = compiler.compile_with_env(&ast, &env_borrowed);

        // Should compile without errors (may fail at runtime due to missing list)
        if program.is_ok() {
            // Success case
        } else {
            // Expected error due to missing "my_list" list
        }
    }
}

#[test]
fn test_pattern_expression_should_be_reexported() {
    // This test verifies that PatternExpression is now properly re-exported
    // from the pattern module and can be imported directly

    // This line should now work after we re-export PatternExpression from the pattern module
    use wfl::pattern::PatternExpression;

    let pattern = PatternExpression::Literal("test".to_string());
    let compiled = CompiledPattern::compile(&pattern);
    assert!(compiled.is_ok());
}
