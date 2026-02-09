use super::helpers::check_arg_count;
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::sync::Arc;

pub fn native_print(args: Vec<Value>) -> Result<Value, RuntimeError> {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        print!("{arg}");
    }
    println!();
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
    let _ = env.define("print", Value::new_native_function("print", native_print));

    let _ = env.define("typeof", Value::new_native_function("typeof", native_typeof));
    let _ = env.define(
        "isnothing",
        Value::new_native_function("isnothing", native_isnothing),
    );

    let _ = env.define("type_of", Value::new_native_function("type_of", native_typeof));
    let _ = env.define(
        "is_nothing",
        Value::new_native_function("is_nothing", native_isnothing),
    );
}
