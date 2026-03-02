use std::rc::Rc;
use std::mem::size_of;

enum Value {
    Binary(Rc<Vec<u8>>),
}

fn main() {
    println!("Size of Value with Rc<Vec<u8>>: {}", size_of::<Value>());
}
