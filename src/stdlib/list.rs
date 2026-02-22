use super::helpers::{
    binary_list_action, check_arg_count, expect_list, expect_number, expect_text,
    unary_list_action, unary_list_op,
};
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

pub fn native_length(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("length", &args, 1)?;

    match &args[0] {
        Value::List(list) => Ok(Value::Number(list.borrow().len() as f64)),
        Value::Text(text) => Ok(Value::Number(text.chars().count() as f64)),
        _ => Err(RuntimeError::new(
            format!("length expects a list or text, got {}", args[0].type_name()),
            0,
            0,
        )),
    }
}

pub fn native_push(args: Vec<Value>) -> Result<Value, RuntimeError> {
    binary_list_action("push", args, |list, item| list.push(item))
}

pub fn native_pop(args: Vec<Value>) -> Result<Value, RuntimeError> {
    unary_list_op("pop", args, |list| {
        if list.is_empty() {
            return Err(RuntimeError::new(
                "Cannot pop from an empty list".to_string(),
                0,
                0,
            ));
        }
        Ok(list.pop().unwrap())
    })
}

pub fn native_contains(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("contains", &args, 2)?;

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
    check_arg_count("indexof", &args, 2)?;

    match &args[0] {
        Value::List(list) => {
            let item = &args[1];
            for (i, value) in list.borrow().iter().enumerate() {
                if value == item {
                    return Ok(Value::Number(i as f64));
                }
            }
            Ok(Value::Number(-1.0))
        }
        Value::Text(text) => {
            let needle = match &args[1] {
                Value::Text(s) => s,
                _ => {
                    return Err(RuntimeError::new(
                        format!(
                            "indexof on text expects a text argument, got {}",
                            args[1].type_name()
                        ),
                        0,
                        0,
                    ));
                }
            };
            match text.find(needle.as_ref()) {
                Some(byte_pos) => {
                    let char_index = text[..byte_pos].chars().count();
                    Ok(Value::Number(char_index as f64))
                }
                None => Ok(Value::Number(-1.0)),
            }
        }
        _ => Err(RuntimeError::new(
            format!(
                "indexof expects a list or text, got {}",
                args[0].type_name()
            ),
            0,
            0,
        )),
    }
}

// --- Batch 3: Basic List Manipulation ---

pub fn native_shift(args: Vec<Value>) -> Result<Value, RuntimeError> {
    unary_list_op("shift", args, |list| {
        if list.is_empty() {
            return Err(RuntimeError::new(
                "Cannot shift from an empty list".to_string(),
                0,
                0,
            ));
        }
        Ok(list.remove(0))
    })
}

pub fn native_unshift(args: Vec<Value>) -> Result<Value, RuntimeError> {
    binary_list_action("unshift", args, |list, item| list.insert(0, item))
}

pub fn native_remove_at(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("remove_at", &args, 2)?;
    let list = expect_list(&args[0])?;
    let index = expect_number(&args[1])? as i64;
    let mut list_ref = list.borrow_mut();
    if index < 0 || index as usize >= list_ref.len() {
        return Err(RuntimeError::new(
            format!(
                "Index {} out of bounds for list of length {}",
                index,
                list_ref.len()
            ),
            0,
            0,
        ));
    }
    Ok(list_ref.remove(index as usize))
}

pub fn native_insert_at(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("insert_at", &args, 3)?;
    let list = expect_list(&args[0])?;
    let index = expect_number(&args[1])? as i64;
    let item = args[2].clone();
    let mut list_ref = list.borrow_mut();
    if index < 0 || index as usize > list_ref.len() {
        return Err(RuntimeError::new(
            format!(
                "Index {} out of bounds for insert into list of length {}",
                index,
                list_ref.len()
            ),
            0,
            0,
        ));
    }
    list_ref.insert(index as usize, item);
    Ok(Value::Null)
}

pub fn native_clear(args: Vec<Value>) -> Result<Value, RuntimeError> {
    unary_list_action("clear", args, |list| list.clear())
}

