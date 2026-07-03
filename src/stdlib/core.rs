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
        Value::Null => Ok(Value::Bool(true)),
        _ => Ok(Value::Bool(false)),
    }
}

pub fn register_core(env: &mut Environment) {
    env.define_native("print", native_print);

    env.define_native("typeof", native_typeof);
    env.define_native("isnothing", native_isnothing);

    env.define_native("type_of", native_typeof);
    env.define_native("is_nothing", native_isnothing);

    // Text constants for natural-language string handling
    let _ = env.define("newline", Value::Text("\n".into()));
    let _ = env.define("tab", Value::Text("\t".into()));
}
