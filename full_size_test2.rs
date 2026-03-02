use std::mem::size_of;
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::Arc;
use std::collections::HashMap;

struct FunctionValue {}
struct NativeFunction {}
struct FutureValue {}
struct CompiledPattern {}
struct ContainerDefinitionValue {}
struct ContainerInstanceValue {}
struct ContainerMethodValue {}
struct ContainerEventValue {}
struct InterfaceDefinitionValue {}

enum ValueVec {
    Number(f64),
    Text(Arc<str>),
    Bool(bool),
    List(Rc<RefCell<Vec<()>>>),
    Object(Rc<RefCell<HashMap<String, ()>>>),
    Function(Rc<FunctionValue>),
    NativeFunction(&'static str, usize),
    Future(Rc<RefCell<FutureValue>>),
    Date(Rc<()>),
    Time(Rc<()>),
    DateTime(Rc<()>),
    Pattern(Rc<CompiledPattern>),
    Binary(Vec<u8>),
    Null,
    Nothing,
    ContainerDefinition(Rc<ContainerDefinitionValue>),
    ContainerInstance(Rc<RefCell<ContainerInstanceValue>>),
    ContainerMethod(Rc<ContainerMethodValue>),
    ContainerEvent(Rc<ContainerEventValue>),
    InterfaceDefinition(Rc<InterfaceDefinitionValue>),
}

enum ValueArcSlice {
    Number(f64),
    Text(Arc<str>),
    Bool(bool),
    List(Rc<RefCell<Vec<()>>>),
    Object(Rc<RefCell<HashMap<String, ()>>>),
    Function(Rc<FunctionValue>),
    NativeFunction(&'static str, usize),
    Future(Rc<RefCell<FutureValue>>),
    Date(Rc<()>),
    Time(Rc<()>),
    DateTime(Rc<()>),
    Pattern(Rc<CompiledPattern>),
    Binary(Arc<[u8]>),
    Null,
    Nothing,
    ContainerDefinition(Rc<ContainerDefinitionValue>),
    ContainerInstance(Rc<RefCell<ContainerInstanceValue>>),
    ContainerMethod(Rc<ContainerMethodValue>),
    ContainerEvent(Rc<ContainerEventValue>),
    InterfaceDefinition(Rc<InterfaceDefinitionValue>),
}

fn main() {
    println!("Size of Value with Vec<u8>: {}", size_of::<ValueVec>());
    println!("Size of Value with Arc<[u8]>: {}", size_of::<ValueArcSlice>());
}
