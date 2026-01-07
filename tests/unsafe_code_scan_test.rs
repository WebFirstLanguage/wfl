/// Test that scans the entire Rust codebase for `unsafe` blocks and fails if any are found.
/// This enforces memory safety standards across the WFL project.

use std::fs;
use std::path::Path;

#[derive(Debug)]
struct UnsafeViolation {
    file_path: String,
    line_number: usize,
    line_content: String,
    unsafe_type: UnsafeType,
}

#[derive(Debug)]
enum UnsafeType {
    UnsafeBlock,     // unsafe { }
    UnsafeFunction,  // unsafe fn
    UnsafeImpl,      // unsafe impl
    UnsafeTrait,     // unsafe trait
}

impl std::fmt::Display for UnsafeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnsafeType::UnsafeBlock => write!(f, "unsafe block"),
            UnsafeType::UnsafeFunction => write!(f, "unsafe function"),
            UnsafeType::UnsafeImpl => write!(f, "unsafe impl"),
            UnsafeType::UnsafeTrait => write!(f, "unsafe trait"),
        }
    }
}

#[test]
fn test_no_unsafe_code() {
    let mut violations = Vec::new();

    // Scan main src directory
    let src_dir = Path::new("src");
    if src_dir.exists() {
        violations.extend(scan_directory_for_unsafe(src_dir));
    }

    // Scan wfl-lsp workspace member
    let lsp_dir = Path::new("wfl-lsp/src");
    if lsp_dir.exists() {
        violations.extend(scan_directory_for_unsafe(lsp_dir));
    }

    // Filter out whitelisted unsafe blocks (unavoidable FFI usage)
    let non_whitelisted: Vec<_> = violations
        .into_iter()
        .filter(|v| !is_whitelisted_unsafe(&v.file_path, v.line_number))
        .collect();

    if !non_whitelisted.is_empty() {
        let error_message = format_violations(&non_whitelisted);
        panic!(
            "\n\n❌ Found {} unsafe code block(s) in the codebase:\n\n{}\n\n\
            WFL enforces memory safety by prohibiting unsafe code.\n\
            Please refactor to use safe Rust alternatives.\n",
            non_whitelisted.len(),
            error_message
        );
    }
}

/// Checks if an unsafe block is whitelisted (unavoidable for FFI or documented safe usage)
fn is_whitelisted_unsafe(file_path: &str, line_number: usize) -> bool {
    // Normalize path separators for cross-platform compatibility
    let normalized_path = file_path.replace('\\', "/");

    matches!(
        (normalized_path.as_str(), line_number),
        // Unix FD test - OwnedFd::from_raw_fd is unavoidable FFI
        // This is necessary for safe RAII-based file descriptor management
        ("src/repl.rs", 405) |
        // Config tests - env var manipulation protected by TEST_ENV_LOCK mutex
        // These are safe because the mutex serializes all access to environment variables
        ("src/config.rs", 667) |  // set_var in test helper
        ("src/config.rs", 669) |  // remove_var in test helper
        ("src/config.rs", 802)    // set_var in test function
    )
}

fn scan_directory_for_unsafe(dir: &Path) -> Vec<UnsafeViolation> {
    let mut violations = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                // Recursively scan subdirectories
                violations.extend(scan_directory_for_unsafe(&path));
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                // Scan Rust files
                violations.extend(scan_file_for_unsafe(&path));
            }
        }
    }

    violations
}

fn scan_file_for_unsafe(file_path: &Path) -> Vec<UnsafeViolation> {
    let mut violations = Vec::new();

    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => return violations,
    };

    let mut in_multiline_comment = false;

    for (line_idx, line) in content.lines().enumerate() {
        let line_number = line_idx + 1;
        let trimmed = line.trim();

        // Handle multi-line comments /* */
        if let Some(start_pos) = trimmed.find("/*") {
            in_multiline_comment = true;
            // Check if the comment closes on the same line
            if trimmed[start_pos..].find("*/").is_some() {
                in_multiline_comment = false;
            }
            // Still check the part before /* for unsafe
            let before_comment = &trimmed[..start_pos];
            if let Some(violation) = check_line_for_unsafe(before_comment, file_path, line_number, line) {
                violations.push(violation);
            }
            continue;
        }

        if in_multiline_comment {
            if trimmed.contains("*/") {
                in_multiline_comment = false;
            }
            continue;
        }

        // Skip single-line comments
        if trimmed.starts_with("//") {
            continue;
        }

        // Check for unsafe in the line (before any single-line comment)
        let code_part = if let Some(comment_pos) = line.find("//") {
            &line[..comment_pos]
        } else {
            line
        };

        if let Some(violation) = check_line_for_unsafe(code_part, file_path, line_number, line) {
            violations.push(violation);
        }
    }

    violations
}

fn check_line_for_unsafe(
    code: &str,
    file_path: &Path,
    line_number: usize,
    original_line: &str,
) -> Option<UnsafeViolation> {
    if !code.contains("unsafe") {
        return None;
    }

    let trimmed = code.trim();

    // Determine the type of unsafe usage
    let unsafe_type = if trimmed.contains("unsafe fn") || code.contains("unsafe fn") {
        UnsafeType::UnsafeFunction
    } else if trimmed.contains("unsafe impl") || code.contains("unsafe impl") {
        UnsafeType::UnsafeImpl
    } else if trimmed.contains("unsafe trait") || code.contains("unsafe trait") {
        UnsafeType::UnsafeTrait
    } else if trimmed.contains("unsafe {") || code.contains("unsafe {") || trimmed == "unsafe" {
        // "unsafe" on its own line followed by { on next line
        UnsafeType::UnsafeBlock
    } else {
        // Catch-all for any other unsafe usage
        UnsafeType::UnsafeBlock
    };

    Some(UnsafeViolation {
        file_path: file_path.display().to_string(),
        line_number,
        line_content: original_line.trim().to_string(),
        unsafe_type,
    })
}

fn format_violations(violations: &[UnsafeViolation]) -> String {
    let mut output = String::new();

    for (idx, violation) in violations.iter().enumerate() {
        output.push_str(&format!(
            "{}. {}:{} - {}\n   {}\n\n",
            idx + 1,
            violation.file_path,
            violation.line_number,
            violation.unsafe_type,
            violation.line_content
        ));
    }

    output
}
