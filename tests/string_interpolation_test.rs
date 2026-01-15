//! Tests for the string interpolation feature
//!
//! String interpolation allows embedding variables directly in strings using {variable} syntax
//! instead of the verbose "text" with variable with "more text" concatenation.

use std::process::Command;

/// Helper to run WFL code and capture output
fn run_wfl_code(code: &str) -> String {
    use std::io::Write;
    let dir = tempfile::tempdir().expect("Failed to create temp directory");
    let file_path = dir.path().join("test.wfl");
    let mut file = std::fs::File::create(&file_path).expect("Failed to create temp file");
    file.write_all(code.as_bytes())
        .expect("Failed to write to temp file");

    let output = Command::new("cargo")
        .args(["run", "--", file_path.to_str().unwrap()])
        .output()
        .expect("Failed to execute command");

    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn test_basic_interpolation() {
    let code = r#"
store name as "World"
display "Hello, {name}!"
"#;
    let output = run_wfl_code(code);
    assert!(
        output.contains("Hello, World!"),
        "Expected 'Hello, World!' in output, got: {}",
        output
    );
}

#[test]
fn test_multiple_interpolations() {
    let code = r#"
store first as "Alice"
store last as "Smith"
display "Name: {first} {last}"
"#;
    let output = run_wfl_code(code);
    assert!(
        output.contains("Name: Alice Smith"),
        "Expected 'Name: Alice Smith' in output, got: {}",
        output
    );
}

#[test]
fn test_number_interpolation() {
    let code = r#"
store count as 42
store price as 19.99
display "Count: {count}, Price: {price}"
"#;
    let output = run_wfl_code(code);
    assert!(
        output.contains("Count: 42, Price: 19.99"),
        "Expected 'Count: 42, Price: 19.99' in output, got: {}",
        output
    );
}

#[test]
fn test_boolean_interpolation() {
    let code = r#"
store is_active as yes
display "Active: {is_active}"
"#;
    let output = run_wfl_code(code);
    assert!(
        output.contains("Active: yes"),
        "Expected 'Active: yes' in output, got: {}",
        output
    );
}

#[test]
fn test_escaped_braces() {
    let code = r#"
display "Literal braces: {{test}}"
display "Just closing: }}"
"#;
    let output = run_wfl_code(code);
    assert!(
        output.contains("Literal braces: {test}"),
        "Expected 'Literal braces: {{test}}' in output, got: {}",
        output
    );
    assert!(
        output.contains("Just closing: }"),
        "Expected 'Just closing: }}' in output, got: {}",
        output
    );
}

#[test]
fn test_interpolation_with_whitespace() {
    let code = r#"
store value as "test"
display "With spaces: { value }"
"#;
    let output = run_wfl_code(code);
    assert!(
        output.contains("With spaces: test"),
        "Expected 'With spaces: test' in output, got: {}",
        output
    );
}

#[test]
fn test_empty_string_interpolation() {
    let code = r#"
store empty_val as ""
display "Empty: '{empty_val}'"
"#;
    let output = run_wfl_code(code);
    assert!(
        output.contains("Empty: ''"),
        "Expected 'Empty: ''' in output, got: {}",
        output
    );
}

#[test]
fn test_mixed_static_and_interpolated() {
    let code = r#"
store a as "start"
store b as "end"
display "{a} middle {b}"
"#;
    let output = run_wfl_code(code);
    assert!(
        output.contains("start middle end"),
        "Expected 'start middle end' in output, got: {}",
        output
    );
}

#[test]
fn test_plain_string_not_affected() {
    let code = r#"
display "No interpolation here"
"#;
    let output = run_wfl_code(code);
    assert!(
        output.contains("No interpolation here"),
        "Expected 'No interpolation here' in output, got: {}",
        output
    );
}

#[test]
fn test_empty_braces_literal() {
    let code = r#"
display "Empty braces: {}"
"#;
    let output = run_wfl_code(code);
    assert!(
        output.contains("Empty braces: {}"),
        "Expected 'Empty braces: {{}}' in output, got: {}",
        output
    );
}
