use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::cell::RefCell;


fn expect_text(value: &Value) -> Result<&str, RuntimeError> {
    match value {
        Value::Text(text) => Ok(text),
        _ => Err(RuntimeError::new(
            format!("Expected text, got {}", value.type_name()),
            0, 0,
        )),
    }
}


pub fn native_list_dir(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("list_dir expects 1 argument, got {}", args.len()),
            0, 0,
        ));
    }

    let path_str = expect_text(&args[0])?;
    let path = Path::new(path_str);

    if !path.exists() {
        return Err(RuntimeError::new(
            format!("Directory does not exist: {}", path_str),
            0, 0,
        ));
    }

    if !path.is_dir() {
        return Err(RuntimeError::new(
            format!("Path is not a directory: {}", path_str),
            0, 0,
        ));
    }

    let entries = fs::read_dir(path).map_err(|e| {
        RuntimeError::new(
            format!("Failed to read directory {}: {}", path_str, e),
            0, 0,
        )
    })?;

    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| {
            RuntimeError::new(
                format!("Failed to read directory entry: {}", e),
                0, 0,
            )
        })?;
        
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();
        files.push(Value::Text(Rc::from(file_name_str.as_ref())));
    }

    Ok(Value::List(Rc::new(RefCell::new(files))))
}

pub fn native_glob(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("glob expects 2 arguments, got {}", args.len()),
            0, 0,
        ));
    }

    let pattern = expect_text(&args[0])?;
    let base_path = expect_text(&args[1])?;

    let full_pattern = if base_path.is_empty() {
        pattern.to_string()
    } else {
        format!("{}/{}", base_path, pattern)
    };

    let paths = glob::glob(&full_pattern).map_err(|e| {
        RuntimeError::new(
            format!("Invalid glob pattern '{}': {}", full_pattern, e),
            0, 0,
        )
    })?;

    let mut files = Vec::new();
    for path_result in paths {
        let path = path_result.map_err(|e| {
            RuntimeError::new(
                format!("Error reading glob result: {}", e),
                0, 0,
            )
        })?;
        
        let path_str = path.to_string_lossy();
        files.push(Value::Text(Rc::from(path_str.as_ref())));
    }

    Ok(Value::List(Rc::new(RefCell::new(files))))
}

pub fn native_rglob(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("rglob expects 2 arguments, got {}", args.len()),
            0, 0,
        ));
    }

    let pattern = expect_text(&args[0])?;
    let base_path = expect_text(&args[1])?;

    let recursive_pattern = if base_path.is_empty() {
        format!("**/{}", pattern)
    } else {
        format!("{}/**/{}", base_path, pattern)
    };

    let paths = glob::glob(&recursive_pattern).map_err(|e| {
        RuntimeError::new(
            format!("Invalid recursive glob pattern '{}': {}", recursive_pattern, e),
            0, 0,
        )
    })?;

    let mut files = Vec::new();
    for path_result in paths {
        let path = path_result.map_err(|e| {
            RuntimeError::new(
                format!("Error reading recursive glob result: {}", e),
                0, 0,
            )
        })?;
        
        let path_str = path.to_string_lossy();
        files.push(Value::Text(Rc::from(path_str.as_ref())));
    }

    Ok(Value::List(Rc::new(RefCell::new(files))))
}

pub fn native_path_join(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.is_empty() {
        return Err(RuntimeError::new(
            "path_join expects at least 1 argument".to_string(),
            0, 0,
        ));
    }

    let mut path = PathBuf::new();
    for arg in &args {
        let component = expect_text(arg)?;
        path.push(component);
    }

    let result = path.to_string_lossy();
    Ok(Value::Text(Rc::from(result.as_ref())))
}

pub fn native_path_basename(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("path_basename expects 1 argument, got {}", args.len()),
            0, 0,
        ));
    }

    let path_str = expect_text(&args[0])?;
    let path = Path::new(path_str);
    
    let basename = path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");

    Ok(Value::Text(Rc::from(basename)))
}

pub fn native_path_dirname(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("path_dirname expects 1 argument, got {}", args.len()),
            0, 0,
        ));
    }

    let path_str = expect_text(&args[0])?;
    let path = Path::new(path_str);
    
    let dirname = path.parent()
        .and_then(|parent| parent.to_str())
        .unwrap_or("");

    Ok(Value::Text(Rc::from(dirname)))
}

pub fn native_makedirs(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("makedirs expects 1 argument, got {}", args.len()),
            0, 0,
        ));
    }

    let path_str = expect_text(&args[0])?;
    let path = Path::new(path_str);

    fs::create_dir_all(path).map_err(|e| {
        RuntimeError::new(
            format!("Failed to create directories '{}': {}", path_str, e),
            0, 0,
        )
    })?;

    Ok(Value::Null)
}

pub fn native_file_mtime(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("file_mtime expects 1 argument, got {}", args.len()),
            0, 0,
        ));
    }

    let path_str = expect_text(&args[0])?;
    let path = Path::new(path_str);

    if !path.exists() {
        return Err(RuntimeError::new(
            format!("File does not exist: {}", path_str),
            0, 0,
        ));
    }

    let metadata = fs::metadata(path).map_err(|e| {
        RuntimeError::new(
            format!("Failed to get metadata for '{}': {}", path_str, e),
            0, 0,
        )
    })?;

    let modified = metadata.modified().map_err(|e| {
        RuntimeError::new(
            format!("Failed to get modification time for '{}': {}", path_str, e),
            0, 0,
        )
    })?;

    let timestamp = modified.duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| {
            RuntimeError::new(
                format!("Failed to convert modification time to timestamp: {}", e),
                0, 0,
            )
        })?;

    Ok(Value::Number(timestamp.as_secs_f64()))
}

