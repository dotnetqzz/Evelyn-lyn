// value.rs — AvelynVal runtime value type

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use std::collections::{HashMap, HashSet};

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
    pub annotations: IndexMap<String, AvelynVal>,
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
    Instance(Rc<RefCell<StructInstance>>),
    Variant(Rc<EnumVariantInstance>),
    Type(TypeDefinition),
    Module(Rc<RefCell<Module>>),
}

#[derive(Clone, Debug)]
pub struct Module {
    pub name: String,
    pub env: Rc<Env>,
    pub exports: HashSet<String>,
}

#[derive(Clone, Debug)]
pub struct StructInstance {
    pub type_name: String,
    pub fields: IndexMap<String, AvelynVal>,
}

#[derive(Clone, Debug)]
pub struct EnumVariantInstance {
    pub type_name: String,
    pub variant_name: String,
    pub values: Vec<AvelynVal>,
}

#[derive(Clone, Debug)]
pub enum TypeDefinition {
    Struct { name: String, fields: Vec<String>, annotations: IndexMap<String, AvelynVal> },
    Enum { name: String, variants: HashMap<String, (usize, Vec<String>)>, annotations: IndexMap<String, AvelynVal> },
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
            AvelynVal::Instance(inst) => {
                let inst = inst.borrow();
                let pairs: Vec<String> = inst.fields.iter()
                    .map(|(k, v)| format!("{}: {}", k, v.format())).collect();
                format!("{}({})", inst.type_name, pairs.join(", "))
            }
            AvelynVal::Variant(var) => {
                if var.values.is_empty() { format!("{}.{}", var.type_name, var.variant_name) }
                else {
                    let vals: Vec<String> = var.values.iter().map(|v| v.format()).collect();
                    format!("{}.{}({})", var.type_name, var.variant_name, vals.join(", "))
                }
            }
            AvelynVal::Type(def) => match def {
                TypeDefinition::Struct { name, .. } => format!("<struct {}>", name),
                TypeDefinition::Enum { name, .. } => format!("<enum {}>", name),
            }
            AvelynVal::Module(m) => format!("<module {}>", m.borrow().name),
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
            AvelynVal::Instance(_) => "instance", // or maybe return the actual type name if possible but type_name() returns &'static str
            AvelynVal::Variant(_) => "variant",
            AvelynVal::Type(_) => "type",
            AvelynVal::Module(_) => "module",
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
            (AvelynVal::Instance(a), AvelynVal::Instance(b)) => {
                let a = a.borrow(); let b = b.borrow();
                if a.type_name != b.type_name || a.fields.len() != b.fields.len() { return false; }
                for (k, av) in a.fields.iter() {
                    if !b.fields.get(k).map(|bv| av.deep_equal(bv)).unwrap_or(false) { return false; }
                }
                true
            }
            (AvelynVal::Variant(a), AvelynVal::Variant(b)) => {
                a.type_name == b.type_name && a.variant_name == b.variant_name &&
                a.values.len() == b.values.len() && a.values.iter().zip(b.values.iter()).all(|(x,y)| x.deep_equal(y))
            }
            (AvelynVal::Module(a), AvelynVal::Module(b)) => Rc::ptr_eq(a, b),
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

    pub fn marshal(&self) -> Vec<u8> {
        let mut out = Vec::new();
        match self {
            AvelynVal::Null => out.push(0),
            AvelynVal::Bool(b) => { out.push(1); out.push(if *b { 1 } else { 0 }); }
            AvelynVal::Int(i) => { out.push(2); out.extend_from_slice(&i.to_be_bytes()); }
            AvelynVal::Float(f) => { out.push(3); out.extend_from_slice(&f.to_be_bytes()); }
            AvelynVal::Str(s) => {
                out.push(4);
                let b = s.as_bytes();
                out.extend_from_slice(&(b.len() as u32).to_be_bytes());
                out.extend_from_slice(b);
            }
            AvelynVal::List(l) => {
                out.push(5);
                let l = l.borrow();
                out.extend_from_slice(&(l.len() as u32).to_be_bytes());
                for item in l.iter() { out.extend(item.marshal()); }
            }
            AvelynVal::Map(m) => {
                out.push(6);
                let m = m.borrow();
                out.extend_from_slice(&(m.len() as u32).to_be_bytes());
                for (k, v) in m.iter() {
                    let kb = k.as_bytes();
                    out.extend_from_slice(&(kb.len() as u32).to_be_bytes());
                    out.extend_from_slice(kb);
                    out.extend(v.marshal());
                }
            }
            AvelynVal::Instance(inst) => {
                out.push(7);
                let inst = inst.borrow();
                let tb = inst.type_name.as_bytes();
                out.extend_from_slice(&(tb.len() as u32).to_be_bytes());
                out.extend_from_slice(tb);
                out.extend_from_slice(&(inst.fields.len() as u32).to_be_bytes());
                for (k, v) in inst.fields.iter() {
                    let kb = k.as_bytes();
                    out.extend_from_slice(&(kb.len() as u32).to_be_bytes());
                    out.extend_from_slice(kb);
                    out.extend(v.marshal());
                }
            }
            AvelynVal::Variant(var) => {
                out.push(8);
                let tb = var.type_name.as_bytes();
                out.extend_from_slice(&(tb.len() as u32).to_be_bytes());
                out.extend_from_slice(tb);
                let vb = var.variant_name.as_bytes();
                out.extend_from_slice(&(vb.len() as u32).to_be_bytes());
                out.extend_from_slice(vb);
                out.extend_from_slice(&(var.values.len() as u32).to_be_bytes());
                for v in &var.values { out.extend(v.marshal()); }
            }
            _ => out.push(0), // Others serialized as null for now
        }
        out
    }

