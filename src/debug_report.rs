use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Write};
use std::fs::{self, File};
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

pub struct SafeDebug<'a> {
    value: &'a Value,
    depth: usize,
    seen: Rc<RefCell<HashSet<*const ()>>>,
}

impl<'a> SafeDebug<'a> {
    pub fn new(value: &'a Value, depth: usize) -> Self {
        Self {
            value,
            depth,
            seen: Rc::new(RefCell::new(HashSet::new())),
        }
    }
}

impl fmt::Debug for SafeDebug<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.depth == 0 {
            return write!(f, "...");
        }

        match self.value {
            Value::List(list) => {
                let ptr = Rc::as_ptr(list) as *const ();

                if !self.seen.borrow_mut().insert(ptr) {
                    return write!(f, "[<cycle>]");
                }

                let borrowed = list.borrow();
                write!(f, "[")?;

                for (i, v) in borrowed.iter().take(16).enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    SafeDebug {
                        value: v,
                        depth: self.depth - 1,
                        seen: Rc::clone(&self.seen),
                    }
                    .fmt(f)?;
                }

                if borrowed.len() > 16 {
                    write!(f, ", ... ({} more items)", borrowed.len() - 16)?;
                }

                self.seen.borrow_mut().remove(&ptr);
                write!(f, "]")
            }
            Value::Object(obj) => {
                let ptr = Rc::as_ptr(obj) as *const ();

                if !self.seen.borrow_mut().insert(ptr) {
                    return write!(f, "{{<cycle>}}");
                }

                let borrowed = obj.borrow();
                write!(f, "{{")?;

                for (i, (k, v)) in borrowed.iter().take(16).enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: ")?;
                    SafeDebug {
                        value: v,
                        depth: self.depth - 1,
                        seen: Rc::clone(&self.seen),
                    }
                    .fmt(f)?;
                }

                if borrowed.len() > 16 {
                    write!(f, ", ... ({} more items)", borrowed.len() - 16)?;
                }

                self.seen.borrow_mut().remove(&ptr);
                write!(f, "}}")
            }
            v => fmt::Debug::fmt(v, f),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub func_name: String,
    pub call_line: usize,
    pub call_col: usize,
    pub locals: Option<HashMap<String, Value>>,
}

impl CallFrame {
    pub fn new(func_name: String, call_line: usize, call_col: usize) -> Self {
        Self {
            func_name,
            call_line,
            call_col,
            locals: None,
        }
    }

    pub fn capture_locals(&mut self, env: &Rc<RefCell<Environment>>) {
        self.locals = Some(env.borrow().values.clone());
    }
}

pub fn create_report(
    error: &RuntimeError,
    call_stack: &[CallFrame],
    source: &str,
    script_path: &str,
) -> Result<PathBuf, std::io::Error> {
    let debug_file_path = generate_debug_filename(script_path);

    let report = generate_report_content(error, call_stack, source, script_path);

    write_report_to_file(&debug_file_path, &report)?;

    Ok(debug_file_path)
}

fn generate_debug_filename(script_path: &str) -> PathBuf {
    let path = Path::new(script_path);
    let stem = path.file_stem().unwrap_or_default();
    let parent = path.parent().unwrap_or_else(|| Path::new(""));

    parent.join(format!("{}_debug.txt", stem.to_string_lossy()))
}

