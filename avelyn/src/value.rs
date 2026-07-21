// value.rs — AvelynVal runtime value type

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use indexmap::IndexMap;

use crate::ast::{ASTNode, Param};
use crate::env::Env;

// ─── Error ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AvelynError {
    pub val: AvelynVal,
    pub line: u32,
    pub file: String,
}

impl AvelynError {
    pub fn new(val: AvelynVal) -> Self { AvelynError { val, line: 0, file: String::new() } }
    pub fn msg(s: &str) -> Self { AvelynError::new(AvelynVal::Str(Rc::new(s.to_string()))) }
    pub fn fmt(s: String) -> Self { AvelynError::new(AvelynVal::Str(Rc::new(s))) }
    pub fn with_line(mut self, l: u32, f: &str) -> Self { self.line = l; self.file = f.to_string(); self }
}

impl fmt::Display for AvelynError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.val.format())
    }
}

// ─── Control flow signals ────────────────────────────────────────────────────

pub enum Signal {
    Return(AvelynVal),
    Break,
    Continue,
    Error(AvelynError),
}

// ─── Native function ─────────────────────────────────────────────────────────

pub type NativeFn = fn(&mut crate::interpreter::Interpreter, Vec<AvelynVal>) -> Result<AvelynVal, AvelynError>;

// ─── AvelynFunc ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct AvelynFunc {
    pub name: Option<String>,
    pub params: Vec<Param>,
    pub body: Vec<ASTNode>,
    pub closure: Rc<Env>,
    pub variadic: bool,
}

// ─── AvelynVal ────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum AvelynVal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Rc<String>),
    List(Rc<RefCell<Vec<AvelynVal>>>),
    Map(Rc<RefCell<IndexMap<String, AvelynVal>>>),
    Func(Rc<AvelynFunc>),
    Native(NativeFn),
    ByteArray(Rc<RefCell<Vec<u8>>>),
}

impl AvelynVal {
    pub fn is_truthy(&self) -> bool {
        match self {
            AvelynVal::Null      => false,
            AvelynVal::Bool(b)   => *b,
            AvelynVal::Int(i)    => *i != 0,
            AvelynVal::Float(f)  => *f != 0.0,
            AvelynVal::Str(s)    => !s.is_empty(),
            AvelynVal::List(l)   => !l.borrow().is_empty(),
            AvelynVal::Map(m)    => !m.borrow().is_empty(),
            AvelynVal::ByteArray(b) => !b.borrow().is_empty(),
            _ => true,
        }
    }

    pub fn is_null(&self) -> bool { matches!(self, AvelynVal::Null) }

    pub fn format(&self) -> String {
        match self {
            AvelynVal::Null      => "null".into(),
            AvelynVal::Bool(b)   => b.to_string(),
            AvelynVal::Int(i)    => i.to_string(),
            AvelynVal::Float(f)  => {
                if f.fract() == 0.0 && f.abs() < 1e15 { format!("{}", *f as i64) }
                else { f.to_string() }
            }
            AvelynVal::Str(s)    => s.as_ref().clone(),
            AvelynVal::List(l)   => {
                let v: Vec<String> = l.borrow().iter().map(|x| x.format()).collect();
                format!("[{}]", v.join(", "))
            }
            AvelynVal::Map(m)    => {
                let pairs: Vec<String> = m.borrow().iter()
                    .map(|(k, v)| format!("{:?}: {}", k, v.format())).collect();
                format!("{{{}}}", pairs.join(", "))
            }
            AvelynVal::Func(_)   => "<function>".into(),
            AvelynVal::Native(_) => "<native>".into(),
            AvelynVal::ByteArray(b) => {
                let v: Vec<String> = b.borrow().iter().map(|x| x.to_string()).collect();
                format!("[{}]", v.join(", "))
            }
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            AvelynVal::Null      => "null",
            AvelynVal::Bool(_)   => "bool",
            AvelynVal::Int(_)    => "int",
            AvelynVal::Float(_)  => "float",
            AvelynVal::Str(_)    => "string",
            AvelynVal::List(_)   => "array",
            AvelynVal::Map(_)    => "map",
            AvelynVal::Func(_)   => "function",
            AvelynVal::Native(_) => "function",
            AvelynVal::ByteArray(_) => "bytearray",
        }
    }

