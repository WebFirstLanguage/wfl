// Mixed content file - has code, comments, and blank lines
use std::collections::HashMap;

/* 
 * This is a multi-line comment
 * explaining the purpose of this module
 */

fn main() {
    // Initialize variables
    let mut map = HashMap::new();
    
    map.insert("key1", "value1");
    map.insert("key2", "value2"); // Inline comment
    
    /* Block comment on same line */ let x = 42;

    for (key, value) in &map {
        println!("{}: {}", key, value);
    }
    
    // Call helper function
    helper_function();
}

/*
Multi-line block comment
with multiple lines
and no asterisks on internal lines
*/
fn helper_function() {
    println!("Helper called");
    
    // Another comment
    let result = calculate(10, 20);
    println!("Calculation result: {}", result);
}

// Final function
fn calculate(a: i32, b: i32) -> i32 {
    a + b // Return sum
}
