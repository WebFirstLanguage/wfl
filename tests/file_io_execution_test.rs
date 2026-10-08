use std::fs;
use std::path::Path;
use wfl::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

// Integration tests that actually execute file I/O operations using the interpreter
#[cfg(test)]
mod file_io_execution_tests {
    use super::*;

    /// Use forward slashes so Windows fixture paths do not become WFL string escapes.
    fn wfl_path(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }

    fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
        payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_string()))
            .expect("panic payload should be a string")
    }

    /// True when the operand does not resolve against the process cwd on this host.
    fn is_absolute_wfl_path(path: &str) -> bool {
        Path::new(path).is_absolute()
    }

    /// Relative operands of `at "..."` write to the process cwd unless rewritten.
    fn relative_at_paths(code: &str) -> Vec<String> {
        let mut names = Vec::new();
        let mut remaining = code;
        while let Some(start) = remaining.find(" at \"") {
            remaining = &remaining[start + 5..];
            let Some(end) = remaining.find('"') else {
                break;
            };
            let name = &remaining[..end];
            if !name.is_empty() && !is_absolute_wfl_path(name) {
                names.push(name.to_string());
            }
            remaining = &remaining[end + 1..];
        }
        names
    }

    fn rewrite_fixture_paths(code: &str, directory: &Path, filenames: &[&str]) -> String {
        // A listed name that is not in the program is a stale rewrite set.
        // An `at "name"` operand that is not listed stays relative and leaks
        // into the repo root — that is the original hygiene defect.
        for filename in filenames {
            assert!(
                code.contains(&format!("\"{filename}\"")),
                "listed fixture {filename:?} does not appear in the WFL source; \
                 the rewrite set is stale"
            );
        }
        for name in relative_at_paths(code) {
            assert!(
                filenames.contains(&name.as_str()),
                "WFL source uses relative path {name:?} that is not in the rewrite set; \
                 it would be created in the repo root"
            );
        }

        let directory_text = wfl_path(directory);
        let mut rewritten = code.replace(
            "list files in \".\"",
            &format!("list files in \"{directory_text}\""),
        );
        for filename in filenames {
            let path = wfl_path(&directory.join(filename));
            rewritten = rewritten.replace(&format!("\"{filename}\""), &format!("\"{path}\""));
        }
        rewritten
    }

    async fn execute_wfl_code(
        code: &str,
        directory: &Path,
        filenames: &[&str],
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Every test owns its files, including on assertion failure. Absolute
        // paths keep concurrent tests independent without changing process cwd.
        let code = rewrite_fixture_paths(code, directory, filenames);
        let tokens = lex_wfl_with_positions(&code);
        let mut parser = Parser::new(&tokens);
        let ast = parser.parse().expect("Failed to parse WFL code");

        let mut interpreter = Interpreter::new();

        // Execute the program
        let result = interpreter.interpret(&ast).await;
        match result {
            Ok(_) => Ok("Program executed successfully".to_string()),
            Err(errors) => {
                let error_msg = errors
                    .iter()
                    .map(|e| format!("{}", e))
                    .collect::<Vec<_>>()
                    .join(", ");
                Err(Box::new(std::io::Error::other(error_msg)))
            }
        }
    }

    #[tokio::test]
    async fn test_basic_file_write_read_execution() {
        let directory = tempfile::tempdir().expect("create isolated file fixture");
        let test_files = ["test_exec_basic.txt"];

        let code = r#"
            open file at "test_exec_basic.txt" for writing as test_file
            wait for write content "Hello from execution test!" into test_file
            close file test_file
            
            open file at "test_exec_basic.txt" for reading as read_file
            wait for store file_data as read content from read_file
            close file read_file
            
            display file_data
        "#;

        // This test should fail initially because we need to verify the interpreter
        // actually creates files and reads content correctly
        let result = execute_wfl_code(code, directory.path(), &test_files).await;
        assert!(
            result.is_ok(),
            "File I/O execution failed: {:?}",
            result.err()
        );

        // Verify the file was actually created
        assert!(
            directory.path().join("test_exec_basic.txt").exists(),
            "Test file was not created by interpreter"
        );

        // Verify file contents
        let file_contents = fs::read_to_string(directory.path().join("test_exec_basic.txt"))
            .expect("Could not read test file");
        assert_eq!(
            file_contents.trim(),
            "Hello from execution test!",
            "File contents don't match expected value"
        );
    }

    #[tokio::test]
    async fn test_file_append_execution() {
        let directory = tempfile::tempdir().expect("create isolated file fixture");
        let test_files = ["test_exec_append.txt"];

        let code = r#"
            open file at "test_exec_append.txt" for writing as initial_file
            wait for write content "Line 1" into initial_file
            close file initial_file
            
            open file at "test_exec_append.txt" for append as append_file
            wait for append content "\\nLine 2" into append_file
            close file append_file
            
            open file at "test_exec_append.txt" for reading as read_file
            wait for store final_content as read content from read_file
            close file read_file
            
            display final_content
        "#;

        let result = execute_wfl_code(code, directory.path(), &test_files).await;
        assert!(
            result.is_ok(),
            "File append execution failed: {:?}",
            result.err()
        );

        let file_contents = fs::read_to_string(directory.path().join("test_exec_append.txt"))
            .expect("Could not read append test file");
        assert_eq!(
            file_contents.trim(),
            "Line 1\\nLine 2",
            "Appended file contents don't match expected value"
        );
    }

    #[tokio::test]
    async fn test_file_exists_execution() {
        let directory = tempfile::tempdir().expect("create isolated file fixture");
        let test_files = ["test_exec_exists.txt", "nonexistent_file.txt"];

        // Create a test file first
        fs::write(
            directory.path().join("test_exec_exists.txt"),
            "test content",
        )
        .expect("Failed to create test file");

        let code = r#"
            store exists_result as file exists at "test_exec_exists.txt"
            check if exists_result:
                display "File exists check passed"
            end check
            
            store missing_result as file exists at "nonexistent_file.txt"
            check if not missing_result:
                display "Missing file check passed"
            end check
        "#;

        let result = execute_wfl_code(code, directory.path(), &test_files).await;
        assert!(
            result.is_ok(),
            "File exists execution failed: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_directory_listing_execution() {
        let directory = tempfile::tempdir().expect("create isolated file fixture");
        let test_files = ["test_dir_1.txt", "test_dir_2.log", "test_dir_3.txt"];

        // Create test files
        for file in &test_files {
            fs::write(directory.path().join(file), "test content")
                .expect("Failed to create test file");
        }

        let code = r#"
            wait for store all_files as list files in "."
            wait for store txt_files as list files in "." with pattern "*.txt"
            
            display "Total files found: " with length of all_files
            display "TXT files found: " with length of txt_files
        "#;

        // The WFL source lists `"."`, not the fixture names. Those files are
        // created in the temp dir in Rust; passing them here would be a stale
        // rewrite set.
        let result = execute_wfl_code(code, directory.path(), &[]).await;
        assert!(
            result.is_ok(),
            "Directory listing execution failed: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_file_deletion_execution() {
        let directory = tempfile::tempdir().expect("create isolated file fixture");
        let test_files = ["test_delete_me.txt"];

        // Create test file first
        fs::write(
            directory.path().join("test_delete_me.txt"),
            "This file should be deleted",
        )
        .expect("Failed to create test file");
        assert!(
            directory.path().join("test_delete_me.txt").exists(),
            "Test file was not created for deletion test"
        );

        let code = r#"
            delete file at "test_delete_me.txt"
            store still_exists as file exists at "test_delete_me.txt"
            check if not still_exists:
                display "File successfully deleted"
            end check
        "#;

        let result = execute_wfl_code(code, directory.path(), &test_files).await;
        assert!(
            result.is_ok(),
            "File deletion execution failed: {:?}",
            result.err()
        );

        // Verify file was actually deleted
        assert!(
            !directory.path().join("test_delete_me.txt").exists(),
            "Test file was not properly deleted by interpreter"
        );
    }

    #[tokio::test]
    async fn test_multiple_files_execution() {
        let directory = tempfile::tempdir().expect("create isolated file fixture");
        let test_files = ["multi_test_1.txt", "multi_test_2.log", "multi_test_3.dat"];

        let code = r#"
            // Create multiple files with different content
            open file at "multi_test_1.txt" for writing as file1
            wait for write content "Content for file 1" into file1
            close file file1
            
            open file at "multi_test_2.log" for writing as file2
            wait for write content "Log data for file 2" into file2
            close file file2
            
            open file at "multi_test_3.dat" for writing as file3
            wait for write content "Binary-like data for file 3" into file3
            close file file3
            
            // Verify all files were created
            store file1_exists as file exists at "multi_test_1.txt"
            store file2_exists as file exists at "multi_test_2.log"
            store file3_exists as file exists at "multi_test_3.dat"
            
            check if file1_exists and file2_exists and file3_exists:
                display "All multiple files created successfully"
            end check
        "#;

        let result = execute_wfl_code(code, directory.path(), &test_files).await;
        assert!(
            result.is_ok(),
            "Multiple files execution failed: {:?}",
            result.err()
        );

        // Verify all files exist with correct content
        for (file, expected_content) in [
            ("multi_test_1.txt", "Content for file 1"),
            ("multi_test_2.log", "Log data for file 2"),
            ("multi_test_3.dat", "Binary-like data for file 3"),
        ] {
            assert!(
                directory.path().join(file).exists(),
                "File {} was not created",
                file
            );
            let content = fs::read_to_string(directory.path().join(file))
                .unwrap_or_else(|_| panic!("Could not read {}", file));
            assert_eq!(
                content.trim(),
                expected_content,
                "Content mismatch in {}",
                file
            );
        }
    }

    #[test]
    fn rewrite_rejects_a_listed_filename_missing_from_source() {
        let directory = tempfile::tempdir().expect("create isolated file fixture");
        let dir = directory.path().to_path_buf();
        let panic = std::panic::catch_unwind(move || {
            rewrite_fixture_paths(
                r#"open file at "present.txt" for writing as fixture"#,
                &dir,
                &["present.txt", "missing_from_source.txt"],
            );
        })
        .expect_err("a stale rewrite set must panic");
        let message = panic_message(panic);
        assert!(
            message.contains("missing_from_source.txt"),
            "diagnostic must name the stale fixture: {message}"
        );
        assert!(
            message.contains("does not appear in the WFL source"),
            "diagnostic must say the rewrite set is stale: {message}"
        );
    }

    #[test]
    fn rewrite_rejects_an_unlisted_nested_relative_path() {
        let directory = tempfile::tempdir().expect("create isolated file fixture");
        let dir = directory.path().to_path_buf();
        let panic = std::panic::catch_unwind(move || {
            rewrite_fixture_paths(
                r#"open file at "subdir/output.txt" for writing as fixture"#,
                &dir,
                &[],
            );
        })
        .expect_err("a nested relative path must panic before WFL runs");
        let message = panic_message(panic);
        assert!(
            message.contains("subdir/output.txt"),
            "diagnostic must name the nested relative path: {message}"
        );
        assert!(
            message.contains("repo root"),
            "diagnostic must describe the leak: {message}"
        );
    }

    fn assert_unlisted_relative_is_rejected(relative: &str) {
        let directory = tempfile::tempdir().expect("create isolated file fixture");
        let dir = directory.path().to_path_buf();
        let code = format!(r#"open file at "{relative}" for writing as fixture"#);
        let result = std::panic::catch_unwind(move || {
            rewrite_fixture_paths(&code, &dir, &[]);
        });
        let panic = match result {
            Err(payload) => payload,
            Ok(_) => panic!("unlisted relative path {relative:?} must panic before WFL runs"),
        };
        let message = panic_message(panic);
        let named = format!("{relative:?}");
        assert!(
            message.contains(&named),
            "diagnostic must name the unlisted path {named}: {message}"
        );
        assert!(
            message.contains("repo root"),
            "diagnostic must describe the leak: {message}"
        );
    }

    #[test]
    fn rewrite_rejects_an_unlisted_relative_path() {
        assert_unlisted_relative_is_rejected("leaked.txt");
    }

    #[test]
    fn rewrite_rejects_drive_relative_and_dot_paths() {
        for relative in ["C:foo", "./x", "../x", r"sub\\x"] {
            assert_unlisted_relative_is_rejected(relative);
        }
    }

    #[test]
    fn rewrite_allows_a_host_absolute_temp_path_without_listing_it() {
        let directory = tempfile::tempdir().expect("create isolated file fixture");
        let absolute = wfl_path(&directory.path().join("already_absolute.txt"));
        let code = format!(r#"open file at "{absolute}" for writing as fixture"#);
        let rewritten = rewrite_fixture_paths(&code, directory.path(), &[]);
        assert!(
            rewritten.contains(&format!("\"{absolute}\"")),
            "host-absolute temp path must stay unlisted: {rewritten}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn rewrite_allows_unlisted_windows_absolute_paths() {
        let directory = tempfile::tempdir().expect("create isolated file fixture");
        let code = r#"
            open file at "D:\\fixtures\\output.txt" for writing as drive_backslashes
            open file at "C:/fixtures/output.txt" for writing as drive_slashes
        "#;
        let rewritten = rewrite_fixture_paths(code, directory.path(), &[]);
        assert!(
            rewritten.contains(r#""D:\\fixtures\\output.txt""#),
            "escaped Windows drive path must stay unlisted: {rewritten}"
        );
        assert!(
            rewritten.contains(r#""C:/fixtures/output.txt""#),
            "forward-slash Windows drive path must stay unlisted: {rewritten}"
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn rewrite_rejects_unlisted_windows_style_paths() {
        for relative in [
            r"D:\\fixtures\\output.txt",
            "C:/fixtures/output.txt",
            r"\\leak.txt",
        ] {
            assert_unlisted_relative_is_rejected(relative);
        }
    }

    #[test]
    fn rewrite_splices_listed_relative_paths() {
        let directory = tempfile::tempdir().expect("create isolated file fixture");
        let rewritten = rewrite_fixture_paths(
            r#"open file at "kept.txt" for writing as fixture"#,
            directory.path(),
            &["kept.txt"],
        );
        let spliced = wfl_path(&directory.path().join("kept.txt"));
        assert!(
            !rewritten.contains("\"kept.txt\""),
            "bare relative name must be rewritten: {rewritten}"
        );
        assert!(
            rewritten.contains(&format!("\"{spliced}\"")),
            "rewritten source must use the temp-dir path: {rewritten}"
        );
    }
}