pub fn native_path_exists(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("path_exists expects 1 argument, got {}", args.len()),
            0, 0,
        ));
    }

    let path_str = expect_text(&args[0])?;
    let path = Path::new(path_str);
    
    Ok(Value::Bool(path.exists()))
}

pub fn native_is_file(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("is_file expects 1 argument, got {}", args.len()),
            0, 0,
        ));
    }

    let path_str = expect_text(&args[0])?;
    let path = Path::new(path_str);
    
    Ok(Value::Bool(path.is_file()))
}

pub fn native_is_dir(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("is_dir expects 1 argument, got {}", args.len()),
            0, 0,
        ));
    }

    let path_str = expect_text(&args[0])?;
    let path = Path::new(path_str);
    
    Ok(Value::Bool(path.is_dir()))
}

pub fn register_filesystem(env: &mut crate::interpreter::environment::Environment) {
    env.define("list_dir", Value::NativeFunction("list_dir", native_list_dir));
    env.define("glob", Value::NativeFunction("glob", native_glob));
    env.define("rglob", Value::NativeFunction("rglob", native_rglob));
    env.define("path_join", Value::NativeFunction("path_join", native_path_join));
    env.define("path_basename", Value::NativeFunction("path_basename", native_path_basename));
    env.define("path_dirname", Value::NativeFunction("path_dirname", native_path_dirname));
    env.define("makedirs", Value::NativeFunction("makedirs", native_makedirs));
    env.define("file_mtime", Value::NativeFunction("file_mtime", native_file_mtime));
    env.define("path_exists", Value::NativeFunction("path_exists", native_path_exists));
    env.define("is_file", Value::NativeFunction("is_file", native_is_file));
    env.define("is_dir", Value::NativeFunction("is_dir", native_is_dir));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::value::Value;
    use std::rc::Rc;
    use tempfile::TempDir;

    #[test]
    fn test_expect_text_success() {
        let value = Value::Text(Rc::from("test"));
        let result = expect_text(&value);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test");
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
            Value::Text(Rc::from("home")),
            Value::Text(Rc::from("user")),
            Value::Text(Rc::from("documents")),
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
        assert!(result.unwrap_err().message.contains("expects at least 1 argument"));
    }

    #[test]
    fn test_native_path_basename() {
        let args = vec![Value::Text(Rc::from("/home/user/test.txt"))];
        let result = native_path_basename(args).unwrap();
        
        if let Value::Text(basename) = result {
            assert_eq!(basename.as_ref(), "test.txt");
        } else {
            panic!("Expected Text value");
        }
    }

    #[test]
    fn test_native_path_dirname() {
        let args = vec![Value::Text(Rc::from("/home/user/test.txt"))];
        let result = native_path_dirname(args).unwrap();
        
        if let Value::Text(dirname) = result {
            assert_eq!(dirname.as_ref(), "/home/user");
        } else {
            panic!("Expected Text value");
        }
    }

    #[test]
    fn test_native_path_exists_current_dir() {
        let args = vec![Value::Text(Rc::from("."))];
        let result = native_path_exists(args).unwrap();
        
        if let Value::Bool(exists) = result {
            assert!(exists); // Current directory should always exist
        } else {
            panic!("Expected Bool value");
        }
    }

    #[test]
    fn test_native_path_exists_nonexistent() {
        let args = vec![Value::Text(Rc::from("/nonexistent/path/that/should/not/exist"))];
        let result = native_path_exists(args).unwrap();
        
        if let Value::Bool(exists) = result {
            assert!(!exists);
        } else {
            panic!("Expected Bool value");
        }
    }

    #[test]
    fn test_native_is_dir_current() {
        let args = vec![Value::Text(Rc::from("."))];
        let result = native_is_dir(args).unwrap();
        
        if let Value::Bool(is_dir) = result {
            assert!(is_dir); // Current directory should be a directory
        } else {
            panic!("Expected Bool value");
        }
    }

    #[test]
    fn test_native_is_file_current() {
        let args = vec![Value::Text(Rc::from("."))];
        let result = native_is_file(args).unwrap();
        
        if let Value::Bool(is_file) = result {
            assert!(!is_file); // Current directory should not be a file
        } else {
            panic!("Expected Bool value");
        }
    }

    #[test]
    fn test_native_list_dir_current() {
        let args = vec![Value::Text(Rc::from("."))];
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
        let args = vec![Value::Text(Rc::from("/nonexistent/directory"))];
        let result = native_list_dir(args);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("does not exist"));
    }

    #[test]
    fn test_native_makedirs_and_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_subdir").join("nested");
        let test_path_str = test_path.to_string_lossy();
        
        let args = vec![Value::Text(Rc::from(test_path_str.as_ref()))];
        let result = native_makedirs(args);
        assert!(result.is_ok());
        
        assert!(test_path.exists());
        assert!(test_path.is_dir());
    }

    #[test]
    fn test_native_glob_basic() {
        let args = vec![
            Value::Text(Rc::from("*.rs")),
            Value::Text(Rc::from("src")),
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
            Value::Text(Rc::from("*.rs")),
            Value::Text(Rc::from("src")),
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
            Value::Text(Rc::from("path1")),
            Value::Text(Rc::from("path2")),
        ]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("expects 1 argument"));
        
        let result = native_glob(vec![Value::Text(Rc::from("*.txt"))]);
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
}
