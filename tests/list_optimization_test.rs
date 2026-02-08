use wfl::analyzer::Analyzer;
use wfl::interpreter::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::TypeChecker;

#[tokio::test]
async fn test_list_sync_optimization() {
    let code = r#"
// Simple literal list (synchronous)
store l1 as [1, 2, 3]

// Mixed literal list (synchronous)
store l2 as [1, "two", true]

// Nested list (synchronous recursion)
store l3 as [[1, 2], [3, 4]]

// List with simple variable (synchronous)
store x as 10
store l4 as [x, (x plus 1)]

// List with mixed sync and async (should fallback correctly)
define action called get_val:
    wait for 1 milliseconds
    return 42
end action

store l5 as [1, (call get_val)]

// List with pure native function (synchronous)
store l6 as [(abs of -5)]
"#;

    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");

    let mut analyzer = Analyzer::new();
    analyzer.analyze(&program).expect("Should analyze");

    let mut type_checker = TypeChecker::new();
    type_checker.check_types(&program).ok();

    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&program)
        .await
        .expect("Should execute successfully");

    let env = interpreter.global_env();
    let env = env.borrow();

    // Verify l1
    let l1 = env.get("l1").expect("l1 exists");
    assert_eq!(l1.to_string(), "[1, 2, 3]");

    // Verify l2
    let l2 = env.get("l2").expect("l2 exists");
    assert_eq!(l2.to_string(), "[1, two, yes]");

    // Verify l3
    let l3 = env.get("l3").expect("l3 exists");
    assert_eq!(l3.to_string(), "[[1, 2], [3, 4]]");

    // Verify l4
    let l4 = env.get("l4").expect("l4 exists");
    assert_eq!(l4.to_string(), "[10, 11]");

    // Verify l5
    let l5 = env.get("l5").expect("l5 exists");
    assert_eq!(l5.to_string(), "[1, 42]");

    // Verify l6
    let l6 = env.get("l6").expect("l6 exists");
    assert_eq!(l6.to_string(), "[5]");
}
