use super::helpers::{
    binary_text_predicate, check_arg_count, expect_number, expect_text, unary_text_op,
    unary_text_op_arc,
};
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// Decode percent-encoded URL string
/// Converts '+' to space and decodes %HH hex sequences
/// Invalid sequences are left as-is
fn percent_decode(s: &str) -> Cow<'_, str> {
    let bytes = s.as_bytes();

    // Optimization: Fast-path for unencoded strings.
    // If there are no '%' or '+' characters, the string is not encoded.
    // We can avoid Vec and String allocation entirely by returning a Cow::Borrowed.
    if !bytes.iter().any(|&b| b == b'%' || b == b'+') {
        return Cow::Borrowed(s);
    }

    let mut result = Vec::with_capacity(bytes.len());
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
    let decoded = String::from_utf8(result)
        .unwrap_or_else(|e| String::from_utf8_lossy(&e.into_bytes()).into_owned());
    Cow::Owned(decoded)
}

/// Parse key-value pairs with URL decoding
/// Used by query string, form data, and cookie parsing
fn parse_key_value_pairs(
    input: &str,
    delimiter: char,
    trim_parts: bool,
    ignore_empty_values: bool,
) -> std::collections::HashMap<String, Value> {
    use std::collections::HashMap;

    let mut params = HashMap::new();

    if input.is_empty() {
        return params;
    }

    for pair in input.split(delimiter) {
        let pair = if trim_parts { pair.trim() } else { pair };
        if pair.is_empty() {
            continue;
        }

        if let Some((key, value)) = pair.split_once('=') {
            let key = if trim_parts { key.trim() } else { key };
            let value = if trim_parts { value.trim() } else { value };

            let decoded_key = percent_decode(key);
            let decoded_value = percent_decode(value);
            params.insert(
                decoded_key.into_owned(),
                Value::Text(Arc::from(decoded_value.as_ref())),
            );
        } else if !ignore_empty_values {
            // Key without value
            let key = if trim_parts { pair.trim() } else { pair };
            let decoded_key = percent_decode(key);
            params.insert(decoded_key.into_owned(), Value::Text(Arc::from("")));
        }
    }

    params
}

// Note: The length function is now provided by the list module
// which handles both text and lists

pub fn native_touppercase(args: Vec<Value>) -> Result<Value, RuntimeError> {
    // Optimization: avoid string allocation if string is already uppercase
    unary_text_op_arc("touppercase", args, |text| {
        // fast path: check if it changes when converted to uppercase
        let is_uppercase = text.chars().all(|c| {
            let mut iter = c.to_uppercase();
            iter.next() == Some(c) && iter.next().is_none()
        });
        if is_uppercase {
            text
        } else {
            Arc::from(text.to_uppercase())
        }
    })
}

pub fn native_tolowercase(args: Vec<Value>) -> Result<Value, RuntimeError> {
    // Optimization: avoid string allocation if string is already lowercase
    unary_text_op_arc("tolowercase", args, |text| {
        // fast path: check if it changes when converted to lowercase
        let is_lowercase = text.chars().all(|c| {
            let mut iter = c.to_lowercase();
            iter.next() == Some(c) && iter.next().is_none()
        });
        if is_lowercase {
            text
        } else {
            Arc::from(text.to_lowercase())
        }
    })
}

