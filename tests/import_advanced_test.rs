// Advanced import features tests
//
// Tests caching, diamond dependencies, nested imports, and containers

mod import_test_helpers;
use import_test_helpers::ImportTestContext;

#[test]
fn test_import_same_file_twice() {
    // Same file imported twice should only execute once
    let helper = r#"
store counter as 0
change counter to counter plus 1
display "Counter: " with counter
"#;

    let main = r#"
load module from "helper.wfl"
load module from "helper.wfl"
"#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("main.wfl", main);
    ctx.add_file("helper.wfl", helper);

    let result = ctx.run("main.wfl");

    // Counter should be 1, not 2 (imported only once)
    assert_eq!(result.trim(), "Counter: 1");
}

#[test]
fn test_diamond_dependency() {
    // Main imports A and B, both A and B import C
    // C should only be imported once
    let file_c = r#"
store shared as "shared_value"
display "C loaded"
"#;

    let file_a = r#"
load module from "file_c.wfl"
display "A loaded"
"#;

    let file_b = r#"
load module from "file_c.wfl"
display "B loaded"
"#;

    let main = r#"
load module from "file_a.wfl"
load module from "file_b.wfl"
display shared
"#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("main.wfl", main);
    ctx.add_file("file_a.wfl", file_a);
    ctx.add_file("file_b.wfl", file_b);
    ctx.add_file("file_c.wfl", file_c);

    let result = ctx.run("main.wfl");

    // Check that C is loaded only once and shared value is accessible
    let lines: Vec<&str> = result.trim().lines().collect();
    assert_eq!(
        lines.len(),
        4,
        "Expected 4 lines of output, got: {}",
        result
    );
    assert_eq!(lines[0], "C loaded");
    assert_eq!(lines[1], "A loaded");
    assert_eq!(lines[2], "B loaded");
    assert_eq!(lines[3], "shared_value");
}

#[test]
fn test_nested_imports() {
    // A imports B, B imports C (chain of imports)
    let file_c = r#"store level as 3"#;

    let file_b = r#"
load module from "file_c.wfl"
store level2 as level plus 1
"#;

    let file_a = r#"
load module from "file_b.wfl"
store level1 as level2 plus 1
display level1
"#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("file_a.wfl", file_a);
    ctx.add_file("file_b.wfl", file_b);
    ctx.add_file("file_c.wfl", file_c);

    let result = ctx.run("file_a.wfl");

    // 3 + 1 + 1 = 5
    assert_eq!(result.trim(), "5");
}

#[test]
fn test_import_with_constants() {
    // Test importing constants/variables
    let constants = r#"
store PI as 3.14159
store E as 2.71828
store APP_NAME as "MyApp"
"#;

    let main = r#"
load module from "constants.wfl"

display "PI: " with PI
display "E: " with E
display "App: " with APP_NAME
"#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("main.wfl", main);
    ctx.add_file("constants.wfl", constants);

    let result = ctx.run("main.wfl");

    let lines: Vec<&str> = result.trim().lines().collect();
    assert_eq!(lines[0], "PI: 3.14159");
    assert_eq!(lines[1], "E: 2.71828");
    assert_eq!(lines[2], "App: MyApp");
}

#[test]
fn test_import_with_actions() {
    let helper = r#"
define action called greet with name:
    display "Hello, " with name with "!"
end action

define action called farewell with name:
    display "Goodbye, " with name with "!"
end action
"#;

    let main = r#"
load module from "helper.wfl"

call greet with "Alice"
call farewell with "Bob"
"#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("main.wfl", main);
    ctx.add_file("helper.wfl", helper);

    let result = ctx.run("main.wfl");

    let lines: Vec<&str> = result.trim().lines().collect();
    assert_eq!(lines[0], "Hello, Alice!");
    assert_eq!(lines[1], "Goodbye, Bob!");
}

