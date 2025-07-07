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

    let list = expect_list(&args[0])?;
    Ok(Value::Number(list.borrow().len() as f64))
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

    let list = expect_list(&args[0])?;
    let item = &args[1];

    for value in list.borrow().iter() {
        if format!("{:?}", value) == format!("{:?}", item) {
            return Ok(Value::Bool(true));
        }
    }

    Ok(Value::Bool(false))
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
        if format!("{:?}", value) == format!("{:?}", item) {
            return Ok(Value::Number(i as f64));
        }
    }

    Ok(Value::Number(-1.0))
}

pub fn register_list(env: &mut Environment) {
    env.define("length", Value::NativeFunction("length", native_length));
    env.define("push", Value::NativeFunction("push", native_push));
    env.define("pop", Value::NativeFunction("pop", native_pop));
    env.define(
        "contains",
        Value::NativeFunction("contains", native_contains),
    );
    env.define("indexof", Value::NativeFunction("indexof", native_indexof));

    env.define(
        "index_of",
        Value::NativeFunction("index_of", native_indexof),
    );
}
