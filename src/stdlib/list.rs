use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
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

fn expect_text(value: &Value) -> Result<Rc<str>, RuntimeError> {
    match value {
        Value::Text(s) => Ok(Rc::clone(s)),
        _ => Err(RuntimeError::new(
            format!("Expected text, got {}", value.type_name()),
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

pub fn native_sort_by(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() < 2 || args.len() > 3 {
        return Err(RuntimeError::new(
            format!("sort_by expects 2 or 3 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let list = expect_list(&args[0])?;
    let sort_type = expect_text(&args[1])?;

    let mut items = list.borrow().clone();

    match sort_type.as_ref() {
        "alpha" => {
            items.sort_by(|a, b| {
                let a_str = match a {
                    Value::Text(s) => s.to_string(),
                    _ => format!("{:?}", a),
                };
                let b_str = match b {
                    Value::Text(s) => s.to_string(),
                    _ => format!("{:?}", b),
                };
                a_str.cmp(&b_str)
            });
        }
        "time" => {
            items.sort_by(|a, b| {
                let a_num = match a {
                    Value::Number(n) => *n,
                    _ => 0.0,
                };
                let b_num = match b {
                    Value::Number(n) => *n,
                    _ => 0.0,
                };
                a_num
                    .partial_cmp(&b_num)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        _ => {
            if args.len() == 3 {
                let custom_order_list = expect_list(&args[2])?;
                let custom_order = custom_order_list.borrow();
                let order_map: HashMap<String, usize> = custom_order
                    .iter()
                    .enumerate()
                    .filter_map(|(i, v)| {
                        if let Value::Text(s) = v {
                            Some((s.to_string(), i))
                        } else {
                            None
                        }
                    })
                    .collect();

                items.sort_by(|a, b| {
                    let a_str = match a {
                        Value::Text(s) => s.to_string(),
                        _ => format!("{:?}", a),
                    };
                    let b_str = match b {
                        Value::Text(s) => s.to_string(),
                        _ => format!("{:?}", b),
                    };

                    let a_order = order_map.get(&a_str).copied().unwrap_or(usize::MAX);
                    let b_order = order_map.get(&b_str).copied().unwrap_or(usize::MAX);

                    match (a_order, b_order) {
                        (usize::MAX, usize::MAX) => a_str.cmp(&b_str),
                        (usize::MAX, _) => std::cmp::Ordering::Greater,
                        (_, usize::MAX) => std::cmp::Ordering::Less,
                        _ => a_order.cmp(&b_order),
                    }
                });
            } else {
                return Err(RuntimeError::new(
                    format!("Unknown sort type: {}", sort_type),
                    0,
                    0,
                ));
            }
        }
    }

    Ok(Value::List(Rc::new(RefCell::new(items))))
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
    env.define("sort_by", Value::NativeFunction("sort_by", native_sort_by));

    env.define(
        "index_of",
        Value::NativeFunction("index_of", native_indexof),
    );
}