    pub fn deep_equal(&self, other: &AvelynVal) -> bool {
        match (self, other) {
            (AvelynVal::Null,      AvelynVal::Null)      => true,
            (AvelynVal::Bool(a),   AvelynVal::Bool(b))   => a == b,
            (AvelynVal::Int(a),    AvelynVal::Int(b))     => a == b,
            (AvelynVal::Float(a),  AvelynVal::Float(b))   => a == b,
            (AvelynVal::Int(a),    AvelynVal::Float(b))   => (*a as f64) == *b,
            (AvelynVal::Float(a),  AvelynVal::Int(b))     => *a == (*b as f64),
            (AvelynVal::Str(a),    AvelynVal::Str(b))     => a == b,
            (AvelynVal::List(a),   AvelynVal::List(b))    => {
                let a = a.borrow(); let b = b.borrow();
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x,y)| x.deep_equal(y))
            }
            (AvelynVal::Map(a), AvelynVal::Map(b)) => {
                let a = a.borrow(); let b = b.borrow();
                if a.len() != b.len() { return false; }
                for (k, av) in a.iter() {
                    if !b.get(k).map(|bv| av.deep_equal(bv)).unwrap_or(false) { return false; }
                }
                true
            }
            _ => false,
        }
    }

    pub fn as_f64(&self) -> f64 {
        match self {
            AvelynVal::Int(i) => *i as f64,
            AvelynVal::Float(f) => *f,
            AvelynVal::Bool(b) => if *b { 1.0 } else { 0.0 },
            _ => 0.0,
        }
    }

    pub fn as_i64(&self) -> i64 {
        match self {
            AvelynVal::Int(i) => *i,
            AvelynVal::Float(f) => *f as i64,
            AvelynVal::Bool(b) => if *b { 1 } else { 0 },
            _ => 0,
        }
    }

    pub fn as_str(&self) -> String { self.format() }

    pub fn to_number(&self) -> AvelynVal {
        match self {
            AvelynVal::Int(i) => AvelynVal::Int(*i),
            AvelynVal::Float(f) => AvelynVal::Float(*f),
            AvelynVal::Bool(b) => AvelynVal::Int(if *b { 1 } else { 0 }),
            AvelynVal::Str(s) => {
                if let Ok(i) = s.parse::<i64>() { AvelynVal::Int(i) }
                else if let Ok(f) = s.parse::<f64>() { AvelynVal::Float(f) }
                else { AvelynVal::Null }
            }
            _ => AvelynVal::Null,
        }
    }

    /// JSON stringify helper
    pub fn json_str(&self) -> String {
        match self {
            AvelynVal::Null => "null".into(),
            AvelynVal::Bool(b) => b.to_string(),
            AvelynVal::Int(i) => i.to_string(),
            AvelynVal::Float(f) => {
                if f.fract() == 0.0 && f.abs() < 1e15 { format!("{}", *f as i64) } else { f.to_string() }
            }
            AvelynVal::Str(s) => format!("\"{}\"", s.replace('"', "\\\"")),
            AvelynVal::List(l) => {
                let items: Vec<String> = l.borrow().iter().map(|v| v.json_str()).collect();
                format!("[{}]", items.join(","))
            }
            AvelynVal::Map(m) => {
                let pairs: Vec<String> = m.borrow().iter()
                    .map(|(k, v)| format!("\"{}\":{}", k, v.json_str())).collect();
                format!("{{{}}}", pairs.join(","))
            }
            _ => "null".into(),
        }
    }
}

impl fmt::Display for AvelynVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.format()) }
}

// Convenience constructors
impl AvelynVal {
    pub fn str(s: impl Into<String>) -> Self { AvelynVal::Str(Rc::new(s.into())) }
    pub fn list(v: Vec<AvelynVal>) -> Self { AvelynVal::List(Rc::new(RefCell::new(v))) }
    pub fn map(m: IndexMap<String, AvelynVal>) -> Self { AvelynVal::Map(Rc::new(RefCell::new(m))) }
}
