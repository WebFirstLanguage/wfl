use super::value::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

#[derive(Debug)]
pub struct Environment {
    pub values: HashMap<String, Value>,
    pub constants: HashSet<String>,
    pub parent: Option<Weak<RefCell<Environment>>>,
}

impl Environment {
    pub fn new_global() -> Rc<RefCell<Self>> {
        #[cfg(feature = "dhat-ad-hoc")]
        dhat::ad_hoc_event(1);

        Rc::new(RefCell::new(Environment {
            values: HashMap::new(),
            constants: HashSet::new(),
            parent: None,
        }))
    }

    pub fn new(parent: &Rc<RefCell<Environment>>) -> Rc<RefCell<Self>> {
        #[cfg(feature = "dhat-ad-hoc")]
        dhat::ad_hoc_event(1);

        Rc::new(RefCell::new(Environment {
            values: HashMap::new(),
            constants: HashSet::new(),
            parent: Some(Rc::downgrade(parent)),
        }))
    }

    #[inline]
    pub fn new_child_env(parent: &Rc<RefCell<Environment>>) -> Rc<RefCell<Self>> {
        #[cfg(feature = "dhat-ad-hoc")]
        dhat::ad_hoc_event(1);

        Rc::new(RefCell::new(Self {
            values: HashMap::new(),
            constants: HashSet::new(),
            parent: Some(Rc::downgrade(parent)),
        }))
    }

    pub fn define(&mut self, name: &str, value: Value) -> Result<(), String> {
        // Check if the variable already exists in current scope
        if self.values.contains_key(name) {
            return Err(format!(
                "Variable '{name}' has already been defined. Use 'change {name} to <value>' to modify it."
            ));
        }

        // Check if the variable exists in parent scopes
        if let Some(parent_weak) = &self.parent
            && let Some(parent) = parent_weak.upgrade()
            && parent.borrow().get(name).is_some()
        {
            return Err(format!(
                "Variable '{name}' has already been defined in an outer scope. Use 'change {name} to <value>' to modify it."
            ));
        }

        self.values.insert(name.to_string(), value);
        Ok(())
    }

    pub fn define_constant(&mut self, name: &str, value: Value) -> Result<(), String> {
        // Check if the variable/constant already exists
        if self.values.contains_key(name) {
            return Err(format!(
                "Variable or constant '{name}' has already been defined."
            ));
        }

        // Check if the variable exists in parent scopes
        if let Some(parent_weak) = &self.parent
            && let Some(parent) = parent_weak.upgrade()
            && parent.borrow().get(name).is_some()
        {
            return Err(format!(
                "Variable or constant '{name}' has already been defined in an outer scope."
            ));
        }

        self.values.insert(name.to_string(), value);
        self.constants.insert(name.to_string());
        Ok(())
    }

    pub fn is_constant(&self, name: &str) -> bool {
        if self.constants.contains(name) {
            true
        } else if let Some(parent_weak) = &self.parent {
            if let Some(parent) = parent_weak.upgrade() {
                parent.borrow().is_constant(name)
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), String> {
        // Check if it's a constant in current scope
        if self.constants.contains(name) {
            return Err(format!("Cannot modify constant '{name}'"));
        }

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
