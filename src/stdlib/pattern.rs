use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use crate::stdlib::helpers::{check_arg_count, expect_pattern, expect_text};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

pub fn register(env: &mut Environment) {
    // Register new pattern functions that work with our pattern system
    let _ = env.define(
        "pattern_matches",
        Value::NativeFunction("pattern_matches", pattern_matches_native),
    );
    let _ = env.define(
        "pattern_find",
        Value::NativeFunction("pattern_find", pattern_find_native),
    );
    let _ = env.define(
        "pattern_find_all",
        Value::NativeFunction("pattern_find_all", pattern_find_all_native),
    );
    // Register pattern_split - this was missing!
    // Note: pattern_split is called directly from the interpreter for PatternSplit expressions,
    // but we don't register it as a standalone function since it uses special syntax
}

/// Native function: pattern_matches(text, pattern) -> boolean
/// Tests if text matches the given compiled pattern
pub fn pattern_matches_native(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("pattern_matches", &args, 2)?;

    let text_str = expect_text(&args[0])?;
    let compiled_pattern = expect_pattern(&args[1])?;

    let matches = compiled_pattern.matches(&text_str);
    Ok(Value::Bool(matches))
}

/// Native function: pattern_find(text, pattern) -> object or null
/// Finds the first match of pattern in text
pub fn pattern_find_native(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("pattern_find", &args, 2)?;

    let text_str = expect_text(&args[0])?;
    let compiled_pattern = expect_pattern(&args[1])?;

    match compiled_pattern.find(&text_str) {
        Some(match_result) => {
            let mut result_map = HashMap::new();
            result_map.insert(
                String::from("matched_text"),
                Value::Text(Arc::from(match_result.matched_text.as_str())),
            );
            result_map.insert(
                String::from("start"),
                Value::Number(match_result.start as f64),
            );
            result_map.insert(String::from("end"), Value::Number(match_result.end as f64));

            // Add captures if any
            if !match_result.captures.is_empty() {
                let mut captures_map = HashMap::new();
                for (name, value) in match_result.captures {
                    captures_map.insert(name, Value::Text(Arc::from(value.as_str())));
                }
                result_map.insert(
                    String::from("captures"),
                    Value::Object(Rc::new(RefCell::new(captures_map))),
                );
            }

            Ok(Value::Object(Rc::new(RefCell::new(result_map))))
        }
        None => Ok(Value::Null),
    }
}

/// Native function: pattern_find_all(text, pattern) -> list
/// Finds all matches of pattern in text
pub fn pattern_find_all_native(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("pattern_find_all", &args, 2)?;

    let text_str = expect_text(&args[0])?;
    let compiled_pattern = expect_pattern(&args[1])?;

    let matches = compiled_pattern.find_all(&text_str);
    let mut result_list = Vec::new();

    for match_result in matches {
        let mut result_map = HashMap::new();
        result_map.insert(
            String::from("matched_text"),
            Value::Text(Arc::from(match_result.matched_text.as_str())),
        );
        result_map.insert(
            String::from("start"),
            Value::Number(match_result.start as f64),
        );
        result_map.insert(String::from("end"), Value::Number(match_result.end as f64));

        // Add captures if any
        if !match_result.captures.is_empty() {
            let mut captures_map = HashMap::new();
            for (name, value) in match_result.captures {
                captures_map.insert(name, Value::Text(Arc::from(value.as_str())));
            }
            result_map.insert(
                String::from("captures"),
                Value::Object(Rc::new(RefCell::new(captures_map))),
            );
        }

        result_list.push(Value::Object(Rc::new(RefCell::new(result_map))));
    }

    Ok(Value::List(Rc::new(RefCell::new(result_list))))
}

/// Native function for pattern replacement (called by interpreter)
pub fn native_pattern_replace(
    args: Vec<Value>,
    line: usize,
    column: usize,
) -> Result<Value, RuntimeError> {
    check_arg_count("pattern_replace", &args, 3)
        .map_err(|e| RuntimeError::new(e.message, line, column))?;

    let text = expect_text(&args[0]).map_err(|e| RuntimeError::new(e.message, line, column))?;

    let _pattern =
        expect_pattern(&args[1]).map_err(|e| RuntimeError::new(e.message, line, column))?;

    let _replacement =
        expect_text(&args[2]).map_err(|e| RuntimeError::new(e.message, line, column))?;

    // TODO: Update to use new pattern system for replacement
    Ok(Value::Text(Arc::clone(&text)))
}

/// Native function for pattern splitting (called by interpreter)
pub fn native_pattern_split(
    args: Vec<Value>,
    line: usize,
    column: usize,
) -> Result<Value, RuntimeError> {
    check_arg_count("pattern_split", &args, 2)
        .map_err(|e| RuntimeError::new(e.message, line, column))?;

    let text = expect_text(&args[0]).map_err(|e| RuntimeError::new(e.message, line, column))?;

    let pattern =
        expect_pattern(&args[1]).map_err(|e| RuntimeError::new(e.message, line, column))?;

    // Find all matches of the pattern in the text
    let matches = pattern.find_all(&text);

    // If no matches, return the entire text as a single element
    if matches.is_empty() {
        let parts = vec![Value::Text(Arc::clone(&text))];
        return Ok(Value::List(Rc::new(RefCell::new(parts))));
    }

    // Extract matched text from match result directly using matched_text
    // Or slice using byte offsets. However, MatchResult currently only has `start` and `end` character indices.
    // In Rust, using `.char_indices()` incrementally is more efficient than building a full vector.
    let mut parts = Vec::new();
    let mut last_end_char = 0;
    let mut last_end_byte = 0;

    let mut char_indices = text.char_indices();
    let mut current_char_idx = 0;

    for match_result in matches {
        // Find start_byte
        let mut start_byte = last_end_byte;
        while current_char_idx < match_result.start {
            if let Some((b_idx, _)) = char_indices.next() {
                start_byte = b_idx;
                current_char_idx += 1;
            } else {
                start_byte = text.len();
                break;
            }
        }
        if current_char_idx == match_result.start {
            // we are at the char, we need the byte index of this char
            start_byte = char_indices.clone().next().map_or(text.len(), |(b, _)| b);
        }

        // Add the text before this match
        if match_result.start > last_end_char
            || (match_result.start == last_end_char && last_end_char == 0)
        {
            let part = &text[last_end_byte..start_byte];
            parts.push(Value::Text(Arc::from(part)));
        } else if match_result.start == last_end_char && last_end_char > 0 {
            // Add empty string for consecutive matches
            parts.push(Value::Text(Arc::from("")));
        }

        // Find last_end_byte
        while current_char_idx < match_result.end {
            if let Some((b_idx, _)) = char_indices.next() {
                last_end_byte = b_idx;
                current_char_idx += 1;
            } else {
                last_end_byte = text.len();
                break;
            }
        }
        if current_char_idx == match_result.end {
            last_end_byte = char_indices.clone().next().map_or(text.len(), |(b, _)| b);
        }

        last_end_char = match_result.end;
    }

    // Add any remaining text after the last match
    if last_end_byte < text.len() {
        let part = &text[last_end_byte..];
        parts.push(Value::Text(Arc::from(part)));
    } else if last_end_byte == text.len() && text.is_empty() {
        // Should not happen, covered by is_empty check above, but for completeness
    }

    Ok(Value::List(Rc::new(RefCell::new(parts))))
}