#[test]
fn test_import_complex_dependency_graph() {
    // Complex graph: Main → A, B, C; A → D; B → D, E; C → E
    // D and E should each be imported only once

    let file_d = r#"store d_value as "D""#;
    let file_e = r#"store e_value as "E""#;

    let file_a = r#"
load module from "file_d.wfl"
store a_value as "A"
"#;

    let file_b = r#"
load module from "file_d.wfl"
load module from "file_e.wfl"
store b_value as "B"
"#;

    let file_c = r#"
load module from "file_e.wfl"
store c_value as "C"
"#;

    let main = r#"
load module from "file_a.wfl"
load module from "file_b.wfl"
load module from "file_c.wfl"

display d_value with " " with e_value with " " with a_value with " " with b_value with " " with c_value
"#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("main.wfl", main);
    ctx.add_file("file_a.wfl", file_a);
    ctx.add_file("file_b.wfl", file_b);
    ctx.add_file("file_c.wfl", file_c);
    ctx.add_file("file_d.wfl", file_d);
    ctx.add_file("file_e.wfl", file_e);

    let result = ctx.run("main.wfl");

    assert_eq!(result.trim(), "D E A B C");
}

#[test]
fn test_import_order_matters() {
    // Variables from imports should be available after import statement
    let config = r#"store DEBUG_MODE as yes"#;

    let main = r#"
display "Before import"
load module from "config.wfl"
display "After import"

check if DEBUG_MODE:
    display "Debug enabled"
end check
"#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("main.wfl", main);
    ctx.add_file("config.wfl", config);

    let result = ctx.run("main.wfl");

    let lines: Vec<&str> = result.trim().lines().collect();
    assert_eq!(lines[0], "Before import");
    assert_eq!(lines[1], "After import");
    assert_eq!(lines[2], "Debug enabled");
}

#[test]
fn test_import_with_conditionals() {
    let checker = r#"
store is_admin as yes
store is_guest as no
"#;

    let main = r#"
load module from "checker.wfl"

check if is_admin:
    display "Admin access granted"
end check

check if is_guest:
    display "Guest mode"
otherwise:
    display "Regular user"
end check
"#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("main.wfl", main);
    ctx.add_file("checker.wfl", checker);

    let result = ctx.run("main.wfl");

    let lines: Vec<&str> = result.trim().lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "Admin access granted");
    assert_eq!(lines[1], "Regular user");
}

#[test]
fn test_nested_imports_in_subdirectories() {
    // Test that imports inside subdirectories resolve relative to the importing file
    // Main: main.wfl
    // Imports: lib/a.wfl
    // lib/a.wfl imports: util.wfl (should resolve to lib/util.wfl, not ./util.wfl)

    let util = r#"store util_value as "from_util""#;

    let lib_a = r#"
load module from "util.wfl"
store a_value as util_value with "_and_a"
"#;

    let main = r#"
load module from "lib/a.wfl"
display a_value
"#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("main.wfl", main);
    ctx.add_file("lib/a.wfl", lib_a);
    ctx.add_file("lib/util.wfl", util);

    let result = ctx.run("main.wfl");

    // Should successfully import util.wfl from lib/ directory
    assert_eq!(result.trim(), "from_util_and_a");
}

#[test]
fn test_deeply_nested_imports_in_subdirectories() {
    // Test deeper nesting: main → lib/a → lib/inner/b → lib/inner/util

    let util = r#"store base as "util""#;

    let inner_b = r#"
load module from "util.wfl"
store b_val as base with "_b"
"#;

    let lib_a = r#"
load module from "inner/b.wfl"
store a_val as b_val with "_a"
"#;

    let main = r#"
load module from "lib/a.wfl"
display a_val
"#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("main.wfl", main);
    ctx.add_file("lib/a.wfl", lib_a);
    ctx.add_file("lib/inner/b.wfl", inner_b);
    ctx.add_file("lib/inner/util.wfl", util);

    let result = ctx.run("main.wfl");

    // Should resolve: lib/inner/util.wfl from lib/inner/b.wfl
    assert_eq!(result.trim(), "util_b_a");
}
