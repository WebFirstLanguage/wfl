use super::helpers::{check_arg_count, check_arg_range, expect_text};
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

pub fn native_list_dir(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("list_dir", &args, 1)?;

    let path_str = expect_text(&args[0])?;
    let path = Path::new(path_str.as_ref());

    if !path.exists() {
        return Err(RuntimeError::new(
            format!("Directory does not exist: {path_str}"),
            0,
            0,
        ));
    }

    if !path.is_dir() {
        return Err(RuntimeError::new(
            format!("Path is not a directory: {path_str}"),
            0,
            0,
        ));
    }

    let entries = fs::read_dir(path).map_err(|e| {
        RuntimeError::new(format!("Failed to read directory {path_str}: {e}"), 0, 0)
    })?;

    let mut files = Vec::new();
    for entry in entries {
        let entry = entry
            .map_err(|e| RuntimeError::new(format!("Failed to read directory entry: {e}"), 0, 0))?;

        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();
        files.push(Value::Text(Arc::from(file_name_str.as_ref())));
    }

    Ok(Value::List(Rc::new(RefCell::new(files))))
}

pub fn native_glob(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("glob", &args, 2)?;

    let pattern = expect_text(&args[0])?;
    let base_path = expect_text(&args[1])?;

    let full_pattern = if base_path.is_empty() {
        pattern.to_string()
    } else {
        format!("{base_path}/{pattern}")
    };

    let paths = glob::glob(&full_pattern).map_err(|e| {
        RuntimeError::new(format!("Invalid glob pattern '{full_pattern}': {e}"), 0, 0)
    })?;

    let mut files = Vec::new();
    for path_result in paths {
        let path = path_result
            .map_err(|e| RuntimeError::new(format!("Error reading glob result: {e}"), 0, 0))?;

        let path_str = path.to_string_lossy();
        files.push(Value::Text(Arc::from(path_str.as_ref())));
    }

    Ok(Value::List(Rc::new(RefCell::new(files))))
}

pub fn native_rglob(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("rglob", &args, 2)?;

    let pattern = expect_text(&args[0])?;
    let base_path = expect_text(&args[1])?;

    let recursive_pattern = if base_path.is_empty() {
        format!("**/{pattern}")
    } else {
        format!("{base_path}/**/{pattern}")
    };

    let paths = glob::glob(&recursive_pattern).map_err(|e| {
        RuntimeError::new(
            format!("Invalid recursive glob pattern '{recursive_pattern}': {e}"),
            0,
            0,
        )
    })?;

    let mut files = Vec::new();
    for path_result in paths {
        let path = path_result.map_err(|e| {
            RuntimeError::new(format!("Error reading recursive glob result: {e}"), 0, 0)
        })?;

        let path_str = path.to_string_lossy();
        files.push(Value::Text(Arc::from(path_str.as_ref())));
    }

    Ok(Value::List(Rc::new(RefCell::new(files))))
}

pub fn native_path_join(args: Vec<Value>) -> Result<Value, RuntimeError> {
    // check_min_arg_count is needed here because it says "expects at least 1 argument"
    // But helper implementation uses check_min_arg_count
    super::helpers::check_min_arg_count("path_join", &args, 1)?;

    let mut path = PathBuf::new();
    for arg in &args {
        let component = expect_text(arg)?;
        path.push(component.as_ref());
    }

    let result = path.to_string_lossy();
    Ok(Value::Text(Arc::from(result.as_ref())))
}

pub fn native_path_basename(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("path_basename", &args, 1)?;

    let path_str = expect_text(&args[0])?;
    let path = Path::new(path_str.as_ref());

    let basename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");

    Ok(Value::Text(Arc::from(basename)))
}

pub fn native_path_dirname(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("path_dirname", &args, 1)?;

    let path_str = expect_text(&args[0])?;
    let path = Path::new(path_str.as_ref());

    let dirname = path
        .parent()
        .and_then(|parent| parent.to_str())
        .unwrap_or("");

    Ok(Value::Text(Arc::from(dirname)))
}

pub fn native_makedirs(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("makedirs", &args, 1)?;

    let path_str = expect_text(&args[0])?;
    let path = Path::new(path_str.as_ref());

    fs::create_dir_all(path).map_err(|e| {
        RuntimeError::new(
            format!("Failed to create directories '{path_str}': {e}"),
            0,
            0,
        )
    })?;

    Ok(Value::Null)
}

