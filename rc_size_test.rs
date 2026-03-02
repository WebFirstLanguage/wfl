use std::mem::size_of;
use std::rc::Rc;

enum Value {
    Binary(Rc<Vec<u8>>),
}

fn main() {
    println!("Size of Value: {}", size_of::<Value>());
    println!("Size of Rc<Vec<u8>>: {}", size_of::<Rc<Vec<u8>>>());
}
