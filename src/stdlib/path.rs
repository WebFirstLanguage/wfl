use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::path::Path;
use std::rc::Rc;

pub fn native_basename(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("basename expects 1 argument, got {}", args.len()),
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

    let path = Path::new(path_str);
    let basename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");

    Ok(Value::Text(Rc::from(basename)))
}

pub fn native_dirname(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("dirname expects 1 argument, got {}", args.len()),
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

    let path = Path::new(path_str);
    let dirname = path
        .parent()
        .and_then(|parent| parent.to_str())
        .unwrap_or(".");

    Ok(Value::Text(Rc::from(dirname)))
}

pub fn native_pathjoin(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() < 2 {
        return Err(RuntimeError::new(
            format!("pathjoin expects at least 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let mut path = std::path::PathBuf::new();
    for arg in args {
        match arg {
            Value::Text(s) => path.push(s.as_ref()),
            _ => {
                return Err(RuntimeError::new(
                    "All arguments must be path strings".to_string(),
                    0,
                    0,
                ));
            }
        }
    }

    Ok(Value::Text(Rc::from(path.to_string_lossy().to_string())))
}

pub fn register_path(env: &mut Environment) {
    env.define(
        "basename",
        Value::NativeFunction("basename", native_basename),
    );
    env.define("dirname", Value::NativeFunction("dirname", native_dirname));
    env.define(
        "pathjoin",
        Value::NativeFunction("pathjoin", native_pathjoin),
    );
}
