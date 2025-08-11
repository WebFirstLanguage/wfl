use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
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

pub fn native_text_contains(args: Vec<Value>) -> Result<Value, RuntimeError> {
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

pub fn native_startswith(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("startswith expects 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let text = expect_text(&args[0])?;
    let prefix = expect_text(&args[1])?;

    let result = text.starts_with(&*prefix);
    Ok(Value::Bool(result))
}

pub fn register_text(env: &mut Environment) {
    // Note: length function is registered in list.rs which handles both List and Text types
    env.define(
        "touppercase",
        Value::NativeFunction("touppercase", native_touppercase),
    );
    env.define(
        "tolowercase",
        Value::NativeFunction("tolowercase", native_tolowercase),
    );
    env.define(
        "substring",
        Value::NativeFunction("substring", native_substring),
    );
    env.define(
        "startswith",
        Value::NativeFunction("startswith", native_startswith),
    );
    env.define(
        "starts_with",
        Value::NativeFunction("starts_with", native_startswith),
    );

    env.define(
        "to_uppercase",
        Value::NativeFunction("to_uppercase", native_touppercase),
    );
    env.define(
        "to_lowercase",
        Value::NativeFunction("to_lowercase", native_tolowercase),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn test_startswith_matching() {
        let args = vec![
            Value::Text(Rc::from("hello world")),
            Value::Text(Rc::from("hello")),
        ];
        let result = native_startswith(args).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn test_startswith_not_matching() {
        let args = vec![
            Value::Text(Rc::from("hello world")),
            Value::Text(Rc::from("world")),
        ];
        let result = native_startswith(args).unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn test_startswith_empty_prefix() {
        let args = vec![
            Value::Text(Rc::from("hello world")),
            Value::Text(Rc::from("")),
        ];
        let result = native_startswith(args).unwrap();
        assert_eq!(result, Value::Bool(true)); // Empty string is a prefix of any string
    }

    #[test]
    fn test_startswith_empty_text() {
        let args = vec![Value::Text(Rc::from("")), Value::Text(Rc::from("hello"))];
        let result = native_startswith(args).unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn test_startswith_wrong_arg_count() {
        let args = vec![Value::Text(Rc::from("hello"))];
        let result = native_startswith(args);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message
                .contains("startswith expects 2 arguments")
        );
    }

    #[test]
    fn test_startswith_wrong_type() {
        let args = vec![Value::Number(123.0), Value::Text(Rc::from("hello"))];
        let result = native_startswith(args);
        assert!(result.is_err());
    }
}
