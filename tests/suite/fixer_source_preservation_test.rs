use std::fs;
use std::path::{Path, PathBuf};

use wfl::fixer::{CodeFixer, validate_source, write_fixed_file};
use wfl::lexer::lex_wfl_with_positions;
use wfl::lexer::token::Token;
use wfl::parser::Parser;

/// Require both the input and the checked formatter result to parse, so the
/// infallible formatter's original-source fallback cannot hide a failure.
fn fix(source: &str) -> String {
    let program = Parser::new(&lex_wfl_with_positions(source))
        .parse()
        .unwrap_or_else(|errors| panic!("original source must parse: {errors:?}\n{source}"));
    let (fixed, _) = CodeFixer::new()
        .fix_checked(&program, source)
        .expect("formatting valid source must succeed");
    Parser::new(&lex_wfl_with_positions(&fixed))
        .parse()
        .unwrap_or_else(|errors| panic!("fixed source must parse: {errors:?}\n{fixed}"));
    fixed
}

/// Remove physical newline tokens while retaining syntax and literal values
/// for comparisons that permit layout changes.
fn significant_tokens(source: &str) -> Vec<Token> {
    lex_wfl_with_positions(source)
        .into_iter()
        .map(|positioned| positioned.token)
        .filter(|token| !matches!(token, Token::Eol | Token::Newline))
        .collect()
}

