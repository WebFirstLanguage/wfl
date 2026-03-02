use std::mem::size_of;

enum Value {
    Number(f64),
    Text(usize),
    Bool(bool),
    List(usize),
    Object(usize),
    Function(usize),
    NativeFunction(&'static str, usize),
    Future(usize),
    Date(usize),
    Time(usize),
    DateTime(usize),
    Pattern(usize),
    Binary(Vec<u8>),
    Null,
    Nothing,
    ContainerDefinition(usize),
    ContainerInstance(usize),
    ContainerMethod(usize),
    ContainerEvent(usize),
    InterfaceDefinition(usize),
}

fn main() {
    println!("Size of Value: {}", size_of::<Value>());
    println!("Size of Vec<u8>: {}", size_of::<Vec<u8>>());
}
