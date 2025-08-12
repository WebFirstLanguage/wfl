use std::process::Command;

/// Test string escaping functions to ensure they properly handle all special characters
#[test]
fn test_string_escaping_with_quotes() {
    let content = r#"This has "quotes" in it"#;
    let escaped = escape_for_wfl_string(content);

    // Should escape double quotes
    assert!(escaped.contains(r#"\""#), "Should escape double quotes");
    // Check that unescaped quotes are replaced (except the literal \" we just added)
    let unescaped_quote_count = content.chars().filter(|&c| c == '"').count();
    let escaped_quote_count = escaped.matches(r#"\""#).count();
    assert_eq!(
        escaped_quote_count, unescaped_quote_count,
        "All quotes should be escaped"
    );
}

#[test]
fn test_string_escaping_with_backslashes() {
    let content = r#"This has \ backslashes"#;
    let escaped = escape_for_wfl_string(content);

    // Should escape backslashes
    assert!(escaped.contains(r"\\"), "Should escape backslashes");
    let backslash_count = content.chars().filter(|&c| c == '\\').count();
    let escaped_backslash_count = escaped.matches(r"\\").count();
    assert_eq!(
        escaped_backslash_count, backslash_count,
        "All backslashes should be escaped"
    );
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

    // Verify proper escaping by checking the structure
    // The WFL code should contain exactly 2 unescaped quotes (the outer ones)
    let quote_positions: Vec<_> = wfl_code
        .char_indices()
        .filter_map(|(i, c)| if c == '"' { Some(i) } else { None })
        .collect();

    // Check that quotes inside the string are escaped
    if quote_positions.len() >= 2 {
        let start_quote = quote_positions[0];
        let end_quote = quote_positions[quote_positions.len() - 1];
        let inner_content = &wfl_code[start_quote + 1..end_quote];

        // The inner content should not contain unescaped quotes
        // but it may contain escaped quotes (\")
        let unescaped_quotes = inner_content
            .chars()
            .enumerate()
            .filter(|(i, c)| {
                *c == '"' && (*i == 0 || inner_content.chars().nth(i - 1) != Some('\\'))
            })
            .count();
        assert_eq!(
            unescaped_quotes, 0,
            "Inner content should not have unescaped quotes"
        );
    }
}

#[test]
#[ignore = "requires release build"]
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
            assert!(
                result.status.success(),
                "WFL program should execute successfully with properly escaped content. Error: {}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        Err(e) => panic!("Failed to run WFL program: {}", e),
    }
}

// Helper function - properly escape strings for embedding in WFL code
fn escape_for_wfl_string(content: &str) -> String {
    content
        .replace('\\', r"\\") // Escape backslashes first
        .replace('"', r#"\""#) // Then escape quotes
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
