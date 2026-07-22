// vm.rs — Stack machine interpreter

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use indexmap::IndexMap;

use crate::bytecode::{Module, Proto};
use crate::value::{NativeFn, SylError, SylVal};
use crate::verifier;

// ---------------------------------------------------------------------------
// Opcodes (must match Instruction.swift exactly)
// ---------------------------------------------------------------------------

pub(crate) const OP_LOAD_CONST:    u8 = 0x01;
pub(crate) const OP_LOAD_NULL:     u8 = 0x02;
pub(crate) const OP_LOAD_TRUE:     u8 = 0x03;
pub(crate) const OP_LOAD_FALSE:    u8 = 0x04;
pub(crate) const OP_LOAD_VAR:      u8 = 0x05;
pub(crate) const OP_STORE_VAR:     u8 = 0x06;
pub(crate) const OP_LOAD_GLOBAL:   u8 = 0x07;
pub(crate) const OP_STORE_GLOBAL:  u8 = 0x08;
pub(crate) const OP_DELETE_VAR:    u8 = 0x09;
pub(crate) const OP_POP:           u8 = 0x0A;
pub(crate) const OP_DUP:           u8 = 0x0B;
pub(crate) const OP_SWAP:          u8 = 0x0C;
pub(crate) const OP_ADD:           u8 = 0x10;
pub(crate) const OP_SUB:           u8 = 0x11;
pub(crate) const OP_MUL:           u8 = 0x12;
pub(crate) const OP_DIV:           u8 = 0x13;
pub(crate) const OP_MOD:           u8 = 0x14;
pub(crate) const OP_POW:           u8 = 0x15;
pub(crate) const OP_FLOORDIV:      u8 = 0x16;
pub(crate) const OP_NEG:           u8 = 0x17;
pub(crate) const OP_EQ:            u8 = 0x20;
pub(crate) const OP_NEQ:           u8 = 0x21;
pub(crate) const OP_LT:            u8 = 0x22;
pub(crate) const OP_GT:            u8 = 0x23;
pub(crate) const OP_LTE:           u8 = 0x24;
pub(crate) const OP_GTE:           u8 = 0x25;
pub(crate) const OP_NOT:           u8 = 0x30;
pub(crate) const OP_AND:           u8 = 0x31;
pub(crate) const OP_OR:            u8 = 0x32;
pub(crate) const OP_BAND:          u8 = 0x38;
pub(crate) const OP_BOR:           u8 = 0x39;
pub(crate) const OP_BXOR:          u8 = 0x3A;
pub(crate) const OP_BNOT:          u8 = 0x3B;
pub(crate) const OP_SHL:           u8 = 0x3C;
pub(crate) const OP_SHR:           u8 = 0x3D;
pub(crate) const OP_USHR:          u8 = 0x3E;
pub(crate) const OP_JUMP:          u8 = 0x40;
pub(crate) const OP_JUMP_IF_F:     u8 = 0x41;
pub(crate) const OP_JUMP_IF_T:     u8 = 0x42;
pub(crate) const OP_JUMP_IF_F_POP: u8 = 0x43;
pub(crate) const OP_JUMP_IF_T_POP: u8 = 0x44;
pub(crate) const OP_MAKE_FUNC:     u8 = 0x50;
pub(crate) const OP_CALL:          u8 = 0x51;
pub(crate) const OP_CALL_NATIVE:   u8 = 0x52;
pub(crate) const OP_RETURN:        u8 = 0x53;
pub(crate) const OP_RETURN_NULL:   u8 = 0x54;
pub(crate) const OP_MAKE_LIST:     u8 = 0x60;
pub(crate) const OP_MAKE_MAP:      u8 = 0x61;
pub(crate) const OP_GET_INDEX:     u8 = 0x62;
pub(crate) const OP_SET_INDEX:     u8 = 0x63;
pub(crate) const OP_SPREAD:        u8 = 0x64;
pub(crate) const OP_GET_ATTR:      u8 = 0x68;
pub(crate) const OP_SET_ATTR:      u8 = 0x69;
pub(crate) const OP_THROW:         u8 = 0x70;
pub(crate) const OP_TRY_BEGIN:     u8 = 0x71;
pub(crate) const OP_TRY_END:       u8 = 0x72;
pub(crate) const OP_CATCH_STORE:   u8 = 0x73;
pub(crate) const OP_STR_CONCAT:    u8 = 0x78;
pub(crate) const OP_FORMAT_VAL:    u8 = 0x79;
pub(crate) const OP_IMPORT:        u8 = 0x80;
pub(crate) const OP_LINE:          u8 = 0xFE;
pub(crate) const OP_NOP:           u8 = 0xFF;

