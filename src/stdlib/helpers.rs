use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::cell::RefCell;
use std::rc::Rc;

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
