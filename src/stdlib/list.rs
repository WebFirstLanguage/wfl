use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::cell::RefCell;
use std::rc::Rc;

fn expect_list(value: &Value) -> Result<Rc<RefCell<Vec<Value>>>, RuntimeError> {
    match value {
        Value::List(list) => Ok(Rc::clone(list)),
        _ => Err(RuntimeError::new(
            format!("Expected a list, got {}", value.type_name()),
            0,
            0,
        )),
    }
}

#[allow(dead_code)]
fn expect_number(value: &Value) -> Result<f64, RuntimeError> {
    match value {
        Value::Number(n) => Ok(*n),
        _ => Err(RuntimeError::new(
            format!("Expected a number, got {}", value.type_name()),
            0,
            0,
        )),
    }
}

pub fn native_length(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("length expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    match &args[0] {
        Value::List(list) => Ok(Value::Number(list.borrow().len() as f64)),
        Value::Text(text) => Ok(Value::Number(text.len() as f64)),
        _ => Err(RuntimeError::new(
            format!("length expects a list or text, got {}", args[0].type_name()),
            0,
            0,
        )),
    }
}

pub fn native_push(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("push expects 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let list = expect_list(&args[0])?;
    let item = args[1].clone();

    list.borrow_mut().push(item);
    Ok(Value::Null)
}

pub fn native_pop(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("pop expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let list = expect_list(&args[0])?;
    let mut list_ref = list.borrow_mut();

    if list_ref.is_empty() {
        return Err(RuntimeError::new(
            "Cannot pop from an empty list".to_string(),
            0,
            0,
        ));
    }

    Ok(list_ref.pop().unwrap())
}

pub fn native_contains(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("contains expects 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    match &args[0] {
        Value::List(list) => {
            let item = &args[1];
            for value in list.borrow().iter() {
                if value == item {
                    return Ok(Value::Bool(true));
                }
            }
            Ok(Value::Bool(false))
        }
        Value::Text(text) => match &args[1] {
            Value::Text(substring) => Ok(Value::Bool(text.contains(substring.as_ref()))),
            _ => Err(RuntimeError::new(
                format!(
                    "contains on text expects a text argument, got {}",
                    args[1].type_name()
                ),
                0,
                0,
            )),
        },
        _ => Err(RuntimeError::new(
            format!(
                "contains expects a list or text, got {}",
                args[0].type_name()
            ),
            0,
            0,
        )),
    }
}

pub fn native_indexof(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("indexof expects 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let list = expect_list(&args[0])?;
    let item = &args[1];

    for (i, value) in list.borrow().iter().enumerate() {
        if value == item {
            return Ok(Value::Number(i as f64));
        }
    }

    Ok(Value::Number(-1.0))
}

pub fn register_list(env: &mut Environment) {
    let _ = env.define("length", Value::NativeFunction("length", native_length));
    let _ = env.define("push", Value::NativeFunction("push", native_push));
    let _ = env.define("pop", Value::NativeFunction("pop", native_pop));
    let _ = env.define(
        "contains",
        Value::NativeFunction("contains", native_contains),
    );
    let _ = env.define("indexof", Value::NativeFunction("indexof", native_indexof));

    let _ = env.define(
        "index_of",
        Value::NativeFunction("index_of", native_indexof),
    );
}
