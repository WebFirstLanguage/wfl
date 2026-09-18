use std::fs;
use std::path::{Path, PathBuf};

use wfl::fixer::CodeFixer;
use wfl::lexer::lex_wfl_with_positions;
use wfl::lexer::token::Token;
use wfl::parser::Parser;

fn fix(source: &str) -> String {
    let program = Parser::new(&lex_wfl_with_positions(source))
        .parse()
        .unwrap_or_else(|errors| panic!("original source must parse: {errors:?}\n{source}"));
    let (fixed, _) = CodeFixer::new().fix(&program, source);
    Parser::new(&lex_wfl_with_positions(&fixed))
        .parse()
        .unwrap_or_else(|errors| panic!("fixed source must parse: {errors:?}\n{fixed}"));
    fixed
}

fn significant_tokens(source: &str) -> Vec<Token> {
    lex_wfl_with_positions(source)
        .into_iter()
        .map(|positioned| positioned.token)
        .filter(|token| !matches!(token, Token::Eol | Token::Newline))
        .collect()
}

fn raw_string_literals(source: &str) -> Vec<String> {
    lex_wfl_with_positions(source)
        .into_iter()
        .filter(|positioned| matches!(positioned.token, Token::StringLiteral(_)))
        .map(|positioned| source[positioned.byte_start..positioned.byte_end].to_owned())
        .collect()
}

#[test]
fn preserves_comments_including_inline_and_comment_only_programs() {
    for source in [
        "// license and explanation\nstore total as 1 // an inline note\n# another comment\ndisplay total # final note\n",
        "// This file intentionally has no statements.\n# Keep both comment syntaxes.\n",
    ] {
        let fixed = fix(source);
        for line in source.lines() {
            assert!(
                fixed.contains(line),
                "formatting discarded a comment or changed its text: {line:?}\n{fixed}"
            );
        }
    }
}

#[test]
fn preserves_escaped_literal_source_and_unicode() {
    let source = concat!(
        "store greeting as \"Grüße 世界 🦀\"\n",
        "store escaped as \"quote: \\\"; slash: \\\\; newline: \\n; tab: \\t\"\n",
        "store markers as \"https://example.test/#part\"\n",
        "display greeting\n",
    );
    let fixed = fix(source);
    assert_eq!(raw_string_literals(&fixed), raw_string_literals(source));
    assert_eq!(significant_tokens(&fixed), significant_tokens(source));
}

#[test]
fn preserves_crlf_and_whitespace_inside_multiline_literals() {
    let source = "store message as \"first  \r\n  second\t \r\nlast\"\r\ndisplay message\r\n";
    let fixed = fix(source);
    assert_eq!(raw_string_literals(&fixed), raw_string_literals(source));
    assert_eq!(
        fixed, source,
        "already formatted CRLF source must be stable"
    );
}

#[test]
fn preserves_parentheses_and_operator_precedence() {
    let source = concat!(
        "store total as (1 + 2) times 3\n",
        "store quotient as 24 divided by (2 times 3)\n",
        "store difference as 10 - (3 - 1)\n",
        "display total\n",
    );
    let fixed = fix(source);
    assert_eq!(
        significant_tokens(&fixed),
        significant_tokens(source),
        "formatting must not alter grouping or operators"
    );
}

#[test]
fn does_not_merge_identifiers_that_have_the_same_snake_case_spelling() {
    let source = concat!(
        "store Counter as 10\n",
        "store counter as 20\n",
        "display Counter\n",
        "display counter\n",
    );
    let fixed = fix(source);
    assert_eq!(
        significant_tokens(&fixed),
        significant_tokens(source),
        "both declarations and both distinct references must survive"
    );
}

#[test]
fn identifier_normalization_does_not_introduce_keywords_or_literal_tokens() {
    for name in ["Store", "Display", "True", "Nothing", "Yes"] {
        let source = format!("store {name} as 10\ndisplay {name}\n");
        let fixed = fix(&source);
        assert_eq!(
            significant_tokens(&fixed),
            significant_tokens(&source),
            "normalizing {name} would turn an identifier into language syntax"
        );
    }
}

#[test]
fn local_variable_renaming_does_not_change_external_member_access() {
    let source = concat!(
        "store userName as \"local\"\n",
        "store profile as parse_json of \"{\\\"userName\\\":\\\"remote\\\"}\"\n",
        "display profile.userName\n",
        "display userName\n",
    );
    let fixed = fix(source);
    assert!(
        fixed.contains("profile.userName"),
        "a member supplied by external JSON must retain its spelling: {fixed}"
    );
    assert_eq!(raw_string_literals(&fixed), raw_string_literals(source));
}