pub fn native_file_mtime(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("file_mtime", &args, 1)?;

    let path_str = expect_text(&args[0])?;
    let path = Path::new(path_str.as_ref());

    if !path.exists() {
        return Err(RuntimeError::new(
            format!("File does not exist: {path_str}"),
            0,
            0,
        ));
    }

    let metadata = fs::metadata(path).map_err(|e| {
        RuntimeError::new(
            format!("Failed to get metadata for '{path_str}': {e}"),
            0,
            0,
        )
    })?;

    let modified = metadata.modified().map_err(|e| {
        RuntimeError::new(
            format!("Failed to get modification time for '{path_str}': {e}"),
            0,
            0,
        )
    })?;

    let timestamp = modified
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| {
            RuntimeError::new(
                format!("Failed to convert modification time to timestamp: {e}"),
                0,
                0,
            )
        })?;

    Ok(Value::Number(timestamp.as_secs_f64()))
}

pub fn native_path_exists(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("path_exists", &args, 1)?;

    let path_str = expect_text(&args[0])?;
    let path = Path::new(path_str.as_ref());

    Ok(Value::Bool(path.exists()))
}

pub fn native_is_file(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("is_file", &args, 1)?;

    let path_str = expect_text(&args[0])?;
    let path = Path::new(path_str.as_ref());

    Ok(Value::Bool(path.is_file()))
}

pub fn native_is_dir(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("is_dir", &args, 1)?;

    let path_str = expect_text(&args[0])?;
    let path = Path::new(path_str.as_ref());

    Ok(Value::Bool(path.is_dir()))
}

pub fn native_count_lines(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("count_lines", &args, 1)?;

    let path_str = expect_text(&args[0])?;
    let path = Path::new(path_str.as_ref());

    if !path.exists() {
        return Err(RuntimeError::new(
            format!("File does not exist: {path_str}"),
            0,
            0,
        ));
    }

    if !path.is_file() {
        return Err(RuntimeError::new(
            format!("Path is not a file: {path_str}"),
            0,
            0,
        ));
    }

    let content = fs::read_to_string(path)
        .map_err(|e| RuntimeError::new(format!("Failed to read file '{path_str}': {e}"), 0, 0))?;

    // Count lines by splitting on newline characters
    // Handle edge case: empty file has 0 lines
    // Handle edge case: file without trailing newline still counts all lines
    let line_count = if content.is_empty() {
        0
    } else {
        // Count newlines and add 1 if the content doesn't end with a newline
        let newline_count = content.matches('\n').count();
        if content.ends_with('\n') {
            newline_count
        } else {
            newline_count + 1
        }
    };

    Ok(Value::Number(line_count as f64))
}

pub fn native_path_extension(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("path_extension", &args, 1)?;

    let path_str = expect_text(&args[0])?;
    let path = Path::new(path_str.as_ref());

    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    Ok(Value::Text(Arc::from(extension)))
}

pub fn native_path_stem(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("path_stem", &args, 1)?;

    let path_str = expect_text(&args[0])?;
    let path = Path::new(path_str.as_ref());

    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

    Ok(Value::Text(Arc::from(stem)))
}

pub fn native_file_size(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("file_size", &args, 1)?;

    let path_str = expect_text(&args[0])?;
    let path = Path::new(path_str.as_ref());

    if !path.exists() {
        return Err(RuntimeError::new(
            format!("File does not exist: {path_str}"),
            0,
            0,
        ));
    }

    let metadata = fs::metadata(path).map_err(|e| {
        RuntimeError::new(
            format!("Failed to get file metadata for '{path_str}': {e}"),
            0,
            0,
        )
    })?;

    if !metadata.is_file() {
        return Err(RuntimeError::new(
            format!("Path is not a file: {path_str}"),
            0,
            0,
        ));
    }

    Ok(Value::Number(metadata.len() as f64))
}

