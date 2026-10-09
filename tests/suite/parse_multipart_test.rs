// Integration tests for parse_multipart (issue #597).
// Detailed part-shape assertions live in stdlib/web.rs unit tests;
// these confirm the builtin is wired into the interpreter and errors cleanly.

use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

async fn run_wfl(code: &str) -> Result<(), String> {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().map_err(|e| format!("parse error: {e:?}"))?;
    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&program)
        .await
        .map_err(|e| format!("runtime error: {e:?}"))?;
    Ok(())
}

#[tokio::test]
async fn test_parse_multipart_runs_from_wfl() {
    // WFL string escapes: \r \n \"
    let code = r#"
        store body as "------bound\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\nHello\r\n------bound--\r\n"
        store ct as "multipart/form-data; boundary=----bound"
        store parts as parse_multipart of body and ct
        store count as length of parts
        check if count is equal to 1:
            store first as parts[0]
            store field_name as first["name"]
            store field_text as first["content"]
            check if field_name is equal to "title":
                check if field_text is equal to "Hello":
                    display "ok"
                otherwise:
                    display "bad content"
                end check
            otherwise:
                display "bad name"
            end check
        otherwise:
            display "bad count"
        end check
    "#;

    run_wfl(code)
        .await
        .expect("parse_multipart should be callable from WFL");
}

#[tokio::test]
async fn test_parse_multipart_missing_boundary_errors() {
    let code = r#"
        store body as "not multipart"
        store ct as "application/json"
        store parts as parse_multipart of body and ct
    "#;

    let err = run_wfl(code)
        .await
        .expect_err("should error without boundary");
    let lower = err.to_lowercase();
    assert!(
        lower.contains("boundary") || lower.contains("parse_multipart"),
        "error should mention boundary, got: {err}"
    );
}
