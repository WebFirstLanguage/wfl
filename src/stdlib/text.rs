use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
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
    let result = text.starts_with(prefix.as_ref());
    Ok(Value::Bool(result))
}

pub fn native_ends_with(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("ends_with expects 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let text = expect_text(&args[0])?;
    let suffix = expect_text(&args[1])?;
    let result = text.ends_with(suffix.as_ref());
    Ok(Value::Bool(result))
}

/// Parse query string into WFL object
/// Usage: parse_query_string("?page=1&limit=10") -> {"page": "1", "limit": "10"}
pub fn native_parse_query_string(args: Vec<Value>) -> Result<Value, RuntimeError> {
    use std::collections::HashMap;

    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("parse_query_string expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let query_str = expect_text(&args[0])?;
    let query_str = query_str.trim_start_matches('?');

    let mut params = HashMap::new();

    if query_str.is_empty() {
        return Ok(Value::Object(Rc::new(RefCell::new(params))));
    }

    // Split by &
    for pair in query_str.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            // URL decode (basic - replace + with space)
            let decoded_key = key.replace("+", " ");
            let decoded_value = value.replace("+", " ");

            params.insert(decoded_key, Value::Text(Rc::from(decoded_value)));
        } else {
            // Key without value
            params.insert(pair.to_string(), Value::Text(Rc::from("")));
        }
    }

    Ok(Value::Object(Rc::new(RefCell::new(params))))
}

/// Parse Cookie header into WFL object
/// Usage: parse_cookies("session_id=abc123; user=alice") -> {"session_id": "abc123", "user": "alice"}
pub fn native_parse_cookies(args: Vec<Value>) -> Result<Value, RuntimeError> {
    use std::collections::HashMap;

    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("parse_cookies expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let cookie_header = expect_text(&args[0])?;
    let mut cookies = HashMap::new();

    if cookie_header.is_empty() {
        return Ok(Value::Object(Rc::new(RefCell::new(cookies))));
    }

    // Split by ;
    for cookie in cookie_header.split(';') {
        let cookie = cookie.trim();
        if let Some((key, value)) = cookie.split_once('=') {
            cookies.insert(key.trim().to_string(), Value::Text(Rc::from(value.trim())));
        }
    }

    Ok(Value::Object(Rc::new(RefCell::new(cookies))))
}

/// Parse URL-encoded form data
/// Usage: parse_form_urlencoded("name=Alice&age=30") -> {"name": "Alice", "age": "30"}
pub fn native_parse_form_urlencoded(args: Vec<Value>) -> Result<Value, RuntimeError> {
    use std::collections::HashMap;

    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!(
                "parse_form_urlencoded expects 1 argument, got {}",
                args.len()
            ),
            0,
            0,
        ));
    }

    let form_data = expect_text(&args[0])?;
    let mut params = HashMap::new();

    if form_data.is_empty() {
        return Ok(Value::Object(Rc::new(RefCell::new(params))));
    }

    // Same parsing as query string
    for pair in form_data.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            let decoded_key = key.replace("+", " ");
            let decoded_value = value.replace("+", " ");

            params.insert(decoded_key, Value::Text(Rc::from(decoded_value)));
        } else {
            params.insert(pair.to_string(), Value::Text(Rc::from("")));
        }
    }

    Ok(Value::Object(Rc::new(RefCell::new(params))))
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

    // New string manipulation functions
    let _ = env.define("trim", Value::NativeFunction("trim", native_trim));
    let _ = env.define(
        "starts_with",
        Value::NativeFunction("starts_with", native_starts_with),
    );
    let _ = env.define(
        "ends_with",
        Value::NativeFunction("ends_with", native_ends_with),
    );

    // Query string and form parsing
    let _ = env.define(
        "parse_query_string",
        Value::NativeFunction("parse_query_string", native_parse_query_string),
    );
    let _ = env.define(
        "parse_cookies",
        Value::NativeFunction("parse_cookies", native_parse_cookies),
    );
    let _ = env.define(
        "parse_form_urlencoded",
        Value::NativeFunction("parse_form_urlencoded", native_parse_form_urlencoded),
    );
}
