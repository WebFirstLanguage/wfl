use std::fs;
use std::process::Command;
use std::time::Instant;

/// Performance tests for file I/O operations with proper string escaping
#[test]
#[ignore = "requires release build"]
fn test_file_io_performance_with_large_content() {
    // Generate ~5.7KB of content with special characters that need escaping
    let large_content = generate_large_content_with_special_chars(5700);

    // Use proper string escaping
    let escaped_content = escape_for_wfl_string(&large_content);

    // Create WFL program that writes and reads the content
    let wfl_program = format!(
        r#"
store test_content as "{}"
write test_content to "large_test_output.txt"
store read_content as read from "large_test_output.txt"
display "Successfully processed " with length of read_content with " characters"
"#,
        escaped_content
    );

    // Write the WFL program to a temporary file
    fs::write("temp_large_performance_test.wfl", &wfl_program)
        .expect("Failed to write test program");

    let start_time = Instant::now();

    // Run the WFL program
    let output = Command::new("./target/release/wfl")
        .arg("temp_large_performance_test.wfl")
        .output()
        .expect("Failed to run WFL program");

    let execution_time = start_time.elapsed();

    // Clean up temporary files
    let _ = fs::remove_file("temp_large_performance_test.wfl");
    let _ = fs::remove_file("large_test_output.txt");

    // Verify the program executed successfully
    if !output.status.success() {
        panic!(
            "WFL program failed to execute. Error: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Basic performance check - should complete within reasonable time
    assert!(
        execution_time.as_millis() < 5000,
        "File I/O performance test took too long: {}ms",
        execution_time.as_millis()
    );

    println!(
        "File I/O performance test completed in {}ms",
        execution_time.as_millis()
    );
}

#[test]
#[ignore = "requires release build"]
fn test_file_io_with_escaped_quotes_and_backslashes() {
    // Content specifically designed to test the escaping issue from the GitHub issue
    let problematic_content =
        r#"This content has "quoted text" and \ backslashes and even \"escaped quotes\""#;

    let escaped_content = escape_for_wfl_string(problematic_content);

    let wfl_program = format!(
        r#"
store problematic_content as "{}"
write problematic_content to "escaped_test_output.txt"
store read_content as read from "escaped_test_output.txt"
display "Content matches: " with (problematic_content is equal to read_content)
"#,
        escaped_content
    );

    fs::write("temp_escaped_test.wfl", &wfl_program).expect("Failed to write test program");

    let output = Command::new("./target/release/wfl")
        .arg("temp_escaped_test.wfl")
        .output()
        .expect("Failed to run WFL program");

    // Clean up
    let _ = fs::remove_file("temp_escaped_test.wfl");
    let _ = fs::remove_file("escaped_test_output.txt");

    if !output.status.success() {
        panic!(
            "WFL program with escaped content failed. Error: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Check that the output indicates content matches
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        output_str.contains("Content matches: true"),
        "Content did not match after round-trip. Output: {}",
        output_str
    );
}

/// Properly escape strings for embedding in WFL code
///
/// This addresses the issue described in GitHub issue #154 by:
/// 1. Escaping backslashes first (to avoid double-escaping)
/// 2. Then escaping double quotes
///
/// This is more robust than the original approach of only escaping quotes.
fn escape_for_wfl_string(content: &str) -> String {
    content
        .replace('\\', r"\\") // Escape backslashes first
        .replace('"', r#"\""#) // Then escape quotes
}

/// Generate large content with special characters for performance testing
///
/// This generates content that includes both quotes and backslashes to ensure
/// our escaping function is properly tested under realistic conditions.
fn generate_large_content_with_special_chars(target_size: usize) -> String {
    let mut content = String::new();

    // Pattern that includes various special characters
    let patterns = [
        r#"Regular text with no special chars. "#,
        r#"Text with "quoted strings" inside. "#,
        r#"Text with \ backslashes present. "#,
        r#"Text with both "quotes" and \ backslashes. "#,
        r#"Text with \"escaped quotes\" already. "#,
        r#"Text with \\escaped backslashes\\ already. "#,
    ];

    let mut pattern_index = 0;
    while content.len() < target_size {
        content.push_str(patterns[pattern_index % patterns.len()]);
        pattern_index += 1;
    }

    content.truncate(target_size);
    content
}

#[cfg(test)]
mod escaping_unit_tests {
    use super::*;

    #[test]
    fn test_escape_quotes_only() {
        let input = r#"Simple "quoted" text"#;
        let result = escape_for_wfl_string(input);
        assert_eq!(result, r#"Simple \"quoted\" text"#);
    }

    #[test]
    fn test_escape_backslashes_only() {
        let input = r"Simple \ backslash text";
        let result = escape_for_wfl_string(input);
        assert_eq!(result, r"Simple \\ backslash text");
    }

    #[test]
    fn test_escape_both_quotes_and_backslashes() {
        let input = r#"Text with "quotes" and \ backslashes"#;
        let result = escape_for_wfl_string(input);
        // Should escape backslashes first, then quotes
        assert_eq!(result, r#"Text with \"quotes\" and \\ backslashes"#);
    }

    #[test]
    fn test_escape_already_escaped_content() {
        let input = r#"Already \"escaped\" and \\ doubled"#;
        let result = escape_for_wfl_string(input);
        // Should properly handle already-escaped content
        assert_eq!(result, r#"Already \\\"escaped\\\" and \\\\ doubled"#);
    }

    #[test]
    fn test_empty_string() {
        let result = escape_for_wfl_string("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_no_special_chars() {
        let input = "Just plain text here";
        let result = escape_for_wfl_string(input);
        assert_eq!(result, input);
    }
}
