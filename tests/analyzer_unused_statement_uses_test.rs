//! Issue #711: ANALYZE-UNUSED must count every statement (and expression)
//! that reads a variable as a use. `expect` was the reported entry point;
//! the same catch-all also skipped `create file`, `create list`, container
//! instantiation, and other operand-bearing statements.
//!
//! These tests parse real WFL (or the published CLI) and call the unused-
//! variable pass. They must fail while those arms are missing.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use wfl::analyzer::Analyzer;
use wfl::analyzer::static_analyzer::StaticAnalyzer;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

fn parse(source: &str) -> wfl::parser::ast::Program {
    let tokens = lex_wfl_with_positions(source);
    Parser::new(&tokens)
        .parse()
        .unwrap_or_else(|errors| panic!("test program must parse: {errors:?}\n{source}"))
}

fn unused_messages(source: &str) -> Vec<String> {
    let program = parse(source);
    Analyzer::new()
        .check_unused_variables(&program, 0)
        .into_iter()
        .filter(|diagnostic| diagnostic.code == "ANALYZE-UNUSED")
        .map(|diagnostic| diagnostic.message)
        .collect()
}

fn assert_used(source: &str, names: &[&str]) {
    let messages = unused_messages(source);
    for name in names {
        assert!(
            !messages.iter().any(|message| message.contains(&format!("'{name}'"))),
            "variable `{name}` is read and must not be unused; got {messages:?}\n{source}"
        );
    }
}

fn assert_unused(source: &str, names: &[&str]) {
    let messages = unused_messages(source);
    for name in names {
        assert!(
            messages.iter().any(|message| message.contains(&format!("'{name}'"))),
            "variable `{name}` is never read and must stay unused; got {messages:?}\n{source}"
        );
    }
}