pub fn native_substring(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("substring", &args, 3)?;

    let text = expect_text(&args[0])?;
    let start = expect_number(&args[1])? as usize;
    let length = expect_number(&args[2])? as usize;

    // Optimization: If start index is larger than the byte length, it's definitely
    // out of bounds (since num_chars <= num_bytes). This avoids iterating for very large starts.
    if start >= text.len() {
        return Ok(Value::Text(Arc::from("")));
    }

    // Optimization: Use chars() iterator which is highly optimized for skipping
    // and as_str() to get slices without allocating intermediate strings.
    let mut chars = text.chars();

    // Skip to start
    if start > 0 && chars.nth(start - 1).is_none() {
        return Ok(Value::Text(Arc::from("")));
    }

    // Slice starting at 'start'
    let start_slice = chars.as_str();

    if length == 0 {
        return Ok(Value::Text(Arc::from("")));
    }

    // Skip 'length' characters to find the end
    // nth(n) skips n items and returns the (n+1)th.
    // We want to consume 'length' items.
    // nth(length - 1) will consume 'length' items.
    if chars.nth(length - 1).is_none() {
        // Length exceeds remaining string, return everything from start
        return Ok(Value::Text(Arc::from(start_slice)));
    }

    // The iterator is now positioned after the substring
    let end_slice = chars.as_str();

    // Calculate byte length of the substring
    let byte_len = start_slice.len() - end_slice.len();

    // Slice the substring from the start slice
    let substring = &start_slice[..byte_len];

    Ok(Value::Text(Arc::from(substring)))
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
        .map(|s| Value::Text(Arc::from(s)))
        .collect();

    Ok(Value::List(Rc::new(RefCell::new(parts))))
}

pub fn native_trim(args: Vec<Value>) -> Result<Value, RuntimeError> {
    // Optimization: avoid string allocation if string is already trimmed
    unary_text_op_arc("trim", args, |text| {
        let trimmed = text.trim();
        if trimmed.len() == text.len() {
            text
        } else {
            Arc::from(trimmed)
        }
    })
}

pub fn native_starts_with(args: Vec<Value>) -> Result<Value, RuntimeError> {
    binary_text_predicate("starts_with", args, |text, prefix| text.starts_with(prefix))
}

pub fn native_ends_with(args: Vec<Value>) -> Result<Value, RuntimeError> {
    binary_text_predicate("ends_with", args, |text, suffix| text.ends_with(suffix))
}

/// Parse query string into WFL object
/// Usage: parse_query_string("?page=1&limit=10") -> {"page": "1", "limit": "10"}
pub fn native_parse_query_string(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("parse_query_string", &args, 1)?;

    let query_str = expect_text(&args[0])?;
    let query_str = query_str.trim_start_matches('?');

    let params = parse_key_value_pairs(query_str, '&', false, false);

    Ok(Value::Object(Rc::new(RefCell::new(params))))
}

/// Parse Cookie header into WFL object
/// Usage: parse_cookies("session_id=abc123; user=alice") -> {"session_id": "abc123", "user": "alice"}
pub fn native_parse_cookies(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("parse_cookies", &args, 1)?;

    let cookie_header = expect_text(&args[0])?;

    // Cookies are separated by ';', parts need trimming, and valueless pairs are typically ignored
    let cookies = parse_key_value_pairs(cookie_header.as_ref(), ';', true, true);

    Ok(Value::Object(Rc::new(RefCell::new(cookies))))
}

/// Parse URL-encoded form data
/// Usage: parse_form_urlencoded("name=Alice&age=30") -> {"name": "Alice", "age": "30"}
pub fn native_parse_form_urlencoded(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("parse_form_urlencoded", &args, 1)?;

    let form_data = expect_text(&args[0])?;

    let params = parse_key_value_pairs(form_data.as_ref(), '&', false, false);

    Ok(Value::Object(Rc::new(RefCell::new(params))))
}

pub fn native_replace(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("replace", &args, 3)?;

    let text = expect_text(&args[0])?;
    let old = expect_text(&args[1])?;
    let new = expect_text(&args[2])?;
    let result = text.replace(old.as_ref(), new.as_ref());
    Ok(Value::Text(Arc::from(result)))
}

pub fn native_last_index_of(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("last_index_of", &args, 2)?;

    let text = expect_text(&args[0])?;
    let needle = expect_text(&args[1])?;
    match text.rfind(needle.as_ref()) {
        Some(byte_pos) => {
            let char_index = text[..byte_pos].chars().count();
            Ok(Value::Number(char_index as f64))
        }
        None => Ok(Value::Number(-1.0)),
    }
}

