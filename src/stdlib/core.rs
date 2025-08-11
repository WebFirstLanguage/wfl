use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::rc::Rc;

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
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("typeof expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let type_name = args[0].type_name();
    Ok(Value::Text(Rc::from(type_name)))
}

pub fn native_isnothing(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("isnothing expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    match &args[0] {
        Value::Null => Ok(Value::Bool(true)),
        _ => Ok(Value::Bool(false)),
    }
}

pub fn native_contains(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("contains expects 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    match (&args[0], &args[1]) {
        // List contains item
        (Value::List(list_rc), item) => {
            let list = list_rc.borrow();
            for value in list.iter() {
                if format!("{value:?}") == format!("{item:?}") {
                    return Ok(Value::Bool(true));
                }
            }
            Ok(Value::Bool(false))
        }
        // Text contains substring
        (Value::Text(text), Value::Text(substring)) => {
            Ok(Value::Bool(text.contains(&**substring)))
        }
        // Invalid combination
        (a, b) => Err(RuntimeError::new(
            format!(
                "Cannot check if {} contains {}. Expected (list, item) or (text, text)",
                a.type_name(),
                b.type_name()
            ),
            0,
            0,
        )),
    }
}

pub fn register_core(env: &mut Environment) {
    env.define("print", Value::NativeFunction("print", native_print));

    env.define("typeof", Value::NativeFunction("typeof", native_typeof));
    env.define(
        "isnothing",
        Value::NativeFunction("isnothing", native_isnothing),
    );

    env.define("type_of", Value::NativeFunction("type_of", native_typeof));
    env.define(
        "is_nothing",
        Value::NativeFunction("is_nothing", native_isnothing),
    );

    env.define(
        "contains",
        Value::NativeFunction("contains", native_contains),
    );
}