pub fn native_copy_file(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("copy_file", &args, 2)?;

    let source_str = expect_text(&args[0])?;
    let dest_str = expect_text(&args[1])?;
    let source = Path::new(source_str.as_ref());
    let dest = Path::new(dest_str.as_ref());

    if !source.exists() {
        return Err(RuntimeError::new(
            format!("Source file does not exist: {source_str}"),
            0,
            0,
        ));
    }

    if !source.is_file() {
        return Err(RuntimeError::new(
            format!("Source path is not a file: {source_str}"),
            0,
            0,
        ));
    }

    fs::copy(source, dest).map_err(|e| {
        RuntimeError::new(
            format!("Failed to copy file from '{source_str}' to '{dest_str}': {e}"),
            0,
            0,
        )
    })?;

    Ok(Value::Null)
}

pub fn native_move_file(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("move_file", &args, 2)?;

    let source_str = expect_text(&args[0])?;
    let dest_str = expect_text(&args[1])?;
    let source = Path::new(source_str.as_ref());
    let dest = Path::new(dest_str.as_ref());

    if !source.exists() {
        return Err(RuntimeError::new(
            format!("Source file does not exist: {source_str}"),
            0,
            0,
        ));
    }

    fs::rename(source, dest).map_err(|e| {
        RuntimeError::new(
            format!("Failed to move file from '{source_str}' to '{dest_str}': {e}"),
            0,
            0,
        )
    })?;

    Ok(Value::Null)
}

pub fn native_remove_file(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("remove_file", &args, 1)?;

    let path_str = expect_text(&args[0])?;
    let path = Path::new(path_str.as_ref());

    if !path.exists() {
        return Err(RuntimeError::new(
            format!("File does not exist: {path_str}"),
            0,
            0,
        ));
    }

    if !path.is_file() {
        return Err(RuntimeError::new(
            format!("Path is not a file: {path_str}"),
            0,
            0,
        ));
    }

    fs::remove_file(path)
        .map_err(|e| RuntimeError::new(format!("Failed to remove file '{path_str}': {e}"), 0, 0))?;

    Ok(Value::Null)
}

pub fn native_remove_dir(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_range("remove_dir", &args, 1, 2)?;

    let path_str = expect_text(&args[0])?;
    let path = Path::new(path_str.as_ref());

    // Check for optional recursive parameter
    let recursive = if args.len() == 2 {
        match &args[1] {
            Value::Bool(b) => *b,
            _ => {
                return Err(RuntimeError::new(
                    format!(
                        "Second argument to remove_dir must be boolean, got {}",
                        args[1].type_name()
                    ),
                    0,
                    0,
                ));
            }
        }
    } else {
        false
    };

    if !path.exists() {
        return Err(RuntimeError::new(
            format!("Directory does not exist: {path_str}"),
            0,
            0,
        ));
    }

    if !path.is_dir() {
        return Err(RuntimeError::new(
            format!("Path is not a directory: {path_str}"),
            0,
            0,
        ));
    }

    if recursive {
        // Recursive deletion (like rm -rf)
        fs::remove_dir_all(path).map_err(|e| {
            RuntimeError::new(
                format!("Failed to remove directory '{path_str}' recursively: {e}"),
                0,
                0,
            )
        })?;
    } else {
        // Only remove empty directories
        fs::remove_dir(path).map_err(|e| {
            RuntimeError::new(
                format!("Failed to remove directory '{path_str}': {e}. Directory may not be empty. Use recursive parameter to force removal."),
                0,
                0,
            )
        })?;
    }

    Ok(Value::Null)
}

