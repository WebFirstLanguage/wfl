//! TOML parsing and serialization (issue #667).
//!
//! Deliberately a mirror of [`crate::stdlib::json`]: the same three-function
//! shape (`parse_*`, `stringify_*`, `stringify_*_pretty`), the same `Value`
//! mapping, and the same error style, so a program that reads one format reads
//! the other the same way.
//!
//! Two places where TOML is not JSON, and what this module does about them:
//!
//! * **A TOML document is always a table.** There is no such thing as a TOML
//!   file whose top level is an array or a bare string, so `stringify_toml`
//!   accepts only an object and says so plainly rather than emitting something
//!   that will not parse back.
//! * **TOML has no null.** Absence is expressed by leaving the key out, so a
//!   `nothing` value is skipped when serializing a table. Inside an *array*
//!   there is no way to leave a hole, so that is an error instead of a silent
//!   change of length.

use super::helpers::{check_arg_count, expect_text};
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// Convert a `toml::Value` to a WFL `Value`.
///
/// Dates and times become text: WFL has `Date`/`Time`/`DateTime` values, but
/// TOML's offset/local datetime distinction does not map cleanly onto them, and
/// a lossless string keeps the round trip honest.
fn toml_to_wfl(value: ::toml::Value) -> Value {
    match value {
        ::toml::Value::String(s) => Value::Text(Arc::from(s)),
        ::toml::Value::Integer(i) => Value::Number(i as f64),
        ::toml::Value::Float(f) => Value::Number(f),
        ::toml::Value::Boolean(b) => Value::Bool(b),
        ::toml::Value::Datetime(dt) => Value::Text(Arc::from(dt.to_string())),
        ::toml::Value::Array(arr) => {
            let items: Vec<Value> = arr.into_iter().map(toml_to_wfl).collect();
            Value::List(Rc::new(RefCell::new(items)))
        }
        ::toml::Value::Table(table) => {
            let mut map = HashMap::new();
            for (key, value) in table {
                map.insert(key, toml_to_wfl(value));
            }
            Value::Object(Rc::new(RefCell::new(map)))
        }
    }
}

/// Convert a WFL `Value` to a `toml::Value`.
///
/// Returns `Ok(None)` for `nothing`/`null`, which the table branch reads as
/// "omit this key". Callers that cannot omit (arrays, the document root) turn
/// that into an error.
fn wfl_to_toml(value: &Value) -> Result<Option<::toml::Value>, RuntimeError> {
    match value {
        Value::Nothing | Value::Null => Ok(None),
        Value::Bool(b) => Ok(Some(::toml::Value::Boolean(*b))),
        Value::Number(n) => {
            if !n.is_finite() {
                return Err(RuntimeError::new(
                    format!("Cannot convert number {n} to TOML: TOML has no infinity or NaN"),
                    0,
                    0,
                ));
            }
            // Whole numbers become TOML integers so a config round-trips as
            // `port = 8080` rather than `port = 8080.0`.
            if n.fract() == 0.0 && *n >= i64::MIN as f64 && *n <= i64::MAX as f64 {
                Ok(Some(::toml::Value::Integer(*n as i64)))
            } else {
                Ok(Some(::toml::Value::Float(*n)))
            }
        }
        Value::Text(s) => Ok(Some(::toml::Value::String(s.to_string()))),
        Value::List(list) => {
            let list_ref = list.borrow();
            let mut items = Vec::with_capacity(list_ref.len());
            for item in list_ref.iter() {
                match wfl_to_toml(item)? {
                    Some(v) => items.push(v),
                    None => {
                        return Err(RuntimeError::new(
                            "Cannot convert list to TOML: TOML arrays cannot contain nothing. \
                             Remove the empty entry, or use a table where the key can be omitted."
                                .to_string(),
                            0,
                            0,
                        ));
                    }
                }
            }
            Ok(Some(::toml::Value::Array(items)))
        }
        Value::Object(obj) => {
            let obj_ref = obj.borrow();
            let mut table = ::toml::map::Map::new();
            for (key, value) in obj_ref.iter() {
                // `nothing` means "absent"; TOML spells that as a missing key.
                if let Some(converted) = wfl_to_toml(value)? {
                    table.insert(key.clone(), converted);
                }
            }
            Ok(Some(::toml::Value::Table(table)))
        }
        other => Err(RuntimeError::new(
            format!("Cannot convert {} to TOML", other.type_name()),
            0,
            0,
        )),
    }
}

/// Convert a WFL value into a TOML document root, which must be a table.
fn wfl_to_toml_document(value: &Value) -> Result<::toml::Table, RuntimeError> {
    match wfl_to_toml(value)? {
        Some(::toml::Value::Table(table)) => Ok(table),
        _ => Err(RuntimeError::new(
            format!(
                "Cannot convert {} to a TOML document: a TOML document is always a table, \
                 so the top level must be an object with named keys",
                value.type_name()
            ),
            0,
            0,
        )),
    }
}

/// Parse TOML text into a WFL value.
///
/// Usage: `parse_toml of text`
pub fn native_parse_toml(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("parse_toml", &args, 1)?;

    let toml_text = expect_text(&args[0])?;

    match toml_text.parse::<::toml::Table>() {
        Ok(table) => Ok(toml_to_wfl(::toml::Value::Table(table))),
        Err(e) => Err(RuntimeError::new(
            format!("Failed to parse TOML: {e}"),
            0,
            0,
        )),
    }
}

