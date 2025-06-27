use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::time::UNIX_EPOCH;

pub fn native_dir_list(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.is_empty() || args.len() > 3 {
        return Err(RuntimeError::new(
            format!("dirlist expects 1-3 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let path_str = match &args[0] {
        Value::Text(s) => s.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                "First argument must be a path string".to_string(),
                0,
                0,
            ));
        }
    };

    let recursive = if args.len() > 1 {
        match &args[1] {
            Value::Bool(b) => *b,
            _ => false,
        }
    } else {
        false
    };

    let pattern = if args.len() > 2 {
        match &args[2] {
            Value::Text(s) => Some(s.as_ref()),
            Value::Null => None,
            _ => {
                return Err(RuntimeError::new(
                    "Pattern must be text or null".to_string(),
                    0,
                    0,
                ));
            }
        }
    } else {
        None
    };

    let mut files = Vec::new();
    collect_files(Path::new(path_str), recursive, pattern, &mut files)?;

    let wfl_files: Vec<Value> = files
        .into_iter()
        .map(|path| Value::Text(Rc::from(path)))
        .collect();

    Ok(Value::List(Rc::new(RefCell::new(wfl_files))))
}

fn collect_files(
    dir: &Path,
    recursive: bool,
    pattern: Option<&str>,
    files: &mut Vec<String>,
) -> Result<(), RuntimeError> {
    let entries = fs::read_dir(dir).map_err(|e| {
        RuntimeError::new(
            format!("Failed to read directory {}: {}", dir.display(), e),
            0,
            0,
        )
    })?;

    for entry in entries {
        let entry =
            entry.map_err(|e| RuntimeError::new(format!("Directory entry error: {}", e), 0, 0))?;
        let path = entry.path();

        if path.is_file() {
            if let Some(pattern) = pattern {
                if matches_pattern(&path, pattern) {
                    files.push(path.to_string_lossy().to_string());
                }
            } else {
                files.push(path.to_string_lossy().to_string());
            }
        } else if path.is_dir() && recursive {
            collect_files(&path, recursive, pattern, files)?;
        }
    }
    Ok(())
}

fn matches_pattern(path: &Path, pattern: &str) -> bool {
    let filename = path.file_name().unwrap_or_default().to_string_lossy();
    simple_glob_match(&filename, pattern)
}

fn simple_glob_match(text: &str, pattern: &str) -> bool {
    if pattern.contains("**") {
        let parts: Vec<&str> = pattern.split("**").collect();
        if parts.len() == 2 {
            let prefix = parts[0];
            let suffix = parts[1];
            return text.starts_with(prefix) && text.ends_with(suffix);
        }
    }

    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        let mut pos = 0;
        for (i, part) in parts.iter().enumerate() {
            if i == 0 {
                if !text[pos..].starts_with(part) {
                    return false;
                }
                pos += part.len();
            } else if i == parts.len() - 1 {
                return text[pos..].ends_with(part);
            } else if let Some(found) = text[pos..].find(part) {
                pos += found + part.len();
            } else {
                return false;
            }
        }
        true
    } else {
        text == pattern
    }
}

pub fn native_file_mtime(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("filemtime expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let path_str = match &args[0] {
        Value::Text(s) => s.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                "Argument must be a path string".to_string(),
                0,
                0,
            ));
        }
    };

    let metadata = fs::metadata(path_str).map_err(|e| {
        RuntimeError::new(
            format!("Failed to get file metadata for {}: {}", path_str, e),
            0,
            0,
        )
    })?;

    let mtime = metadata
        .modified()
        .map_err(|e| RuntimeError::new(format!("Failed to get modification time: {}", e), 0, 0))?;

    let timestamp = mtime
        .duration_since(UNIX_EPOCH)
        .map_err(|e| RuntimeError::new(format!("Invalid timestamp: {}", e), 0, 0))?
        .as_secs_f64();

    Ok(Value::Number(timestamp))
}

pub fn register_fs(env: &mut Environment) {
    env.define("dirlist", Value::NativeFunction("dirlist", native_dir_list));
    env.define(
        "filemtime",
        Value::NativeFunction("filemtime", native_file_mtime),
    );
}