fn generate_report_content(
    error: &RuntimeError,
    call_stack: &[CallFrame],
    source: &str,
    script_path: &str,
) -> String {
    let mut report = String::new();

    writeln!(&mut report, "=== WFL Debug Report ===").unwrap();
    writeln!(&mut report, "Script: {script_path}").unwrap();
    writeln!(
        &mut report,
        "Time: {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    )
    .unwrap();
    writeln!(&mut report, "\n=== Error Summary ===").unwrap();
    writeln!(
        &mut report,
        "Runtime error at line {}, column {}: {}",
        error.line, error.column, error.message
    )
    .unwrap();

    writeln!(&mut report, "\n=== Stack Trace ===").unwrap();
    if call_stack.is_empty() {
        writeln!(
            &mut report,
            "In main script at line {}, column {}",
            error.line, error.column
        )
        .unwrap();
    } else {
        for (i, frame) in call_stack.iter().enumerate().rev() {
            if i == call_stack.len() - 1 {
                writeln!(
                    &mut report,
                    "At action \"{}\" (line {}, column {})",
                    frame.func_name, error.line, error.column
                )
                .unwrap();
            } else {
                writeln!(
                    &mut report,
                    "Called from action \"{}\" (line {}, column {})",
                    frame.func_name, frame.call_line, frame.call_col
                )
                .unwrap();
            }
        }
    }

    writeln!(&mut report, "\n=== Source Code ===").unwrap();
    add_source_snippet(&mut report, source, error.line);

    if !call_stack.is_empty() {
        let frame = &call_stack[call_stack.len() - 1];
        writeln!(&mut report, "\n=== Action Body ===").unwrap();

        extract_function_body(&mut report, source, &frame.func_name);
    }

    writeln!(&mut report, "\n=== Local Variables ===").unwrap();
    if let Some(frame) = call_stack.last() {
        if let Some(locals) = &frame.locals {
            for (name, value) in locals {
                writeln!(&mut report, "{} = {:?}", name, SafeDebug::new(value, 4)).unwrap();
            }
        } else {
            writeln!(&mut report, "(No local variables captured)").unwrap();
        }
    } else {
        writeln!(&mut report, "(No local variables in global scope)").unwrap();
    }

    report
}

fn add_source_snippet(report: &mut String, source: &str, error_line: usize) {
    let lines: Vec<&str> = source.lines().collect();
    let err_line_index = error_line.saturating_sub(1); // 0-based index

    let start_line = err_line_index.saturating_sub(2);
    let end_line = std::cmp::min(err_line_index + 2, lines.len().saturating_sub(1));

    lines
        .iter()
        .enumerate()
        .skip(start_line)
        .take(end_line - start_line + 1)
        .for_each(|(i, line)| {
            let line_marker = if i == err_line_index { ">> " } else { "   " };
            writeln!(report, "{}{}: {}", line_marker, i + 1, line).unwrap();
        });
}

fn extract_function_body(report: &mut String, source: &str, func_name: &str) {
    let lines: Vec<&str> = source.lines().collect();

    let mut start_line = None;
    let mut end_line = None;

    for (i, line) in lines.iter().enumerate() {
        if line.contains(&format!("define action called {func_name}"))
            || line.contains(&format!("action called {func_name}"))
        {
            start_line = Some(i);
        } else if start_line.is_some() && line.contains("end action") {
            end_line = Some(i);
            break;
        }
    }

    if let (Some(start), Some(end)) = (start_line, end_line) {
        lines
            .iter()
            .enumerate()
            .skip(start)
            .take(end - start + 1)
            .for_each(|(i, line)| {
                writeln!(report, "{}: {}", i + 1, line).unwrap();
            });
    } else {
        writeln!(report, "(Could not locate action body)").unwrap();
    }
}

fn write_report_to_file(file_path: &Path, content: &str) -> Result<(), std::io::Error> {
    let mut file = File::create(file_path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

/// Clean up old debug files in the current directory
/// This function removes debug files older than the specified age
pub fn cleanup_old_debug_files(max_age: Duration) -> Result<usize, std::io::Error> {
    let current_dir = std::env::current_dir()?;
    cleanup_debug_files_in_dir(&current_dir, max_age)
}

/// Clean up debug files in a specific directory
/// Returns the number of files cleaned up
pub fn cleanup_debug_files_in_dir(dir: &Path, max_age: Duration) -> Result<usize, std::io::Error> {
    let mut cleaned_count = 0;

    if !dir.is_dir() {
        return Ok(0);
    }

    let entries = fs::read_dir(dir)?;
    let now = SystemTime::now();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Skip directories
        if !path.is_file() {
            continue;
        }

        // Check if it's a debug file or temporary test file
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            let is_debug_file =
                filename.ends_with("_debug.txt") || filename.starts_with("temp_test_");

            if is_debug_file {
                // Check file age
                if let Ok(metadata) = fs::metadata(&path)
                    && let Ok(modified) = metadata.modified()
                    && let Ok(age) = now.duration_since(modified)
                    && age > max_age
                {
                    // Try to remove the file
                    if fs::remove_file(&path).is_ok() {
                        cleaned_count += 1;
                        log::debug!("Cleaned up old debug file: {:?}", path);
                    }
                }
            }
        }
    }

    Ok(cleaned_count)
}

/// Clean up debug files from test environment
/// This is specifically designed for test environments to clean up
/// temporary files created during testing
pub fn cleanup_test_debug_files() -> Result<usize, std::io::Error> {
    // In test environments, clean up files older than 1 hour
    let max_age = Duration::from_secs(60 * 60); // 1 hour
    cleanup_old_debug_files(max_age)
}

/// Clean up debug files with a more aggressive age threshold
/// This can be used in CI environments or for manual cleanup
pub fn cleanup_stale_debug_files() -> Result<usize, std::io::Error> {
    // Clean up files older than 10 minutes for more aggressive cleanup
    let max_age = Duration::from_secs(10 * 60); // 10 minutes
    cleanup_old_debug_files(max_age)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::environment::Environment;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_debug_report_generation_simple() {
        let error = RuntimeError::new("Test error".to_string(), 1, 1);
        let mut call_frame = CallFrame::new("test_function".to_string(), 1, 1);

        let env = Environment::new_global();

        {
            let mut env_mut = env.borrow_mut();
            let _ = env_mut.define("x", Value::Number(42.0));
            let _ = env_mut.define("y", Value::Text("hello".into()));
        }

        call_frame.capture_locals(&env);

        let call_stack = vec![call_frame];

        let script_content = "store x as 42\nstore y as \"hello\"";

        let temp_dir = tempdir().unwrap();
        let script_path = temp_dir.path().join("test_script.wfl");
        fs::write(&script_path, script_content).unwrap();

        let report_path = create_report(
            &error,
            &call_stack,
            script_content,
            script_path.to_str().unwrap(),
        )
        .unwrap();

        assert!(report_path.exists());

        let report_content = fs::read_to_string(report_path).unwrap();

        assert!(report_content.contains("=== WFL Debug Report ==="));
        assert!(report_content.contains("=== Error Summary ==="));
        assert!(report_content.contains("Test error"));
        assert!(report_content.contains("=== Stack Trace ==="));
        assert!(report_content.contains("test_function"));
        assert!(report_content.contains("=== Source Code ==="));
        assert!(report_content.contains("=== Local Variables ==="));
        assert!(report_content.contains("x = 42"));
        assert!(report_content.contains("y = \"hello\""));
    }

    #[test]
    fn test_safe_debug_with_cyclic_list() {
        use crate::interpreter::value::Value;
        use std::cell::RefCell;
        use std::rc::Rc;

        let list = Rc::new(RefCell::new(Vec::<Value>::new()));
        let list_value = Value::List(Rc::clone(&list));

        list.borrow_mut().push(list_value.clone());

        let safe_debug = SafeDebug::new(&list_value, 4);
        let debug_output = format!("{safe_debug:?}");

        assert!(debug_output.contains("<cycle>"));

        assert!(
            debug_output.len() < 1000,
            "Debug output size: {} bytes exceeds 1000 byte limit",
            debug_output.len()
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_report_failure_message() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        use tempfile::tempdir;

        let error = RuntimeError::new("Test error".to_string(), 1, 1);
        let call_frame = CallFrame::new("test_function".to_string(), 1, 1);
        let call_stack = vec![call_frame];
        let script_content = "store x as 42";

        let temp_dir = tempdir().unwrap();
        let script_path = temp_dir.path().join("test_script.wfl");
        fs::write(&script_path, script_content).unwrap();

        let mut perms = temp_dir.path().metadata().unwrap().permissions();
        perms.set_mode(0o444); // read-only
        fs::set_permissions(temp_dir.path(), perms).unwrap();

        let result = create_report(
            &error,
            &call_stack,
            script_content,
            script_path.to_str().unwrap(),
        );

        assert!(result.is_err());

        let mut perms = temp_dir.path().metadata().unwrap().permissions();
        perms.set_mode(0o755); // rwx for owner
        fs::set_permissions(temp_dir.path(), perms).unwrap();
    }

    #[test]
    #[cfg(windows)]
    #[ignore]
    fn test_report_failure_message_windows() {
        println!("Skipping write permission test on Windows");
    }
}
