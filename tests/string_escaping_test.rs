use std::process::Command;

/// Test string escaping functions to ensure they properly handle all special characters
#[test]
fn test_string_escaping_with_quotes() {
    let content = r#"This has "quotes" in it"#;
    let escaped = escape_for_wfl_string(content);
    
    // Should escape both backslashes and quotes
    assert!(escaped.contains(r#"\""#), "Should escape double quotes");
    assert!(!escaped.contains(r#"""#), "Should not contain unescaped quotes");
}

#[test]
fn test_string_escaping_with_backslashes() {
    let content = r#"This has \ backslashes"#;
    let escaped = escape_for_wfl_string(content);
    
    // Should escape backslashes first
    assert!(escaped.contains(r"\\"), "Should escape backslashes");
    assert!(!escaped.contains(r"\"), "Should not contain single backslashes");
}

#[test]
fn test_string_escaping_with_both_quotes_and_backslashes() {
    let content = r#"This has both "quotes" and \ backslashes"#;
    let escaped = escape_for_wfl_string(content);
    
    // Should handle both correctly - backslashes must be escaped first
    assert!(escaped.contains(r#"\\"#), "Should escape backslashes");
    assert!(escaped.contains(r#"\""#), "Should escape quotes");
    
    // The result should be valid when used in WFL string
    let wfl_code = format!(r#"store content as "{}""#, escaped);
    
    // Should not have syntax errors when parsed
    assert!(!wfl_code.contains(r#"""#), "Final WFL should not have unescaped quotes");
}

#[test]
fn test_file_io_performance_with_large_content() {
    // This test should pass with proper escaping
    let large_content = generate_test_content_with_special_chars(5700);
    let escaped = escape_for_wfl_string(&large_content);
    
    // Create WFL program that writes and reads the content
    let wfl_program = format!(
        r#"
store test_content as "{}"
write test_content to "test_output.txt"
store read_content as read from "test_output.txt"
display "Content length: " with length of read_content
"#,
        escaped
    );
    
    // Write the WFL program to a temporary file
    std::fs::write("temp_performance_test.wfl", wfl_program).unwrap();
    
    // Run the WFL program and check it executes without error
    let output = Command::new("./target/release/wfl")
        .arg("temp_performance_test.wfl")
        .output();
    
    // Clean up
    let _ = std::fs::remove_file("temp_performance_test.wfl");
    let _ = std::fs::remove_file("test_output.txt");
    
    // This will fail until we implement proper escaping
    match output {
        Ok(result) => {
            assert!(result.status.success(), 
                "WFL program should execute successfully with properly escaped content. Error: {}", 
                String::from_utf8_lossy(&result.stderr));
        }
        Err(e) => panic!("Failed to run WFL program: {}", e),
    }
}

// Helper functions that should be implemented
fn escape_for_wfl_string(content: &str) -> String {
    // This function doesn't exist yet - test should fail
    unimplemented!("String escaping function not yet implemented")
}

fn generate_test_content_with_special_chars(size: usize) -> String {
    // Generate content that includes quotes and backslashes to test escaping
    let mut content = String::new();
    let pattern = r#"Sample text with "quotes" and \ backslashes. "#;
    
    while content.len() < size {
        content.push_str(pattern);
    }
    
    content.truncate(size);
    content
}