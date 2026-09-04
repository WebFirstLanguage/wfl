//! Diamond includes: two files that both `include from` the same shared file
//! must be usable together. The shared file's definitions already live in the
//! scope after the first include, so a second include of the same file into
//! that scope (or a scope that can already see it) is a no-op rather than a
//! fatal "already defined" error. A genuine cycle is still rejected.

use std::fs;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;
use wfl::exec::budget::{BudgetLimits, ExecutionBudget};
use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

async fn run(main_file: &Path) -> Result<Interpreter, String> {
    run_with(main_file, Interpreter::new()).await
}

async fn run_with(main_file: &Path, mut interpreter: Interpreter) -> Result<Interpreter, String> {
    let source = fs::read_to_string(main_file).expect("read main");
    let tokens = lex_wfl_with_positions(&source);
    let ast = Parser::new(&tokens)
        .parse()
        .unwrap_or_else(|e| panic!("Parse failed: {e:?}"));
    interpreter.set_source_file(main_file.to_path_buf());
    match interpreter.interpret(&ast).await {
        Ok(_) => Ok(interpreter),
        Err(errors) => Err(errors
            .iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
            .join("\n")),
    }
}

fn global(interpreter: &Interpreter, name: &str) -> Value {
    interpreter
        .global_env()
        .borrow()
        .get(name)
        .unwrap_or_else(|| panic!("`{name}` is not defined in the global scope"))
}

fn assert_text(interpreter: &Interpreter, name: &str, expected: &str) {
    match global(interpreter, name) {
        Value::Text(t) => assert_eq!(&*t, expected, "value of `{name}`"),
        other => panic!("`{name}` should be text, got {other:?}"),
    }
}

fn assert_number(interpreter: &Interpreter, name: &str, expected: f64) {
    match global(interpreter, name) {
        Value::Number(n) => assert_eq!(n, expected, "value of `{name}`"),
        other => panic!("`{name}` should be a number, got {other:?}"),
    }
}

fn write_diamond(dir: &Path) {
    fs::write(
        dir.join("util.wfl"),
        r#"
store util_loads as 1
define action called shout with parameters msg:
    give back msg with "!"
end action
"#,
    )
    .unwrap();
    fs::write(
        dir.join("auth.wfl"),
        r#"
include from "util.wfl"
define action called auth_check with parameters who:
    give back shout of who
end action
"#,
    )
    .unwrap();
    fs::write(
        dir.join("render.wfl"),
        r#"
include from "util.wfl"
define action called render_page with parameters title:
    give back shout of title
end action
"#,
    )
    .unwrap();
}

#[tokio::test]
async fn diamond_include_reaches_shared_file_twice_without_error() {
    let dir = TempDir::new().unwrap();
    write_diamond(dir.path());
    let main = dir.path().join("main.wfl");
    fs::write(
        &main,
        r#"
include from "auth.wfl"
include from "render.wfl"
store a as auth_check of "alice"
store r as render_page of "home"
"#,
    )
    .unwrap();
    let interpreter = run(&main)
        .await
        .unwrap_or_else(|e| panic!("diamond include failed: {e}"));
    assert_text(&interpreter, "a", "alice!");
    assert_text(&interpreter, "r", "home!");
    // The shared leaf executed once: its `store` did not run a second time.
    assert_number(&interpreter, "util_loads", 1.0);
}

#[tokio::test]
async fn direct_double_include_of_same_file_is_a_no_op() {
    let dir = TempDir::new().unwrap();
    write_diamond(dir.path());
    let main = dir.path().join("main.wfl");
    fs::write(
        &main,
        r#"
include from "util.wfl"
include from "util.wfl"
store s as shout of "x"
"#,
    )
    .unwrap();
    let interpreter = run(&main)
        .await
        .unwrap_or_else(|e| panic!("double include failed: {e}"));
    assert_text(&interpreter, "s", "x!");
}

#[tokio::test]
async fn include_inside_action_body_repeats_per_call_scope() {
    // Each call of `prepare_greeting` gets a fresh local scope that cannot see the
    // previous call's include, so the file must run again for that scope.
    let dir = TempDir::new().unwrap();
    write_diamond(dir.path());
    let main = dir.path().join("main.wfl");
    fs::write(
        &main,
        r#"
define action called prepare_greeting with parameters who:
    include from "util.wfl"
    give back shout of who
end action
store first as prepare_greeting of "a"
store second as prepare_greeting of "b"
"#,
    )
    .unwrap();
    let interpreter = run(&main)
        .await
        .unwrap_or_else(|e| panic!("per-call include failed: {e}"));
    assert_text(&interpreter, "first", "a!");
    assert_text(&interpreter, "second", "b!");
}