// ---------------------------------------------------------------------------
// Call frame
// ---------------------------------------------------------------------------

struct Frame {
    proto: Rc<Proto>,
    ip: usize,
    locals: Vec<SylVal>,
    /// Stack of (catch_pc) for TRY_BEGIN/TRY_END within this frame
    try_stack: Vec<usize>,
    /// Caught value to push after jumping to catch
    caught: Option<SylVal>,
}

impl Frame {
    fn new(proto: Rc<Proto>, args: Vec<SylVal>) -> Self {
        let local_count = proto.local_count as usize;
        let arity = proto.arity as usize;
        let is_variadic = proto.is_variadic;

        let mut locals = vec![SylVal::Null; local_count];

        if is_variadic && arity > 0 {
            let normal = arity - 1;
            for (i, v) in args.iter().take(normal).enumerate() {
                if i < local_count { locals[i] = v.clone(); }
            }
            // gather rest into list
            let rest: Vec<SylVal> = args.into_iter().skip(normal).collect();
            if normal < local_count {
                locals[normal] = SylVal::List(Rc::new(RefCell::new(rest)));
            }
        } else {
            for (i, v) in args.into_iter().enumerate() {
                if i < local_count { locals[i] = v; }
            }
        }

        Frame { proto, ip: 0, locals, try_stack: Vec::new(), caught: None }
    }
}

// ---------------------------------------------------------------------------
// The VM
// ---------------------------------------------------------------------------

pub struct Vm {
    pub globals: HashMap<String, SylVal>,
    pub natives: HashMap<String, NativeFn>,
    stack: Vec<SylVal>,
}

impl Vm {
    pub fn new() -> Self {
        Vm { globals: HashMap::new(), natives: HashMap::new(), stack: Vec::with_capacity(256) }
    }

    pub fn register_native(&mut self, name: &str, f: NativeFn) {
        self.natives.insert(name.to_string(), f);
        self.globals.insert(name.to_string(), SylVal::Native(f));
    }

    pub fn run_module(&mut self, module: &Module) -> Result<(), String> {
        if module.protos.is_empty() { return Ok(()); }

        verifier::verify_module(module)?;

        let main_proto = module.protos.last()
            .ok_or_else(|| "Bytecode Error: module contains no function prototypes".to_string())?
            .clone();
        let result = self.exec_frame(module, main_proto, vec![]);
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }

    // -----------------------------------------------------------------------
    // Frame execution
    // -----------------------------------------------------------------------

