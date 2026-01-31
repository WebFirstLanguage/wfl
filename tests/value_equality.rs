use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wfl::interpreter::value::{ContainerInstanceValue, Value};
use wfl::stdlib::list::native_contains;

#[test]
fn test_list_equality() {
    let list1 = Value::List(Rc::new(RefCell::new(vec![
        Value::Number(1.0),
        Value::Number(2.0),
    ])));
    let list2 = Value::List(Rc::new(RefCell::new(vec![
        Value::Number(1.0),
        Value::Number(2.0),
    ])));
    let list3 = Value::List(Rc::new(RefCell::new(vec![
        Value::Number(1.0),
        Value::Number(3.0),
    ])));

    assert_eq!(list1, list2, "Lists with same content should be equal");
    assert_ne!(
        list1, list3,
        "Lists with different content should not be equal"
    );
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
    assert_ne!(
        obj1, obj3,
        "Objects with different content should not be equal"
    );
}

#[test]
fn test_nested_equality() {
    // Nested lists
    let inner1 = Value::List(Rc::new(RefCell::new(vec![Value::Number(1.0)])));
    let inner2 = Value::List(Rc::new(RefCell::new(vec![Value::Number(1.0)])));

    let list1 = Value::List(Rc::new(RefCell::new(vec![inner1])));
    let list2 = Value::List(Rc::new(RefCell::new(vec![inner2])));

    assert_eq!(
        list1, list2,
        "Nested lists with same content should be equal"
    );
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
    assert_eq!(
        result3,
        Value::Bool(true),
        "Should find structurally equal object"
    );
}

#[test]
fn test_cyclic_list_equality() {
    let list1_rc = Rc::new(RefCell::new(vec![]));
    let list1 = Value::List(list1_rc.clone());
    list1_rc.borrow_mut().push(list1.clone());

    let list2_rc = Rc::new(RefCell::new(vec![]));
    let list2 = Value::List(list2_rc.clone());
    list2_rc.borrow_mut().push(list2.clone());

    assert_eq!(list1, list2, "Cyclic lists should be equal and not stack overflow");
}

#[test]
fn test_cyclic_object_equality() {
    let obj1_rc = Rc::new(RefCell::new(HashMap::new()));
    let obj1 = Value::Object(obj1_rc.clone());
    obj1_rc.borrow_mut().insert("self".to_string(), obj1.clone());

    let obj2_rc = Rc::new(RefCell::new(HashMap::new()));
    let obj2 = Value::Object(obj2_rc.clone());
    obj2_rc.borrow_mut().insert("self".to_string(), obj2.clone());

    assert_eq!(obj1, obj2, "Cyclic objects should be equal and not stack overflow");
}

#[test]
fn test_comparison_with_borrowed_value() {
    let list1_rc = Rc::new(RefCell::new(vec![Value::Number(1.0)]));
    let list1 = Value::List(list1_rc.clone());

    let list2_rc = Rc::new(RefCell::new(vec![Value::Number(1.0)]));
    let list2 = Value::List(list2_rc.clone());

    // Mutably borrow list1
    let _borrow = list1_rc.borrow_mut();

    // Should return false (or not panic) when comparing a borrowed value
    // because we can't inspect its contents safely
    assert_ne!(list1, list2, "Comparison with borrowed value should not be equal");
}

#[test]
fn test_container_parent_comparison() {
    // Create Parent Instance 1
    let parent1 = Rc::new(RefCell::new(ContainerInstanceValue {
        container_type: "Parent".to_string(),
        properties: HashMap::from([("p".to_string(), Value::Number(1.0))]),
        parent: None,
        line: 0,
        column: 0,
    }));

    // Create Child 1 with Parent 1
    let child1 = Value::ContainerInstance(Rc::new(RefCell::new(ContainerInstanceValue {
        container_type: "Child".to_string(),
        properties: HashMap::new(),
        parent: Some(parent1),
        line: 0,
        column: 0,
    })));

    // Create Parent Instance 2 (Identical to Parent 1)
    let parent2 = Rc::new(RefCell::new(ContainerInstanceValue {
        container_type: "Parent".to_string(),
        properties: HashMap::from([("p".to_string(), Value::Number(1.0))]),
        parent: None,
        line: 0,
        column: 0,
    }));

     // Create Child 2 with Parent 2
    let child2 = Value::ContainerInstance(Rc::new(RefCell::new(ContainerInstanceValue {
        container_type: "Child".to_string(),
        properties: HashMap::new(),
        parent: Some(parent2),
        line: 0,
        column: 0,
    })));

    // Create Parent Instance 3 (Different Property)
    let parent3 = Rc::new(RefCell::new(ContainerInstanceValue {
        container_type: "Parent".to_string(),
        properties: HashMap::from([("p".to_string(), Value::Number(2.0))]),
        parent: None,
        line: 0,
        column: 0,
    }));

    // Create Child 3 with Parent 3
    let child3 = Value::ContainerInstance(Rc::new(RefCell::new(ContainerInstanceValue {
        container_type: "Child".to_string(),
        properties: HashMap::new(),
        parent: Some(parent3),
        line: 0,
        column: 0,
    })));

    assert_eq!(child1, child2, "Containers with identical parents should be equal");
    assert_ne!(child1, child3, "Containers with different parents should not be equal");
}
