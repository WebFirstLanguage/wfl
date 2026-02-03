use wfl::interpreter::value::Value;
use wfl::stdlib::filesystem::native_remove_dir;
use wfl::stdlib::time::native_create_time;
use std::rc::Rc;

fn main() {
    // Verify remove_dir with 0 args returns error, not panic
    let result = native_remove_dir(vec![]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().message, "remove_dir expects at least 1 argument, got 0");
    println!("remove_dir check passed");

    // Verify create_time with 3 args works
    let args = vec![
        Value::Number(10.0),
        Value::Number(30.0),
        Value::Number(45.0),
    ];
    let result = native_create_time(args);
    assert!(result.is_ok());
    println!("create_time check passed");
}
