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

/// Regression test for Docs/Archive/FRAMEWORK_FINAL_REPORT.md, which reported
/// "Unexpected end of line in expression" for `wait for request comes in on
/// <server> as <req>` and parsing issues for try/catch in the main request loop.
#[test]
fn test_parse_web_request_loop_with_try_catch() {
    use wfl::lexer::lex_wfl_with_positions;
    use wfl::parser::Parser;

    let source = r#"
        listen on port 8080 as web_server
        main loop:
            try:
                wait for request comes in on web_server as req
                respond to req with "ok" and content_type "text/plain"
            when error:
                display "request failed"
            end try
        end loop
    "#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Should parse try/when error around wait-for-request inside main loop: {:?}",
        result.err()
    );
}

/// Same shape with catch-style error handling and a request property access.
#[test]
fn test_parse_web_request_loop_with_catch_and_properties() {
    use wfl::lexer::lex_wfl_with_positions;
    use wfl::parser::Parser;

    let source = r#"
        listen on port 8081 as web_server
        store request_count as 0
        main loop:
            try:
                wait for request comes in on web_server as req
                change request_count to request_count plus 1
                store request_path as path of req
                store agent as header "User-Agent" of req
                respond to req with request_path and content_type "text/plain"
            catch:
                display "request failed"
            end try
        end loop
    "#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Should parse catch around request handling with property and header access: {:?}",
        result.err()
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
