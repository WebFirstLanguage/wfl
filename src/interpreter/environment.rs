use super::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

#[derive(Debug)]
pub struct Environment {
    pub values: HashMap<String, Value>,
    pub parent: Option<Weak<RefCell<Environment>>>,
}

impl Environment {
    pub fn new_global() -> Rc<RefCell<Self>> {
        #[cfg(feature = "dhat-ad-hoc")]
        dhat::ad_hoc_event(1);

        Rc::new(RefCell::new(Environment {
            values: HashMap::new(),
            parent: None,
        }))
    }

    pub fn new(parent: &Rc<RefCell<Environment>>) -> Rc<RefCell<Self>> {
        #[cfg(feature = "dhat-ad-hoc")]
        dhat::ad_hoc_event(1);

        Rc::new(RefCell::new(Environment {
            values: HashMap::new(),
            parent: Some(Rc::downgrade(parent)),
        }))
    }

    #[inline]
    pub fn new_child_env(parent: &Rc<RefCell<Environment>>) -> Rc<RefCell<Self>> {
        #[cfg(feature = "dhat-ad-hoc")]
        dhat::ad_hoc_event(1);

        Rc::new(RefCell::new(Self {
            values: HashMap::new(),
            parent: Some(Rc::downgrade(parent)),
        }))
    }

    pub fn define(&mut self, name: &str, value: Value) {
        self.values.insert(name.to_string(), value);
    }

    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), String> {
        if self.values.contains_key(name) {
            self.values.insert(name.to_string(), value);
            Ok(())
        } else if let Some(parent_weak) = &self.parent {
            if let Some(parent) = parent_weak.upgrade() {
                parent.borrow_mut().assign(name, value)
            } else {
                Err("Parent environment no longer exists".to_string())
            }
        } else {
            Err(format!("Undefined variable '{name}'"))
        }
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.values.get(name) {
            Some(value.clone())
        } else if let Some(parent_weak) = &self.parent {
            if let Some(parent) = parent_weak.upgrade() {
                parent.borrow().get(name)
            } else {
                None
            }
        } else {
            None
        }
    }
}
