/// Test for main loop parsing
///
/// This test ensures that main loops parse correctly both standalone and inside try blocks.
/// It verifies the fix for the bug where EOL tokens between statements in main loops caused
/// "Unexpected end of line in expression" errors.

#[test]
fn test_parse_main_loop_standalone() {
    use wfl::lexer::lex_wfl_with_positions;
    use wfl::parser::Parser;

    let source = r#"
        main loop:
            display "test"
            break
        end loop
    "#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Should successfully parse standalone main loop"
    );
}

#[test]
fn test_parse_main_loop_inside_try() {
    use wfl::lexer::lex_wfl_with_positions;
    use wfl::parser::Parser;

    let source = r#"
        try:
            main loop:
                display "test"
                break
            end loop
        catch:
            display "error"
        end try
    "#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Should successfully parse main loop inside try block"
    );
}

#[test]
fn test_parse_main_loop_multiple_statements() {
    use wfl::lexer::lex_wfl_with_positions;
    use wfl::parser::Parser;

    let source = r#"
        main loop:
            display "line 1"
            display "line 2"
            display "line 3"
            break
        end loop
    "#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Should successfully parse main loop with multiple statements and EOLs"
    );
}

#[test]
fn test_parse_nested_try_in_main_loop() {
    use wfl::lexer::lex_wfl_with_positions;
    use wfl::parser::Parser;

    let source = r#"
        main loop:
            try:
                display "inner try"
            catch:
                display "inner catch"
            end try
            break
        end loop
    "#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Should successfully parse try block nested inside main loop"
    );
}
