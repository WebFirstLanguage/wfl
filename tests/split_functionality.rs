use std::process::Command;
use std::fs;

fn run_wfl(code: &str) -> String {
    // Write temporary WFL file
    let temp_file = "temp_test_split.wfl";
    fs::write(temp_file, code).expect("Failed to write temp file");
    
    // Run the WFL interpreter
    let output = Command::new("./target/release/wfl.exe")
        .arg(temp_file)
        .output()
        .expect("Failed to execute WFL");
    
    // Clean up
    fs::remove_file(temp_file).ok();
    
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn test_string_split_basic() {
    let result = run_wfl(r#"
        store text as "a,b,c"
        store parts as split text by ","
        display length of parts
    "#);
    assert_eq!(result.trim(), "3");
}

#[test]
fn test_string_split_space() {
    let result = run_wfl(r#"
        store text as "hello world test"
        store parts as split text by " "
        display parts[0]
        display parts[1]
        display parts[2]
    "#);
    let lines: Vec<&str> = result.trim().lines().collect();
    assert_eq!(lines[0], "hello");
    assert_eq!(lines[1], "world");
    assert_eq!(lines[2], "test");
}

#[test]
fn test_string_split_empty() {
    let result = run_wfl(r#"
        store text as ""
        store parts as split text by ","
        display length of parts
    "#);
    assert_eq!(result.trim(), "1");
}

#[test]
fn test_string_split_no_delimiter() {
    let result = run_wfl(r#"
        store text as "hello"
        store parts as split text by ","
        display length of parts
        display parts[0]
    "#);
    let lines: Vec<&str> = result.trim().lines().collect();
    assert_eq!(lines[0], "1");
    assert_eq!(lines[1], "hello");
}

#[test]
fn test_string_split_adjacent_delimiters() {
    let result = run_wfl(r#"
        store text as "a,,b"
        store parts as split text by ","
        display length of parts
    "#);
    assert_eq!(result.trim(), "3");
}

#[test]
fn test_pattern_split_registered() {
    let result = run_wfl(r#"
        create pattern comma: ","
        store parts as split "x,y,z" on pattern comma
        display length of parts
    "#);
    assert_eq!(result.trim(), "3");
}

#[test]
fn test_pattern_split_basic() {
    let result = run_wfl(r#"
        store text as "a,b,c"
        create pattern comma: ","
        store parts as split text on pattern comma
        display parts[0]
        display parts[1]
        display parts[2]
    "#);
    let lines: Vec<&str> = result.trim().lines().collect();
    assert_eq!(lines[0], "a");
    assert_eq!(lines[1], "b");
    assert_eq!(lines[2], "c");
}

#[test]
fn test_pattern_split_whitespace() {
    let result = run_wfl(r#"
        store text as "hello   world    test"
        create pattern spaces: one or more space
        store parts as split text on pattern spaces
        display length of parts
    "#);
    assert_eq!(result.trim(), "3");
}

#[test]
fn test_split_returns_list() {
    let result = run_wfl(r#"
        store text as "a,b,c"
        store parts as split text by ","
        count from 0 to length of parts - 1:
            display parts[counter]
        end count
    "#);
    let lines: Vec<&str> = result.trim().lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "a");
    assert_eq!(lines[1], "b");
    assert_eq!(lines[2], "c");
}

#[test]
fn test_split_type_error_non_text() {
    let result = run_wfl(r#"
        store num as 123
        store parts as split num by ","
    "#);
    assert!(result.contains("error") || result.contains("Error"));
}

#[test]
fn test_split_type_error_non_text_delimiter() {
    let result = run_wfl(r#"
        store text as "a,b,c"
        store parts as split text by 123
    "#);
    assert!(result.contains("error") || result.contains("Error"));
}