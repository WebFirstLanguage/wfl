use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use glob::{MatchOptions, glob_with};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{LazyLock, Mutex};

static STREAM_HANDLES: LazyLock<Mutex<HashMap<String, BufWriter<File>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static HANDLE_COUNTER: LazyLock<Mutex<u64>> = LazyLock::new(|| Mutex::new(0));

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

fn validate_path_security(path: &str) -> Result<PathBuf, RuntimeError> {
    let path_buf = PathBuf::from(path);

    match path_buf.canonicalize() {
        Ok(canonical) => {
            let canonical_str = canonical.to_string_lossy().replace('\\', "/");
            if canonical_str.starts_with("../")
                || canonical_str.starts_with("/tmp")
                || canonical_str.starts_with("/var")
            {
                return Err(RuntimeError::new(
                    "Output path must be inside repository root".to_string(),
                    0,
                    0,
                ));
            }
            Ok(canonical)
        }
        Err(_) => {
            let current_dir = std::env::current_dir().map_err(|_| {
                RuntimeError::new("Could not get current directory".to_string(), 0, 0)
            })?;
            let resolved = current_dir.join(&path_buf);
            Ok(resolved)
        }
    }
}

pub fn native_glob(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("glob expects 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let dir = expect_text(&args[0])?;
    let pattern = expect_text(&args[1])?;

    let search_pattern = format!("{dir}/{pattern}");
    let options = MatchOptions {
        case_sensitive: true,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };

    match glob_with(&search_pattern, options) {
        Ok(paths) => {
            let mut results = Vec::new();
            for entry in paths {
                match entry {
                    Ok(path) => {
                        let path_str = path.to_string_lossy().replace('\\', "/");
                        results.push(Value::Text(Rc::from(path_str.as_ref())));
                    }
                    Err(_) => continue,
                }
            }
            Ok(Value::List(Rc::new(RefCell::new(results))))
        }
        Err(e) => Err(RuntimeError::new(
            format!("Glob pattern error: {e}"),
            0,
            0,
        )),
    }
}

pub fn native_rglob(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("rglob expects 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let dir = expect_text(&args[0])?;
    let pattern = expect_text(&args[1])?;

    let search_pattern = format!("{dir}/**/{pattern}");
    let options = MatchOptions {
        case_sensitive: true,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };

    match glob_with(&search_pattern, options) {
        Ok(paths) => {
            let mut results = Vec::new();
            for entry in paths {
                match entry {
                    Ok(path) => {
                        if path.is_symlink() {
                            continue;
                        }
                        let path_str = path.to_string_lossy().replace('\\', "/");
                        results.push(Value::Text(Rc::from(path_str.as_ref())));
                    }
                    Err(_) => continue,
                }
            }
            Ok(Value::List(Rc::new(RefCell::new(results))))
        }
        Err(e) => Err(RuntimeError::new(
            format!("Recursive glob pattern error: {e}"),
            0,
            0,
        )),
    }
}

pub fn native_read_text(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.is_empty() || args.len() > 2 {
        return Err(RuntimeError::new(
            format!("read_text expects 1 or 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let path = expect_text(&args[0])?;
    let encoding = if args.len() == 2 {
        expect_text(&args[1])?
    } else {
        Rc::from("utf-8")
    };

    let file_path = Path::new(&*path);

    if !file_path.exists() {
        return Err(RuntimeError::new(
            format!("File does not exist: {path}"),
            0,
            0,
        ));
    }

    match encoding.as_ref() {
        "utf-8" => match std::fs::read_to_string(file_path) {
            Ok(content) => Ok(Value::Text(Rc::from(content))),
            Err(e) => Err(RuntimeError::new(
                format!("Failed to read file as UTF-8: {e}"),
                0,
                0,
            )),
        },
        "latin-1" => match std::fs::read(file_path) {
            Ok(bytes) => {
                let content = bytes.iter().map(|&b| b as char).collect::<String>();
                Ok(Value::Text(Rc::from(content)))
            }
            Err(e) => Err(RuntimeError::new(
                format!("Failed to read file as Latin-1: {e}"),
                0,
                0,
            )),
        },
        "auto" => match std::fs::read_to_string(file_path) {
            Ok(content) => Ok(Value::Text(Rc::from(content))),
            Err(_) => match std::fs::read(file_path) {
                Ok(bytes) => {
                    let content = bytes.iter().map(|&b| b as char).collect::<String>();
                    Ok(Value::Text(Rc::from(content)))
                }
                Err(e) => Err(RuntimeError::new(
                    format!("Failed to read file with auto encoding: {e}"),
                    0,
                    0,
                )),
            },
        },
        _ => Err(RuntimeError::new(
            format!("Unsupported encoding: {encoding}"),
            0,
            0,
        )),
    }
}

pub fn native_write_stream_open(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("write_stream_open expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let path = expect_text(&args[0])?;
    let validated_path = validate_path_security(&path)?;

    if let Some(parent) = validated_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                RuntimeError::new(format!("Failed to create directory: {e}"), 0, 0)
            })?;
        }
    }

    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&validated_path)
        .map_err(|e| RuntimeError::new(format!("Failed to open file for writing: {e}"), 0, 0))?;

    let writer = BufWriter::new(file);

    let mut counter = HANDLE_COUNTER.lock().unwrap();
    *counter += 1;
    let handle_id = format!("stream_{}", *counter);

    let mut handles = STREAM_HANDLES.lock().unwrap();
    handles.insert(handle_id.clone(), writer);

    Ok(Value::Text(Rc::from(handle_id)))
}

pub fn native_write_stream_write(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() < 2 || args.len() > 3 {
        return Err(RuntimeError::new(
            format!(
                "write_stream_write expects 2 or 3 arguments, got {}",
                args.len()
            ),
            0,
            0,
        ));
    }

    let handle_id = expect_text(&args[0])?;
    let content = expect_text(&args[1])?;
    let _chunk_size = if args.len() == 3 {
        expect_number(&args[2])? as usize
    } else {
        65536
    };

    let mut handles = STREAM_HANDLES.lock().unwrap();

    if let Some(writer) = handles.get_mut(&*handle_id) {
        writer
            .write_all(content.as_bytes())
            .map_err(|e| RuntimeError::new(format!("Failed to write to stream: {e}"), 0, 0))?;
        writer
            .flush()
            .map_err(|e| RuntimeError::new(format!("Failed to flush stream: {e}"), 0, 0))?;
        Ok(Value::Null)
    } else {
        Err(RuntimeError::new(
            format!("Invalid stream handle: {handle_id}"),
            0,
            0,
        ))
    }
}

pub fn native_write_stream_close(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("write_stream_close expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let handle_id = expect_text(&args[0])?;

    let mut handles = STREAM_HANDLES.lock().unwrap();

    if let Some(mut writer) = handles.remove(&*handle_id) {
        writer.flush().map_err(|e| {
            RuntimeError::new(
                format!("Failed to flush stream before closing: {e}"),
                0,
                0,
            )
        })?;
        Ok(Value::Null)
    } else {
        Err(RuntimeError::new(
            format!("Invalid stream handle: {handle_id}"),
            0,
            0,
        ))
    }
}

pub fn native_file_exists(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("file_exists expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let path = expect_text(&args[0])?;
    let file_path = Path::new(&*path);
    Ok(Value::Bool(file_path.exists()))
}

pub fn native_create_dir(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("create_dir expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let path = expect_text(&args[0])?;
    let dir_path = Path::new(&*path);

    std::fs::create_dir_all(dir_path)
        .map_err(|e| RuntimeError::new(format!("Failed to create directory: {e}"), 0, 0))?;

    Ok(Value::Null)
}

pub fn native_file_mtime(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("file_mtime expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let path = expect_text(&args[0])?;
    let file_path = Path::new(&*path);

    if !file_path.exists() {
        return Err(RuntimeError::new(
            format!("File does not exist: {path}"),
            0,
            0,
        ));
    }

    match file_path.metadata() {
        Ok(metadata) => match metadata.modified() {
            Ok(modified) => {
                let timestamp = modified
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as f64;
                Ok(Value::Number(timestamp))
            }
            Err(e) => Err(RuntimeError::new(
                format!("Failed to get modification time: {e}"),
                0,
                0,
            )),
        },
        Err(e) => Err(RuntimeError::new(
            format!("Failed to get file metadata: {e}"),
            0,
            0,
        )),
    }
}

pub fn register_fs(env: &mut Environment) {
    env.define("glob", Value::NativeFunction("glob", native_glob));
    env.define("rglob", Value::NativeFunction("rglob", native_rglob));
    env.define(
        "read_text",
        Value::NativeFunction("read_text", native_read_text),
    );
    env.define(
        "write_stream_open",
        Value::NativeFunction("write_stream_open", native_write_stream_open),
    );
    env.define(
        "write_stream_write",
        Value::NativeFunction("write_stream_write", native_write_stream_write),
    );
    env.define(
        "write_stream_close",
        Value::NativeFunction("write_stream_close", native_write_stream_close),
    );
    env.define(
        "file_exists",
        Value::NativeFunction("file_exists", native_file_exists),
    );
    env.define(
        "create_dir",
        Value::NativeFunction("create_dir", native_create_dir),
    );
    env.define(
        "file_mtime",
        Value::NativeFunction("file_mtime", native_file_mtime),
    );
}
