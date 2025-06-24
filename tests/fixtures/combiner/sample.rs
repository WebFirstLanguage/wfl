use std::collections::HashMap;

pub struct TestStruct {
    pub field1: String,
    pub field2: i32,
}

impl TestStruct {
    pub fn new(field1: String, field2: i32) -> Self {
        Self { field1, field2 }
    }
    
    pub fn get_field1(&self) -> &str {
        &self.field1
    }
}

fn main() {
    let test = TestStruct::new("Hello".to_string(), 42);
    println!("Field1: {}", test.get_field1());
}
