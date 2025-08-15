use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::cell::RefCell;
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

fn expect_number(value: &Value) -> Result<f64, RuntimeError> {
    match value {
        Value::Number(n) => Ok(*n),
        _ => Err(RuntimeError::new(
            format!("Expected a number, got {}", value.type_name()),
            0,
            0,
        )),
    }
}

// Note: The length function is now provided by the list module
// which handles both text and lists

pub fn native_touppercase(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("touppercase expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let text = expect_text(&args[0])?;
    let uppercase = text.to_uppercase();
    Ok(Value::Text(Rc::from(uppercase)))
}

pub fn native_tolowercase(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("tolowercase expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let text = expect_text(&args[0])?;
    let lowercase = text.to_lowercase();
    Ok(Value::Text(Rc::from(lowercase)))
}

pub fn native_contains(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("contains expects 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let text = expect_text(&args[0])?;
    let substring = expect_text(&args[1])?;

    Ok(Value::Bool(text.contains(&*substring)))
}

pub fn native_substring(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::new(
            format!("substring expects 3 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let text = expect_text(&args[0])?;
    let start = expect_number(&args[1])? as usize;
    let length = expect_number(&args[2])? as usize;

    let text_str = text.to_string();
    let chars: Vec<char> = text_str.chars().collect();

    if start >= chars.len() {
        return Ok(Value::Text(Rc::from("")));
    }

    let end = (start + length).min(chars.len());
    let substring: String = chars[start..end].iter().collect();

    Ok(Value::Text(Rc::from(substring)))
}

pub fn native_string_split(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("string_split expects 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let text = expect_text(&args[0])?;
    let delimiter = expect_text(&args[1])?;

    // Handle empty delimiter
    if delimiter.is_empty() {
        return Err(RuntimeError::new(
            "Empty delimiter not allowed in string split".to_string(),
            0,
            0,
        ));
    }

    // Split the text by the delimiter
    let parts: Vec<Value> = text
        .split(delimiter.as_ref())
        .map(|s| Value::Text(Rc::from(s)))
        .collect();

    Ok(Value::List(Rc::new(RefCell::new(parts))))
}

pub fn register_text(env: &mut Environment) {
    // Note: length function is registered by the list module instead
    let _ = env.define(
        "touppercase",
        Value::NativeFunction("touppercase", native_touppercase),
    );
    let _ = env.define(
        "tolowercase",
        Value::NativeFunction("tolowercase", native_tolowercase),
    );
    let _ = env.define(
        "contains",
        Value::NativeFunction("contains", native_contains),
    );
    let _ = env.define(
        "substring",
        Value::NativeFunction("substring", native_substring),
    );
    let _ = env.define(
        "string_split",
        Value::NativeFunction("string_split", native_string_split),
    );

    let _ = env.define(
        "to_uppercase",
        Value::NativeFunction("to_uppercase", native_touppercase),
    );
    let _ = env.define(
        "to_lowercase",
        Value::NativeFunction("to_lowercase", native_tolowercase),
    );
}