/// Convert a WFL value to TOML text.
///
/// Usage: `stringify_toml of value`
pub fn native_stringify_toml(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("stringify_toml", &args, 1)?;

    let table = wfl_to_toml_document(&args[0])?;

    match ::toml::to_string(&table) {
        Ok(text) => Ok(Value::Text(Arc::from(text))),
        Err(e) => Err(RuntimeError::new(
            format!("Failed to write TOML: {e}"),
            0,
            0,
        )),
    }
}

/// Convert a WFL value to pretty-printed TOML text.
///
/// Usage: `stringify_toml_pretty of value`
pub fn native_stringify_toml_pretty(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("stringify_toml_pretty", &args, 1)?;

    let table = wfl_to_toml_document(&args[0])?;

    match ::toml::to_string_pretty(&table) {
        Ok(text) => Ok(Value::Text(Arc::from(text))),
        Err(e) => Err(RuntimeError::new(
            format!("Failed to write TOML: {e}"),
            0,
            0,
        )),
    }
}

/// Register all TOML functions in the environment.
pub fn register_toml(env: &mut Environment) {
    env.define_native("parse_toml", native_parse_toml);
    env.define_native("stringify_toml", native_stringify_toml);
    env.define_native("stringify_toml_pretty", native_stringify_toml_pretty);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Value {
        Value::Text(Arc::from(s))
    }

    #[test]
    fn parse_toml_maps_scalars_onto_wfl_values() {
        let parsed = native_parse_toml(vec![text(
            "name = \"wfl\"\nport = 8080\nratio = 0.5\ndebug = true\n",
        )])
        .expect("valid TOML should parse");

        let Value::Object(obj) = parsed else {
            panic!("a TOML document should parse to an object");
        };
        let obj = obj.borrow();
        assert!(matches!(obj.get("name"), Some(Value::Text(_))));
        assert!(matches!(obj.get("port"), Some(Value::Number(n)) if *n == 8080.0));
        assert!(matches!(obj.get("ratio"), Some(Value::Number(n)) if *n == 0.5));
        assert!(matches!(obj.get("debug"), Some(Value::Bool(true))));
    }

    #[test]
    fn parse_toml_rejects_malformed_input() {
        assert!(native_parse_toml(vec![text("[unclosed")]).is_err());
        assert!(native_parse_toml(vec![text("key = ")]).is_err());
        // Duplicate keys are an error in TOML, not last-wins.
        assert!(native_parse_toml(vec![text("a = 1\na = 2\n")]).is_err());
    }

    #[test]
    fn whole_numbers_round_trip_as_integers() {
        let parsed = native_parse_toml(vec![text("port = 8080\n")]).unwrap();
        let out = native_stringify_toml(vec![parsed]).unwrap();
        let Value::Text(out) = out else {
            panic!("expected text");
        };
        assert!(
            out.contains("port = 8080") && !out.contains("8080.0"),
            "a whole number should stay an integer, got: {out}"
        );
    }

    #[test]
    fn nothing_valued_keys_are_omitted_from_tables() {
        let mut map = HashMap::new();
        map.insert("kept".to_string(), text("yes"));
        map.insert("dropped".to_string(), Value::Nothing);
        let value = Value::Object(Rc::new(RefCell::new(map)));

        let Value::Text(out) = native_stringify_toml(vec![value]).unwrap() else {
            panic!("expected text");
        };
        assert!(out.contains("kept"), "present keys must survive: {out}");
        assert!(
            !out.contains("dropped"),
            "a nothing-valued key has no TOML spelling and must be omitted: {out}"
        );
    }

    #[test]
    fn nothing_inside_an_array_is_an_error_not_a_silent_drop() {
        let list = Value::List(Rc::new(RefCell::new(vec![text("a"), Value::Nothing])));
        let mut map = HashMap::new();
        map.insert("items".to_string(), list);
        let value = Value::Object(Rc::new(RefCell::new(map)));

        let err = native_stringify_toml(vec![value])
            .expect_err("dropping an array element would change its length");
        assert!(err.message.contains("nothing"), "got: {}", err.message);
    }

    #[test]
    fn a_top_level_list_is_not_a_toml_document() {
        let list = Value::List(Rc::new(RefCell::new(vec![text("a")])));
        let err = native_stringify_toml(vec![list]).expect_err("TOML documents are tables");
        assert!(err.message.contains("table"), "got: {}", err.message);
    }

    #[test]
    fn round_trip_preserves_nested_structure() {
        let src = "[server]\nhost = \"localhost\"\nports = [1, 2]\n";
        let parsed = native_parse_toml(vec![text(src)]).unwrap();
        let Value::Text(out) = native_stringify_toml_pretty(vec![parsed]).unwrap() else {
            panic!("expected text");
        };
        let reparsed = native_parse_toml(vec![Value::Text(out)]).unwrap();

        let Value::Object(obj) = reparsed else {
            panic!("expected object");
        };
        let server = obj.borrow().get("server").cloned().expect("server table");
        let Value::Object(server) = server else {
            panic!("expected nested table");
        };
        assert!(matches!(server.borrow().get("host"), Some(Value::Text(_))));
    }

    #[test]
    fn non_finite_numbers_are_rejected() {
        let mut map = HashMap::new();
        map.insert("x".to_string(), Value::Number(f64::INFINITY));
        let value = Value::Object(Rc::new(RefCell::new(map)));
        assert!(native_stringify_toml(vec![value]).is_err());
    }
}