pub fn native_slice(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("slice", &args, 3)?;
    let list = expect_list(&args[0])?;
    let start = expect_number(&args[1])? as i64;
    let end = expect_number(&args[2])? as i64;
    let list_ref = list.borrow();
    let len = list_ref.len() as i64;
    let start = start.max(0).min(len) as usize;
    let end = end.max(0).min(len) as usize;
    let sliced = if start <= end {
        list_ref[start..end].to_vec()
    } else {
        vec![]
    };
    Ok(Value::List(Rc::new(RefCell::new(sliced))))
}

pub fn native_concat(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("concat", &args, 2)?;
    let list_a = expect_list(&args[0])?;
    let list_b = expect_list(&args[1])?;
    let mut result = list_a.borrow().clone();
    result.extend(list_b.borrow().iter().cloned());
    Ok(Value::List(Rc::new(RefCell::new(result))))
}

// --- Batch 4: List Utilities ---

pub fn native_join(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("join", &args, 2)?;
    let list = expect_list(&args[0])?;
    let delimiter = expect_text(&args[1])?;
    let list_ref = list.borrow();
    let parts: Vec<String> = list_ref.iter().map(|v| v.to_string()).collect();
    let result = parts.join(delimiter.as_ref());
    Ok(Value::Text(Arc::from(result)))
}

pub fn native_unique(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("unique", &args, 1)?;
    let list = expect_list(&args[0])?;
    let list_ref = list.borrow();
    let mut seen_keys = std::collections::HashSet::new();
    let mut result = Vec::new();
    for item in list_ref.iter() {
        let key = format!("{:?}:{}", std::mem::discriminant(item), item);
        if seen_keys.insert(key) {
            result.push(item.clone());
        }
    }
    Ok(Value::List(Rc::new(RefCell::new(result))))
}

pub fn native_count(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("count", &args, 2)?;
    let list = expect_list(&args[0])?;
    let target = &args[1];
    let list_ref = list.borrow();
    let count = list_ref.iter().filter(|v| *v == target).count();
    Ok(Value::Number(count as f64))
}

pub fn native_fill(args: Vec<Value>) -> Result<Value, RuntimeError> {
    binary_list_action("fill", args, |list, value| {
        for item in list.iter_mut() {
            *item = value.clone();
        }
    })
}

// --- Batch 5: Sort & Reverse ---

fn value_type_discriminant(v: &Value) -> u8 {
    match v {
        Value::Number(_) => 0,
        Value::Text(_) => 1,
        Value::Bool(_) => 2,
        Value::Null | Value::Nothing => 3,
        _ => 4,
    }
}

/// Compare two WFL values for sorting.
///
/// **Type grouping** (ascending): Numbers (0) < Text (1) < Bool (2) < Null/Nothing (3) < Other (4).
/// Values of different types are ordered by their group discriminant.
///
/// **Within-type ordering:**
/// - Numbers: `f64::total_cmp` — deterministic even for NaN (NaN sorts after all finite numbers).
/// - Text: lexicographic via `Ord` on `str`.
/// - Bool: `false` (0) < `true` (1).
/// - Null/Nothing & Other: all compare equal.
///
/// **Stability:** Rust's `sort_by` is a stable sort, so equal elements keep their original order.
fn compare_values(a: &Value, b: &Value) -> std::cmp::Ordering {
    let da = value_type_discriminant(a);
    let db = value_type_discriminant(b);
    if da != db {
        return da.cmp(&db);
    }
    match (a, b) {
        (Value::Number(na), Value::Number(nb)) => na.total_cmp(nb),
        (Value::Text(sa), Value::Text(sb)) => sa.cmp(sb),
        (Value::Bool(ba), Value::Bool(bb)) => ba.cmp(bb),
        _ => std::cmp::Ordering::Equal,
    }
}

pub fn native_sort(args: Vec<Value>) -> Result<Value, RuntimeError> {
    unary_list_action("sort", args, |list| list.sort_by(compare_values))
}

pub fn native_reverse_list(args: Vec<Value>) -> Result<Value, RuntimeError> {
    unary_list_action("reverse_list", args, |list| list.reverse())
}

// --- Batch 6: List Search ---

pub fn native_find(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("find", &args, 2)?;
    let list = expect_list(&args[0])?;
    let target = &args[1];
    let list_ref = list.borrow();
    for item in list_ref.iter() {
        if item == target {
            return Ok(item.clone());
        }
    }
    Ok(Value::Nothing)
}