const MAX_PAD_WIDTH: usize = 1024;

fn validated_pad_width(raw: f64) -> Result<usize, RuntimeError> {
    if !raw.is_finite() || raw < 0.0 {
        return Err(RuntimeError::new(
            format!("pad width must be a finite non-negative number, got {raw}"),
            0,
            0,
        ));
    }
    Ok((raw as usize).min(MAX_PAD_WIDTH))
}

fn perform_pad(args: Vec<Value>, pad_left: bool) -> Result<Value, RuntimeError> {
    let func_name = if pad_left { "padleft" } else { "padright" };
    check_arg_count(func_name, &args, 2)?;

    let text = expect_text(&args[0])?;
    let width = validated_pad_width(expect_number(&args[1])?)?;
    let len = text.chars().count();

    if len >= width {
        Ok(Value::Text(Arc::clone(&text)))
    } else {
        let padding_len = width - len;
        let capacity = text.len() + padding_len;
        let mut result = String::with_capacity(capacity);

        if pad_left {
            for _ in 0..padding_len {
                result.push(' ');
            }
            result.push_str(&text);
        } else {
            result.push_str(&text);
            for _ in 0..padding_len {
                result.push(' ');
            }
        }
        Ok(Value::Text(Arc::from(result)))
    }
}

pub fn native_padleft(args: Vec<Value>) -> Result<Value, RuntimeError> {
    perform_pad(args, true)
}

pub fn native_padright(args: Vec<Value>) -> Result<Value, RuntimeError> {
    perform_pad(args, false)
}

pub fn native_capitalize(args: Vec<Value>) -> Result<Value, RuntimeError> {
    unary_text_op("capitalize", args, |text| {
        let mut chars = text.chars();
        match chars.next() {
            Some(c) => {
                // Optimization: Avoid intermediate String allocation from collect()
                // and the overhead of the format! macro by pre-allocating the
                // result string and pushing characters/slices directly.
                let mut result = String::with_capacity(text.len());
                for u in c.to_uppercase() {
                    result.push(u);
                }
                result.push_str(chars.as_str());
                result
            }
            None => String::new(),
        }
    })
}