    pub fn unmarshal(bytes: &[u8]) -> (Self, usize) {
        if bytes.is_empty() { return (AvelynVal::Null, 0); }
        let tag = bytes[0];
        let mut pos = 1;
        match tag {
            0 => (AvelynVal::Null, 1),
            1 => (AvelynVal::Bool(bytes[1] != 0), 2),
            2 => {
                let i = i64::from_be_bytes(bytes[pos..pos+8].try_into().unwrap());
                (AvelynVal::Int(i), 9)
            }
            3 => {
                let f = f64::from_be_bytes(bytes[pos..pos+8].try_into().unwrap());
                (AvelynVal::Float(f), 9)
            }
            4 => {
                let len = u32::from_be_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
                pos += 4;
                let s = String::from_utf8_lossy(&bytes[pos..pos+len]).to_string();
                (AvelynVal::str(s), pos + len)
            }
            5 => {
                let count = u32::from_be_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
                pos += 4;
                let mut list = Vec::new();
                for _ in 0..count {
                    let (item, size) = Self::unmarshal(&bytes[pos..]);
                    list.push(item);
                    pos += size;
                }
                (AvelynVal::list(list), pos)
            }
            6 => {
                let count = u32::from_be_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
                pos += 4;
                let mut map = IndexMap::new();
                for _ in 0..count {
                    let klen = u32::from_be_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
                    pos += 4;
                    let k = String::from_utf8_lossy(&bytes[pos..pos+klen]).to_string();
                    pos += klen;
                    let (v, vsize) = Self::unmarshal(&bytes[pos..]);
                    map.insert(k, v);
                    pos += vsize;
                }
                (AvelynVal::map(map), pos)
            }
            7 => {
                let tlen = u32::from_be_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
                pos += 4;
                let type_name = String::from_utf8_lossy(&bytes[pos..pos+tlen]).to_string();
                pos += tlen;
                let flen = u32::from_be_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
                pos += 4;
                let mut fields = IndexMap::new();
                for _ in 0..flen {
                    let klen = u32::from_be_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
                    pos += 4;
                    let k = String::from_utf8_lossy(&bytes[pos..pos+klen]).to_string();
                    pos += klen;
                    let (v, vsize) = Self::unmarshal(&bytes[pos..]);
                    fields.insert(k, v);
                    pos += vsize;
                }
                (AvelynVal::Instance(Rc::new(RefCell::new(StructInstance { type_name, fields }))), pos)
            }
            8 => {
                let tlen = u32::from_be_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
                pos += 4;
                let type_name = String::from_utf8_lossy(&bytes[pos..pos+tlen]).to_string();
                pos += tlen;
                let vlen = u32::from_be_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
                pos += 4;
                let variant_name = String::from_utf8_lossy(&bytes[pos..pos+vlen]).to_string();
                pos += vlen;
                let vcount = u32::from_be_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
                pos += 4;
                let mut values = Vec::new();
                for _ in 0..vcount {
                    let (v, vsize) = Self::unmarshal(&bytes[pos..]);
                    values.push(v);
                    pos += vsize;
                }
                (AvelynVal::Variant(Rc::new(EnumVariantInstance { type_name, variant_name, values })), pos)
            }
            _ => (AvelynVal::Null, 1),
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