#[tokio::test]
async fn include_already_visible_from_outer_scope_is_skipped_inside_action() {
    // `util.wfl` is included at top level; an action body that includes it
    // again can already see its definitions, so the include must not re-run
    // the file into the local scope (which would collide with the outer one).
    let dir = TempDir::new().unwrap();
    write_diamond(dir.path());
    let main = dir.path().join("main.wfl");
    fs::write(
        &main,
        r#"
include from "util.wfl"
define action called prepare_greeting with parameters who:
    include from "util.wfl"
    give back shout of who
end action
store first as prepare_greeting of "a"
"#,
    )
    .unwrap();
    let interpreter = run(&main)
        .await
        .unwrap_or_else(|e| panic!("nested re-include failed: {e}"));
    assert_text(&interpreter, "first", "a!");
    assert_number(&interpreter, "util_loads", 1.0);
}

#[tokio::test]
async fn genuine_include_cycle_is_still_rejected() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.wfl"), "include from \"b.wfl\"\n").unwrap();
    fs::write(dir.path().join("b.wfl"), "include from \"a.wfl\"\n").unwrap();
    let err = run(&dir.path().join("a.wfl"))
        .await
        .err()
        .expect("a.wfl <-> b.wfl cycle must fail");
    assert!(
        err.contains("Circular dependency detected"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn include_in_a_loop_body_runs_again_in_each_recycled_iteration_scope() {
    // Loop iteration scopes are recycled (cleared and reused). A file that
    // installs only a plain variable leaves no weak references behind, so
    // the recycled scope must forget the include along with the variable.
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("val.wfl"), "store loop_val as 7\n").unwrap();
    let main = dir.path().join("main.wfl");
    fs::write(
        &main,
        r#"
store total as 0
count from 1 to 3:
    include from "val.wfl"
    change total to total plus loop_val
end count
"#,
    )
    .unwrap();
    let interpreter = run(&main)
        .await
        .unwrap_or_else(|e| panic!("loop include failed: {e}"));
    assert_number(&interpreter, "total", 21.0);
}

#[tokio::test]
async fn side_effect_only_file_still_runs_on_every_include() {
    // Backward compatibility: a file that defines nothing has nothing for a
    // later include to collide with, so it keeps running each time.
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("bump.wfl"), "change n to n plus 1\n").unwrap();
    let main = dir.path().join("main.wfl");
    fs::write(
        &main,
        r#"
store n as 0
include from "bump.wfl"
include from "bump.wfl"
"#,
    )
    .unwrap();
    let interpreter = run(&main)
        .await
        .unwrap_or_else(|e| panic!("repeated side-effect include failed: {e}"));
    assert_number(&interpreter, "n", 2.0);
}

#[tokio::test]
async fn already_visible_include_is_a_no_op_even_at_the_import_depth_ceiling() {
    // With a ceiling of one nested file, `wrapper.wfl` may not enter another
    // file. Its include of `util.wfl` is already satisfied by the top-level
    // include, so it must be a no-op rather than a depth error.
    let dir = TempDir::new().unwrap();
    write_diamond(dir.path());
    fs::write(
        dir.path().join("wrapper.wfl"),
        "include from \"util.wfl\"\nstore via_wrapper as shout of \"w\"\n",
    )
    .unwrap();
    let main = dir.path().join("main.wfl");
    fs::write(
        &main,
        "include from \"util.wfl\"\ninclude from \"wrapper.wfl\"\n",
    )
    .unwrap();
    let mut interpreter = Interpreter::new();
    interpreter.set_budget(Arc::new(ExecutionBudget::new(BudgetLimits {
        max_import_depth: 1,
        ..BudgetLimits::default()
    })));
    let interpreter = run_with(&main, interpreter)
        .await
        .unwrap_or_else(|e| panic!("no-op include must not consume import depth: {e}"));
    assert_text(&interpreter, "via_wrapper", "w!");
}

#[tokio::test]
async fn load_module_cannot_overload_an_outer_action_and_is_rejected_before_running() {
    // The module runs in an isolated scope where the runtime rejects any
    // same-name definition. The analyzer must reject it first, so the
    // module's earlier statements (here: a mutation of `ran`) never run.
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("mod.wfl"),
        r#"
change ran to "yes"
define action called greet with parameters a and b:
    give back a with b
end action
"#,
    )
    .unwrap();
    let main = dir.path().join("main.wfl");
    fs::write(
        &main,
        r#"
store ran as "no"
define action called greet with parameters who:
    give back "hi " with who
end action
load module from "mod.wfl"
"#,
    )
    .unwrap();
    let err = run(&main)
        .await
        .err()
        .expect("a module redefining an outer action must fail");
    assert!(
        err.contains("Semantic error in module") && err.contains("outer scope"),
        "expected an analysis-time rejection, got: {err}"
    );
}
