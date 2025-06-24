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

pub fn native_length(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("length expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let text = expect_text(&args[0])?;
    Ok(Value::Number(text.len() as f64))
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

pub fn native_split(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("split expects 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let text = expect_text(&args[0])?;
    let delimiter = expect_text(&args[1])?;

    let parts: Vec<Value> = text
        .split(&*delimiter)
        .map(|s| Value::Text(Rc::from(s)))
        .collect();

    Ok(Value::List(Rc::new(std::cell::RefCell::new(parts))))
}

pub fn native_join(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("join expects 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let list = match &args[0] {
        Value::List(list_ref) => list_ref.borrow().clone(),
        _ => {
            return Err(RuntimeError::new(
                format!("Expected list, got {}", args[0].type_name()),
                0,
                0,
            ));
        }
    };

    let delimiter = expect_text(&args[1])?;

    let text_parts: Result<Vec<String>, RuntimeError> = list
        .iter()
        .map(|v| match v {
            Value::Text(s) => Ok(s.to_string()),
            _ => Err(RuntimeError::new(
                format!("List contains non-text value: {}", v.type_name()),
                0,
                0,
            )),
        })
        .collect();

    let parts = text_parts?;
    let result = parts.join(&*delimiter);
    Ok(Value::Text(Rc::from(result)))
}

pub fn native_replace(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::new(
            format!("replace expects 3 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let text = expect_text(&args[0])?;
    let old_str = expect_text(&args[1])?;
    let new_str = expect_text(&args[2])?;

    #[allow(clippy::explicit_auto_deref)]
    let result = text.replace(&*old_str, &*new_str);
    Ok(Value::Text(Rc::from(result)))
}

pub fn native_trim(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("trim expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let text = expect_text(&args[0])?;
    let trimmed = text.trim();
    Ok(Value::Text(Rc::from(trimmed)))
}

pub fn native_starts_with(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("starts_with expects 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let text = expect_text(&args[0])?;
    let prefix = expect_text(&args[1])?;

    Ok(Value::Bool(text.starts_with(&*prefix)))
}

pub fn register_text(env: &mut Environment) {
    env.define("length", Value::NativeFunction("length", native_length));
    env.define(
        "touppercase",
        Value::NativeFunction("touppercase", native_touppercase),
    );
    env.define(
        "tolowercase",
        Value::NativeFunction("tolowercase", native_tolowercase),
    );
    env.define(
        "contains",
        Value::NativeFunction("contains", native_contains),
    );
    env.define(
        "substring",
        Value::NativeFunction("substring", native_substring),
    );
    env.define("split", Value::NativeFunction("split", native_split));
    env.define("join", Value::NativeFunction("join", native_join));
    env.define("replace", Value::NativeFunction("replace", native_replace));
    env.define("trim", Value::NativeFunction("trim", native_trim));
    env.define(
        "starts_with",
        Value::NativeFunction("starts_with", native_starts_with),
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
