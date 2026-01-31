use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use wfl::interpreter::value::Value;
use wfl::stdlib::list::native_contains;

#[test]
fn test_list_equality() {
    let list1 = Value::List(Rc::new(RefCell::new(vec![Value::Number(1.0), Value::Number(2.0)])));
    let list2 = Value::List(Rc::new(RefCell::new(vec![Value::Number(1.0), Value::Number(2.0)])));
    let list3 = Value::List(Rc::new(RefCell::new(vec![Value::Number(1.0), Value::Number(3.0)])));

    assert_eq!(list1, list2, "Lists with same content should be equal");
    assert_ne!(list1, list3, "Lists with different content should not be equal");
}

#[test]
fn test_object_equality() {
    let mut map1 = HashMap::new();
    map1.insert("a".to_string(), Value::Number(1.0));
    let obj1 = Value::Object(Rc::new(RefCell::new(map1)));

    let mut map2 = HashMap::new();
    map2.insert("a".to_string(), Value::Number(1.0));
    let obj2 = Value::Object(Rc::new(RefCell::new(map2)));

    let mut map3 = HashMap::new();
    map3.insert("a".to_string(), Value::Number(2.0));
    let obj3 = Value::Object(Rc::new(RefCell::new(map3)));

    assert_eq!(obj1, obj2, "Objects with same content should be equal");
    assert_ne!(obj1, obj3, "Objects with different content should not be equal");
}

#[test]
fn test_nested_equality() {
    // Nested lists
    let inner1 = Value::List(Rc::new(RefCell::new(vec![Value::Number(1.0)])));
    let inner2 = Value::List(Rc::new(RefCell::new(vec![Value::Number(1.0)])));

    let list1 = Value::List(Rc::new(RefCell::new(vec![inner1])));
    let list2 = Value::List(Rc::new(RefCell::new(vec![inner2])));

    assert_eq!(list1, list2, "Nested lists with same content should be equal");
}

#[test]
fn test_native_contains() {
    let list_val = Value::List(Rc::new(RefCell::new(vec![Value::Number(10.0)])));
    let args = vec![list_val.clone(), Value::Number(10.0)];
    let result = native_contains(args).unwrap();
    assert_eq!(result, Value::Bool(true));

    let args2 = vec![list_val.clone(), Value::Number(20.0)];
    let result2 = native_contains(args2).unwrap();
    assert_eq!(result2, Value::Bool(false));

    // Test with object in list
    let mut map = HashMap::new();
    map.insert("k".to_string(), Value::Number(1.0));
    let obj = Value::Object(Rc::new(RefCell::new(map)));

    let list_obj = Value::List(Rc::new(RefCell::new(vec![obj.clone()])));

    // Construct identical object
    let mut map2 = HashMap::new();
    map2.insert("k".to_string(), Value::Number(1.0));
    let obj2 = Value::Object(Rc::new(RefCell::new(map2)));

    let args3 = vec![list_obj.clone(), obj2];
    let result3 = native_contains(args3).unwrap();
    assert_eq!(result3, Value::Bool(true), "Should find structurally equal object");
}