pub fn native_every(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("every", &args, 2)?;
    let list = expect_list(&args[0])?;
    let target = &args[1];
    let list_ref = list.borrow();
    let result = list_ref.iter().all(|v| v == target);
    Ok(Value::Bool(result))
}

pub fn native_some(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("some", &args, 2)?;
    let list = expect_list(&args[0])?;
    let target = &args[1];
    let list_ref = list.borrow();
    let result = list_ref.iter().any(|v| v == target);
    Ok(Value::Bool(result))
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

    // Batch 3: Basic List Manipulation
    let _ = env.define("shift", Value::NativeFunction("shift", native_shift));
    let _ = env.define("unshift", Value::NativeFunction("unshift", native_unshift));
    let _ = env.define(
        "remove_at",
        Value::NativeFunction("remove_at", native_remove_at),
    );
    let _ = env.define(
        "removeat",
        Value::NativeFunction("removeat", native_remove_at),
    );
    let _ = env.define(
        "insert_at",
        Value::NativeFunction("insert_at", native_insert_at),
    );
    let _ = env.define(
        "insertat",
        Value::NativeFunction("insertat", native_insert_at),
    );
    let _ = env.define("clear", Value::NativeFunction("clear", native_clear));
    let _ = env.define("slice", Value::NativeFunction("slice", native_slice));
    let _ = env.define("concat", Value::NativeFunction("concat", native_concat));
    let _ = env.define(
        "includes",
        Value::NativeFunction("includes", native_contains),
    );

    // Batch 4: List Utilities
    let _ = env.define("join", Value::NativeFunction("join", native_join));
    let _ = env.define("unique", Value::NativeFunction("unique", native_unique));
    let _ = env.define("count", Value::NativeFunction("count", native_count));
    let _ = env.define("size", Value::NativeFunction("size", native_length));
    let _ = env.define("fill", Value::NativeFunction("fill", native_fill));

    // Batch 5: Sort & Reverse
    let _ = env.define("sort", Value::NativeFunction("sort", native_sort));
    let _ = env.define(
        "reverse_list",
        Value::NativeFunction("reverse_list", native_reverse_list),
    );

    // Batch 6: List Search
    let _ = env.define("find", Value::NativeFunction("find", native_find));
    let _ = env.define(
        "find_index",
        Value::NativeFunction("find_index", native_indexof),
    );
    let _ = env.define("every", Value::NativeFunction("every", native_every));
    let _ = env.define("some", Value::NativeFunction("some", native_some));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_list(items: Vec<Value>) -> Value {
        Value::List(Rc::new(RefCell::new(items)))
    }

    // --- indexof with text ---
    #[test]
    fn test_indexof_text() {
        let result = native_indexof(vec![
            Value::Text(Arc::from("hello world")),
            Value::Text(Arc::from("world")),
        ])
        .unwrap();
        assert_eq!(result, Value::Number(6.0));
    }

    #[test]
    fn test_indexof_text_not_found() {
        let result = native_indexof(vec![
            Value::Text(Arc::from("hello")),
            Value::Text(Arc::from("xyz")),
        ])
        .unwrap();
        assert_eq!(result, Value::Number(-1.0));
    }

    // --- Batch 3 tests ---
    #[test]
    fn test_shift() {
        let list = make_list(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ]);
        let result = native_shift(vec![list.clone()]).unwrap();
        assert_eq!(result, Value::Number(1.0));
        if let Value::List(l) = &list {
            assert_eq!(l.borrow().len(), 2);
        }
    }

    #[test]
    fn test_shift_empty() {
        let list = make_list(vec![]);
        assert!(native_shift(vec![list]).is_err());
    }

    #[test]
    fn test_unshift() {
        let list = make_list(vec![Value::Number(2.0)]);
        native_unshift(vec![list.clone(), Value::Number(1.0)]).unwrap();
        if let Value::List(l) = &list {
            assert_eq!(l.borrow()[0], Value::Number(1.0));
            assert_eq!(l.borrow().len(), 2);
        }
    }

    #[test]
    fn test_remove_at() {
        let list = make_list(vec![
            Value::Text(Arc::from("a")),
            Value::Text(Arc::from("b")),
            Value::Text(Arc::from("c")),
        ]);
        let removed = native_remove_at(vec![list.clone(), Value::Number(1.0)]).unwrap();
        assert_eq!(removed, Value::Text(Arc::from("b")));
        if let Value::List(l) = &list {
            assert_eq!(l.borrow().len(), 2);
        }
    }

    #[test]
    fn test_remove_at_out_of_bounds() {
        let list = make_list(vec![Value::Number(1.0)]);
        assert!(native_remove_at(vec![list, Value::Number(5.0)]).is_err());
    }

    #[test]
    fn test_insert_at() {
        let list = make_list(vec![Value::Number(1.0), Value::Number(3.0)]);
        native_insert_at(vec![list.clone(), Value::Number(1.0), Value::Number(2.0)]).unwrap();
        if let Value::List(l) = &list {
            assert_eq!(l.borrow()[1], Value::Number(2.0));
            assert_eq!(l.borrow().len(), 3);
        }
    }

    #[test]
    fn test_insert_at_out_of_bounds() {
        let list = make_list(vec![]);
        assert!(native_insert_at(vec![list, Value::Number(5.0), Value::Number(1.0)]).is_err());
    }

    #[test]
    fn test_clear() {
        let list = make_list(vec![Value::Number(1.0), Value::Number(2.0)]);
        native_clear(vec![list.clone()]).unwrap();
        if let Value::List(l) = &list {
            assert!(l.borrow().is_empty());
        }
    }

    #[test]
    fn test_slice() {
        let list = make_list(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
            Value::Number(4.0),
        ]);
        let result = native_slice(vec![list, Value::Number(1.0), Value::Number(3.0)]).unwrap();
        if let Value::List(l) = result {
            let l = l.borrow();
            assert_eq!(l.len(), 2);
            assert_eq!(l[0], Value::Number(2.0));
            assert_eq!(l[1], Value::Number(3.0));
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_slice_clamps_bounds() {
        let list = make_list(vec![Value::Number(1.0), Value::Number(2.0)]);
        let result = native_slice(vec![list, Value::Number(-5.0), Value::Number(100.0)]).unwrap();
        if let Value::List(l) = result {
            assert_eq!(l.borrow().len(), 2);
        }
    }

    #[test]
    fn test_concat() {
        let a = make_list(vec![Value::Number(1.0)]);
        let b = make_list(vec![Value::Number(2.0)]);
        let result = native_concat(vec![a, b]).unwrap();
        if let Value::List(l) = result {
            let l = l.borrow();
            assert_eq!(l.len(), 2);
            assert_eq!(l[0], Value::Number(1.0));
            assert_eq!(l[1], Value::Number(2.0));
        }
    }

    // --- Batch 4 tests ---
    #[test]
    fn test_join() {
        let list = make_list(vec![
            Value::Text(Arc::from("a")),
            Value::Text(Arc::from("b")),
            Value::Text(Arc::from("c")),
        ]);
        let result = native_join(vec![list, Value::Text(Arc::from(", "))]).unwrap();
        assert_eq!(result, Value::Text(Arc::from("a, b, c")));
    }

    #[test]
    fn test_join_numbers() {
        let list = make_list(vec![Value::Number(1.0), Value::Number(2.0)]);
        let result = native_join(vec![list, Value::Text(Arc::from("-"))]).unwrap();
        assert_eq!(result, Value::Text(Arc::from("1-2")));
    }

    #[test]
    fn test_unique() {
        let list = make_list(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(1.0),
            Value::Number(3.0),
            Value::Number(2.0),
        ]);
        let result = native_unique(vec![list]).unwrap();
        if let Value::List(l) = result {
            let l = l.borrow();
            assert_eq!(l.len(), 3);
            assert_eq!(l[0], Value::Number(1.0));
            assert_eq!(l[1], Value::Number(2.0));
            assert_eq!(l[2], Value::Number(3.0));
        }
    }

    #[test]
    fn test_count() {
        let list = make_list(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(1.0),
        ]);
        let result = native_count(vec![list, Value::Number(1.0)]).unwrap();
        assert_eq!(result, Value::Number(2.0));
    }

    #[test]
    fn test_count_zero() {
        let list = make_list(vec![Value::Number(1.0)]);
        let result = native_count(vec![list, Value::Number(99.0)]).unwrap();
        assert_eq!(result, Value::Number(0.0));
    }

    #[test]
    fn test_fill() {
        let list = make_list(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ]);
        native_fill(vec![list.clone(), Value::Number(0.0)]).unwrap();
        if let Value::List(l) = &list {
            let l = l.borrow();
            assert!(l.iter().all(|v| *v == Value::Number(0.0)));
        }
    }

    // --- Batch 5 tests ---
    #[test]
    fn test_sort_numbers() {
        let list = make_list(vec![
            Value::Number(3.0),
            Value::Number(1.0),
            Value::Number(2.0),
        ]);
        native_sort(vec![list.clone()]).unwrap();
        if let Value::List(l) = &list {
            let l = l.borrow();
            assert_eq!(l[0], Value::Number(1.0));
            assert_eq!(l[1], Value::Number(2.0));
            assert_eq!(l[2], Value::Number(3.0));
        }
    }

    #[test]
    fn test_sort_negative_numbers() {
        let list = make_list(vec![
            Value::Number(1.0),
            Value::Number(-3.0),
            Value::Number(-1.0),
            Value::Number(2.0),
        ]);
        native_sort(vec![list.clone()]).unwrap();
        if let Value::List(l) = &list {
            let l = l.borrow();
            assert_eq!(l[0], Value::Number(-3.0));
            assert_eq!(l[1], Value::Number(-1.0));
            assert_eq!(l[2], Value::Number(1.0));
            assert_eq!(l[3], Value::Number(2.0));
        }
    }

    #[test]
    fn test_sort_text() {
        let list = make_list(vec![
            Value::Text(Arc::from("banana")),
            Value::Text(Arc::from("apple")),
            Value::Text(Arc::from("cherry")),
        ]);
        native_sort(vec![list.clone()]).unwrap();
        if let Value::List(l) = &list {
            let l = l.borrow();
            assert_eq!(l[0], Value::Text(Arc::from("apple")));
            assert_eq!(l[1], Value::Text(Arc::from("banana")));
            assert_eq!(l[2], Value::Text(Arc::from("cherry")));
        }
    }

    #[test]
    fn test_reverse_list() {
        let list = make_list(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ]);
        native_reverse_list(vec![list.clone()]).unwrap();
        if let Value::List(l) = &list {
            let l = l.borrow();
            assert_eq!(l[0], Value::Number(3.0));
            assert_eq!(l[1], Value::Number(2.0));
            assert_eq!(l[2], Value::Number(1.0));
        }
    }

    // --- Batch 6 tests ---
    #[test]
    fn test_find() {
        let list = make_list(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ]);
        let result = native_find(vec![list, Value::Number(2.0)]).unwrap();
        assert_eq!(result, Value::Number(2.0));
    }

    #[test]
    fn test_find_not_found() {
        let list = make_list(vec![Value::Number(1.0)]);
        let result = native_find(vec![list, Value::Number(99.0)]).unwrap();
        assert_eq!(result, Value::Nothing);
    }

    #[test]
    fn test_every_true() {
        let list = make_list(vec![
            Value::Number(5.0),
            Value::Number(5.0),
            Value::Number(5.0),
        ]);
        let result = native_every(vec![list, Value::Number(5.0)]).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn test_every_false() {
        let list = make_list(vec![Value::Number(5.0), Value::Number(3.0)]);
        let result = native_every(vec![list, Value::Number(5.0)]).unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn test_every_empty_list() {
        let list = make_list(vec![]);
        let result = native_every(vec![list, Value::Number(5.0)]).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn test_some_true() {
        let list = make_list(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ]);
        let result = native_some(vec![list, Value::Number(2.0)]).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn test_some_false() {
        let list = make_list(vec![Value::Number(1.0), Value::Number(2.0)]);
        let result = native_some(vec![list, Value::Number(99.0)]).unwrap();
        assert_eq!(result, Value::Bool(false));
    }
}
