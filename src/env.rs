use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::value::Value;

#[derive(Default)]
pub struct Environment {
    pub vars: HashMap<String, Value>,
    pub parent: Option<Rc<RefCell<Environment>>>,
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            vars: HashMap::new(),
            parent: None,
        }
    }

    pub fn with_parent(parent: Rc<RefCell<Environment>>) -> Self {
        Environment {
            vars: HashMap::new(),
            parent: Some(parent),
        }
    }

    pub fn define(&mut self, name: String, value: Value) {
        self.vars.insert(name, value);
    }

    pub fn get_local(&self, name: &str) -> Option<Value> {
        self.vars.get(name).cloned()
    }

    pub fn set_local(&mut self, name: &str, value: Value) -> bool {
        if let Some(slot) = self.vars.get_mut(name) {
            *slot = value;
            true
        } else {
            false
        }
    }

    pub fn contains_local(&self, name: &str) -> bool {
        self.vars.contains_key(name)
    }

    pub fn parent(&self) -> Option<Rc<RefCell<Environment>>> {
        self.parent.clone()
    }
}