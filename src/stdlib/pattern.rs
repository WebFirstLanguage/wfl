use crate::exec::budget::ExecutionBudget;
use crate::interpreter::environment::Environment;
use crate::interpreter::error::{ErrorKind, RuntimeError};
use crate::interpreter::value::Value;
use crate::pattern::PatternError;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// Map a pattern-VM error to a `RuntimeError`. A budget breach (ReDoS
/// step/state ceiling or cancellation) becomes a catchable `ResourceLimit`
/// error rather than a silent empty result; other pattern errors are general.
fn pattern_err(err: PatternError) -> RuntimeError {
    let kind = match err {
        // A pattern that outruns the wall-clock deadline is a timeout.
        PatternError::Timeout { .. } => ErrorKind::Timeout,
        PatternError::StepLimitExceeded
        | PatternError::StateLimitExceeded
        | PatternError::Cancelled => ErrorKind::ResourceLimit,
        _ => ErrorKind::General,
    };
    RuntimeError::with_kind(err.to_string(), 0, 0, kind)
}

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

    let budget = ExecutionBudget::current_or_default();
    let matches = compiled_pattern
        .matches_with_budget(text_str, &budget)
        .map_err(pattern_err)?;
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

    let budget = ExecutionBudget::current_or_default();
    match compiled_pattern
        .find_with_budget(text_str, &budget)
        .map_err(pattern_err)?
    {
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

    let budget = ExecutionBudget::current_or_default();
    let matches = compiled_pattern
        .find_all_with_budget(text_str, &budget)
        .map_err(pattern_err)?;
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
    Ok(Value::Text(Arc::from(text)))
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
    let budget = ExecutionBudget::current_or_default();
    let matches = pattern
        .find_all_with_budget(text, &budget)
        .map_err(pattern_err)?;

    // If no matches, return the entire text as a single element
    if matches.is_empty() {
        let parts = vec![Value::Text(Arc::from(text))];
        return Ok(Value::List(Rc::new(RefCell::new(parts))));
    }

    // Split the text at match positions without an O(N) allocation
    let mut parts = Vec::new();
    let mut last_end_char = 0;

    let mut chars = text.chars();
    let mut current_char_idx = 0;
    let mut current_byte_idx = 0;

    for match_result in matches {
        while current_char_idx < last_end_char {
            if let Some(c) = chars.next() {
                current_byte_idx += c.len_utf8();
                current_char_idx += 1;
            } else {
                break;
            }
        }
        let last_end_byte = current_byte_idx;

        while current_char_idx < match_result.start {
            if let Some(c) = chars.next() {
                current_byte_idx += c.len_utf8();
                current_char_idx += 1;
            } else {
                break;
            }
        }
        let start_byte = current_byte_idx;

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
        last_end_char = match_result.end;
    }

    // Advance to the end of the last match
    while current_char_idx < last_end_char {
        if let Some(c) = chars.next() {
            current_byte_idx += c.len_utf8();
            current_char_idx += 1;
        } else {
            break;
        }
    }
    let final_last_end_byte = current_byte_idx;

    // Add any remaining text after the last match
    // Unconditionally push the remainder, similar to how String::split works when a match is at the end
    if final_last_end_byte <= text.len() {
        let part = &text[final_last_end_byte..];
        parts.push(Value::Text(Arc::from(part)));
    }

    Ok(Value::List(Rc::new(RefCell::new(parts))))
}
