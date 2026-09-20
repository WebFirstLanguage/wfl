use super::helpers::check_arg_count;
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::sync::Arc;

pub fn native_print(args: Vec<Value>) -> Result<Value, RuntimeError> {
    let mut line = String::new();
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            line.push(' ');
        }
        line.push_str(&arg.to_string());
    }
    crate::interpreter::io_capture::emit_line(&line);
    Ok(Value::Null)
}

pub fn native_typeof(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("typeof", &args, 1)?;

    let type_name = args[0].type_name();
    Ok(Value::Text(Arc::from(type_name)))
}

pub fn native_isnothing(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("isnothing", &args, 1)?;

    match &args[0] {
        Value::Null | Value::Nothing => Ok(Value::Bool(true)),
        _ => Ok(Value::Bool(false)),
    }
}

/// Raise an application failure through the ordinary error-unwinding path.
/// Never stringify a non-text argument: it may contain confidential record data.
pub fn native_raise_error(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("raise_error", &args, 1)?;
    match &args[0] {
        Value::Text(message) if !message.trim().is_empty() => {
            Err(RuntimeError::new(message.to_string(), 0, 0))
        }
        _ => Err(RuntimeError::new(
            "raise_error expects a nonempty text message describing the operation, cause and corrective action. Do not include confidential values.".to_string(),
            0,
            0,
        )),
    }
}

/// Locate this interpreter without a shell, PATH search, or lossy path text.
pub fn native_current_executable(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("current_executable", &args, 0)?;
    let path = std::env::current_exe().map_err(|error| {
        RuntimeError::new(
            format!("current_executable could not locate the running program: {error}"),
            0,
            0,
        )
    })?;
    let path = path.to_str().ok_or_else(|| RuntimeError::new(
        "current_executable cannot represent the running program's path as Unicode text; use a Unicode executable path".to_string(), 0, 0,
    ))?;
    Ok(Value::Text(Arc::from(path)))
}

pub fn register_core(env: &mut Environment) {
    env.define_native("print", native_print);
    env.define_native("current_executable", native_current_executable);

    env.define_native("typeof", native_typeof);
    env.define_native("isnothing", native_isnothing);

    env.define_native("type_of", native_typeof);
    env.define_native("is_nothing", native_isnothing);
    env.define_native("raise_error", native_raise_error);

    // Text constants for natural-language string handling
    let _ = env.define("newline", Value::Text("\n".into()));
    let _ = env.define("tab", Value::Text("\t".into()));

    // The version of the interpreter actually running this program (#602).
    // Exposed as an immutable constant so programs can self-report the running
    // interpreter's version without shelling out to `wfl --version`.
    let _ = env.define("wfl_version", Value::Text(crate::version::VERSION.into()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isnothing_accepts_both_legacy_no_value_variants() {
        assert_eq!(
            native_isnothing(vec![Value::Null]).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            native_isnothing(vec![Value::Nothing]).unwrap(),
            Value::Bool(true)
        );
    }
}