pub fn native_reverse_text(args: Vec<Value>) -> Result<Value, RuntimeError> {
    unary_text_op("reverse", args, |text| {
        text.chars().rev().collect::<String>()
    })
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

    // Aliases for split, startswith, endswith
    let _ = env.define("split", Value::NativeFunction("split", native_string_split));
    let _ = env.define(
        "startswith",
        Value::NativeFunction("startswith", native_starts_with),
    );
    let _ = env.define(
        "endswith",
        Value::NativeFunction("endswith", native_ends_with),
    );

    // New text manipulation functions
    let _ = env.define("replace", Value::NativeFunction("replace", native_replace));
    let _ = env.define(
        "last_index_of",
        Value::NativeFunction("last_index_of", native_last_index_of),
    );
    let _ = env.define(
        "lastindexof",
        Value::NativeFunction("lastindexof", native_last_index_of),
    );
    let _ = env.define("padleft", Value::NativeFunction("padleft", native_padleft));
    let _ = env.define(
        "padright",
        Value::NativeFunction("padright", native_padright),
    );
    let _ = env.define(
        "capitalize",
        Value::NativeFunction("capitalize", native_capitalize),
    );
    let _ = env.define(
        "reverse",
        Value::NativeFunction("reverse", native_reverse_text),
    );
    let _ = env.define(
        "reverse_text",
        Value::NativeFunction("reverse_text", native_reverse_text),
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

    #[test]
    fn test_replace() {
        let result = native_replace(vec![
            Value::Text(Arc::from("hello world")),
            Value::Text(Arc::from("world")),
            Value::Text(Arc::from("rust")),
        ])
        .unwrap();
        assert_eq!(result, Value::Text(Arc::from("hello rust")));
    }

    #[test]
    fn test_replace_multiple() {
        let result = native_replace(vec![
            Value::Text(Arc::from("aaa")),
            Value::Text(Arc::from("a")),
            Value::Text(Arc::from("b")),
        ])
        .unwrap();
        assert_eq!(result, Value::Text(Arc::from("bbb")));
    }

    #[test]
    fn test_last_index_of() {
        let result = native_last_index_of(vec![
            Value::Text(Arc::from("abcabc")),
            Value::Text(Arc::from("bc")),
        ])
        .unwrap();
        assert_eq!(result, Value::Number(4.0));
    }

    #[test]
    fn test_last_index_of_not_found() {
        let result = native_last_index_of(vec![
            Value::Text(Arc::from("hello")),
            Value::Text(Arc::from("xyz")),
        ])
        .unwrap();
        assert_eq!(result, Value::Number(-1.0));
    }

    #[test]
    fn test_padleft() {
        let result =
            native_padleft(vec![Value::Text(Arc::from("hi")), Value::Number(5.0)]).unwrap();
        assert_eq!(result, Value::Text(Arc::from("   hi")));
    }

    #[test]
    fn test_padleft_no_padding_needed() {
        let result =
            native_padleft(vec![Value::Text(Arc::from("hello")), Value::Number(3.0)]).unwrap();
        assert_eq!(result, Value::Text(Arc::from("hello")));
    }

    #[test]
    fn test_padright() {
        let result =
            native_padright(vec![Value::Text(Arc::from("hi")), Value::Number(5.0)]).unwrap();
        assert_eq!(result, Value::Text(Arc::from("hi   ")));
    }

    #[test]
    fn test_capitalize() {
        let result = native_capitalize(vec![Value::Text(Arc::from("hello"))]).unwrap();
        assert_eq!(result, Value::Text(Arc::from("Hello")));
    }

    #[test]
    fn test_capitalize_empty() {
        let result = native_capitalize(vec![Value::Text(Arc::from(""))]).unwrap();
        assert_eq!(result, Value::Text(Arc::from("")));
    }

    #[test]
    fn test_capitalize_unicode_expansion() {
        let result = native_capitalize(vec![Value::Text(Arc::from("ßeta"))]).unwrap();
        assert_eq!(result, Value::Text(Arc::from("SSeta")));
    }

    #[test]
    fn test_reverse_text() {
        let result = native_reverse_text(vec![Value::Text(Arc::from("hello"))]).unwrap();
        assert_eq!(result, Value::Text(Arc::from("olleh")));
    }

    #[test]
    fn test_reverse_text_empty() {
        let result = native_reverse_text(vec![Value::Text(Arc::from(""))]).unwrap();
        assert_eq!(result, Value::Text(Arc::from("")));
    }

    #[test]
    fn test_text_arg_count() {
        assert!(native_replace(vec![Value::Text(Arc::from("a"))]).is_err());
        assert!(native_last_index_of(vec![Value::Text(Arc::from("a"))]).is_err());
        assert!(native_padleft(vec![]).is_err());
        assert!(native_padright(vec![]).is_err());
        assert!(native_capitalize(vec![]).is_err());
        assert!(native_reverse_text(vec![]).is_err());
    }

    #[test]
    fn test_last_index_of_unicode_char_index() {
        // "café" — é is 2 bytes in UTF-8, so byte offset != char offset
        let result = native_last_index_of(vec![
            Value::Text(Arc::from("café café")),
            Value::Text(Arc::from("é")),
        ])
        .unwrap();
        // Last 'é' is at char index 8 (c-a-f-é- -c-a-f-é)
        assert_eq!(result, Value::Number(8.0));
    }

    #[test]
    fn test_padleft_negative_width_errors() {
        assert!(native_padleft(vec![Value::Text(Arc::from("hi")), Value::Number(-5.0),]).is_err());
    }

    #[test]
    fn test_padleft_infinite_width_errors() {
        assert!(
            native_padleft(vec![
                Value::Text(Arc::from("hi")),
                Value::Number(f64::INFINITY),
            ])
            .is_err()
        );
    }

    #[test]
    fn test_padright_nan_width_errors() {
        assert!(
            native_padright(vec![Value::Text(Arc::from("hi")), Value::Number(f64::NAN),]).is_err()
        );
    }

    #[test]
    fn test_padleft_clamps_large_width() {
        // Should not OOM — clamped to MAX_PAD_WIDTH (1024)
        let result = native_padleft(vec![
            Value::Text(Arc::from("x")),
            Value::Number(1_000_000.0),
        ])
        .unwrap();
        if let Value::Text(t) = result {
            assert_eq!(t.len(), 1024);
        } else {
            panic!("Expected text");
        }
    }

    #[test]
    fn test_substring() {
        // Normal case
        let result = native_substring(vec![
            Value::Text(Arc::from("hello world")),
            Value::Number(6.0),
            Value::Number(5.0),
        ])
        .unwrap();
        assert_eq!(result, Value::Text(Arc::from("world")));
    }

    #[test]
    fn test_substring_out_of_bounds_start() {
        let result = native_substring(vec![
            Value::Text(Arc::from("hello")),
            Value::Number(10.0),
            Value::Number(5.0),
        ])
        .unwrap();
        assert_eq!(result, Value::Text(Arc::from("")));
    }

    #[test]
    fn test_substring_length_exceeds_end() {
        let result = native_substring(vec![
            Value::Text(Arc::from("hello")),
            Value::Number(2.0),
            Value::Number(10.0),
        ])
        .unwrap();
        assert_eq!(result, Value::Text(Arc::from("llo")));
    }

    #[test]
    fn test_substring_empty_string() {
        let result = native_substring(vec![
            Value::Text(Arc::from("")),
            Value::Number(0.0),
            Value::Number(5.0),
        ])
        .unwrap();
        assert_eq!(result, Value::Text(Arc::from("")));
    }

    #[test]
    fn test_substring_zero_length() {
        let result = native_substring(vec![
            Value::Text(Arc::from("hello")),
            Value::Number(1.0),
            Value::Number(0.0),
        ])
        .unwrap();
        assert_eq!(result, Value::Text(Arc::from("")));
    }

    #[test]
    fn test_substring_unicode() {
        // "café" -> c(1), a(1), f(1), é(2)
        // Indices: 0, 1, 2, 3(4)
        // Length 4 chars.

        let result = native_substring(vec![
            Value::Text(Arc::from("café")),
            Value::Number(1.0),
            Value::Number(3.0),
        ])
        .unwrap();
        assert_eq!(result, Value::Text(Arc::from("afé")));
    }

    #[test]
    fn test_substring_unicode_middle() {
        // "heéllo"
        // 0: h
        // 1: e
        // 2: é (2 bytes)
        // 3: l
        // 4: l
        // 5: o

        let result = native_substring(vec![
            Value::Text(Arc::from("heéllo")),
            Value::Number(2.0),
            Value::Number(1.0),
        ])
        .unwrap();
        assert_eq!(result, Value::Text(Arc::from("é")));
    }

    #[test]
    fn test_substring_emoji() {
        // "👋 world"
        // 0: 👋 (4 bytes)
        // 1: space
        // 2: w

        let result = native_substring(vec![
            Value::Text(Arc::from("👋 world")),
            Value::Number(0.0),
            Value::Number(1.0),
        ])
        .unwrap();
        assert_eq!(result, Value::Text(Arc::from("👋")));

        let result = native_substring(vec![
            Value::Text(Arc::from("👋 world")),
            Value::Number(1.0),
            Value::Number(5.0),
        ])
        .unwrap();
        assert_eq!(result, Value::Text(Arc::from(" worl")));
    }
}