    fn exec_frame(
        &mut self,
        module: &Module,
        proto: Rc<Proto>,
        args: Vec<SylVal>,
    ) -> Result<SylVal, SylError> {
        let mut frame = Frame::new(proto, args);

        macro_rules! fetch_u8 {
            () => {{
                let b = frame.proto.code[frame.ip];
                frame.ip += 1;
                b
            }};
        }
        macro_rules! fetch_u16 {
            () => {{
                let hi = frame.proto.code[frame.ip] as u16;
                let lo = frame.proto.code[frame.ip + 1] as u16;
                frame.ip += 2;
                (hi << 8) | lo
            }};
        }
        macro_rules! fetch_i16 {
            () => {{
                fetch_u16!() as i16
            }};
        }

        loop {
            let op = fetch_u8!();

            match op {
                // ── Constants ────────────────────────────────────────────
                OP_LOAD_CONST => {
                    let idx = fetch_u16!() as usize;
                    self.stack.push(module.pool[idx].clone());
                }
                OP_LOAD_NULL  => self.stack.push(SylVal::Null),
                OP_LOAD_TRUE  => self.stack.push(SylVal::Bool(true)),
                OP_LOAD_FALSE => self.stack.push(SylVal::Bool(false)),

                // ── Variables ─────────────────────────────────────────────
                OP_LOAD_VAR => {
                    let slot = fetch_u16!() as usize;
                    self.stack.push(frame.locals[slot].clone());
                }
                OP_STORE_VAR => {
                    let slot = fetch_u16!() as usize;
                    let v = self.stack.pop().unwrap_or(SylVal::Null);
                    frame.locals[slot] = v;
                }
                OP_DELETE_VAR => {
                    let slot = fetch_u16!() as usize;
                    frame.locals[slot] = SylVal::Null;
                }
                OP_LOAD_GLOBAL => {
                    let idx = fetch_u16!() as usize;
                    let key = match &module.pool[idx] {
                        SylVal::Str(s) => s.as_ref().clone(),
                        other => other.format(),
                    };
                    let v = self.globals.get(&key).cloned().unwrap_or(SylVal::Null);
                    self.stack.push(v);
                }
                OP_STORE_GLOBAL => {
                    let idx = fetch_u16!() as usize;
                    let key = match &module.pool[idx] {
                        SylVal::Str(s) => s.as_ref().clone(),
                        other => other.format(),
                    };
                    let v = self.stack.pop().unwrap_or(SylVal::Null);
                    self.globals.insert(key, v);
                }

                // ── Stack ops ─────────────────────────────────────────────
                OP_POP  => { self.stack.pop(); }
                OP_DUP  => { let v = self.stack.last().cloned().unwrap_or(SylVal::Null); self.stack.push(v); }
                OP_SWAP => {
                    let len = self.stack.len();
                    if len >= 2 { self.stack.swap(len - 1, len - 2); }
                }

                // ── Arithmetic ────────────────────────────────────────────
                OP_ADD => {
                    let b = self.stack.pop().unwrap_or(SylVal::Null);
                    let a = self.stack.pop().unwrap_or(SylVal::Null);
                    let r = match (&a, &b) {
                        (SylVal::Int(x), SylVal::Int(y)) => SylVal::Int(x.wrapping_add(*y)),
                        (SylVal::Float(x), SylVal::Float(y)) => SylVal::Float(x + y),
                        (SylVal::Int(x), SylVal::Float(y)) => SylVal::Float(*x as f64 + y),
                        (SylVal::Float(x), SylVal::Int(y)) => SylVal::Float(x + *y as f64),
                        (SylVal::Str(x), SylVal::Str(y)) => SylVal::Str(Rc::new(format!("{}{}", x, y))),
                        (SylVal::Str(x), _) => SylVal::Str(Rc::new(format!("{}{}", x, b.format()))),
                        (_, SylVal::Str(y)) => SylVal::Str(Rc::new(format!("{}{}", a.format(), y))),
                        _ => return Err(SylError::fmt(format!("cannot add {} and {}", a.type_name(), b.type_name()))),
                    };
                    self.stack.push(r);
                }
                OP_SUB => {
                    let b = self.stack.pop().unwrap_or(SylVal::Null);
                    let a = self.stack.pop().unwrap_or(SylVal::Null);
                    self.stack.push(arith_op(&a, &b, |x,y| x-y, |x,y| x-y)?);
                }
                OP_MUL => {
                    let b = self.stack.pop().unwrap_or(SylVal::Null);
                    let a = self.stack.pop().unwrap_or(SylVal::Null);
                    self.stack.push(arith_op(&a, &b, |x,y| x.wrapping_mul(y), |x,y| x*y)?);
                }
                OP_DIV => {
                    let b = self.stack.pop().unwrap_or(SylVal::Null);
                    let a = self.stack.pop().unwrap_or(SylVal::Null);
                    let af = a.as_f64(); let bf = b.as_f64();
                    self.stack.push(SylVal::Float(af / bf));
                }
                OP_MOD => {
                    let b = self.stack.pop().unwrap_or(SylVal::Null);
                    let a = self.stack.pop().unwrap_or(SylVal::Null);
                    let r = match (&a, &b) {
                        (SylVal::Int(x), SylVal::Int(y)) if *y != 0 => SylVal::Int(x % y),
                        _ => SylVal::Float(a.as_f64() % b.as_f64()),
                    };
                    self.stack.push(r);
                }
                OP_POW => {
                    let b = self.stack.pop().unwrap_or(SylVal::Null);
                    let a = self.stack.pop().unwrap_or(SylVal::Null);
                    self.stack.push(SylVal::Float(a.as_f64().powf(b.as_f64())));
                }
                OP_FLOORDIV => {
                    let b = self.stack.pop().unwrap_or(SylVal::Null);
                    let a = self.stack.pop().unwrap_or(SylVal::Null);
                    let r = match (&a, &b) {
                        (SylVal::Int(x), SylVal::Int(y)) if *y != 0 => SylVal::Int(x.div_euclid(*y)),
                        _ => SylVal::Float((a.as_f64() / b.as_f64()).floor()),
                    };
                    self.stack.push(r);
                }
                OP_NEG => {
                    let a = self.stack.pop().unwrap_or(SylVal::Null);
                    let r = match a {
                        SylVal::Int(i) => SylVal::Int(-i),
                        SylVal::Float(f) => SylVal::Float(-f),
                        _ => return Err(SylError::msg("unary minus on non-number")),
                    };
                    self.stack.push(r);
                }

                // ── Comparison ────────────────────────────────────────────
                OP_EQ => {
                    let b = self.stack.pop().unwrap_or(SylVal::Null);
                    let a = self.stack.pop().unwrap_or(SylVal::Null);
                    self.stack.push(SylVal::Bool(a.deep_equal(&b)));
                }
                OP_NEQ => {
                    let b = self.stack.pop().unwrap_or(SylVal::Null);
                    let a = self.stack.pop().unwrap_or(SylVal::Null);
                    self.stack.push(SylVal::Bool(!a.deep_equal(&b)));
                }
                OP_LT => {
                    let b = self.stack.pop().unwrap_or(SylVal::Null);
                    let a = self.stack.pop().unwrap_or(SylVal::Null);
                    self.stack.push(SylVal::Bool(cmp_lt(&a, &b)));
                }
                OP_GT => {
                    let b = self.stack.pop().unwrap_or(SylVal::Null);
                    let a = self.stack.pop().unwrap_or(SylVal::Null);
                    self.stack.push(SylVal::Bool(cmp_lt(&b, &a)));
                }
                OP_LTE => {
                    let b = self.stack.pop().unwrap_or(SylVal::Null);
                    let a = self.stack.pop().unwrap_or(SylVal::Null);
                    self.stack.push(SylVal::Bool(!cmp_lt(&b, &a)));
                }
                OP_GTE => {
                    let b = self.stack.pop().unwrap_or(SylVal::Null);
                    let a = self.stack.pop().unwrap_or(SylVal::Null);
                    self.stack.push(SylVal::Bool(!cmp_lt(&a, &b)));
                }

                // ── Logical ───────────────────────────────────────────────
                OP_NOT => {
                    let a = self.stack.pop().unwrap_or(SylVal::Null);
                    self.stack.push(SylVal::Bool(!a.is_truthy()));
                }
                OP_AND => {
                    // short-circuit AND: pop both, push result
                    let b = self.stack.pop().unwrap_or(SylVal::Null);
                    let a = self.stack.pop().unwrap_or(SylVal::Null);
                    self.stack.push(if a.is_truthy() { b } else { a });
                }
                OP_OR => {
                    let b = self.stack.pop().unwrap_or(SylVal::Null);
                    let a = self.stack.pop().unwrap_or(SylVal::Null);
                    self.stack.push(if a.is_truthy() { a } else { b });
                }

                // ── Bitwise ───────────────────────────────────────────────
                OP_BAND => { let b=self.stack.pop().unwrap_or(SylVal::Null); let a=self.stack.pop().unwrap_or(SylVal::Null); self.stack.push(SylVal::Int(a.as_i64() & b.as_i64())); }
                OP_BOR  => { let b=self.stack.pop().unwrap_or(SylVal::Null); let a=self.stack.pop().unwrap_or(SylVal::Null); self.stack.push(SylVal::Int(a.as_i64() | b.as_i64())); }
                OP_BXOR => { let b=self.stack.pop().unwrap_or(SylVal::Null); let a=self.stack.pop().unwrap_or(SylVal::Null); self.stack.push(SylVal::Int(a.as_i64() ^ b.as_i64())); }
                OP_BNOT => { let a=self.stack.pop().unwrap_or(SylVal::Null); self.stack.push(SylVal::Int(!a.as_i64())); }
                OP_SHL  => { let b=self.stack.pop().unwrap_or(SylVal::Null); let a=self.stack.pop().unwrap_or(SylVal::Null); self.stack.push(SylVal::Int(a.as_i64() << (b.as_i64() & 63))); }
                OP_SHR  => { let b=self.stack.pop().unwrap_or(SylVal::Null); let a=self.stack.pop().unwrap_or(SylVal::Null); self.stack.push(SylVal::Int(a.as_i64() >> (b.as_i64() & 63))); }
                OP_USHR => {
                    let b = self.stack.pop().unwrap_or(SylVal::Null);
                    let a = self.stack.pop().unwrap_or(SylVal::Null);
                    self.stack.push(SylVal::Int(((a.as_i64() as u64) >> (b.as_i64() & 63)) as i64));
                }

                // ── Jumps ─────────────────────────────────────────────────
                OP_JUMP => {
                    let off = fetch_i16!() as isize;
                    frame.ip = (frame.ip as isize + off) as usize;
                }
                OP_JUMP_IF_F => {
                    let off = fetch_i16!() as isize;
                    if !self.stack.last().map(|v| v.is_truthy()).unwrap_or(false) {
                        frame.ip = (frame.ip as isize + off) as usize;
                    }
                }
                OP_JUMP_IF_T => {
                    let off = fetch_i16!() as isize;
                    if self.stack.last().map(|v| v.is_truthy()).unwrap_or(false) {
                        frame.ip = (frame.ip as isize + off) as usize;
                    }
                }
                OP_JUMP_IF_F_POP => {
                    let off = fetch_i16!() as isize;
                    let v = self.stack.pop().unwrap_or(SylVal::Null);
                    if !v.is_truthy() { frame.ip = (frame.ip as isize + off) as usize; }
                }
                OP_JUMP_IF_T_POP => {
                    let off = fetch_i16!() as isize;
                    let v = self.stack.pop().unwrap_or(SylVal::Null);
                    if v.is_truthy() { frame.ip = (frame.ip as isize + off) as usize; }
                }

                // ── Collections ───────────────────────────────────────────
                OP_MAKE_LIST => {
                    let n = fetch_u16!() as usize;
                    let start = self.stack.len().saturating_sub(n);
                    let items: Vec<SylVal> = self.stack.drain(start..).collect();
                    self.stack.push(SylVal::List(Rc::new(RefCell::new(items))));
                }
                OP_MAKE_MAP => {
                    let n = fetch_u16!() as usize;
                    let start = self.stack.len().saturating_sub(n * 2);
                    let pairs: Vec<SylVal> = self.stack.drain(start..).collect();
                    let mut map = IndexMap::new();
                    for chunk in pairs.chunks(2) {
                        if chunk.len() == 2 {
                            map.insert(chunk[0].format(), chunk[1].clone());
                        }
                    }
                    self.stack.push(SylVal::Map(Rc::new(RefCell::new(map))));
                }
                OP_GET_INDEX => {
                    let idx = self.stack.pop().unwrap_or(SylVal::Null);
                    let obj = self.stack.pop().unwrap_or(SylVal::Null);
                    let result = match &obj {
                        SylVal::List(l) => {
                            let i = idx.as_i64();
                            let items = l.borrow();
                            let len = items.len() as i64;
                            let i = if i < 0 { len + i } else { i };
                            if i >= 0 && (i as usize) < items.len() {
                                items[i as usize].clone()
                            } else { SylVal::Null }
                        }
                        SylVal::Map(m) => {
                            m.borrow().get(&idx.format()).cloned().unwrap_or(SylVal::Null)
                        }
                        SylVal::Str(s) => {
                            let i = idx.as_i64();
                            let char_count = s.chars().count() as i64;
                            let i = if i < 0 { char_count + i } else { i };
                            if i >= 0 {
                                if let Some(ch) = s.chars().nth(i as usize) {
                                    SylVal::Str(Rc::new(ch.to_string()))
                                } else { SylVal::Null }
                            } else { SylVal::Null }
                        }
                        _ => SylVal::Null,
                    };
                    self.stack.push(result);
                }
                OP_SET_INDEX => {
                    let val = self.stack.pop().unwrap_or(SylVal::Null);
                    let idx = self.stack.pop().unwrap_or(SylVal::Null);
                    let obj = self.stack.pop().unwrap_or(SylVal::Null);
                    match &obj {
                        SylVal::List(l) => {
                            let i = idx.as_i64();
                            let mut items = l.borrow_mut();
                            let len = items.len() as i64;
                            let i = if i < 0 { len + i } else { i };
                            if i >= 0 && (i as usize) < items.len() { items[i as usize] = val; }
                        }
                        SylVal::Map(m) => {
                            m.borrow_mut().insert(idx.format(), val);
                        }
                        _ => {}
                    }
                }
                OP_SPREAD => {
                    let list = self.stack.pop().unwrap_or(SylVal::Null);
                    if let SylVal::List(l) = list {
                        for item in l.borrow().iter() {
                            self.stack.push(item.clone());
                        }
                    }
                }
                OP_GET_ATTR => {
                    let idx = fetch_u16!() as usize;
                    let key = match &module.pool[idx] {
                        SylVal::Str(s) => s.as_ref().clone(),
                        other => other.format(),
                    };
                    let obj = self.stack.pop().unwrap_or(SylVal::Null);
                    let result = match &obj {
                        SylVal::Map(m) => m.borrow().get(&key).cloned().unwrap_or(SylVal::Null),
                        _ => SylVal::Null,
                    };
                    self.stack.push(result);
                }
                OP_SET_ATTR => {
                    let idx = fetch_u16!() as usize;
                    let key = match &module.pool[idx] {
                        SylVal::Str(s) => s.as_ref().clone(),
                        other => other.format(),
                    };
                    let val = self.stack.pop().unwrap_or(SylVal::Null);
                    let obj = self.stack.pop().unwrap_or(SylVal::Null);
                    if let SylVal::Map(m) = &obj {
                        m.borrow_mut().insert(key, val);
                    }
                }

                // ── String ops ────────────────────────────────────────────
                OP_FORMAT_VAL => {
                    let v = self.stack.pop().unwrap_or(SylVal::Null);
                    self.stack.push(SylVal::Str(Rc::new(v.format())));
                }
                OP_STR_CONCAT => {
                    let b = self.stack.pop().unwrap_or(SylVal::Null);
                    let a = self.stack.pop().unwrap_or(SylVal::Null);
                    self.stack.push(SylVal::Str(Rc::new(format!("{}{}", a.format(), b.format()))));
                }

                // ── Exception handling ────────────────────────────────────
                OP_THROW => {
                    let v = self.stack.pop().unwrap_or(SylVal::Null);
                    return Err(SylError { value: v });
                }
                OP_TRY_BEGIN => {
                    let catch_off = fetch_u16!() as usize;
                    frame.try_stack.push(catch_off);
                }
                OP_TRY_END => {
                    frame.try_stack.pop();
                }
                OP_CATCH_STORE => {
                    let slot = fetch_u16!() as usize;
                    let v = frame.caught.take().unwrap_or(SylVal::Null);
                    frame.locals[slot] = v;
                }

                // ── Functions ─────────────────────────────────────────────
                OP_MAKE_FUNC => {
                    let proto_idx = fetch_u16!() as usize;
                    let proto = module.protos[proto_idx].clone();
                    self.stack.push(SylVal::Func(proto));
                }
                OP_CALL => {
                    let argc = fetch_u16!() as usize;
                    // stack: ... callee arg0 arg1 ... arg(n-1)
                    let stack_len = self.stack.len();
                    if stack_len < argc + 1 {
                        return Err(SylError::msg("call: stack underflow"));
                    }
                    let callee = self.stack[stack_len - argc - 1].clone();
                    let args: Vec<SylVal> = self.stack.drain(stack_len - argc..).collect();
                    self.stack.pop(); // remove callee

                    match callee {
                        SylVal::Func(proto) => {
                            let ret = match self.exec_frame(module, proto, args) {
                                Ok(v) => v,
                                Err(e) => {
                                    // propagate to nearest try in this frame
                                    if let Some(catch_pc) = frame.try_stack.pop() {
                                        frame.caught = Some(e.value);
                                        frame.ip = catch_pc;
                                        self.stack.push(SylVal::Null); // placeholder
                                        continue;
                                    }
                                    return Err(e);
                                }
                            };
                            self.stack.push(ret);
                        }
                        SylVal::Native(f) => {
                            let ret = match f(self, &args) {
                                Ok(v) => v,
                                Err(e) => {
                                    if let Some(catch_pc) = frame.try_stack.pop() {
                                        frame.caught = Some(e.value);
                                        frame.ip = catch_pc;
                                        self.stack.push(SylVal::Null);
                                        continue;
                                    }
                                    return Err(e);
                                }
                            };
                            self.stack.push(ret);
                        }
                        other => {
                            let msg = format!("call of non-function value ({})", other.type_name());
                            if let Some(catch_pc) = frame.try_stack.pop() {
                                frame.caught = Some(SylVal::Str(Rc::new(msg)));
                                frame.ip = catch_pc;
                                self.stack.push(SylVal::Null);
                            } else {
                                return Err(SylError::fmt(msg));
                            }
                        }
                    }
                }
                OP_CALL_NATIVE => {
                    let idx = fetch_u16!() as usize;
                    let argc = fetch_u8!() as usize;
                    let name = &module.native_names[idx];
                    let f = self.natives.get(name).copied();
                    let stack_len = self.stack.len();
                    let args: Vec<SylVal> = self.stack.drain(stack_len - argc..).collect();
                    match f {
                        Some(f) => {
                            let ret = f(self, &args)?;
                            self.stack.push(ret);
                        }
                        None => {
                            if let Some(SylVal::Func(proto)) = self.globals.get(name).cloned() {
                                let ret = match self.exec_frame(module, proto, args) {
                                    Ok(v) => v,
                                    Err(e) => return Err(e),
                                };
                                self.stack.push(ret);
                            } else {
                                return Err(SylError::fmt(format!("undefined native or function '{}'", name)));
                            }
                        }
                    }
                }
                OP_RETURN => {
                    return Ok(self.stack.pop().unwrap_or(SylVal::Null));
                }
                OP_RETURN_NULL => {
                    return Ok(SylVal::Null);
                }

                // ── Import ────────────────────────────────────────────────
                OP_IMPORT => {
                    let _idx = fetch_u16!();
                    // stdlib imports are handled at vm startup; dynamic not supported yet
                }

                // ── Debug/misc ────────────────────────────────────────────
                OP_LINE => { let _ = fetch_u16!(); }
                OP_NOP  => {}

                _ => {
                    let pc = frame.ip - 1;
                    return Err(SylError::fmt(format!("unknown opcode 0x{:02X} at pc={}", op, pc)));
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Call a Sylvel function from Rust (used by stdlib)
    // -----------------------------------------------------------------------
    pub fn call_value(&mut self, module: &Module, callee: &SylVal, args: &[SylVal]) -> Result<SylVal, SylError> {
        match callee {
            SylVal::Func(proto) => self.exec_frame(module, proto.clone(), args.to_vec()),
            SylVal::Native(f) => f(self, args),
            _ => Ok(SylVal::Null),
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn arith_op(a: &SylVal, b: &SylVal, int_op: fn(i64, i64) -> i64, flt_op: fn(f64, f64) -> f64) -> Result<SylVal, SylError> {
    match (a, b) {
        (SylVal::Int(x), SylVal::Int(y)) => Ok(SylVal::Int(int_op(*x, *y))),
        (SylVal::Float(x), SylVal::Float(y)) => Ok(SylVal::Float(flt_op(*x, *y))),
        (SylVal::Int(x), SylVal::Float(y)) => Ok(SylVal::Float(flt_op(*x as f64, *y))),
        (SylVal::Float(x), SylVal::Int(y)) => Ok(SylVal::Float(flt_op(*x, *y as f64))),
        _ => Err(SylError::fmt(format!("cannot operate on {} and {}", a.type_name(), b.type_name()))),
    }
}

fn cmp_lt(a: &SylVal, b: &SylVal) -> bool {
    match (a, b) {
        (SylVal::Int(x), SylVal::Int(y)) => x < y,
        (SylVal::Float(x), SylVal::Float(y)) => x < y,
        (SylVal::Int(x), SylVal::Float(y)) => (*x as f64) < *y,
        (SylVal::Float(x), SylVal::Int(y)) => *x < (*y as f64),
        (SylVal::Str(x), SylVal::Str(y)) => x.as_str() < y.as_str(),
        _ => false,
    }
}
