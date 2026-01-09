// Circular dependency detection tests
//
// Tests that circular imports are properly detected and reported

mod import_test_helpers;
use import_test_helpers::ImportTestContext;

#[test]
fn test_circular_import_direct() {
    // A imports B, B imports A
    let file_a = r#"
load module from "file_b.wfl"
store a_value as "A"
"#;

    let file_b = r#"
load module from "file_a.wfl"
store b_value as "B"
"#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("file_a.wfl", file_a);
    ctx.add_file("file_b.wfl", file_b);

    let result = ctx.run("file_a.wfl");

    // Check that circular dependency is detected
    assert!(
        result.contains("circular") || result.contains("Circular"),
        "Expected circular dependency error, got: {}",
        result
    );
    assert!(
        result.contains("file_a.wfl") && result.contains("file_b.wfl"),
        "Error should mention both files in the cycle, got: {}",
        result
    );
}

#[test]
fn test_circular_import_indirect() {
    // A → B → C → A (three-way circular dependency)
    let file_a = r#"load module from "file_b.wfl""#;
    let file_b = r#"load module from "file_c.wfl""#;
    let file_c = r#"load module from "file_a.wfl""#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("file_a.wfl", file_a);
    ctx.add_file("file_b.wfl", file_b);
    ctx.add_file("file_c.wfl", file_c);

    let result = ctx.run("file_a.wfl");

    // Check that circular dependency is detected
    assert!(
        result.contains("circular") || result.contains("Circular"),
        "Expected circular dependency error, got: {}",
        result
    );

    // Should mention at least the files involved
    assert!(
        result.contains("file_a") && result.contains("file_b") && result.contains("file_c"),
        "Error should mention all files in the cycle, got: {}",
        result
    );
}

#[test]
fn test_circular_import_self() {
    // File imports itself
    let file_a = r#"
load module from "file_a.wfl"
store value as "A"
"#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("file_a.wfl", file_a);

    let result = ctx.run("file_a.wfl");

    // Check that circular dependency is detected
    assert!(
        result.contains("circular") || result.contains("Circular"),
        "Expected circular dependency error for self-import, got: {}",
        result
    );
}

#[test]
fn test_circular_import_complex() {
    // A → B → C → D → B (circular at B)
    let file_a = r#"load module from "file_b.wfl""#;
    let file_b = r#"load module from "file_c.wfl""#;
    let file_c = r#"load module from "file_d.wfl""#;
    let file_d = r#"load module from "file_b.wfl""#;

    let mut ctx = ImportTestContext::new();
    ctx.add_file("file_a.wfl", file_a);
    ctx.add_file("file_b.wfl", file_b);
    ctx.add_file("file_c.wfl", file_c);
    ctx.add_file("file_d.wfl", file_d);

    let result = ctx.run("file_a.wfl");

    // Check that circular dependency is detected
    assert!(
        result.contains("circular") || result.contains("Circular"),
        "Expected circular dependency error, got: {}",
        result
    );

    // Should mention file_b which is where the cycle occurs
    assert!(
        result.contains("file_b"),
        "Error should mention file_b which is in the cycle, got: {}",
        result
    );
}
