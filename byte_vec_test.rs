use std::mem::size_of;

enum Value {
    Binary(Vec<u8>),
}

fn main() {
    println!("Size of Value with Vec<u8>: {}", size_of::<Value>());
}
