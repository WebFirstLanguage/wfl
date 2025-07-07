use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::path::{Path, PathBuf};
use std::rc::Rc;

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

pub fn native_path_join(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("path_join expects 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let p1 = expect_text(&args[0])?;
    let p2 = expect_text(&args[1])?;

    let path1 = Path::new(&*p1);
    let joined = path1.join(&*p2);
    let result = joined.to_string_lossy().replace('\\', "/");

    Ok(Value::Text(Rc::from(result)))
}

pub fn native_path_basename(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("path_basename expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let path_str = expect_text(&args[0])?;
    let path = Path::new(&*path_str);

    match path.file_name() {
        Some(name) => {
            let basename = name.to_string_lossy();
            Ok(Value::Text(Rc::from(basename.as_ref())))
        }
        None => Ok(Value::Text(Rc::from(""))),
    }
}

pub fn native_path_dirname(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("path_dirname expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let path_str = expect_text(&args[0])?;
    let path = Path::new(&*path_str);

    match path.parent() {
        Some(parent) => {
            let dirname = parent.to_string_lossy().replace('\\', "/");
            Ok(Value::Text(Rc::from(dirname)))
        }
        None => Ok(Value::Text(Rc::from("."))),
    }
}

pub fn native_path_relpath(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("path_relpath expects 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let path_str = expect_text(&args[0])?;
    let base_str = expect_text(&args[1])?;

    let path = Path::new(&*path_str);
    let base = Path::new(&*base_str);

    match path.strip_prefix(base) {
        Ok(relative) => {
            let result = relative.to_string_lossy().replace('\\', "/");
            Ok(Value::Text(Rc::from(result)))
        }
        Err(_) => {
            let result = path.to_string_lossy().replace('\\', "/");
            Ok(Value::Text(Rc::from(result)))
        }
    }
}

pub fn native_path_normalize(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("path_normalize expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let path_str = expect_text(&args[0])?;
    let path = PathBuf::from(&*path_str);

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push("..");
                }
            }
            std::path::Component::CurDir => {}
            _ => {
                normalized.push(component);
            }
        }
    }

    let result = normalized.to_string_lossy().replace('\\', "/");
    Ok(Value::Text(Rc::from(result)))
}

pub fn register_path(env: &mut Environment) {
    env.define(
        "path_join",
        Value::NativeFunction("path_join", native_path_join),
    );
    env.define(
        "path_basename",
        Value::NativeFunction("path_basename", native_path_basename),
    );
    env.define(
        "path_dirname",
        Value::NativeFunction("path_dirname", native_path_dirname),
    );
    env.define(
        "path_relpath",
        Value::NativeFunction("path_relpath", native_path_relpath),
    );
    env.define(
        "path_normalize",
        Value::NativeFunction("path_normalize", native_path_normalize),
    );
}
