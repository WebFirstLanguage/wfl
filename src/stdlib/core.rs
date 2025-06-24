use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::rc::Rc;

pub fn native_print(args: Vec<Value>) -> Result<Value, RuntimeError> {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        print!("{}", arg);
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

pub fn native_number(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("number expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    match &args[0] {
        Value::Text(s) => match s.parse::<f64>() {
            Ok(n) => Ok(Value::Number(n)),
            Err(_) => Err(RuntimeError::new(
                format!("Cannot convert '{}' to number", s),
                0,
                0,
            )),
        },
        Value::Number(n) => Ok(Value::Number(*n)),
        _ => Err(RuntimeError::new(
            format!("Cannot convert {} to number", args[0].type_name()),
            0,
            0,
        )),
    }
}

pub fn native_command_line_args(_args: Vec<Value>) -> Result<Value, RuntimeError> {
    let args: Vec<Value> = std::env::args()
        .skip(1)
        .map(|arg| Value::Text(Rc::from(arg)))
        .collect();
    Ok(Value::List(Rc::new(std::cell::RefCell::new(args))))
}

pub fn native_char_to_ascii(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("char_to_ascii expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    match &args[0] {
        Value::Text(s) => {
            if s.len() != 1 {
                return Err(RuntimeError::new(
                    format!("char_to_ascii expects single character, got '{}'", s),
                    0,
                    0,
                ));
            }
            let ch = s.chars().next().unwrap();
            Ok(Value::Number(ch as u8 as f64))
        }
        _ => Err(RuntimeError::new(
            format!("Expected text, got {}", args[0].type_name()),
            0,
            0,
        )),
    }
}

pub fn native_ascii_to_char(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("ascii_to_char expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    match &args[0] {
        Value::Number(n) => {
            let ascii_code = *n as u8;
            if ascii_code > 127 {
                return Err(RuntimeError::new(
                    format!("ASCII code {} is out of range (0-127)", ascii_code),
                    0,
                    0,
                ));
            }
            let ch = ascii_code as char;
            Ok(Value::Text(Rc::from(ch.to_string())))
        }
        _ => Err(RuntimeError::new(
            format!("Expected number, got {}", args[0].type_name()),
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

    env.define("number", Value::NativeFunction("number", native_number));
    env.define(
        "command_line_args",
        Value::NativeFunction("command_line_args", native_command_line_args),
    );
    env.define(
        "char_to_ascii",
        Value::NativeFunction("char_to_ascii", native_char_to_ascii),
    );
    env.define(
        "ascii_to_char",
        Value::NativeFunction("ascii_to_char", native_ascii_to_char),
    );
}
