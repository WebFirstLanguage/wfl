use super::helpers::{
    binary_list_val_op, check_arg_count, expect_list, expect_number, unary_list_op,
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
    binary_list_val_op("push", args, |list, item| {
        list.borrow_mut().push(item);
        Ok(Value::Null)
    })
}

pub fn native_pop(args: Vec<Value>) -> Result<Value, RuntimeError> {
    unary_list_op("pop", args, |list| {
        let mut list_ref = list.borrow_mut();
        if list_ref.is_empty() {
            return Err(RuntimeError::new(
                "Cannot pop from an empty list".to_string(),
                0,
                0,
            ));
        }
        Ok(list_ref.pop().unwrap())
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
        let mut list_ref = list.borrow_mut();
        if list_ref.is_empty() {
            return Err(RuntimeError::new(
                "Cannot shift from an empty list".to_string(),
                0,
                0,
            ));
        }
        Ok(list_ref.remove(0))
    })
}

pub fn native_unshift(args: Vec<Value>) -> Result<Value, RuntimeError> {
    binary_list_val_op("unshift", args, |list, item| {
        list.borrow_mut().insert(0, item);
        Ok(Value::Null)
    })
}

pub fn native_remove_at(args: Vec<Value>) -> Result<Value, RuntimeError> {
    binary_list_val_op("remove_at", args, |list, index_val| {
        let index = super::helpers::expect_number(&index_val)? as i64;
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
    })
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
    unary_list_op("clear", args, |list| {
        list.borrow_mut().clear();
        Ok(Value::Null)
    })
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
    binary_list_val_op("concat", args, |list_a, list_b_val| {
        let list_b = super::helpers::expect_list(&list_b_val)?;
        let list_a_ref = list_a.borrow();
        let list_b_ref = list_b.borrow();
        // Optimization: Pre-calculate total capacity to avoid intermediate allocations and re-allocations
        let mut result = Vec::with_capacity(list_a_ref.len() + list_b_ref.len());
        result.extend(list_a_ref.iter().cloned());
        result.extend(list_b_ref.iter().cloned());
        Ok(Value::List(Rc::new(RefCell::new(result))))
    })
}

// --- Batch 4: List Utilities ---

pub fn native_join(args: Vec<Value>) -> Result<Value, RuntimeError> {
    binary_list_val_op("join", args, |list, delimiter_val| {
        let delimiter = super::helpers::expect_text(&delimiter_val)?;
        let list_ref = list.borrow();

        if list_ref.is_empty() {
            return Ok(Value::Text(Arc::from("")));
        }

        let delimiter_len = delimiter.len();
        let mut total_len = 0;
        let list_len = list_ref.len();

        for item in list_ref.iter() {
            match item {
                Value::Text(s) => total_len += s.len(),
                Value::Number(_) => total_len += 20,
                Value::Bool(b) => total_len += if *b { 3 } else { 2 },
                _ => total_len += 16,
            }
        }

        if list_len > 1 {
            total_len += (list_len - 1) * delimiter_len;
        }

        let mut result = String::with_capacity(total_len);

        for (i, item) in list_ref.iter().enumerate() {
            if i > 0 {
                result.push_str(&delimiter);
            }

            match item {
                Value::Text(s) => result.push_str(s),
                _ => {
                    use std::fmt::Write;
                    let _ = write!(result, "{}", item);
                }
            }
        }

        Ok(Value::Text(Arc::from(result)))
    })
}

pub fn native_unique(args: Vec<Value>) -> Result<Value, RuntimeError> {
    unary_list_op("unique", args, |list| {
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
    })
}

pub fn native_count(args: Vec<Value>) -> Result<Value, RuntimeError> {
    binary_list_val_op("count", args, |list, target| {
        let list_ref = list.borrow();
        let count = list_ref.iter().filter(|v| **v == target).count();
        Ok(Value::Number(count as f64))
    })
}

pub fn native_fill(args: Vec<Value>) -> Result<Value, RuntimeError> {
    binary_list_val_op("fill", args, |list, value| {
        let mut list_ref = list.borrow_mut();
        for item in list_ref.iter_mut() {
            *item = value.clone();
        }
        Ok(Value::Null)
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
    unary_list_op("sort", args, |list| {
        list.borrow_mut().sort_by(compare_values);
        Ok(Value::Null)
    })
}

pub fn native_reverse_list(args: Vec<Value>) -> Result<Value, RuntimeError> {
    unary_list_op("reverse_list", args, |list| {
        list.borrow_mut().reverse();
        Ok(Value::Null)
    })
}

// --- Batch 6: List Search ---

pub fn native_find(args: Vec<Value>) -> Result<Value, RuntimeError> {
    binary_list_val_op("find", args, |list, target| {
        let list_ref = list.borrow();
        for item in list_ref.iter() {
            if *item == target {
                return Ok(item.clone());
            }
        }
        Ok(Value::Nothing)
    })
}

pub fn native_every(args: Vec<Value>) -> Result<Value, RuntimeError> {
    binary_list_val_op("every", args, |list, target| {
        let list_ref = list.borrow();
        let result = list_ref.iter().all(|v| *v == target);
        Ok(Value::Bool(result))
    })
}

pub fn native_some(args: Vec<Value>) -> Result<Value, RuntimeError> {
    binary_list_val_op("some", args, |list, target| {
        let list_ref = list.borrow();
        let result = list_ref.contains(&target);
        Ok(Value::Bool(result))
    })
}

pub fn register_list(env: &mut Environment) {
    let _ = env.define_native("length", native_length);
    let _ = env.define_native("push", native_push);
    let _ = env.define_native("pop", native_pop);
    let _ = env.define_native("contains", native_contains);
    let _ = env.define_native("indexof", native_indexof);
    let _ = env.define_native("index_of", native_indexof);

    // Batch 3: Basic List Manipulation
    let _ = env.define_native("shift", native_shift);
    let _ = env.define_native("unshift", native_unshift);
    let _ = env.define_native("remove_at", native_remove_at);
    let _ = env.define_native("removeat", native_remove_at);
    let _ = env.define_native("insert_at", native_insert_at);
    let _ = env.define_native("insertat", native_insert_at);
    let _ = env.define_native("clear", native_clear);
    let _ = env.define_native("slice", native_slice);
    let _ = env.define_native("concat", native_concat);
    let _ = env.define_native("includes", native_contains);

    // Batch 4: List Utilities
    let _ = env.define_native("join", native_join);
    let _ = env.define_native("unique", native_unique);
    let _ = env.define_native("count", native_count);
    let _ = env.define_native("size", native_length);
    let _ = env.define_native("fill", native_fill);

    // Batch 5: Sort & Reverse
    let _ = env.define_native("sort", native_sort);
    let _ = env.define_native("reverse_list", native_reverse_list);

    // Batch 6: List Search
    let _ = env.define_native("find", native_find);
    let _ = env.define_native("find_index", native_indexof);
    let _ = env.define_native("every", native_every);
    let _ = env.define_native("some", native_some);
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
