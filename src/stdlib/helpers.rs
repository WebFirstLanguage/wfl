use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use std::cell::RefCell;
use std::rc::Rc;

pub fn check_arg_count(args: &[Value], expected: usize, name: &str) -> Result<(), RuntimeError> {
    if args.len() != expected {
        return Err(RuntimeError::new(
            format!(
                "{} expects {} argument{}, got {}",
                name,
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

// Helper for when a function accepts a range of arguments or just a minimum
pub fn check_min_arg_count(args: &[Value], min: usize, name: &str) -> Result<(), RuntimeError> {
    if args.len() < min {
        return Err(RuntimeError::new(
            format!(
                "{} expects at least {} argument{}, got {}",
                name,
                min,
                if min == 1 { "" } else { "s" },
                args.len()
            ),
            0,
            0,
        ));
    }
    Ok(())
}

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

pub fn expect_date(value: &Value) -> Result<Rc<NaiveDate>, RuntimeError> {
    match value {
        Value::Date(d) => Ok(Rc::clone(d)),
        _ => Err(RuntimeError::new(
            format!("Expected a Date, got {}", value.type_name()),
            0,
            0,
        )),
    }
}

pub fn expect_time(value: &Value) -> Result<Rc<NaiveTime>, RuntimeError> {
    match value {
        Value::Time(t) => Ok(Rc::clone(t)),
        _ => Err(RuntimeError::new(
            format!("Expected a Time, got {}", value.type_name()),
            0,
            0,
        )),
    }
}

pub fn expect_datetime(value: &Value) -> Result<Rc<NaiveDateTime>, RuntimeError> {
    match value {
        Value::DateTime(dt) => Ok(Rc::clone(dt)),
        _ => Err(RuntimeError::new(
            format!("Expected a DateTime, got {}", value.type_name()),
            0,
            0,
        )),
    }
}