#[test]
fn local_action_renaming_does_not_change_public_method_names() {
    let source = concat!(
        "define action called getName:\n",
        "    give back \"local\"\n",
        "end action\n",
        "create container Example:\n",
        "    action getName:\n",
        "        give back \"method\"\n",
        "    end\n",
        "end\n",
        "create new Example as sample:\n",
        "end\n",
        "display sample.getName()\n",
        "call getName\n",
    );
    let fixed = fix(source);
    assert!(fixed.contains("action getName:"), "{fixed}");
    assert!(fixed.contains("sample.getName()"), "{fixed}");
}

#[test]
fn preserves_public_container_property_and_method_names() {
    let source = concat!(
        "create container Example:\n",
        "    property displayName: Text\n",
        "    action getName:\n",
        "        display \"example\"\n",
        "    end\n",
        "end\n",
        "create new Example as sample:\n",
        "    displayName is \"demo\"\n",
        "end\n",
        "sample.getName()\n",
    );
    let fixed = fix(source);
    assert_eq!(significant_tokens(&fixed), significant_tokens(source));
}

#[test]
fn preserves_constants_tests_and_pattern_definitions() {
    let source = concat!(
        "store new constant LIMIT as 10\n",
        "create pattern digits_only:\n",
        "    one or more digit\n",
        "end pattern\n",
        "describe \"preservation\":\n",
        "    test \"constant remains available\":\n",
        "        expect LIMIT to equal 10\n",
        "    end test\n",
        "end describe\n",
    );
    let fixed = fix(source);
    assert_eq!(significant_tokens(&fixed), significant_tokens(source));
}

#[test]
fn fixing_is_idempotent_after_safe_local_variable_renaming() {
    let source = "store userName as \"Ada\"  \ndisplay userName\n";
    let fixed = fix(source);
    assert!(fixed.contains("store user_name as"), "{fixed}");
    assert!(fixed.contains("display user_name"), "{fixed}");
    let program = Parser::new(&lex_wfl_with_positions(&fixed))
        .parse()
        .expect("first fix parses");
    let (second, summary) = CodeFixer::new().fix(&program, &fixed);
    assert_eq!(second, fixed, "a second fix must be a no-op");
    assert_eq!(summary.total(), 0, "no-op fixes must report zero changes");
}

fn collect_programs(directory: &Path, paths: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("read TestPrograms directory") {
        let path = entry.expect("read TestPrograms entry").path();
        if path.is_dir() {
            collect_programs(&path, paths);
        } else if path.extension().is_some_and(|extension| extension == "wfl") {
            paths.push(path);
        }
    }
}

// Ignore permitted identifier styling, while retaining every literal, operator,
// keyword and punctuation token. Dedicated tests above check collision handling,
// consistent references, and public API names without normalization.
fn tokens_with_normalized_identifier_style(source: &str) -> Vec<Token> {
    significant_tokens(source)
        .into_iter()
        .map(|token| match token {
            Token::Identifier(name) => Token::Identifier(
                name.chars()
                    .filter(|character| *character != '_' && !character.is_whitespace())
                    .flat_map(char::to_lowercase)
                    .collect(),
            ),
            other => other,
        })
        .collect()
}

#[test]
fn all_parseable_test_programs_preserve_syntax_literals_and_idempotence() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("TestPrograms");
    let mut paths = Vec::new();
    collect_programs(&root, &mut paths);
    paths.sort();

    let mut parsed_count = 0;
    let mut failures = Vec::new();
    for path in paths {
        let source = fs::read_to_string(&path).expect("read WFL test program");
        let Ok(program) = Parser::new(&lex_wfl_with_positions(&source)).parse() else {
            // Formatting has a parsed-program precondition. Parse failures in
            // the existing corpus remain the parser/integration suite's job.
            continue;
        };
        parsed_count += 1;
        let relative = path.strip_prefix(&root).expect("path within TestPrograms");
        let (fixed, _) = CodeFixer::new().fix(&program, &source);
        let reparsed = Parser::new(&lex_wfl_with_positions(&fixed)).parse();
        if let Err(errors) = &reparsed {
            failures.push(format!(
                "{}: does not reparse: {:?}",
                relative.display(),
                errors.first()
            ));
        }
        if tokens_with_normalized_identifier_style(&fixed)
            != tokens_with_normalized_identifier_style(&source)
        {
            failures.push(format!("{}: changed syntax or values", relative.display()));
        }
        if raw_string_literals(&fixed) != raw_string_literals(&source) {
            failures.push(format!("{}: changed literal source", relative.display()));
        }
        if let Ok(reparsed) = reparsed {
            let (second, _) = CodeFixer::new().fix(&reparsed, &fixed);
            if second != fixed {
                failures.push(format!("{}: not idempotent", relative.display()));
            }
        }
    }

    assert!(parsed_count > 0, "corpus coverage must not be vacuous");
    assert!(
        failures.is_empty(),
        "{} failures across {parsed_count} parsed programs (first 30):\n{}",
        failures.len(),
        failures.into_iter().take(30).collect::<Vec<_>>().join("\n")
    );
    eprintln!("Verified source preservation for {parsed_count} WFL programs");
}
