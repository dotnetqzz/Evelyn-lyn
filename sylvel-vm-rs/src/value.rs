// value.rs — SylVal: the single runtime value type

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use indexmap::IndexMap;

use crate::bytecode::Proto;

// ---------------------------------------------------------------------------
// Native function type
// ---------------------------------------------------------------------------

pub type NativeFn = fn(&mut crate::vm::Vm, &[SylVal]) -> Result<SylVal, SylError>;

// ---------------------------------------------------------------------------
// Runtime error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SylError {
    pub value: SylVal, // the thrown Sylvel value
}

impl SylError {
    pub fn msg(s: &str) -> Self {
        SylError { value: SylVal::Str(Rc::new(s.to_string())) }
    }
    pub fn fmt(s: String) -> Self {
        SylError { value: SylVal::Str(Rc::new(s)) }
    }
}

impl fmt::Display for SylError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value.format())
    }
}

// ---------------------------------------------------------------------------
// Iterator state
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct SylIter {
    pub source: SylVal,
    pub pos: usize,
}

impl SylIter {
    pub fn next_val(&mut self) -> SylVal {
        match &self.source {
            SylVal::List(l) => {
                let items = l.borrow();
                if self.pos < items.len() {
                    let v = items[self.pos].clone();
                    self.pos += 1;
                    v
                } else {
                    SylVal::Null
                }
            }
            SylVal::Str(s) => {
                let bytes = s.as_bytes();
                if self.pos < bytes.len() {
                    let ch = bytes[self.pos] as char;
                    self.pos += 1;
                    SylVal::Str(Rc::new(ch.to_string()))
                } else {
                    SylVal::Null
                }
            }
            _ => SylVal::Null,
        }
    }
}

// ---------------------------------------------------------------------------
// The value enum
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum SylVal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Rc<String>),
    List(Rc<RefCell<Vec<SylVal>>>),
    Map(Rc<RefCell<IndexMap<String, SylVal>>>),
    Func(Rc<Proto>),
    Native(NativeFn),
    Iter(Rc<RefCell<SylIter>>),
}

impl SylVal {
    pub fn is_truthy(&self) -> bool {
        match self {
            SylVal::Null => false,
            SylVal::Bool(b) => *b,
            SylVal::Int(i) => *i != 0,
            SylVal::Float(f) => *f != 0.0,
            SylVal::Str(s) => !s.is_empty(),
            SylVal::List(l) => !l.borrow().is_empty(),
            SylVal::Map(m) => !m.borrow().is_empty(),
            _ => true,
        }
    }

    pub fn format(&self) -> String {
        match self {
            SylVal::Null => "null".to_string(),
            SylVal::Bool(b) => b.to_string(),
            SylVal::Int(i) => i.to_string(),
            SylVal::Float(f) => {
                if f.fract() == 0.0 && f.abs() < 1e15 {
                    format!("{:.1}", f)
                } else {
                    format!("{}", f)
                }
            }
            SylVal::Str(s) => s.as_ref().clone(),
            SylVal::List(l) => {
                let items: Vec<String> = l.borrow().iter().map(|v| v.format()).collect();
                format!("[{}]", items.join(", "))
            }
            SylVal::Map(m) => {
                let entries: Vec<String> = m
                    .borrow()
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.format()))
                    .collect();
                format!("{{{}}}", entries.join(", "))
            }
            SylVal::Func(_) => "<function>".to_string(),
            SylVal::Native(_) => "<native>".to_string(),
            SylVal::Iter(_) => "<iter>".to_string(),
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            SylVal::Null => "null",
            SylVal::Bool(_) => "bool",
            SylVal::Int(_) => "int",
            SylVal::Float(_) => "float",
            SylVal::Str(_) => "string",
            SylVal::List(_) => "list",
            SylVal::Map(_) => "map",
            SylVal::Func(_) => "function",
            SylVal::Native(_) => "native",
            SylVal::Iter(_) => "iter",
        }
    }

    pub fn deep_equal(&self, other: &SylVal) -> bool {
        match (self, other) {
            (SylVal::Null, SylVal::Null) => true,
            (SylVal::Bool(a), SylVal::Bool(b)) => a == b,
            (SylVal::Int(a), SylVal::Int(b)) => a == b,
            (SylVal::Float(a), SylVal::Float(b)) => a == b,
            (SylVal::Int(a), SylVal::Float(b)) => (*a as f64) == *b,
            (SylVal::Float(a), SylVal::Int(b)) => *a == (*b as f64),
            (SylVal::Str(a), SylVal::Str(b)) => a == b,
            (SylVal::List(a), SylVal::List(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.deep_equal(y))
            }
            (SylVal::Map(a), SylVal::Map(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                if a.len() != b.len() { return false; }
                for (k, av) in a.iter() {
                    match b.get(k) {
                        Some(bv) => { if !av.deep_equal(bv) { return false; } }
                        None => return false,
                    }
                }
                true
            }
            _ => false,
        }
    }

    pub fn as_f64(&self) -> f64 {
        match self {
            SylVal::Int(i) => *i as f64,
            SylVal::Float(f) => *f,
            _ => 0.0,
        }
    }

    pub fn as_i64(&self) -> i64 {
        match self {
            SylVal::Int(i) => *i,
            SylVal::Float(f) => *f as i64,
            _ => 0,
        }
    }
}

impl fmt::Display for SylVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}
