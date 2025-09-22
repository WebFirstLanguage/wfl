use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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
    if args.len() != 2 {
        return Err(RuntimeError::new(
            "pattern_matches requires exactly 2 arguments (text, pattern)".to_string(),
            0,
            0,
        ));
    }

    let text_str = match &args[0] {
        Value::Text(s) => s.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                "First argument to pattern_matches must be text".to_string(),
                0,
                0,
            ));
        }
    };

    let compiled_pattern = match &args[1] {
        Value::Pattern(p) => p,
        _ => {
            return Err(RuntimeError::new(
                "Second argument to pattern_matches must be a compiled pattern".to_string(),
                0,
                0,
            ));
        }
    };

    let matches = compiled_pattern.matches(text_str);
    Ok(Value::Bool(matches))
}

/// Native function: pattern_find(text, pattern) -> object or null
/// Finds the first match of pattern in text
pub fn pattern_find_native(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            "pattern_find requires exactly 2 arguments (text, pattern)".to_string(),
            0,
            0,
        ));
    }

    let text_str = match &args[0] {
        Value::Text(s) => s.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                "First argument to pattern_find must be text".to_string(),
                0,
                0,
            ));
        }
    };

    let compiled_pattern = match &args[1] {
        Value::Pattern(p) => p,
        _ => {
            return Err(RuntimeError::new(
                "Second argument to pattern_find must be a compiled pattern".to_string(),
                0,
                0,
            ));
        }
    };

    match compiled_pattern.find(text_str) {
        Some(match_result) => {
            let mut result_map = HashMap::new();
            result_map.insert(
                "matched_text".to_string(),
                Value::Text(Rc::from(match_result.matched_text.as_str())),
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
                    captures_map.insert(name, Value::Text(Rc::from(value.as_str())));
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
    if args.len() != 2 {
        return Err(RuntimeError::new(
            "pattern_find_all requires exactly 2 arguments (text, pattern)".to_string(),
            0,
            0,
        ));
    }

    let text_str = match &args[0] {
        Value::Text(s) => s.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                "First argument to pattern_find_all must be text".to_string(),
                0,
                0,
            ));
        }
    };

    let compiled_pattern = match &args[1] {
        Value::Pattern(p) => p,
        _ => {
            return Err(RuntimeError::new(
                "Second argument to pattern_find_all must be a compiled pattern".to_string(),
                0,
                0,
            ));
        }
    };

    let matches = compiled_pattern.find_all(text_str);
    let mut result_list = Vec::new();

    for match_result in matches {
        let mut result_map = HashMap::new();
        result_map.insert(
            "matched_text".to_string(),
            Value::Text(Rc::from(match_result.matched_text.as_str())),
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
                captures_map.insert(name, Value::Text(Rc::from(value.as_str())));
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

    let text = match &args[0] {
        Value::Text(t) => t.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                "First argument must be text".to_string(),
                line,
                column,
            ));
        }
    };

    let _pattern = match &args[1] {
        Value::Pattern(p) => p.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                "Second argument must be a pattern".to_string(),
                line,
                column,
            ));
        }
    };

    let _replacement = match &args[2] {
        Value::Text(t) => t.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                "Third argument must be text".to_string(),
                line,
                column,
            ));
        }
    };

    // TODO: Update to use new pattern system for replacement
    Ok(Value::Text(Rc::from(text)))
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

    let text = match &args[0] {
        Value::Text(t) => t.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                "First argument must be text".to_string(),
                line,
                column,
            ));
        }
    };

    let pattern = match &args[1] {
        Value::Pattern(p) => p,
        _ => {
            return Err(RuntimeError::new(
                "Second argument must be a pattern".to_string(),
                line,
                column,
            ));
        }
    };

    // Find all matches of the pattern in the text
    let matches = pattern.find_all(text);

    // If no matches, return the entire text as a single element
    if matches.is_empty() {
        let parts = vec![Value::Text(Rc::from(text))];
        return Ok(Value::List(Rc::new(RefCell::new(parts))));
    }

    // Build character-to-byte index mapping
    let char_to_byte: Vec<usize> = text.char_indices().map(|(byte_idx, _)| byte_idx).collect();
    let mut char_to_byte = char_to_byte;
    char_to_byte.push(text.len()); // Add final byte position

    // Split the text at match positions
    let mut parts = Vec::new();
    let mut last_end_char = 0;

    for match_result in matches {
        // Convert character indices to byte indices
        let start_byte = if match_result.start < char_to_byte.len() {
            char_to_byte[match_result.start]
        } else {
            text.len()
        };
        let last_end_byte = if last_end_char < char_to_byte.len() {
            char_to_byte[last_end_char]
        } else {
            text.len()
        };

        // Add the text before this match
        if match_result.start > last_end_char
            || (match_result.start == last_end_char && last_end_char == 0)
        {
            let part = &text[last_end_byte..start_byte];
            parts.push(Value::Text(Rc::from(part)));
        } else if match_result.start == last_end_char && last_end_char > 0 {
            // Add empty string for consecutive matches
            parts.push(Value::Text(Rc::from("")));
        }
        last_end_char = match_result.end;
    }

    // Add any remaining text after the last match
    if last_end_char < char_to_byte.len() {
        let last_end_byte = char_to_byte[last_end_char];
        let part = &text[last_end_byte..];
        parts.push(Value::Text(Rc::from(part)));
    }

    Ok(Value::List(Rc::new(RefCell::new(parts))))
}
