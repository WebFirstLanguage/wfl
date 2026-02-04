use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::cell::RefCell;
use std::rc::Rc;

/// Checks if the number of arguments matches the expected count.
pub fn check_arg_count(
    func_name: &str,
    args: &[Value],
    expected: usize,
) -> Result<(), RuntimeError> {
    if args.len() != expected {
        return Err(RuntimeError::new(
            format!(
                "{} expects {} argument{}, got {}",
                func_name,
                expected,
                if expected == 1 { "" } else { "s" },
                args.len()
            ),
            0,
            0,
        ));
    }
    Ok(())
}

/// Checks if the number of arguments is at least min_count.
pub fn check_min_arg_count(
    func_name: &str,
    args: &[Value],
    min_count: usize,
) -> Result<(), RuntimeError> {
    if args.len() < min_count {
        return Err(RuntimeError::new(
            format!(
                "{} expects at least {} argument{}, got {}",
                func_name,
                min_count,
                if min_count == 1 { "" } else { "s" },
                args.len()
            ),
            0,
            0,
        ));
    }
    Ok(())
}

/// Checks if the number of arguments is within the range [min, max].
pub fn check_arg_range(
    func_name: &str,
    args: &[Value],
    min: usize,
    max: usize,
) -> Result<(), RuntimeError> {
    if args.len() < min || args.len() > max {
        return Err(RuntimeError::new(
            format!(
                "{} expects between {} and {} arguments, got {}",
                func_name,
                min,
                max,
                args.len()
            ),
            0,
            0,
        ));
    }
    Ok(())
}

/// Expects a number value and returns it as f64.
pub fn expect_number(value: &Value) -> Result<f64, RuntimeError> {
    match value {
        Value::Number(n) => Ok(*n),
        _ => Err(RuntimeError::new(
            format!("Expected a number, got {}", value.type_name()),
            0,
            0,
        )),
    }
}

/// Expects a text value and returns it as Rc<str>.
pub fn expect_text(value: &Value) -> Result<Rc<str>, RuntimeError> {
    match value {
        Value::Text(s) => Ok(Rc::clone(s)),
        _ => Err(RuntimeError::new(
            format!("Expected text, got {}", value.type_name()),
            0,
            0,
        )),
    }
}

/// Expects a list value and returns it as Rc<RefCell<Vec<Value>>>.
pub fn expect_list(value: &Value) -> Result<Rc<RefCell<Vec<Value>>>, RuntimeError> {
    match value {
        Value::List(list) => Ok(Rc::clone(list)),
        _ => Err(RuntimeError::new(
            format!("Expected a list, got {}", value.type_name()),
            0,
            0,
        )),
    }
}

/// Expects a boolean value and returns it.
pub fn expect_bool(value: &Value) -> Result<bool, RuntimeError> {
    match value {
        Value::Bool(b) => Ok(*b),
        _ => Err(RuntimeError::new(
            format!("Expected a boolean, got {}", value.type_name()),
            0,
            0,
        )),
    }
}

/// Expects a Date value.
pub fn expect_date(value: &Value) -> Result<Rc<chrono::NaiveDate>, RuntimeError> {
    match value {
        Value::Date(d) => Ok(Rc::clone(d)),
        _ => Err(RuntimeError::new(
            format!("Expected a Date, got {}", value.type_name()),
            0,
            0,
        )),
    }
}

/// Expects a Time value.
pub fn expect_time(value: &Value) -> Result<Rc<chrono::NaiveTime>, RuntimeError> {
    match value {
        Value::Time(t) => Ok(Rc::clone(t)),
        _ => Err(RuntimeError::new(
            format!("Expected a Time, got {}", value.type_name()),
            0,
            0,
        )),
    }
}

/// Expects a DateTime value.
pub fn expect_datetime(value: &Value) -> Result<Rc<chrono::NaiveDateTime>, RuntimeError> {
    match value {
        Value::DateTime(dt) => Ok(Rc::clone(dt)),
        _ => Err(RuntimeError::new(
            format!("Expected a DateTime, got {}", value.type_name()),
            0,
            0,
        )),
    }
}
