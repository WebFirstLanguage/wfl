pub mod core;
pub mod filesystem;
pub mod legacy_pattern;
pub mod list;
pub mod math;
pub mod pattern;
pub mod pattern_test;
pub mod text;
pub mod time;
pub mod typechecker;

use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;

fn native_length_dispatcher(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("length expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    match &args[0] {
        Value::Text(_) => text::native_length(args),
        Value::List(_) => list::native_length(args),
        _ => Err(RuntimeError::new(
            format!(
                "length function not defined for type {}",
                args[0].type_name()
            ),
            0,
            0,
        )),
    }
}

fn native_contains_dispatcher(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("contains expects 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    match &args[0] {
        Value::Text(_) => text::native_contains(args),
        Value::List(_) => list::native_contains(args),
        _ => Err(RuntimeError::new(
            format!(
                "contains function not defined for type {}",
                args[0].type_name()
            ),
            0,
            0,
        )),
    }
}

pub fn register_stdlib(env: &mut Environment) {
    core::register_core(env);
    filesystem::register_filesystem(env);
    math::register_math(env);

    // Register individual functions from text and list modules, except overloaded ones
    text::register_text_specific(env);
    list::register_list_specific(env);

    // Register dispatchers for overloaded functions
    env.define(
        "length",
        Value::NativeFunction("length", native_length_dispatcher),
    );
    env.define(
        "contains",
        Value::NativeFunction("contains", native_contains_dispatcher),
    );

    pattern::register(env);
    time::register_time(env);
}