pub fn register_filesystem(env: &mut crate::interpreter::environment::Environment) {
    let _ = env.define(
        "list_dir",
        Value::NativeFunction("list_dir", native_list_dir),
    );
    let _ = env.define("glob", Value::NativeFunction("glob", native_glob));
    let _ = env.define("rglob", Value::NativeFunction("rglob", native_rglob));
    let _ = env.define(
        "path_join",
        Value::NativeFunction("path_join", native_path_join),
    );
    let _ = env.define(
        "path_basename",
        Value::NativeFunction("path_basename", native_path_basename),
    );
    let _ = env.define(
        "path_dirname",
        Value::NativeFunction("path_dirname", native_path_dirname),
    );
    let _ = env.define(
        "makedirs",
        Value::NativeFunction("makedirs", native_makedirs),
    );
    let _ = env.define(
        "file_mtime",
        Value::NativeFunction("file_mtime", native_file_mtime),
    );
    let _ = env.define(
        "path_exists",
        Value::NativeFunction("path_exists", native_path_exists),
    );
    let _ = env.define("is_file", Value::NativeFunction("is_file", native_is_file));
    let _ = env.define("is_dir", Value::NativeFunction("is_dir", native_is_dir));
    let _ = env.define(
        "count_lines",
        Value::NativeFunction("count_lines", native_count_lines),
    );
    let _ = env.define(
        "path_extension",
        Value::NativeFunction("path_extension", native_path_extension),
    );
    let _ = env.define(
        "path_stem",
        Value::NativeFunction("path_stem", native_path_stem),
    );
    let _ = env.define(
        "file_size",
        Value::NativeFunction("file_size", native_file_size),
    );
    let _ = env.define(
        "copy_file",
        Value::NativeFunction("copy_file", native_copy_file),
    );
    let _ = env.define(
        "move_file",
        Value::NativeFunction("move_file", native_move_file),
    );
    let _ = env.define(
        "remove_file",
        Value::NativeFunction("remove_file", native_remove_file),
    );
    let _ = env.define(
        "remove_dir",
        Value::NativeFunction("remove_dir", native_remove_dir),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::value::Value;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[test]
    fn test_expect_text_success() {
        let value = Value::Text(Arc::from("test"));
        let result = expect_text(&value);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_ref(), "test");
    }

    #[test]
    fn test_expect_text_failure() {
        let value = Value::Number(42.0);
        let result = expect_text(&value);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Expected text"));
    }

    #[test]
    fn test_native_path_join() {
        let args = vec![
            Value::Text(Arc::from("home")),
            Value::Text(Arc::from("user")),
            Value::Text(Arc::from("documents")),
        ];
        let result = native_path_join(args).unwrap();

        if let Value::Text(path) = result {
            assert!(path.contains("home"));
            assert!(path.contains("user"));
            assert!(path.contains("documents"));
        } else {
            panic!("Expected Text value");
        }
    }

    #[test]
    fn test_native_path_join_empty_args() {
        let args = vec![];
        let result = native_path_join(args);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message
                .contains("expects at least 1 argument")
        );
    }

    #[test]
    fn test_native_path_basename() {
        let args = vec![Value::Text(Arc::from("/home/user/test.txt"))];
        let result = native_path_basename(args).unwrap();

        if let Value::Text(basename) = result {
            assert_eq!(basename.as_ref(), "test.txt");
        } else {
            panic!("Expected Text value");
        }
    }

    #[test]
    fn test_native_path_dirname() {
        let args = vec![Value::Text(Arc::from("/home/user/test.txt"))];
        let result = native_path_dirname(args).unwrap();

        if let Value::Text(dirname) = result {
            assert_eq!(dirname.as_ref(), "/home/user");
        } else {
            panic!("Expected Text value");
        }
    }

    #[test]
    fn test_native_path_exists_current_dir() {
        let args = vec![Value::Text(Arc::from("."))];
        let result = native_path_exists(args).unwrap();

        if let Value::Bool(exists) = result {
            assert!(exists); // Current directory should always exist
        } else {
            panic!("Expected Bool value");
        }
    }

    #[test]
    fn test_native_path_exists_nonexistent() {
        let args = vec![Value::Text(Arc::from(
            "/nonexistent/path/that/should/not/exist",
        ))];
        let result = native_path_exists(args).unwrap();

        if let Value::Bool(exists) = result {
            assert!(!exists);
        } else {
            panic!("Expected Bool value");
        }
    }

    #[test]
    fn test_native_is_dir_current() {
        let args = vec![Value::Text(Arc::from("."))];
        let result = native_is_dir(args).unwrap();

        if let Value::Bool(is_dir) = result {
            assert!(is_dir); // Current directory should be a directory
        } else {
            panic!("Expected Bool value");
        }
    }

    #[test]
    fn test_native_is_file_current() {
        let args = vec![Value::Text(Arc::from("."))];
        let result = native_is_file(args).unwrap();

        if let Value::Bool(is_file) = result {
            assert!(!is_file); // Current directory should not be a file
        } else {
            panic!("Expected Bool value");
        }
    }

    #[test]
    fn test_native_list_dir_current() {
        let args = vec![Value::Text(Arc::from("."))];
        let result = native_list_dir(args).unwrap();

        if let Value::List(list) = result {
            let list_ref = list.borrow();
            assert!(!list_ref.is_empty()); // Current directory should have some contents
        } else {
            panic!("Expected List value");
        }
    }

    #[test]
    fn test_native_list_dir_nonexistent() {
        let args = vec![Value::Text(Arc::from("/nonexistent/directory"))];
        let result = native_list_dir(args);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("does not exist"));
    }

    #[test]
    fn test_native_makedirs_and_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_subdir").join("nested");
        let test_path_str = test_path.to_string_lossy();

        let args = vec![Value::Text(Arc::from(test_path_str.as_ref()))];
        let result = native_makedirs(args);
        assert!(result.is_ok());

        assert!(test_path.exists());
        assert!(test_path.is_dir());
    }

    #[test]
    fn test_native_glob_basic() {
        let args = vec![
            Value::Text(Arc::from("*.rs")),
            Value::Text(Arc::from("src")),
        ];
        let result = native_glob(args).unwrap();

        if let Value::List(list) = result {
            let list_ref = list.borrow();
            assert!(!list_ref.is_empty());
        } else {
            panic!("Expected List value");
        }
    }

    #[test]
    fn test_native_rglob_basic() {
        let args = vec![
            Value::Text(Arc::from("*.rs")),
            Value::Text(Arc::from("src")),
        ];
        let result = native_rglob(args).unwrap();

        if let Value::List(list) = result {
            let list_ref = list.borrow();
            assert!(!list_ref.is_empty());
        } else {
            panic!("Expected List value");
        }
    }

    #[test]
    fn test_function_argument_count_validation() {
        let result = native_list_dir(vec![]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("expects 1 argument"));

        let result = native_path_basename(vec![
            Value::Text(Arc::from("path1")),
            Value::Text(Arc::from("path2")),
        ]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("expects 1 argument"));

        let result = native_glob(vec![Value::Text(Arc::from("*.txt"))]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("expects 2 arguments"));
    }

    #[test]
    fn test_type_validation() {
        let result = native_path_exists(vec![Value::Number(42.0)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Expected text"));

        let result = native_is_dir(vec![Value::Bool(true)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Expected text"));
    }

    #[test]
    fn test_native_count_lines_success() {
        use std::fs::File;
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        let test_file_path = temp_dir.path().join("test_lines.txt");

        // Create test file with known line count
        let test_content = "Line 1\nLine 2\nLine 3\n";
        let mut file = File::create(&test_file_path).unwrap();
        file.write_all(test_content.as_bytes()).unwrap();

        let args = vec![Value::Text(Arc::from(
            test_file_path.to_string_lossy().as_ref(),
        ))];
        let result = native_count_lines(args).unwrap();

        if let Value::Number(count) = result {
            assert_eq!(count, 3.0);
        } else {
            panic!("Expected Number value");
        }
    }

    #[test]
    fn test_native_count_lines_empty_file() {
        use std::fs::File;

        let temp_dir = TempDir::new().unwrap();
        let test_file_path = temp_dir.path().join("empty.txt");

        // Create empty file
        File::create(&test_file_path).unwrap();

        let args = vec![Value::Text(Arc::from(
            test_file_path.to_string_lossy().as_ref(),
        ))];
        let result = native_count_lines(args).unwrap();

        if let Value::Number(count) = result {
            assert_eq!(count, 0.0);
        } else {
            panic!("Expected Number value");
        }
    }

    #[test]
    fn test_native_count_lines_no_trailing_newline() {
        use std::fs::File;
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        let test_file_path = temp_dir.path().join("no_trailing_newline.txt");

        // Create file without trailing newline
        let test_content = "Line 1\nLine 2\nLine 3";
        let mut file = File::create(&test_file_path).unwrap();
        file.write_all(test_content.as_bytes()).unwrap();

        let args = vec![Value::Text(Arc::from(
            test_file_path.to_string_lossy().as_ref(),
        ))];
        let result = native_count_lines(args).unwrap();

        if let Value::Number(count) = result {
            assert_eq!(count, 3.0);
        } else {
            panic!("Expected Number value");
        }
    }

    #[test]
    fn test_native_count_lines_file_not_found() {
        let args = vec![Value::Text(Arc::from("/nonexistent/file.txt"))];
        let result = native_count_lines(args);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("does not exist"));
    }

    #[test]
    fn test_native_count_lines_wrong_argument_count() {
        let result = native_count_lines(vec![]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("expects 1 argument"));

        let result = native_count_lines(vec![
            Value::Text(Arc::from("file1.txt")),
            Value::Text(Arc::from("file2.txt")),
        ]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("expects 1 argument"));
    }

    #[test]
    fn test_native_count_lines_wrong_argument_type() {
        let result = native_count_lines(vec![Value::Number(42.0)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Expected text"));
    }

    #[test]
    fn test_native_count_lines_directory_not_file() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        let args = vec![Value::Text(Arc::from(dir_path.to_string_lossy().as_ref()))];
        let result = native_count_lines(args);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("not a file"));
    }

    // Tests for path_extension
    #[test]
    fn test_native_path_extension_with_ext() {
        let args = vec![Value::Text(Arc::from("document.txt"))];
        let result = native_path_extension(args).unwrap();
        assert_eq!(result, Value::Text(Arc::from("txt")));
    }

    #[test]
    fn test_native_path_extension_multiple_dots() {
        let args = vec![Value::Text(Arc::from("archive.tar.gz"))];
        let result = native_path_extension(args).unwrap();
        assert_eq!(result, Value::Text(Arc::from("gz")));
    }

    #[test]
    fn test_native_path_extension_no_ext() {
        let args = vec![Value::Text(Arc::from("README"))];
        let result = native_path_extension(args).unwrap();
        assert_eq!(result, Value::Text(Arc::from("")));
    }

    #[test]
    fn test_native_path_extension_wrong_args() {
        let result = native_path_extension(vec![]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("expects 1 argument"));
    }

    // Tests for path_stem
    #[test]
    fn test_native_path_stem_with_ext() {
        let args = vec![Value::Text(Arc::from("document.txt"))];
        let result = native_path_stem(args).unwrap();
        assert_eq!(result, Value::Text(Arc::from("document")));
    }

    #[test]
    fn test_native_path_stem_no_ext() {
        let args = vec![Value::Text(Arc::from("README"))];
        let result = native_path_stem(args).unwrap();
        assert_eq!(result, Value::Text(Arc::from("README")));
    }

    #[test]
    fn test_native_path_stem_with_path() {
        let args = vec![Value::Text(Arc::from("/home/user/file.txt"))];
        let result = native_path_stem(args).unwrap();
        assert_eq!(result, Value::Text(Arc::from("file")));
    }

    #[test]
    fn test_native_path_stem_wrong_args() {
        let result = native_path_stem(vec![]);
        assert!(result.is_err());
    }

    // Tests for file_size
    #[test]
    fn test_native_file_size_success() {
        use std::fs::File;
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        let mut file = File::create(&test_file).unwrap();
        file.write_all(b"12345").unwrap();

        let args = vec![Value::Text(Arc::from(test_file.to_string_lossy().as_ref()))];
        let result = native_file_size(args).unwrap();

        assert_eq!(result, Value::Number(5.0));
    }

    #[test]
    fn test_native_file_size_empty_file() {
        use std::fs::File;

        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("empty.txt");
        File::create(&test_file).unwrap();

        let args = vec![Value::Text(Arc::from(test_file.to_string_lossy().as_ref()))];
        let result = native_file_size(args).unwrap();

        assert_eq!(result, Value::Number(0.0));
    }

    #[test]
    fn test_native_file_size_not_found() {
        let args = vec![Value::Text(Arc::from("nonexistent.txt"))];
        let result = native_file_size(args);

        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("does not exist"));
    }

    #[test]
    fn test_native_file_size_wrong_args() {
        let result = native_file_size(vec![]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("expects 1 argument"));
    }

    #[test]
    fn test_native_file_size_rejects_directory() {
        let temp_dir = TempDir::new().unwrap();

        let args = vec![Value::Text(Arc::from(
            temp_dir.path().to_string_lossy().as_ref(),
        ))];
        let result = native_file_size(args);

        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Path is not a file"));
    }

    // Tests for copy_file
    #[test]
    fn test_native_copy_file_success() {
        use std::fs::File;
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let dest = temp_dir.path().join("dest.txt");

        let mut file = File::create(&source).unwrap();
        file.write_all(b"content").unwrap();

        let args = vec![
            Value::Text(Arc::from(source.to_string_lossy().as_ref())),
            Value::Text(Arc::from(dest.to_string_lossy().as_ref())),
        ];
        let result = native_copy_file(args);

        assert!(result.is_ok());
        assert!(dest.exists());
        assert_eq!(fs::read_to_string(&dest).unwrap(), "content");
    }

    #[test]
    fn test_native_copy_file_source_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path().join("dest.txt");

        let args = vec![
            Value::Text(Arc::from("nonexistent.txt")),
            Value::Text(Arc::from(dest.to_string_lossy().as_ref())),
        ];
        let result = native_copy_file(args);

        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("does not exist"));
    }

    #[test]
    fn test_native_copy_file_wrong_args() {
        let result = native_copy_file(vec![Value::Text(Arc::from("only_one"))]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("expects 2 arguments"));
    }

    // Tests for move_file
    #[test]
    fn test_native_move_file_success() {
        use std::fs::File;
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let dest = temp_dir.path().join("dest.txt");

        let mut file = File::create(&source).unwrap();
        file.write_all(b"content").unwrap();

        let args = vec![
            Value::Text(Arc::from(source.to_string_lossy().as_ref())),
            Value::Text(Arc::from(dest.to_string_lossy().as_ref())),
        ];
        let result = native_move_file(args);

        assert!(result.is_ok());
        assert!(dest.exists());
        assert!(!source.exists());
    }

    #[test]
    fn test_native_move_file_source_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let dest = temp_dir.path().join("dest.txt");

        let args = vec![
            Value::Text(Arc::from("nonexistent.txt")),
            Value::Text(Arc::from(dest.to_string_lossy().as_ref())),
        ];
        let result = native_move_file(args);

        assert!(result.is_err());
    }

    #[test]
    fn test_native_move_file_wrong_args() {
        let result = native_move_file(vec![]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("expects 2 arguments"));
    }

    // Tests for remove_file
    #[test]
    fn test_native_remove_file_success() {
        use std::fs::File;

        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("to_remove.txt");
        File::create(&test_file).unwrap();

        let args = vec![Value::Text(Arc::from(test_file.to_string_lossy().as_ref()))];
        let result = native_remove_file(args);

        assert!(result.is_ok());
        assert!(!test_file.exists());
    }

    #[test]
    fn test_native_remove_file_not_found() {
        let args = vec![Value::Text(Arc::from("nonexistent.txt"))];
        let result = native_remove_file(args);

        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("does not exist"));
    }

    #[test]
    fn test_native_remove_file_wrong_args() {
        let result = native_remove_file(vec![]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("expects 1 argument"));
    }

    // Tests for remove_dir
    #[test]
    fn test_native_remove_dir_empty() {
        let temp_dir = TempDir::new().unwrap();
        let test_subdir = temp_dir.path().join("empty_dir");
        fs::create_dir(&test_subdir).unwrap();

        // Remove without recursive flag (default)
        let args = vec![Value::Text(Arc::from(
            test_subdir.to_string_lossy().as_ref(),
        ))];
        let result = native_remove_dir(args);

        assert!(result.is_ok());
        assert!(!test_subdir.exists());
    }

    #[test]
    fn test_native_remove_dir_nonempty_without_recursive() {
        use std::fs::File;

        let temp_dir = TempDir::new().unwrap();
        let test_subdir = temp_dir.path().join("nonempty_dir");
        fs::create_dir(&test_subdir).unwrap();
        File::create(test_subdir.join("file.txt")).unwrap();

        // Try to remove without recursive - should fail
        let args = vec![Value::Text(Arc::from(
            test_subdir.to_string_lossy().as_ref(),
        ))];
        let result = native_remove_dir(args);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().message;
        assert!(err_msg.contains("not empty") || err_msg.contains("Failed to remove"));
    }

    #[test]
    fn test_native_remove_dir_recursive() {
        use std::fs::File;

        let temp_dir = TempDir::new().unwrap();
        let test_subdir = temp_dir.path().join("recursive_dir");
        fs::create_dir(&test_subdir).unwrap();
        File::create(test_subdir.join("file.txt")).unwrap();

        // Remove with recursive flag
        let args = vec![
            Value::Text(Arc::from(test_subdir.to_string_lossy().as_ref())),
            Value::Bool(true), // recursive = true
        ];
        let result = native_remove_dir(args);

        assert!(result.is_ok());
        assert!(!test_subdir.exists());
    }

    #[test]
    fn test_native_remove_dir_wrong_args() {
        let result = native_remove_dir(vec![]);
        assert!(result.is_err());
    }
}
