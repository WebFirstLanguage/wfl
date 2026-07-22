// Backward-compatibility for `write line <ident> to <target>`.
//
// WFL allows space-separated identifiers, so `write line payload to out` could
// mean either the classic file write of a variable literally named "line
// payload", or the new streaming form (`write line <value> to <stream>`). The
// merged form must not silently break the pre-existing file write: the runtime
// picks the interpretation from the target type, and static analysis must not
// reject the file-write reading (nor falsely warn the variable is unused).

use wfl::Interpreter;
use wfl::analyzer::Analyzer;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::Statement;

fn parse(code: &str) -> Vec<Statement> {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    parser
        .parse()
        .unwrap_or_else(|e| panic!("parse error: {e:?}"))
        .statements
}

#[test]
fn test_write_line_multiword_variable_parses_with_fallback() {
    // The ambiguous merged form carries a classic-file-write fallback so the
    // interpreter can disambiguate on the target type at runtime.
    let stmt = &parse(r#"write line payload to out"#)[0];
    match stmt {
        Statement::StreamWriteStatement {
            fallback_content,
            is_line,
            ..
        } => {
            assert!(*is_line);
            assert!(
                fallback_content.is_some(),
                "merged `write line <ident>` must keep a file-write fallback"
            );
        }
        other => panic!("expected StreamWriteStatement, got {other:?}"),
    }

    // The unambiguous literal form (never a valid classic file write) has none.
    let stmt = &parse(r#"write line "x" to out"#)[0];
    match stmt {
        Statement::StreamWriteStatement {
            fallback_content, ..
        } => assert!(
            fallback_content.is_none(),
            "literal-valued stream write needs no fallback"
        ),
        other => panic!("expected StreamWriteStatement, got {other:?}"),
    }

    // `write chunk` has the same ambiguity and fallback contract as `write line`.
    let stmt = &parse(r#"write chunk payload to out"#)[0];
    match stmt {
        Statement::StreamWriteStatement {
            fallback_content,
            is_line,
            ..
        } => {
            assert!(!*is_line, "write chunk must not be a line write");
            assert!(
                fallback_content.is_some(),
                "merged `write chunk <ident>` must keep a file-write fallback"
            );
        }
        other => panic!("expected StreamWriteStatement, got {other:?}"),
    }
    let stmt = &parse(r#"write chunk "x" to out"#)[0];
    match stmt {
        Statement::StreamWriteStatement {
            fallback_content, ..
        } => assert!(
            fallback_content.is_none(),
            "literal-valued chunk write needs no fallback"
        ),
        other => panic!("expected StreamWriteStatement, got {other:?}"),
    }
}

#[test]
fn test_write_multiword_line_variable_to_file_still_works() {
    // A pre-existing program: a variable literally named `line note` written to a
    // file path. Must analyze cleanly (no undefined-variable error, no spurious
    // unused warning) and, at runtime, write the VARIABLE'S value to the file —
    // not stream-write the token "note".
    // Unique temp dir per invocation so parallel/sharded runs cannot collide or
    // delete each other's output; `TempDir` cleans up on drop (even on panic).
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("wfl_write_line_backcompat.txt");
    let path_str = path.to_string_lossy().replace('\\', "/");

    let code = format!(
        r#"store line note as "kept across versions"
write line note to "{path_str}""#
    );

    let program = {
        let tokens = lex_wfl_with_positions(&code);
        let mut parser = Parser::new(&tokens);
        parser.parse().expect("parse")
    };

    // Static analysis must accept the file-write reading.
    let mut analyzer = Analyzer::new();
    analyzer
        .analyze(&program)
        .expect("semantic analysis must accept `write line <var> to <file>`");

    // Runtime writes the variable's value to the file.
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut interp = Interpreter::new();
        interp.interpret(&program).await.expect("interpret");
    });

    let contents = std::fs::read_to_string(&path).expect("output file should exist");
    assert_eq!(
        contents, "kept across versions",
        "the variable `line note` must be written to the file, not the token `note`"
    );
    let _ = std::fs::remove_file(&path);
}
