use super::*;
use crate::lexer::lex_wfl_with_positions;
use crate::parser::Parser;
use crate::diagnostics::{WflDiagnostic, Severity};

#[test]
fn test_unused_variable_detection() {
    let input = "store x as 10\nstore y as 20\ndisplay x";
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();
    
    let mut analyzer = Analyzer::new();
    let diagnostics = analyzer.check_unused_variables(&program, 0);
    
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("y"));
    assert_eq!(diagnostics[0].code, "ANALYZE-UNUSED");
    assert_eq!(diagnostics[0].severity, Severity::Warning);
}

#[test]
fn test_unreachable_code_detection() {
    let input = "define action called test:\n  give back 10\n  display \"This is unreachable\"\nend action";
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();
    
    let mut analyzer = Analyzer::new();
    let diagnostics = analyzer.check_unreachable_code(&program, 0);
    
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("Unreachable"));
    assert_eq!(diagnostics[0].code, "ANALYZE-UNREACHABLE");
}

#[test]
fn test_shadowing_detection() {
    let input = "store x as 10\ndefine action called test:\n  store x as 20\n  display x\nend action";
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();
    
    let mut analyzer = Analyzer::new();
    let diagnostics = analyzer.check_shadowing(&program, 0);
    
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("shadows"));
    assert_eq!(diagnostics[0].code, "ANALYZE-SHADOW");
}

#[test]
fn test_inconsistent_returns() {
    let input = "define action called test returns number:\n  if x > 0 then\n    give back 10\n  end\nend action";
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();
    
    let mut analyzer = Analyzer::new();
    let diagnostics = analyzer.check_inconsistent_returns(&program, 0);
    
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("inconsistent return"));
    assert_eq!(diagnostics[0].code, "ANALYZE-RETURN");
}

#[test]
fn test_static_analyzer_integration() {
    let input = "store x as 10\nstore unused as 20\ndefine action called test returns number:\n  if x > 0 then\n    give back 10\n  end\nend action";
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();
    
    let mut analyzer = Analyzer::new();
    let diagnostics = analyzer.analyze_static(&program, 0);
    
    assert!(diagnostics.len() >= 2);
    assert!(diagnostics.iter().any(|d| d.code == "ANALYZE-UNUSED"));
    assert!(diagnostics.iter().any(|d| d.code == "ANALYZE-RETURN"));
}

#[test]
fn test_wait_for_variable_definition() {
    let input = "wait for open file \"test.txt\" as file1 and read content into currentLog\ndisplay currentLog";
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();
    
    let mut analyzer = Analyzer::new();
    analyzer.analyze(&program);
    
    assert_eq!(analyzer.errors.len(), 0);

    assert!(analyzer.current_scope.resolve("currentLog").is_some());
}

// ===== Phase 4: Tests for action call validation =====

#[test]
fn test_undefined_action_call() {
    let input = r#"
action greet with name:
    print with name
end

call unknownAction with "test"
    "#;
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();

    let mut analyzer = Analyzer::new();
    let _ = analyzer.analyze(&program);

    // Should have error for undefined action
    assert!(!analyzer.errors.is_empty(), "Should have semantic errors");
    assert!(
        analyzer.errors.iter().any(|e| e.message.contains("Undefined action 'unknownAction'")),
        "Should report undefined action 'unknownAction', got: {:?}",
        analyzer.errors
    );
}

#[test]
fn test_action_call_wrong_arg_count() {
    let input = r#"
action greet with name:
    print with name
end

call greet with "Alice" and "Bob"
    "#;
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();

    let mut analyzer = Analyzer::new();
    let _ = analyzer.analyze(&program);

    // Should have error for wrong argument count
    assert!(!analyzer.errors.is_empty(), "Should have semantic errors");
    assert!(
        analyzer.errors.iter().any(|e| e.message.contains("expects 1 argument(s), but 2 were provided")),
        "Should report wrong argument count, got: {:?}",
        analyzer.errors
    );
}

#[test]
fn test_valid_action_call() {
    let input = r#"
action greet with name:
    print with name
end

call greet with "Alice"
    "#;
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();

    let mut analyzer = Analyzer::new();
    let _ = analyzer.analyze(&program);

    // Should have no errors
    assert!(
        analyzer.errors.is_empty(),
        "Should have no semantic errors, got: {:?}",
        analyzer.errors
    );
}

#[test]
fn test_recursive_action_call() {
    let input = r#"
action factorial with n:
    check if n is less than or equal to 1:
        return 1
    end
    return n times (call factorial with n minus 1)
end

store result as call factorial with 5
    "#;
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();

    let mut analyzer = Analyzer::new();
    let _ = analyzer.analyze(&program);

    // Should have no errors (recursive calls should work)
    assert!(
        analyzer.errors.is_empty(),
        "Recursive action calls should be valid, got: {:?}",
        analyzer.errors
    );
}

#[test]
fn test_forward_action_reference() {
    let input = r#"
action first:
    call second with "test"
end

action second with msg:
    print with msg
end
    "#;
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();

    let mut analyzer = Analyzer::new();
    let _ = analyzer.analyze(&program);

    // Should have no errors (forward references should work)
    assert!(
        analyzer.errors.is_empty(),
        "Forward action references should be valid, got: {:?}",
        analyzer.errors
    );
}

#[test]
fn test_builtin_action_call_validation() {
    let input = r#"
print with "Hello"
    "#;
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();

    let mut analyzer = Analyzer::new();
    let _ = analyzer.analyze(&program);

    // Should have no errors (builtin functions should be recognized)
    assert!(
        analyzer.errors.is_empty(),
        "Builtin function calls should be valid, got: {:?}",
        analyzer.errors
    );
}

#[test]
fn test_action_not_a_function_error() {
    let input = r#"
store x as 10
call x with "test"
    "#;
    let tokens = lex_wfl_with_positions(input);
    let program = Parser::new(&tokens).parse().unwrap();

    let mut analyzer = Analyzer::new();
    let _ = analyzer.analyze(&program);

    // Should have error that 'x' is not an action
    assert!(!analyzer.errors.is_empty(), "Should have semantic errors");
    assert!(
        analyzer.errors.iter().any(|e| e.message.contains("'x' is not an action")),
        "Should report 'x' is not an action, got: {:?}",
        analyzer.errors
    );
}
