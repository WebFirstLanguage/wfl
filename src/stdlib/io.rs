use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::time::SystemTime;

fn expect_text(value: &Value) -> Result<Rc<str>, RuntimeError> {
    match value {
        Value::Text(s) => Ok(Rc::clone(s)),
        _ => Err(RuntimeError::new(
            format!("Expected text, got {}", value.type_name()),
            0,
            0,
        )),
    }
}

fn expect_number(value: &Value) -> Result<f64, RuntimeError> {
    match value {
        Value::Number(n) => Ok(*n),
        _ => Err(RuntimeError::new(
            format!("Expected number, got {}", value.type_name()),
            0,
            0,
        )),
    }
}

fn expect_list(value: &Value) -> Result<Rc<RefCell<Vec<Value>>>, RuntimeError> {
    match value {
        Value::List(list) => Ok(Rc::clone(list)),
        _ => Err(RuntimeError::new(
            format!("Expected list, got {}", value.type_name()),
            0,
            0,
        )),
    }
}

pub fn native_walk_dir(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.is_empty() || args.len() > 2 {
        return Err(RuntimeError::new(
            format!("walk_dir expects 1 or 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let dir_path = expect_text(&args[0])?;
    let max_depth = if args.len() > 1 {
        expect_number(&args[1])? as usize
    } else {
        100
    };

    let mut files = Vec::new();
    let mut file_count = 0;
    const MAX_FILES: usize = 10_000;

    fn walk_recursive(
        dir: &Path,
        current_depth: usize,
        max_depth: usize,
        files: &mut Vec<Value>,
        file_count: &mut usize,
    ) -> Result<(), RuntimeError> {
        if current_depth > max_depth {
            return Err(RuntimeError::new(
                format!("Maximum recursion depth {} exceeded", max_depth),
                0,
                0,
            ));
        }

        if *file_count > MAX_FILES {
            return Err(RuntimeError::new(
                format!("Maximum file count {} exceeded", MAX_FILES),
                0,
                0,
            ));
        }

        let entries = fs::read_dir(dir).map_err(|e| {
            RuntimeError::new(
                format!("Failed to read directory {}: {}", dir.display(), e),
                0,
                0,
            )
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                RuntimeError::new(format!("Failed to read directory entry: {}", e), 0, 0)
            })?;

            let path = entry.path();
            if path.is_file() {
                let path_str = path.to_string_lossy().replace('\\', "/");
                files.push(Value::Text(Rc::from(path_str.as_ref())));
                *file_count += 1;
            } else if path.is_dir() {
                walk_recursive(&path, current_depth + 1, max_depth, files, file_count)?;
            }
        }

        Ok(())
    }

    let path = Path::new(dir_path.as_ref());
    if !path.exists() {
        return Err(RuntimeError::new(
            format!("Directory does not exist: {}", dir_path),
            0,
            0,
        ));
    }

    if !path.is_dir() {
        return Err(RuntimeError::new(
            format!("Path is not a directory: {}", dir_path),
            0,
            0,
        ));
    }

    walk_recursive(path, 0, max_depth, &mut files, &mut file_count)?;

    Ok(Value::List(Rc::new(RefCell::new(files))))
}

pub fn native_glob_files(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("glob_files expects 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let pattern = expect_text(&args[0])?;
    let files_list = expect_list(&args[1])?;
    let files_borrowed = files_list.borrow();

    let mut matched_files = Vec::new();

    for file_value in files_borrowed.iter() {
        let file_path = expect_text(file_value)?;
        let normalized_path = file_path.replace('\\', "/");

        if matches_glob_pattern(&normalized_path, &pattern)? {
            matched_files.push(Value::Text(Rc::from(normalized_path.as_str())));
        }
    }

    Ok(Value::List(Rc::new(RefCell::new(matched_files))))
}

fn matches_glob_pattern(path: &str, pattern: &str) -> Result<bool, RuntimeError> {
    if let Some(suffix) = pattern.strip_prefix("**/") {
        if suffix.contains("**") {
            return Err(RuntimeError::new(
                "Multiple ** patterns not supported".to_string(),
                0,
                0,
            ));
        }
        return Ok(path.ends_with(suffix) || path.contains(&format!("/{}", suffix)));
    }

    if pattern.starts_with("*.") {
        let extension = &pattern[1..];
        return Ok(path.ends_with(extension));
    }

    if pattern.contains('/') && pattern.ends_with("*") {
        let prefix = &pattern[..pattern.len() - 1];
        return Ok(path.starts_with(prefix));
    }

    Ok(path == pattern)
}

pub fn native_file_mtime(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("file_mtime expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let file_path = expect_text(&args[0])?;
    let path = Path::new(file_path.as_ref());

    let metadata = fs::metadata(path).map_err(|e| {
        RuntimeError::new(
            format!("Failed to get metadata for {}: {}", file_path, e),
            0,
            0,
        )
    })?;

    let mtime = metadata
        .modified()
        .map_err(|e| {
            RuntimeError::new(
                format!("Failed to get modification time for {}: {}", file_path, e),
                0,
                0,
            )
        })?
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| {
            RuntimeError::new(
                format!("Invalid modification time for {}: {}", file_path, e),
                0,
                0,
            )
        })?
        .as_secs() as f64;

    Ok(Value::Number(mtime))
}

pub fn native_normalize_path(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("normalize_path expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let path = expect_text(&args[0])?;
    let normalized = path.replace('\\', "/");
    Ok(Value::Text(Rc::from(normalized.as_str())))
}

pub fn native_parse_cli_args(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("parse_cli_args expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let args_list = expect_list(&args[0])?;
    let args_borrowed = args_list.borrow();
    let mut parsed_args = HashMap::new();
    let mut i = 0;

    while i < args_borrowed.len() {
        let arg = expect_text(&args_borrowed[i])?;

        if arg.as_ref() == "--help" || arg.as_ref() == "-h" {
            return Err(RuntimeError::new(get_help_text(), 2, 0));
        }

        if let Some(flag_name) = arg.strip_prefix("--") {
            if flag_name.contains('=') {
                let parts: Vec<&str> = flag_name.splitn(2, '=').collect();
                parsed_args.insert(parts[0].to_string(), Value::Text(Rc::from(parts[1])));
            } else {
                match flag_name {
                    "no-toc" | "recursive" | "all-files" | "no-txt" => {
                        parsed_args.insert(flag_name.to_string(), Value::Bool(true));
                    }
                    _ => {
                        i += 1;
                        if i >= args_borrowed.len() {
                            return Err(RuntimeError::new(
                                format!("Flag --{} requires a value", flag_name),
                                2,
                                0,
                            ));
                        }
                        let value = expect_text(&args_borrowed[i])?;
                        parsed_args.insert(flag_name.to_string(), Value::Text(value));
                    }
                }
            }
        } else if arg.starts_with('-') && arg.len() == 2 {
            let flag_char = &arg[1..];
            match flag_char {
                "h" => {
                    return Err(RuntimeError::new(get_help_text(), 2, 0));
                }
                "o" | "i" | "s" | "l" | "p" => {
                    i += 1;
                    if i >= args_borrowed.len() {
                        return Err(RuntimeError::new(
                            format!("Flag -{} requires a value", flag_char),
                            2,
                            0,
                        ));
                    }
                    let value = expect_text(&args_borrowed[i])?;
                    let full_name = match flag_char {
                        "o" => "output",
                        "i" => "input",
                        "s" => "sort",
                        "l" => "header-level",
                        "p" => "separator",
                        _ => flag_char,
                    };
                    parsed_args.insert(full_name.to_string(), Value::Text(value));
                }
                "r" => {
                    parsed_args.insert("recursive".to_string(), Value::Bool(true));
                }
                "a" => {
                    parsed_args.insert("all-files".to_string(), Value::Bool(true));
                }
                _ => {
                    return Err(RuntimeError::new(
                        format!("Unknown flag: -{}", flag_char),
                        2,
                        0,
                    ));
                }
            }
        } else {
            parsed_args.insert("positional".to_string(), Value::Text(Rc::clone(&arg)));
        }

        i += 1;
    }

    let mut result_list = Vec::new();
    for (key, value) in parsed_args {
        result_list.push(Value::Text(Rc::from(key.as_str())));
        result_list.push(value);
    }

    Ok(Value::List(Rc::new(RefCell::new(result_list))))
}

fn get_help_text() -> String {
    "usage: combiner.wfl [-h] [-o OUTPUT] [-i INPUT] [--type {docs,src,both}]\n                    [-r] [--no-toc] [-s SORT] [-l HEADER_LEVEL]\n                    [-p SEPARATOR] [-a] [--no-txt]\n\nWFL File Combiner - Combine multiple files into markdown and text files\n\noptions:\n  -h, --help            show this help message and exit\n  -o OUTPUT, --output OUTPUT\n                        Path and filename for the combined output file\n  -i INPUT, --input INPUT\n                        Directory containing files (default: based on --type)\n  --type {docs,src,both}\n                        Type of files to process: 'docs' for markdown files,\n                        'src' for Rust files, or 'both' to process both types\n  -r, --recursive       Search subdirectories for files (always enabled for\n                        src)\n  --no-toc              Disable table of contents (enabled by default)\n  -s SORT, --sort SORT  Sort files by: 'alpha', 'time', or comma-separated\n                        list for custom order\n  -l HEADER_LEVEL, --header-level HEADER_LEVEL\n                        Starting level for file headers (default: 1)\n  -p SEPARATOR, --separator SEPARATOR\n                        Custom separator between files (default: horizontal\n                        rule)\n  -a, --all-files       Include all .md files in Docs, not just those with\n                        'wfl-' in the name\n  --no-txt              Disable output to .txt format (by default outputs to\n                        both .md and .txt)".to_string()
}

pub fn native_read_file_simple(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("read_file_simple expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let file_path = expect_text(&args[0])?;

    let content = fs::read_to_string(file_path.as_ref()).map_err(|e| {
        RuntimeError::new(format!("Failed to read file {}: {}", file_path, e), 0, 0)
    })?;

    let normalized_content = content.replace("\r\n", "\n").replace('\r', "\n");
    Ok(Value::Text(Rc::from(normalized_content.as_str())))
}

pub fn native_write_file_simple(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("write_file_simple expects 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let file_path = expect_text(&args[0])?;
    let content = expect_text(&args[1])?;

    let normalized_content = content.replace("\r\n", "\n").replace('\r', "\n");

    if let Some(parent) = Path::new(file_path.as_ref()).parent() {
        fs::create_dir_all(parent).map_err(|e| {
            RuntimeError::new(
                format!("Failed to create directory {}: {}", parent.display(), e),
                0,
                0,
            )
        })?;
    }

    fs::write(file_path.as_ref(), normalized_content).map_err(|e| {
        RuntimeError::new(format!("Failed to write file {}: {}", file_path, e), 0, 0)
    })?;

    Ok(Value::Text(Rc::from("File written successfully")))
}

pub fn register_io(env: &mut Environment) {
    env.define(
        "walk_dir",
        Value::NativeFunction("walk_dir", native_walk_dir),
    );
    env.define(
        "glob_files",
        Value::NativeFunction("glob_files", native_glob_files),
    );
    env.define(
        "file_mtime",
        Value::NativeFunction("file_mtime", native_file_mtime),
    );
    env.define(
        "normalize_path",
        Value::NativeFunction("normalize_path", native_normalize_path),
    );
    env.define(
        "parse_cli_args",
        Value::NativeFunction("parse_cli_args", native_parse_cli_args),
    );
    env.define(
        "read_file_simple",
        Value::NativeFunction("read_file_simple", native_read_file_simple),
    );
    env.define(
        "write_file_simple",
        Value::NativeFunction("write_file_simple", native_write_file_simple),
    );
}
