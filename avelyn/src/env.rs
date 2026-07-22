// env.rs — Scoped variable environment

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::value::AvelynVal;

#[derive(Debug, Clone)]
pub struct Env {
    vars: RefCell<HashMap<String, AvelynVal>>,
    parent: Option<Rc<Env>>,
}

impl Env {
    pub fn new() -> Rc<Self> {
        Rc::new(Env { vars: RefCell::new(HashMap::new()), parent: None })
    }

    pub fn child(parent: Rc<Self>) -> Rc<Self> {
        Rc::new(Env { vars: RefCell::new(HashMap::new()), parent: Some(parent) })
    }

    pub fn declare(&self, name: &str, val: AvelynVal) {
        self.vars.borrow_mut().insert(name.to_string(), val);
    }

    pub fn set(self: &Rc<Self>, name: &str, val: AvelynVal) {
        if let Some(env) = self.find_owner(name) {
            env.vars.borrow_mut().insert(name.to_string(), val);
        } else {
            self.vars.borrow_mut().insert(name.to_string(), val);
        }
    }

    pub fn get(&self, name: &str) -> Option<AvelynVal> {
        if let Some(v) = self.vars.borrow().get(name) { return Some(v.clone()); }
        self.parent.as_ref()?.get(name)
    }

    fn find_owner(self: &Rc<Self>, name: &str) -> Option<Rc<Env>> {
        if self.vars.borrow().contains_key(name) { return Some(self.clone()); }
        self.parent.as_ref()?.find_owner(name)
    }
}

impl Default for Env {
    fn default() -> Self {
        Env { vars: RefCell::new(HashMap::new()), parent: None }
    }
}
