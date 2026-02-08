use super::helpers::{check_arg_count, expect_number, expect_text};
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
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

/// Parse key-value pairs with optional URL decoding
/// Used by query string, form data, and cookie parsing
fn parse_key_value_pairs(
    input: &str,
    delimiter: char,
    should_trim: bool,
    should_decode: bool,
) -> std::collections::HashMap<String, Value> {
    use std::collections::HashMap;

    let mut params = HashMap::new();

    if input.is_empty() {
        return params;
    }

    for pair in input.split(delimiter) {
        let pair = if should_trim { pair.trim() } else { pair };
        if pair.is_empty() {
            continue;
        }

        if let Some((key, value)) = pair.split_once('=') {
            let key = if should_trim { key.trim() } else { key };
            let value = if should_trim { value.trim() } else { value };

            let decoded_key = if should_decode {
                percent_decode(key)
            } else {
                key.to_string()
            };
            let decoded_value = if should_decode {
                percent_decode(value)
            } else {
                value.to_string()
            };
            params.insert(decoded_key, Value::Text(Rc::from(decoded_value)));
        } else {
            // Key without value
            let key = if should_trim { pair.trim() } else { pair };
            let decoded_key = if should_decode {
                percent_decode(key)
            } else {
                key.to_string()
            };
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

    // Query strings should always be decoded
    let params = parse_key_value_pairs(query_str, '&', false, true);

    Ok(Value::Object(Rc::new(RefCell::new(params))))
}

/// Parse Cookie header into WFL object
/// Usage: parse_cookies("session_id=abc123; user=alice") -> {"session_id": "abc123", "user": "alice"}
pub fn native_parse_cookies(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("parse_cookies", &args, 1)?;

    let cookie_header = expect_text(&args[0])?;

    // Cookies should NOT be percent-decoded (RFC 6265)
    // Values like "abc+def" or "%20" should be preserved as-is
    let cookies = parse_key_value_pairs(&cookie_header, ';', true, false);

    Ok(Value::Object(Rc::new(RefCell::new(cookies))))
}

/// Parse URL-encoded form data
/// Usage: parse_form_urlencoded("name=Alice&age=30") -> {"name": "Alice", "age": "30"}
pub fn native_parse_form_urlencoded(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("parse_form_urlencoded", &args, 1)?;

    let form_data = expect_text(&args[0])?;

    // Form data should always be decoded
    let params = parse_key_value_pairs(form_data.as_ref(), '&', false, true);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::value::Value;

    #[test]
    fn test_parse_query_string() {
        let args = vec![Value::Text(Rc::from("?page=1&limit=10&sort=asc"))];
        let result = native_parse_query_string(args).unwrap();

        match result {
            Value::Object(obj) => {
                let map = obj.borrow();
                assert_eq!(map.get("page").unwrap(), &Value::Text(Rc::from("1")));
                assert_eq!(map.get("limit").unwrap(), &Value::Text(Rc::from("10")));
                assert_eq!(map.get("sort").unwrap(), &Value::Text(Rc::from("asc")));
            }
            _ => panic!("Expected Object"),
        }
    }

    #[test]
    fn test_parse_query_string_decoding() {
        let args = vec![Value::Text(Rc::from("name=John%20Doe&city=New+York"))];
        let result = native_parse_query_string(args).unwrap();

        match result {
            Value::Object(obj) => {
                let map = obj.borrow();
                assert_eq!(map.get("name").unwrap(), &Value::Text(Rc::from("John Doe")));
                assert_eq!(map.get("city").unwrap(), &Value::Text(Rc::from("New York")));
            }
            _ => panic!("Expected Object"),
        }
    }

    #[test]
    fn test_parse_query_string_empty_value() {
        let args = vec![Value::Text(Rc::from("flag=&option"))];
        let result = native_parse_query_string(args).unwrap();

        match result {
            Value::Object(obj) => {
                let map = obj.borrow();
                assert_eq!(map.get("flag").unwrap(), &Value::Text(Rc::from("")));
                assert_eq!(map.get("option").unwrap(), &Value::Text(Rc::from("")));
            }
            _ => panic!("Expected Object"),
        }
    }

    #[test]
    fn test_parse_cookies() {
        // Test standard cookie parsing AND ensure no percent decoding happens
        // "user=alice%20bob" should remain "alice%20bob", not "alice bob"
        // "token=abc+def" should remain "abc+def", not "abc def"
        let args = vec![Value::Text(Rc::from(
            "session_id=abc123; user=alice%20bob; token=abc+def; Secure",
        ))];
        let result = native_parse_cookies(args).unwrap();

        match result {
            Value::Object(obj) => {
                let map = obj.borrow();
                assert_eq!(
                    map.get("session_id").unwrap(),
                    &Value::Text(Rc::from("abc123"))
                );
                // Verify NO decoding happened
                assert_eq!(
                    map.get("user").unwrap(),
                    &Value::Text(Rc::from("alice%20bob"))
                );
                assert_eq!(map.get("token").unwrap(), &Value::Text(Rc::from("abc+def")));
                assert_eq!(map.get("Secure").unwrap(), &Value::Text(Rc::from("")));
            }
            _ => panic!("Expected Object"),
        }
    }

    #[test]
    fn test_parse_cookies_trimming() {
        // Spaces around semicolons and equals
        let args = vec![Value::Text(Rc::from(" key1 = val1 ;  key2=val2  "))];
        let result = native_parse_cookies(args).unwrap();

        match result {
            Value::Object(obj) => {
                let map = obj.borrow();
                assert_eq!(map.get("key1").unwrap(), &Value::Text(Rc::from("val1")));
                assert_eq!(map.get("key2").unwrap(), &Value::Text(Rc::from("val2")));
            }
            _ => panic!("Expected Object"),
        }
    }

    #[test]
    fn test_parse_form_urlencoded() {
        let args = vec![Value::Text(Rc::from("field1=value1&field2=value2"))];
        let result = native_parse_form_urlencoded(args).unwrap();

        match result {
            Value::Object(obj) => {
                let map = obj.borrow();
                assert_eq!(map.get("field1").unwrap(), &Value::Text(Rc::from("value1")));
                assert_eq!(map.get("field2").unwrap(), &Value::Text(Rc::from("value2")));
            }
            _ => panic!("Expected Object"),
        }
    }
}
