// env.rs — Scoped variable environment

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::value::{AvelynError, AvelynVal};

/// A single binding: the value and whether it was declared with `var` (mutable)
/// or `let` (immutable).
#[derive(Debug, Clone)]
struct Binding {
    val: AvelynVal,
    mutable: bool,
}

#[derive(Debug, Clone)]
pub struct Env {
    vars: RefCell<HashMap<String, Binding>>,
    parent: Option<Rc<Env>>,
}

impl Env {
    pub fn new() -> Rc<Self> {
        Rc::new(Env { vars: RefCell::new(HashMap::new()), parent: None })
    }

    pub fn child(parent: Rc<Self>) -> Rc<Self> {
        Rc::new(Env { vars: RefCell::new(HashMap::new()), parent: Some(parent) })
    }

    /// Declare a new binding in **this** scope.
    /// `mutable = true` → declared with `var`; `false` → declared with `let`.
    pub fn declare(&self, name: &str, val: AvelynVal, mutable: bool) {
        self.vars.borrow_mut().insert(name.to_string(), Binding { val, mutable });
    }

    /// Assign to an existing binding anywhere in the scope chain.
    /// Returns `Err` if the name is not found or the binding is immutable.
    pub fn set(self: &Rc<Self>, name: &str, val: AvelynVal) -> Result<(), AvelynError> {
        if let Some(env) = self.find_owner(name) {
            let mut vars = env.vars.borrow_mut();
            let binding = vars.get_mut(name).unwrap(); // safe: find_owner confirmed it exists
            if !binding.mutable {
                return Err(AvelynError::fmt(format!(
                    "ImmutabilityError: cannot assign to immutable binding '{}' declared with 'let'",
                    name
                )));
            }
            binding.val = val;
            Ok(())
        } else {
            // Name not yet in scope — create as mutable (implicit global assignment)
            self.vars.borrow_mut().insert(name.to_string(), Binding { val, mutable: true });
            Ok(())
        }
    }

    pub fn get(&self, name: &str) -> Option<AvelynVal> {
        if let Some(b) = self.vars.borrow().get(name) { return Some(b.val.clone()); }
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
