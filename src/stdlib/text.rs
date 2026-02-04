use super::helpers::{check_arg_count, expect_number, expect_text};
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use crate::stdlib::helpers::{expect_number, expect_text};
use std::cell::RefCell;
use std::rc::Rc;

/// Decode percent-encoded URL string
/// Converts '+' to space and decodes %HH hex sequences
/// Invalid sequences are left as-is
fn percent_decode(s: &str) -> String {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                result.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                // Try to parse the next two characters as hex
                let hex_str = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                if let Ok(byte) = u8::from_str_radix(hex_str, 16) {
                    result.push(byte);
                    i += 3;
                } else {
                    // Invalid hex sequence, keep the '%' as-is
                    result.push(bytes[i]);
                    i += 1;
                }
            }
            _ => {
                result.push(bytes[i]);
                i += 1;
            }
        }
    }

    // Convert bytes to String, replacing invalid UTF-8 with replacement character
    String::from_utf8(result)
        .unwrap_or_else(|e| String::from_utf8_lossy(&e.into_bytes()).into_owned())
}

/// Parse key-value pairs with URL decoding
/// Used by both query string and form data parsing
fn parse_key_value_pairs(input: &str, delimiter: char) -> std::collections::HashMap<String, Value> {
    use std::collections::HashMap;

    let mut params = HashMap::new();

    if input.is_empty() {
        return params;
    }

    for pair in input.split(delimiter) {
        if let Some((key, value)) = pair.split_once('=') {
            let decoded_key = percent_decode(key);
            let decoded_value = percent_decode(value);
            params.insert(decoded_key, Value::Text(Rc::from(decoded_value)));
        } else {
            // Key without value
            let decoded_key = percent_decode(pair);
            params.insert(decoded_key, Value::Text(Rc::from("")));
        }
    }

    params
}

// Note: The length function is now provided by the list module
// which handles both text and lists

pub fn native_touppercase(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("touppercase", &args, 1)?;

    let text = expect_text(&args[0])?;
    let uppercase = text.to_uppercase();
    Ok(Value::Text(Rc::from(uppercase)))
}

pub fn native_tolowercase(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("tolowercase", &args, 1)?;

    let text = expect_text(&args[0])?;
    let lowercase = text.to_lowercase();
    Ok(Value::Text(Rc::from(lowercase)))
}

pub fn native_contains(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("contains", &args, 2)?;

    let text = expect_text(&args[0])?;
    let substring = expect_text(&args[1])?;

    Ok(Value::Bool(text.contains(&*substring)))
}

pub fn native_substring(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("substring", &args, 3)?;

    let text = expect_text(&args[0])?;
    let start = expect_number(&args[1])? as usize;
    let length = expect_number(&args[2])? as usize;

    // Optimization: If start index is larger than the byte length, it's definitely
    // out of bounds (since num_chars <= num_bytes). This avoids iterating for very large starts.
    if start >= text.len() {
        return Ok(Value::Text(Rc::from("")));
    }

    // Optimization: Avoid intermediate String and Vec<char> allocations
    // by iterating directly over characters.
    // Note: skip(start) implicitly handles out-of-bounds start (returns empty).
    let substring: String = text.chars().skip(start).take(length).collect();

    Ok(Value::Text(Rc::from(substring)))
}

pub fn native_string_split(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("string_split", &args, 2)?;

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
    check_arg_count("trim", &args, 1)?;

    let text = expect_text(&args[0])?;
    let trimmed = text.trim();
    Ok(Value::Text(Rc::from(trimmed)))
}

pub fn native_starts_with(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("starts_with", &args, 2)?;

    let text = expect_text(&args[0])?;
    let prefix = expect_text(&args[1])?;
    let result = text.starts_with(prefix.as_ref());
    Ok(Value::Bool(result))
}

pub fn native_ends_with(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("ends_with", &args, 2)?;

    let text = expect_text(&args[0])?;
    let suffix = expect_text(&args[1])?;
    let result = text.ends_with(suffix.as_ref());
    Ok(Value::Bool(result))
}

/// Parse query string into WFL object
/// Usage: parse_query_string("?page=1&limit=10") -> {"page": "1", "limit": "10"}
pub fn native_parse_query_string(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("parse_query_string", &args, 1)?;

    let query_str = expect_text(&args[0])?;
    let query_str = query_str.trim_start_matches('?');

    let params = parse_key_value_pairs(query_str, '&');

    Ok(Value::Object(Rc::new(RefCell::new(params))))
}

/// Parse Cookie header into WFL object
/// Usage: parse_cookies("session_id=abc123; user=alice") -> {"session_id": "abc123", "user": "alice"}
pub fn native_parse_cookies(args: Vec<Value>) -> Result<Value, RuntimeError> {
    use std::collections::HashMap;

    check_arg_count("parse_cookies", &args, 1)?;

    let cookie_header = expect_text(&args[0])?;
    let mut cookies = HashMap::new();

    if cookie_header.is_empty() {
        return Ok(Value::Object(Rc::new(RefCell::new(cookies))));
    }

    // Split by ;
    for cookie in cookie_header.split(';') {
        let cookie = cookie.trim();
        if let Some((key, value)) = cookie.split_once('=') {
            let decoded_key = percent_decode(key.trim());
            let decoded_value = percent_decode(value.trim());
            cookies.insert(decoded_key, Value::Text(Rc::from(decoded_value)));
        }
    }

    Ok(Value::Object(Rc::new(RefCell::new(cookies))))
}

/// Parse URL-encoded form data
/// Usage: parse_form_urlencoded("name=Alice&age=30") -> {"name": "Alice", "age": "30"}
pub fn native_parse_form_urlencoded(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("parse_form_urlencoded", &args, 1)?;

    let form_data = expect_text(&args[0])?;

    let params = parse_key_value_pairs(form_data.as_ref(), '&');

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
