// Basic import functionality tests following TDD methodology
//
// These tests drive the implementation of WFL's import system.
// Each test should initially fail, then pass after implementing the feature.

mod import_test_helpers;
use import_test_helpers::ImportTestContext;

#[test]
fn test_import_simple_file() {
    let helper = r#"
define action called greet:
    display "Hello from helper!"
end action
"#;

    let main = r#"
load module from "helper.wfl"
call greet
"#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("main.wfl", main);
    ctx.add_file("helper.wfl", helper);

    let result = ctx.run("main.wfl");
    assert_eq!(result.trim(), "Hello from helper!");
}

#[test]
fn test_import_shares_variables() {
    let helper = r#"store greeting as "Hello from imported module!""#;

    let main = r#"
load module from "helper.wfl"
display greeting
"#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("main.wfl", main);
    ctx.add_file("helper.wfl", helper);

    let result = ctx.run("main.wfl");
    assert_eq!(result.trim(), "Hello from imported module!");
}

#[test]
fn test_multiple_imports() {
    let helper1 = r#"store value1 as "first""#;
    let helper2 = r#"store value2 as "second""#;

    let main = r#"
load module from "helper1.wfl"
load module from "helper2.wfl"
display value1 with " " with value2
"#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("main.wfl", main);
    ctx.add_file("helper1.wfl", helper1);
    ctx.add_file("helper2.wfl", helper2);

    let result = ctx.run("main.wfl");
    assert_eq!(result.trim(), "first second");
}
