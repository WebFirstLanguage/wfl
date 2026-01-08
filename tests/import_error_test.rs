// Error handling tests for the import system
//
// Tests path resolution, missing files, and syntax errors in imported files

mod import_test_helpers;
use import_test_helpers::ImportTestContext;

#[test]
fn test_import_relative_path() {
    let helper = r#"store LIB_CONSTANT as "from lib""#;

    let main = r#"
load module from "lib/helper.wfl"
display LIB_CONSTANT
"#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("main.wfl", main);
    ctx.add_file("lib/helper.wfl", helper);

    let result = ctx.run("main.wfl");
    assert_eq!(result.trim(), "from lib");
}

#[test]
fn test_import_missing_file_error() {
    let main = r#"load module from "nonexistent.wfl""#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("main.wfl", main);

    let result = ctx.run("main.wfl");

    // Check that error message contains key information
    assert!(
        result.contains("error") || result.contains("Error"),
        "Expected error message, got: {}",
        result
    );
    assert!(
        result.contains("nonexistent.wfl"),
        "Error should mention the missing file, got: {}",
        result
    );
    assert!(
        result.contains("Cannot find") || result.contains("not found") || result.contains("Searched"),
        "Error should indicate file not found, got: {}",
        result
    );
}

#[test]
fn test_import_file_with_syntax_error() {
    let helper = r#"store value as"#; // Missing value - syntax error

    let main = r#"load module from "helper.wfl""#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("main.wfl", main);
    ctx.add_file("helper.wfl", helper);

    let result = ctx.run("main.wfl");

    // Check that error is reported
    assert!(
        result.contains("error") || result.contains("Error"),
        "Expected error message, got: {}",
        result
    );
    assert!(
        result.contains("helper.wfl"),
        "Error should mention the imported file, got: {}",
        result
    );
}

#[test]
fn test_import_parent_directory() {
    let helper = r#"store PARENT_VALUE as "from parent""#;

    let main = r#"
load module from "../helper.wfl"
display PARENT_VALUE
"#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("helper.wfl", helper);
    ctx.add_file("subdir/main.wfl", main);

    let result = ctx.run("subdir/main.wfl");
    assert_eq!(result.trim(), "from parent");
}

#[test]
fn test_import_deep_nested_path() {
    let helper = r#"store DEEP_VALUE as "from deep""#;

    let main = r#"
load module from "a/b/c/helper.wfl"
display DEEP_VALUE
"#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("main.wfl", main);
    ctx.add_file("a/b/c/helper.wfl", helper);

    let result = ctx.run("main.wfl");
    assert_eq!(result.trim(), "from deep");
}

#[test]
fn test_import_simplified_syntax() {
    let helper = r#"
define action called greet:
    display "Hello from simplified!"
end action
"#;

    let main = r#"
load "helper.wfl"
call greet
"#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("main.wfl", main);
    ctx.add_file("helper.wfl", helper);

    let result = ctx.run("main.wfl");
    assert_eq!(result.trim(), "Hello from simplified!");
}
