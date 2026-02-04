use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use crate::stdlib::helpers::expect_text;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Convert serde_json::Value to WFL Value
fn json_to_wfl(json: serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Nothing,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_f64() {
                Value::Number(i)
            } else {
                Value::Nothing
            }
        }
        serde_json::Value::String(s) => Value::Text(Rc::from(s)),
        serde_json::Value::Array(arr) => {
            let wfl_list: Vec<Value> = arr.into_iter().map(json_to_wfl).collect();
            Value::List(Rc::new(RefCell::new(wfl_list)))
        }
        serde_json::Value::Object(obj) => {
            let mut wfl_map = HashMap::new();
            for (key, value) in obj {
                wfl_map.insert(key, json_to_wfl(value));
            }
            Value::Object(Rc::new(RefCell::new(wfl_map)))
        }
    }
}

/// Convert WFL Value to serde_json::Value
fn wfl_to_json(value: &Value) -> Result<serde_json::Value, RuntimeError> {
    match value {
        Value::Nothing => Ok(serde_json::Value::Null),
        Value::Null => Ok(serde_json::Value::Null),
        Value::Bool(b) => Ok(serde_json::Value::Bool(*b)),
        Value::Number(n) => {
            if let Some(num) = serde_json::Number::from_f64(*n) {
                Ok(serde_json::Value::Number(num))
            } else {
                Err(RuntimeError::new(
                    format!("Cannot convert number {} to JSON", n),
                    0,
                    0,
                ))
            }
        }
        Value::Text(s) => Ok(serde_json::Value::String(s.to_string())),
        Value::List(list) => {
            let list_ref = list.borrow();
            let json_arr: Result<Vec<serde_json::Value>, RuntimeError> =
                list_ref.iter().map(wfl_to_json).collect();
            Ok(serde_json::Value::Array(json_arr?))
        }
        Value::Object(obj) => {
            let obj_ref = obj.borrow();
            let mut json_obj = serde_json::Map::new();
            for (key, value) in obj_ref.iter() {
                json_obj.insert(key.clone(), wfl_to_json(value)?);
            }
            Ok(serde_json::Value::Object(json_obj))
        }
        _ => Err(RuntimeError::new(
            format!("Cannot convert {} to JSON", value.type_name()),
            0,
            0,
        )),
    }
}

/// Parse JSON string to WFL value
/// Usage: parse_json(json_text)
pub fn native_parse_json(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("parse_json expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let json_text = expect_text(&args[0])?;

    match serde_json::from_str::<serde_json::Value>(&json_text) {
        Ok(json) => Ok(json_to_wfl(json)),
        Err(e) => Err(RuntimeError::new(
            format!("Failed to parse JSON: {}", e),
            0,
            0,
        )),
    }
}

/// Convert WFL value to JSON string
/// Usage: stringify_json(value)
pub fn native_stringify_json(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("stringify_json expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let json_value = wfl_to_json(&args[0])?;

    match serde_json::to_string(&json_value) {
        Ok(json_str) => Ok(Value::Text(Rc::from(json_str))),
        Err(e) => Err(RuntimeError::new(
            format!("Failed to stringify JSON: {}", e),
            0,
            0,
        )),
    }
}

/// Convert WFL value to pretty-printed JSON string
/// Usage: stringify_json_pretty(value)
pub fn native_stringify_json_pretty(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!(
                "stringify_json_pretty expects 1 argument, got {}",
                args.len()
            ),
            0,
            0,
        ));
    }

    let json_value = wfl_to_json(&args[0])?;

    match serde_json::to_string_pretty(&json_value) {
        Ok(json_str) => Ok(Value::Text(Rc::from(json_str))),
        Err(e) => Err(RuntimeError::new(
            format!("Failed to stringify JSON: {}", e),
            0,
            0,
        )),
    }
}

/// Register all JSON functions in the environment
pub fn register_json(env: &mut Environment) {
    let _ = env.define(
        "parse_json",
        Value::NativeFunction("parse_json", native_parse_json),
    );
    let _ = env.define(
        "stringify_json",
        Value::NativeFunction("stringify_json", native_stringify_json),
    );
    let _ = env.define(
        "stringify_json_pretty",
        Value::NativeFunction("stringify_json_pretty", native_stringify_json_pretty),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_object() {
        let json = r#"{"name": "Alice", "age": 30, "active": true}"#;
        let result = native_parse_json(vec![Value::Text(Rc::from(json))]);
        assert!(result.is_ok());

        if let Ok(Value::Object(obj)) = result {
            let obj_ref = obj.borrow();
            assert_eq!(obj_ref.len(), 3);
            assert!(matches!(obj_ref.get("name"), Some(Value::Text(_))));
            assert!(matches!(obj_ref.get("age"), Some(Value::Number(_))));
            assert!(matches!(obj_ref.get("active"), Some(Value::Bool(true))));
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_parse_json_array() {
        let json = r#"[1, 2, 3, "test"]"#;
        let result = native_parse_json(vec![Value::Text(Rc::from(json))]);
        assert!(result.is_ok());

        if let Ok(Value::List(list)) = result {
            let list_ref = list.borrow();
            assert_eq!(list_ref.len(), 4);
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_stringify_json() {
        let mut obj = HashMap::new();
        obj.insert("name".to_string(), Value::Text(Rc::from("Bob")));
        obj.insert("age".to_string(), Value::Number(25.0));
        obj.insert("active".to_string(), Value::Bool(false));

        let wfl_obj = Value::Object(Rc::new(RefCell::new(obj)));
        let result = native_stringify_json(vec![wfl_obj]);
        assert!(result.is_ok());

        if let Ok(Value::Text(json_str)) = result {
            // Parse back to verify it's valid JSON
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json_str);
            assert!(parsed.is_ok());
        } else {
            panic!("Expected text");
        }
    }

    #[test]
    fn test_parse_invalid_json() {
        let json = r#"{"invalid": }"#;
        let result = native_parse_json(vec![Value::Text(Rc::from(json))]);
        assert!(result.is_err());
    }

    #[test]
    fn test_round_trip() {
        let json = r#"{"user": {"name": "Charlie", "scores": [95, 87, 91]}, "verified": true}"#;

        // Parse JSON
        let parsed = native_parse_json(vec![Value::Text(Rc::from(json))]).unwrap();

        // Stringify back
        let stringified = native_stringify_json(vec![parsed]).unwrap();

        // Parse again to verify
        if let Value::Text(json_str) = stringified {
            let reparsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
            assert!(reparsed.is_object());
        } else {
            panic!("Expected text");
        }
    }
}
