//! Issue #708: an include-exposed variable can be read and `store`d, but
//! `change` is a fatal ANALYZE-SEMANTIC "not defined" that stops the
//! program. The runtime already mutates the shared binding; the analyzer
//! must agree.

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;
use wfl::analyzer::Analyzer;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

fn wfl(args: &[&str], path: &Path) -> (Option<i32>, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_wfl"))
        .args(args)
        .arg(path)
        .env("NO_COLOR", "1")
        .current_dir(path.parent().expect("file has a parent"))
        .output()
        .expect("run WFL");
    (
        output.status.code(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    )
}

fn analyze_errors(source: &str) -> Vec<String> {
    let tokens = lex_wfl_with_positions(source);
    let program = Parser::new(&tokens)
        .parse()
        .unwrap_or_else(|errors| panic!("test program must parse: {errors:?}\n{source}"));
    let mut analyzer = Analyzer::new();
    match analyzer.analyze(&program) {
        Ok(()) => analyzer
            .get_warnings()
            .iter()
            .map(|error| error.message.clone())
            .collect(),
        Err(errors) => errors.into_iter().map(|error| error.message).collect(),
    }
}

/// Issue repro: `change` of an included variable must run, not exit 3.
#[test]
fn changing_an_included_variable_runs() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("inc_lib.wfl"), "store shared_flag as no\n").unwrap();
    let main = dir.path().join("main.wfl");
    fs::write(
        &main,
        r#"
include from "inc_lib.wfl"
display shared_flag
change shared_flag to yes
display shared_flag
"#,
    )
    .unwrap();

    let (status, output) = wfl(&[], &main);
    assert_eq!(
        status,
        Some(0),
        "included variable change must run: {output}"
    );
    assert!(
        !output.contains("Variable 'shared_flag' is not defined"),
        "change of an included variable must not be a fatal undefined: {output}"
    );
    let lowered = output.to_lowercase();
    assert!(
        lowered.contains("no") && lowered.contains("yes"),
        "expected no then yes: {output}"
    );
}

/// The included file's own action must observe the includer's `change`.
#[test]
fn included_module_observes_caller_change() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("inc_lib.wfl"),
        r#"
store shared_val as "original"
define action called show_shared:
    display shared_val
end action
"#,
    )
    .unwrap();
    let main = dir.path().join("main.wfl");
    fs::write(
        &main,
        r#"
include from "inc_lib.wfl"
change shared_val to "overwritten"
call show_shared
"#,
    )
    .unwrap();

    let (status, output) = wfl(&[], &main);
    assert_eq!(
        status,
        Some(0),
        "caller change of an included variable must run: {output}"
    );
    assert!(
        output.contains("overwritten"),
        "included action must see the mutated binding: {output}"
    );
}

/// Without `include from`, `change` of an unknown name stays fatal.
#[test]
fn changing_an_undefined_variable_without_includes_is_still_fatal() {
    let errors = analyze_errors("change shared_flag to yes\n");
    assert!(
        errors
            .iter()
            .any(|message| message.contains("Variable 'shared_flag' is not defined")),
        "no-include programs must still reject unknown change targets: {errors:?}"
    );
}

/// The analyzer must not fatal-error `change` when the file uses `include from`.
#[test]
fn analyzer_does_not_fatal_change_when_program_has_includes() {
    let tokens = lex_wfl_with_positions(
        r#"
include from "inc_lib.wfl"
change shared_flag to yes
"#,
    );
    let program = Parser::new(&tokens).parse().expect("must parse");
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&program);
    assert!(
        result.is_ok(),
        "include-aware change must not be a fatal error: {result:?}"
    );
    assert!(
        analyzer
            .get_warnings()
            .iter()
            .any(|warning| warning.message.contains("Undefined variable 'shared_flag'")),
        "change of an unresolved name with includes present must warn as a variable, not an action: {:?}",
        analyzer.get_warnings()
    );
    assert!(
        analyzer
            .get_warnings()
            .iter()
            .all(|warning| !warning.message.contains("Undefined action 'shared_flag'")),
        "include-aware change must not reuse the action warning: {:?}",
        analyzer.get_warnings()
    );
}