/// Extract original literal spans so changed escapes or multiline whitespace
/// cannot pass merely because their decoded token values still match.
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
    for name in ["Store", "Display", "Nothing", "Undefined", "Missing"] {
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
fn local_variable_renaming_does_not_change_map_keys() {
    let source = concat!(
        "store userName as \"local\"\n",
        "create map profile:\n",
        "    userName is \"remote\"\n",
        "end map\n",
        "display stringify_json of profile\n",
        "display userName\n",
    );
    let fixed = fix(source);
    assert!(
        fixed.contains("userName is \"remote\""),
        "map keys are data, even when their spelling matches a local variable: {fixed}"
    );
}

#[test]
fn local_variable_renaming_does_not_change_pattern_capture_names() {
    let source = concat!(
        "store userName as \"local\"\n",
        "create pattern person:\n",
        "    capture {one or more letter} as userName\n",
        "end pattern\n",
        "display userName\n",
    );
    let fixed = fix(source);
    assert!(
        fixed.contains("capture {one or more letter} as userName"),
        "pattern capture names are externally visible keys: {fixed}"
    );
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
fn local_variable_renaming_does_not_change_public_method_parameters() {
    let source = concat!(
        "store userName as \"local\"\n",
        "create container Example:\n",
        "    action greet needs userName: Text:\n",
        "        display userName\n",
        "    end\n",
        "end\n",
        "display userName\n",
    );
    let fixed = fix(source);
    assert!(
        fixed.contains("action greet needs userName: Text:"),
        "public named parameters must preserve their spelling: {fixed}"
    );
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
    let (second, summary) = CodeFixer::new()
        .fix_checked(&program, &fixed)
        .expect("second fix succeeds");
    assert_eq!(second, fixed, "a second fix must be a no-op");
    assert_eq!(summary.total(), 0, "no-op fixes must report zero changes");
}

#[test]
fn keyword_prefixes_in_identifiers_remain_valid_source() {
    let source = concat!(
        "store content_type as \"text/plain\"\n",
        "store list_items as [1, 2]\n",
        "store is_active as yes\n",
        "display content_type\n",
    );
    validate_source(source).expect("valid underscore identifiers pass strict validation");
    assert_eq!(significant_tokens(&fix(source)), significant_tokens(source));
}

#[test]
fn rejects_unrecognized_bytes_and_malformed_literals() {
    for source in [
        "store total as 1 @\n",
        "display \"unterminated\n",
        "display \"invalid \\q escape\"\n",
        "store huge as 99999999999999999999999999999999999999\n",
    ] {
        assert!(
            validate_source(source).is_err(),
            "invalid source must be rejected without dropping tokens: {source:?}"
        );
    }
}

#[test]
fn atomic_write_refuses_to_replace_a_concurrently_modified_source() {
    let directory = tempfile::tempdir().expect("test directory");
    let path = directory.path().join("program.wfl");
    let original = "store userName as 1\n";
    let newer = "store userName as 2\n";
    fs::write(&path, newer).expect("write newer edit");

    let error = write_fixed_file(&path, original, "store user_name as 1\n")
        .expect_err("a stale formatter must not overwrite newer editor changes");

    assert!(error.to_string().contains("changed"), "{error}");
    assert_eq!(fs::read_to_string(&path).expect("read source"), newer);
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

/// Construct the documented lock marker for a real source path. The canonical
/// basename digest names the destination independently of its replaceable inode.
fn formatter_lock_path(path: &Path) -> PathBuf {
    use sha2::{Digest, Sha256};

    let destination = fs::canonicalize(path).expect("canonical source path");
    let name = destination.file_name().expect("source filename");
    let digest = Sha256::digest(name.as_encoded_bytes());
    destination.with_file_name(format!(".wfl-fix-{digest:x}.lock"))
}

#[test]
fn atomic_write_respects_another_writer_lock_and_releases_its_own_lock() {
    use std::sync::mpsc;
    use std::time::Duration;

    let directory = tempfile::tempdir().expect("test directory");
    let path = directory.path().join("program.wfl");
    let other_path = directory.path().join("other.wfl");
    let original = "store userName as 1\n";
    let fixed = "store user_name as 1\n";
    fs::write(&path, original).expect("write source");
    fs::write(&other_path, original).expect("write unrelated source");
    let lock_path = formatter_lock_path(&path);
    let held_path = lock_path.clone();
    let (ready_tx, ready_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let owner = std::thread::spawn(move || {
        let lock = fs::File::create_new(&held_path).expect("claim writer lock");
        ready_tx.send(()).expect("announce lock ownership");
        // Bound the fixture's lifetime even if an implementation blocks on the
        // lock rather than reporting contention immediately.
        let released = release_rx.recv_timeout(Duration::from_secs(10));
        drop(lock);
        fs::remove_file(&held_path).expect("owner releases its lock");
        released.expect("formatter must not wait for an occupied lock");
    });
    ready_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("writer claims lock before formatter starts");

    let blocked = write_fixed_file(&path, original, fixed);
    let preserved_source = fs::read_to_string(&path).expect("read protected source");
    let preserved_lock = lock_path.exists();
    let unrelated = write_fixed_file(&other_path, original, fixed);
    let unrelated_lock_left = formatter_lock_path(&other_path).exists();
    release_tx.send(()).expect("allow owner to release lock");
    owner.join().expect("lock owner completes");

    let error = blocked.expect_err("an active writer must exclude another formatter");
    assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock, "{error}");
    assert!(error.to_string().contains("lock"), "{error}");
    assert_eq!(preserved_source, original);
    assert!(
        preserved_lock,
        "a non-owner must not remove the writer lock"
    );
    unrelated.expect("an occupied source must not block unrelated files");
    assert_eq!(fs::read_to_string(&other_path).unwrap(), fixed);
    assert!(!unrelated_lock_left, "successful writes release their lock");

    write_fixed_file(&path, original, fixed).expect("retry after owner releases lock");
    assert_eq!(fs::read_to_string(&path).unwrap(), fixed);
    assert!(!lock_path.exists(), "retry must release its own lock");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
}

#[test]
fn atomic_write_preserves_an_abandoned_lock_until_explicit_recovery() {
    let directory = tempfile::tempdir().expect("test directory");
    let path = directory.path().join("program.wfl");
    let original = "store userName as 1\n";
    fs::write(&path, original).expect("write source");
    let lock_path = formatter_lock_path(&path);
    fs::write(&lock_path, "owner information must survive").expect("abandoned lock");

    let error = write_fixed_file(&path, original, "store user_name as 1\n")
        .expect_err("a formatter must not guess that an existing lock is abandoned");

    assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock, "{error}");
    assert!(error.to_string().contains("stopped"), "{error}");
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    assert_eq!(
        fs::read_to_string(&lock_path).unwrap(),
        "owner information must survive"
    );
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
}

/// Append every nested WFL corpus file, failing on unreadable directories or
/// entries rather than silently reducing the regression coverage.
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

/// Ignore identifier styling while retaining every literal, operator, keyword,
/// and punctuation token. Separate tests check collisions, reference updates,
/// and public API names without this normalization.
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

/// Compare normalized tokens, raw literal spans, and second-pass output for
/// every parseable corpus file; require nonzero coverage and report each failure.
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
        let (fixed, _) = match CodeFixer::new().fix_checked(&program, &source) {
            Ok(result) => result,
            Err(error) => {
                failures.push(format!(
                    "{}: formatting failed: {error}",
                    relative.display()
                ));
                continue;
            }
        };
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
            let (second, _) = CodeFixer::new()
                .fix_checked(&reparsed, &fixed)
                .expect("second corpus fix succeeds");
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
