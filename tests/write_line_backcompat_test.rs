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
use wfl::parser::ast::{Expression, Statement};

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
fn test_write_line_with_continuation_parses_for_both_readings() {
    // `write line payload with "!" to out`: the value must absorb the `with`
    // continuation for the stream reading, and the classic file-write fallback
    // must mirror it (leading variable `line payload`, same continuation) — not
    // truncate at the bare variable and fail at `with`.
    let stmt = &parse(r#"write line payload with "!" to out"#)[0];
    match stmt {
        Statement::StreamWriteStatement {
            value,
            fallback_content,
            ..
        } => {
            // Stream reading: Concatenation(Variable("payload"), "!").
            match value {
                Expression::Concatenation { left, .. } => match &**left {
                    Expression::Variable(name, ..) => assert_eq!(name, "payload"),
                    other => panic!("stream value left should be Variable(payload), got {other:?}"),
                },
                other => panic!("stream value should be a Concatenation, got {other:?}"),
            }
            // Classic file-write fallback: Concatenation(Variable("line payload"), "!").
            let fb = fallback_content
                .as_ref()
                .expect("merged `write line <ident> with ...` keeps a file-write fallback");
            match &**fb {
                Expression::Concatenation { left, .. } => match &**left {
                    Expression::Variable(name, ..) => assert_eq!(name, "line payload"),
                    other => {
                        panic!("fallback left should be Variable(line payload), got {other:?}")
                    }
                },
                other => panic!("fallback should mirror the continuation, got {other:?}"),
            }
        }
        other => panic!("expected StreamWriteStatement, got {other:?}"),
    }
}

#[test]
fn test_write_line_variable_with_continuation_to_file_preserves_concatenation() {
    // A pre-existing classic file write with a `with` continuation on a variable
    // literally named `line note`. Before continuation parsing this failed to
    // parse (the interception expected `to` right after the variable); it must
    // now write the concatenated value to the file.
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("wfl_write_line_continuation.txt");
    let path_str = path.to_string_lossy().replace('\\', "/");

    let code = format!(
        r#"store line note as "kept"
write line note with "!" to "{path_str}""#
    );

    let program = {
        let tokens = lex_wfl_with_positions(&code);
        let mut parser = Parser::new(&tokens);
        parser.parse().expect("parse")
    };

    let mut analyzer = Analyzer::new();
    analyzer
        .analyze(&program)
        .expect("analysis must accept `write line <var> with ... to <file>`");

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut interp = Interpreter::new();
        interp.interpret(&program).await.expect("interpret");
    });

    let contents = std::fs::read_to_string(&path).expect("output file should exist");
    assert_eq!(
        contents, "kept!",
        "the concatenated value (variable `line note` with \"!\") must reach the file"
    );
}

#[test]
fn test_write_line_builtin_named_variable_with_continuation_preserves_concatenation() {
    // Regression (maintainer review): the stream reading of `length with "!"`
    // desugars to an ActionCall because `length` is a builtin. The classic
    // file-write reading must still be the variable `line length` concatenated
    // with "!" — parsed INDEPENDENTLY (cursor rewind), not derived from the
    // stream AST, which would drop the `with "!"` and write only "kept".
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("wfl_write_line_builtin_named.txt");
    let path_str = path.to_string_lossy().replace('\\', "/");

    let code = format!(
        r#"store line length as "kept"
write line length with "!" to "{path_str}""#
    );

    let program = {
        let tokens = lex_wfl_with_positions(&code);
        let mut parser = Parser::new(&tokens);
        parser.parse().expect("parse")
    };

    let mut analyzer = Analyzer::new();
    analyzer
        .analyze(&program)
        .expect("analysis must accept the builtin-named continuation file write");

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut interp = Interpreter::new();
        interp.interpret(&program).await.expect("interpret");
    });

    let contents = std::fs::read_to_string(&path).expect("output file should exist");
    assert_eq!(
        contents, "kept!",
        "the classic file write must keep the `with \"!\"` continuation even though `length` is a builtin"
    );
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

#[test]
fn test_ambiguous_write_line_flags_undefined_in_continuation() {
    // The continuation (everything right of the ambiguous lead) is shared by both
    // readings, so a genuinely undefined variable there must still be caught even
    // though the leading operand itself is ambiguous.
    let code = "listen on port 8080 as srv\nstore payload as \"x\"\nwrite line payload with missing_suffix to srv";
    let tokens = lex_wfl_with_positions(code);
    let program = Parser::new(&tokens).parse().expect("parse");
    let mut analyzer = Analyzer::new();
    let errors = analyzer
        .analyze(&program)
        .expect_err("`missing_suffix` in the continuation is undefined");
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("missing_suffix") && e.message.contains("not defined")),
        "expected an undefined-variable error naming `missing_suffix`, got: {errors:?}"
    );
}

#[test]
fn test_ambiguous_write_line_accepts_valid_classic_with_continuation() {
    // A valid pre-existing program: a variable literally named `line path`,
    // written with a continuation to a file. The split stream name `path` is
    // undefined, but the classic file-write reading resolves — analysis must NOT
    // reject it (no false positive from the ambiguous split).
    let code = "store line path as \"/tmp/x\"\nstore suffix as \"!\"\nwrite line path with suffix to \"/tmp/out\"";
    let tokens = lex_wfl_with_positions(code);
    let program = Parser::new(&tokens).parse().expect("parse");
    let mut analyzer = Analyzer::new();
    assert!(
        analyzer.analyze(&program).is_ok(),
        "a valid classic `write line <var> with ... to <file>` must not be rejected: {:?}",
        analyzer.get_errors()
    );
}

#[test]
fn test_ambiguous_write_line_still_flags_when_neither_candidate_defined() {
    // The ambiguous form defers definedness to runtime, but a genuine typo where
    // NEITHER reading resolves (`payload` as a stream value, nor `line payload`
    // as a file-write variable) must still be caught by static analysis.
    let code = "listen on port 8080 as srv\nwrite line payload to srv";
    let tokens = lex_wfl_with_positions(code);
    let program = Parser::new(&tokens).parse().expect("parse");

    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&program);
    let errors = result.expect_err("neither `payload` nor `line payload` is defined");
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("line payload") && e.message.contains("not defined")),
        "expected an undefined-variable error naming `line payload`, got: {errors:?}"
    );
}
