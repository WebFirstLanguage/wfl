use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wfl::interpreter::value::Value;

#[test]
fn self_cycle_formats_and_deep_clones_without_recursing_forever() {
    let source_list = Rc::new(RefCell::new(Vec::new()));
    let source = Value::List(Rc::clone(&source_list));
    source_list.borrow_mut().push(source.clone());

    assert_eq!(source.to_string(), "[<cycle>]");
    assert_eq!(format!("{source:?}"), "[<cycle>]");

    let cloned = source.deep_clone();
    let Value::List(cloned_list) = cloned else {
        panic!("a cloned list must remain a list");
    };
    assert!(
        !Rc::ptr_eq(&source_list, &cloned_list),
        "deep_clone must isolate the clone from its source"
    );

    let cloned_child = cloned_list.borrow()[0].clone();
    let Value::List(cloned_child_list) = cloned_child else {
        panic!("the self-reference must remain a list reference");
    };
    assert!(
        Rc::ptr_eq(&cloned_list, &cloned_child_list),
        "the cloned self-reference must point at the cloned list"
    );
}

#[test]
fn mutual_cycle_preserves_cycles_and_shared_identity_in_the_clone() {
    let source_list = Rc::new(RefCell::new(Vec::new()));
    let source_object = Rc::new(RefCell::new(HashMap::new()));

    let shared_object = Value::Object(Rc::clone(&source_object));
    source_list
        .borrow_mut()
        .extend([shared_object.clone(), shared_object]);
    source_object
        .borrow_mut()
        .insert("back".to_string(), Value::List(Rc::clone(&source_list)));
    let source = Value::List(Rc::clone(&source_list));

    assert_eq!(source.to_string(), "[<cycle>, <cycle>]");
    assert_eq!(format!("{source:?}"), "[{back: <cycle>}, {back: <cycle>}]");

    let Value::List(cloned_list) = source.deep_clone() else {
        panic!("a cloned list must remain a list");
    };
    let cloned_items = cloned_list.borrow();
    let Value::Object(first_object) = &cloned_items[0] else {
        panic!("the cloned list must contain its object");
    };
    let Value::Object(second_object) = &cloned_items[1] else {
        panic!("the cloned list must contain its shared object twice");
    };
    assert!(
        Rc::ptr_eq(first_object, second_object),
        "shared source objects must remain shared within the cloned graph"
    );
    assert!(
        !Rc::ptr_eq(&source_object, first_object),
        "the cloned object must be independent from its source"
    );

    let cloned_back_reference = first_object
        .borrow()
        .get("back")
        .expect("cloned object must retain its back-reference")
        .clone();
    let Value::List(cloned_back_list) = cloned_back_reference else {
        panic!("the cloned back-reference must remain a list");
    };
    assert!(
        Rc::ptr_eq(&cloned_list, &cloned_back_list),
        "the mutual cycle must point back into the cloned graph"
    );
}

#[test]
fn formatting_stops_at_a_bounded_depth_for_acyclic_values() {
    let mut value = Value::Number(1.0);
    for _ in 0..128 {
        value = Value::List(Rc::new(RefCell::new(vec![value])));
    }

    assert!(value.to_string().contains("<max-depth>"));
    assert!(format!("{value:?}").contains("<max-depth>"));
}
