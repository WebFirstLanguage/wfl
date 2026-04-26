use super::helpers::{check_arg_count, expect_pattern, expect_text};
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

pub fn register(env: &mut Environment) {
    // Register new pattern functions that work with our pattern system
    env.define_native("pattern_matches", pattern_matches_native);
    env.define_native("pattern_find", pattern_find_native);
    env.define_native("pattern_find_all", pattern_find_all_native);
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

    let matches = compiled_pattern.matches(text_str.as_ref());
    Ok(Value::Bool(matches))
}

/// Native function: pattern_find(text, pattern) -> object or null
/// Finds the first match of pattern in text
pub fn pattern_find_native(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("pattern_find", &args, 2)?;
    let text_str = expect_text(&args[0])?;
    let compiled_pattern = expect_pattern(&args[1])?;

    match compiled_pattern.find(text_str.as_ref()) {
        Some(match_result) => {
            let mut result_map = HashMap::new();
            result_map.insert(
                "matched_text".to_string(),
                Value::Text(Arc::from(match_result.matched_text.as_str())),
            );
            result_map.insert(
                "start".to_string(),
                Value::Number(match_result.start as f64),
            );
            result_map.insert("end".to_string(), Value::Number(match_result.end as f64));

            // Add captures if any
            if !match_result.captures.is_empty() {
                let mut captures_map = HashMap::new();
                for (name, value) in match_result.captures {
                    captures_map.insert(name, Value::Text(Arc::from(value.as_str())));
                }
                result_map.insert(
                    "captures".to_string(),
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

    let matches = compiled_pattern.find_all(text_str.as_ref());
    let mut result_list = Vec::new();

    for match_result in matches {
        let mut result_map = HashMap::new();
        result_map.insert(
            "matched_text".to_string(),
            Value::Text(Arc::from(match_result.matched_text.as_str())),
        );
        result_map.insert(
            "start".to_string(),
            Value::Number(match_result.start as f64),
        );
        result_map.insert("end".to_string(), Value::Number(match_result.end as f64));

        // Add captures if any
        if !match_result.captures.is_empty() {
            let mut captures_map = HashMap::new();
            for (name, value) in match_result.captures {
                captures_map.insert(name, Value::Text(Arc::from(value.as_str())));
            }
            result_map.insert(
                "captures".to_string(),
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
    if args.len() != 3 {
        return Err(RuntimeError::new(
            "pattern_replace requires exactly 3 arguments".to_string(),
            line,
            column,
        ));
    }

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
    if args.len() != 2 {
        return Err(RuntimeError::new(
            "pattern_split requires exactly 2 arguments".to_string(),
            line,
            column,
        ));
    }

    let text = expect_text(&args[0]).map_err(|e| RuntimeError::new(e.message, line, column))?;
    let pattern =
        expect_pattern(&args[1]).map_err(|e| RuntimeError::new(e.message, line, column))?;

    // Find all matches of the pattern in the text
    let matches = pattern.find_all(text.as_ref());

    // If no matches, return the entire text as a single element
    if matches.is_empty() {
        let parts = vec![Value::Text(Arc::clone(&text))];
        return Ok(Value::List(Rc::new(RefCell::new(parts))));
    }

    // Build character-to-byte index mapping
    let char_to_byte: Vec<usize> = text
        .as_ref()
        .char_indices()
        .map(|(byte_idx, _)| byte_idx)
        .collect();
    let mut char_to_byte = char_to_byte;
    char_to_byte.push(text.as_ref().len()); // Add final byte position

    // Split the text at match positions
    let mut parts = Vec::new();
    let mut last_end_char = 0;

    for match_result in matches {
        // Convert character indices to byte indices
        let start_byte = if match_result.start < char_to_byte.len() {
            char_to_byte[match_result.start]
        } else {
            text.as_ref().len()
        };
        let last_end_byte = if last_end_char < char_to_byte.len() {
            char_to_byte[last_end_char]
        } else {
            text.as_ref().len()
        };

        // Add the text before this match
        if match_result.start > last_end_char
            || (match_result.start == last_end_char && last_end_char == 0)
        {
            let part = &text.as_ref()[last_end_byte..start_byte];
            parts.push(Value::Text(Arc::from(part)));
        } else if match_result.start == last_end_char && last_end_char > 0 {
            // Add empty string for consecutive matches
            parts.push(Value::Text(Arc::from("")));
        }
        last_end_char = match_result.end;
    }

    // Add any remaining text after the last match
    if last_end_char < char_to_byte.len() {
        let last_end_byte = char_to_byte[last_end_char];
        let part = &text.as_ref()[last_end_byte..];
        parts.push(Value::Text(Arc::from(part)));
    }

    Ok(Value::List(Rc::new(RefCell::new(parts))))
}