fn analyze_cli(path: &Path) -> (Option<i32>, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_wfl"))
        .args(["--analyze"])
        .arg(path)
        .env("NO_COLOR", "1")
        .output()
        .expect("run wfl --analyze");
    (
        output.status.code(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    )
}

/// The issue's original reproduction: a binding used only by `expect`.
#[test]
fn expect_subject_and_operand_count_as_uses() {
    let source = r#"
describe "demo":
    test "a value used only inside expect":
        store v as "c"
        expect v to equal "c"
    end test
end describe
"#;
    assert_used(source, &["v"]);
    assert_eq!(unused_messages(source), Vec::<String>::new());
}

#[test]
fn expect_both_operands_and_subject_expression_count_as_uses() {
    assert_used(
        r#"
store v as "c"
store w as "c"
expect v to equal w
"#,
        &["v", "w"],
    );
    assert_used(
        r#"
define action called twice with parameters n:
    return n * 2
end action
store base_val as 5
expect twice of base_val to equal 10
"#,
        &["base_val"],
    );
}

/// The issue's control: a truly unread binding still warns.
#[test]
fn genuinely_unread_bindings_still_warn() {
    let source = r#"
store v as "c"
store dead as "never read"
expect v to equal "c"
"#;
    assert_used(source, &["v"]);
    assert_unused(source, &["dead"]);
}

/// `create file at <path> with <content>` reads both operands.
#[test]
fn create_file_operands_count_as_uses() {
    let source = r#"
store target_path as "out.txt"
store body_text as "hello"
create file at target_path with body_text
store dead as "never read"
"#;
    assert_used(source, &["target_path", "body_text"]);
    assert_unused(source, &["dead"]);
}

/// `create list` / `add <value>` reads the seed expression.
#[test]
fn create_list_seed_counts_as_a_use() {
    let source = r#"
store seed_val as 7
create list nums:
    add seed_val
end list
store dead as "never read"
"#;
    assert_used(source, &["seed_val"]);
    assert_unused(source, &["dead"]);
}

/// Container instantiation property initializers read their values.
#[test]
fn container_instantiation_initializer_counts_as_a_use() {
    let source = r#"
create container Person:
    property nm: Text
end

store who_name as "Ada"
create new Person as person_one:
    nm is who_name
end
store dead as "never read"
"#;
    assert_used(source, &["who_name"]);
    assert_unused(source, &["dead"]);
}

/// Remaining operand-bearing statements that used to fall through `_ => {}`.
#[test]
fn unhandled_statement_operands_count_as_uses() {
    let source = r#"
store dir_path as "tmp-dir"
create directory at dir_path

store delete_path as "gone.txt"
delete file at delete_path

store delete_dir as "gone-dir"
delete directory at delete_dir

store include_path as "mod.wfl"
include from include_path

store cmd as "echo"
store cmd_args as ["hi"]
execute command cmd with arguments cmd_args

store get_url as "http://example.invalid"
open url at get_url and read content as get_body

store post_url as "http://example.invalid"
store post_body as "payload"
open url at post_url with method "POST" and body post_body and read content as post_body_text

store request_url as "http://example.invalid"
store request_method as "QUERY"
store request_body as "q"
open url at request_url with method request_method and headers request_headers and body request_body and read response as request_reply

store wait_ms as 5
wait for wait_ms milliseconds

store exit_code as 0
exit program with code exit_code

store map_value as 1
create map mapped:
    "k" is map_value
end map
store request_headers as mapped

store date_text as "2026-01-01"
create date dated as date_text

store time_text as "12:00:00"
create time timed as time_text

store listen_port as 0
store redirect_port as 1
listen on port listen_port redirecting to port redirect_port as redirect_server

store exists_path as "missing.txt"
check if file exists at exists_path:
    display "present"
end check

store spawn_cmd as "echo"
wait for spawn command spawn_cmd as spawned_proc
wait for read output from process spawned_proc as spawn_out
kill process spawned_proc

store dead as "never read"
"#;
    assert_used(
        source,
        &[
            "dir_path",
            "delete_path",
            "delete_dir",
            "include_path",
            "cmd",
            "cmd_args",
            "get_url",
            "post_url",
            "post_body",
            "request_url",
            "request_method",
            "request_headers",
            "request_body",
            "wait_ms",
            "exit_code",
            "map_value",
            "date_text",
            "time_text",
            "listen_port",
            "redirect_port",
            "exists_path",
            "spawn_cmd",
        ],
    );
    assert_unused(source, &["dead"]);
}

/// The issue's gated-suite examples: assertion-only bindings must stay clean.
#[test]
fn gated_expect_only_bindings_are_not_unused() {
    assert_used(
        r#"
store quotient as 10 / 4
expect quotient to equal 2.5
"#,
        &["quotient"],
    );
}

/// Suggested suite-level guard from the issue: gated `*.test.wfl` files
/// must not emit ANALYZE-UNUSED for bindings their statements actually read.
#[test]
fn gated_test_programs_do_not_emit_analyze_unused() {
    let mut files = Vec::new();
    collect_test_wfl(Path::new("TestPrograms"), &mut files);
    assert!(
        !files.is_empty(),
        "expected gated TestPrograms/*.test.wfl files"
    );

    let mut failures = Vec::new();
    for path in &files {
        let source = fs::read_to_string(path).unwrap_or_else(|error| {
            panic!("read {}: {error}", path.display())
        });
        let tokens = lex_wfl_with_positions(&source);
        let Ok(program) = Parser::new(&tokens).parse() else {
            continue;
        };
        let unused: Vec<_> = Analyzer::new()
            .check_unused_variables(&program, 0)
            .into_iter()
            .filter(|diagnostic| diagnostic.code == "ANALYZE-UNUSED")
            .map(|diagnostic| diagnostic.message)
            .collect();
        if !unused.is_empty() {
            failures.push(format!("{}: {unused:?}", path.display()));
        }
    }
    assert!(
        failures.is_empty(),
        "ANALYZE-UNUSED on gated test programs:\n{}",
        failures.join("\n")
    );
}

fn collect_test_wfl(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|error| {
        panic!("read {}: {error}", dir.display())
    });
    for entry in entries {
        let entry = entry.expect("directory entry");
        let path = entry.path();
        if path.is_dir() {
            collect_test_wfl(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("wfl")
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".test.wfl"))
        {
            out.push(path);
        }
    }
}

/// Real-binary reproduction of the issue's `wfl --analyze` command.
#[test]
fn analyze_cli_accepts_expect_create_file_create_list_and_container_uses() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("repro.wfl");
    fs::write(
        &path,
        r#"
describe "demo":
    test "a value used only inside expect":
        store v as "c"
        expect v to equal "c"
    end test
end describe

store target_path as "out.txt"
store body_text as "hello"
create file at target_path with body_text

store seed_val as 7
create list nums:
    add seed_val
end list

create container Person:
    property nm: Text
end
store who_name as "Ada"
create new Person as person_one:
    nm is who_name
end

store dead as "never read"
"#,
    )
    .unwrap();

    let (status, output) = analyze_cli(&path);
    assert_eq!(status, Some(1), "a genuine unused binding must exit 1: {output}");
    for name in ["v", "target_path", "body_text", "seed_val", "who_name"] {
        assert!(
            !output.contains(&format!("Unused variable '{name}'")),
            "`{name}` is read and must not warn: {output}"
        );
    }
    assert!(
        output.contains("Unused variable 'dead'"),
        "genuinely unused variable was hidden: {output}"
    );
}
