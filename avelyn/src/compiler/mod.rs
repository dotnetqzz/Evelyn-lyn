// compiler/mod.rs — Compiler frontend & AST-to-bytecode translator

pub mod instruction;
pub mod writer;

use std::collections::HashSet;
use crate::ast::ASTNode;
use crate::compiler::instruction::Opcode;

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
}

pub struct ConstantPool {
    pub entries: Vec<Constant>,
}

impl ConstantPool {
    pub fn new() -> Self { ConstantPool { entries: Vec::new() } }

    pub fn add_string(&mut self, s: impl Into<String>) -> u16 {
        let s = s.into();
        if let Some(idx) = self.entries.iter().position(|e| e == &Constant::Str(s.clone())) {
            return idx as u16;
        }
        self.entries.push(Constant::Str(s));
        (self.entries.len() - 1) as u16
    }

    pub fn add_int(&mut self, i: i64) -> u16 {
        if let Some(idx) = self.entries.iter().position(|e| e == &Constant::Int(i)) {
            return idx as u16;
        }
        self.entries.push(Constant::Int(i));
        (self.entries.len() - 1) as u16
    }

    pub fn add_double(&mut self, f: f64) -> u16 {
        if let Some(idx) = self.entries.iter().position(|e| e == &Constant::Float(f)) {
            return idx as u16;
        }
        self.entries.push(Constant::Float(f));
        (self.entries.len() - 1) as u16
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let count = self.entries.len() as u32;
        out.extend_from_slice(&count.to_be_bytes());
        for entry in &self.entries {
            match entry {
                Constant::Null => out.push(0x00),
                Constant::Bool(b) => {
                    out.push(0x01);
                    out.push(if *b { 1 } else { 0 });
                }
                Constant::Int(i) => {
                    out.push(0x02);
                    out.extend_from_slice(&i.to_be_bytes());
                }
                Constant::Float(f) => {
                    out.push(0x03);
                    out.extend_from_slice(&f.to_be_bytes());
                }
                Constant::Str(s) => {
                    out.push(0x04);
                    let bytes = s.as_bytes();
                    out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
                    out.extend_from_slice(bytes);
                }
            }
        }
        out
    }
}

pub struct FunctionProto {
    pub name: String,
    pub arity: u8,
    pub is_variadic: bool,
    pub local_count: u16,
    pub code: Vec<u8>,
    pub line_map: Vec<(u32, u32)>,
    pub locals: Vec<(String, u16, bool)>, // (name, slot, immutable)
    pub scope_depth: usize,
}

impl FunctionProto {
    pub fn new(name: impl Into<String>) -> Self {
        FunctionProto {
            name: name.into(),
            arity: 0,
            is_variadic: false,
            local_count: 0,
            code: Vec::new(),
            line_map: Vec::new(),
            locals: Vec::new(),
            scope_depth: 0,
        }
    }

    pub fn declare_local(&mut self, name: &str, immutable: bool) -> u16 {
        let slot = self.local_count;
        self.local_count += 1;
        self.locals.push((name.to_string(), slot, immutable));
        slot
    }

    pub fn resolve_local(&self, name: &str) -> Option<u16> {
        for (n, slot, _) in self.locals.iter().rev() {
            if n == name { return Some(*slot); }
        }
        None
    }
}

pub struct ModuleState {
    pub pool: ConstantPool,
    pub native_table: Vec<String>,
    pub protos: Vec<FunctionProto>,
}

impl ModuleState {
    pub fn new() -> Self {
        ModuleState {
            pool: ConstantPool::new(),
            native_table: Vec::new(),
            protos: Vec::new(),
        }
    }

    pub fn native_index(&mut self, name: &str) -> u16 {
        if let Some(idx) = self.native_table.iter().position(|n| n == name) {
            return idx as u16;
        }
        self.native_table.push(name.to_string());
        (self.native_table.len() - 1) as u16
    }

    pub fn global_index(&mut self, name: &str) -> u16 {
        self.pool.add_string(name)
    }
}

pub struct Compiler {
    pub module: ModuleState,
    function_stack: Vec<FunctionProto>,
    immutable_globals: HashSet<String>,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            module: ModuleState::new(),
            function_stack: Vec::new(),
            immutable_globals: HashSet::new(),
        }
    }

    fn current(&mut self) -> &mut FunctionProto {
        self.function_stack.last_mut().unwrap()
    }

    fn emit(&mut self, op: Opcode) {
        self.current().code.push(op as u8);
    }

    fn emit_u16(&mut self, op: Opcode, operand: u16) {
        self.current().code.push(op as u8);
        self.current().code.extend_from_slice(&operand.to_be_bytes());
    }

    // CallNative needs both the native index AND how many args are already
    // sitting on the stack, otherwise the VM has no way to know how many
    // values to pop before invoking the native function.
    fn emit_call_native(&mut self, native_idx: u16, argc: u8) {
        self.current().code.push(Opcode::CallNative as u8);
        self.current().code.extend_from_slice(&native_idx.to_be_bytes());
        self.current().code.push(argc);
    }

    pub fn compile(mut self, ast: &[ASTNode]) -> Result<ModuleState, String> {
        let main_proto = FunctionProto::new("<main>");
        self.function_stack.push(main_proto);
        for node in ast { self.compile_node(node)?; }
        self.emit(Opcode::ReturnNull);
        let finished = self.function_stack.pop().unwrap();
        self.module.protos.push(finished);
        Ok(self.module)
    }

    fn compile_node(&mut self, node: &ASTNode) -> Result<(), String> {
        match node {
            ASTNode::Null => self.emit(Opcode::LoadNull),
            ASTNode::Bool(b) => self.emit(if *b { Opcode::LoadTrue } else { Opcode::LoadFalse }),
            ASTNode::Int(i) => {
                let idx = self.module.pool.add_int(*i);
                self.emit_u16(Opcode::LoadConst, idx);
            }
            ASTNode::Float(f) => {
                let idx = self.module.pool.add_double(*f);
                self.emit_u16(Opcode::LoadConst, idx);
            }
            ASTNode::Str(s) => {
                let idx = self.module.pool.add_string(s);
                self.emit_u16(Opcode::LoadConst, idx);
            }
            ASTNode::Var(name) => {
                if let Some(slot) = self.current().resolve_local(name) {
                    self.emit_u16(Opcode::LoadVar, slot);
                } else {
                    let idx = self.module.global_index(name);
                    self.emit_u16(Opcode::LoadGlobal, idx);
                }
            }
            ASTNode::Decl { name, value, mutable } => {
                self.compile_node(value)?;
                if self.function_stack.len() > 1 || self.current().scope_depth > 0 {
                    let slot = self.current().declare_local(name, !mutable);
                    self.emit_u16(Opcode::StoreVar, slot);
                } else {
                    let idx = self.module.global_index(name);
                    self.emit_u16(Opcode::StoreGlobal, idx);
                    if !mutable { self.immutable_globals.insert(name.clone()); }
                }
            }
            ASTNode::Assign { name, value } => {
                self.compile_node(value)?;
                if let Some(slot) = self.current().resolve_local(name) {
                    self.emit_u16(Opcode::StoreVar, slot);
                } else {
                    let idx = self.module.global_index(name);
                    self.emit_u16(Opcode::StoreGlobal, idx);
                }
            }
            ASTNode::PrintCall(arg) => {
                self.compile_node(arg)?;
                let idx = self.module.native_index("print");
                self.emit_call_native(idx, 1);
            }
            ASTNode::Return(expr) => {
                self.compile_node(expr)?;
                self.emit(Opcode::Return);
            }
            ASTNode::BinOp { left, op, right } => {
                self.compile_node(left)?;
                self.compile_node(right)?;
                match op.as_str() {
                    "+" => self.emit(Opcode::Add),
                    "-" => self.emit(Opcode::Sub),
                    "*" => self.emit(Opcode::Mul),
                    "/" => self.emit(Opcode::Div),
                    "%" => self.emit(Opcode::Mod),
                    "**" => self.emit(Opcode::Pow),
                    "//" => self.emit(Opcode::FloorDiv),
                    "==" => self.emit(Opcode::Eq),
                    "!=" => self.emit(Opcode::Neq),
                    "<" => self.emit(Opcode::Lt),
                    ">" => self.emit(Opcode::Gt),
                    "<=" => self.emit(Opcode::Lte),
                    ">=" => self.emit(Opcode::Gte),
                    "&" => self.emit(Opcode::Band),
                    "|" => self.emit(Opcode::Bor),
                    "^" => self.emit(Opcode::Bxor),
                    "<<" => self.emit(Opcode::Shl),
                    ">>" => self.emit(Opcode::Shr),
                    ">>>" => self.emit(Opcode::Ushr),
                    _ => self.emit(Opcode::Add),
                }
            }
            ASTNode::UnaryOp { op, operand } => {
                self.compile_node(operand)?;
                match op.as_str() {
                    "-" => self.emit(Opcode::Neg),
                    "!" | "not" => self.emit(Opcode::Not),
                    "~" => self.emit(Opcode::Bnot),
                    _ => {}
                }
            }
            ASTNode::FuncCall { name, args } => {
                let is_native = ["print", "len", "type", "str", "int", "float", "bool", "range", "abs", "sqrt"].contains(&name.as_str());
                if is_native {
                    for arg in args { self.compile_node(arg)?; }
                    let idx = self.module.native_index(name);
                    self.emit_call_native(idx, args.len() as u8);
                } else {
                    let idx = self.module.global_index(name);
                    self.emit_u16(Opcode::LoadGlobal, idx);
                    for arg in args { self.compile_node(arg)?; }
                    self.emit_u16(Opcode::Call, args.len() as u16);
                }
            }
            ASTNode::CallExpr { callee, args } => {
                self.compile_node(callee)?;
                for arg in args { self.compile_node(arg)?; }
                self.emit_u16(Opcode::Call, args.len() as u16);
            }
            ASTNode::ArrayLit(elements) => {
                for elem in elements { self.compile_node(elem)?; }
                self.emit_u16(Opcode::MakeList, elements.len() as u16);
            }
            ASTNode::MapLit(pairs) => {
                for (k, v) in pairs { self.compile_node(k)?; self.compile_node(v)?; }
                self.emit_u16(Opcode::MakeMap, pairs.len() as u16);
            }
            ASTNode::Subscript { target, index } => {
                self.compile_node(target)?;
                self.compile_node(index)?;
                self.emit(Opcode::GetIndex);
            }
            ASTNode::IndexAssign { target, index, value } => {
                if let Some(slot) = self.current().resolve_local(target) {
                    self.emit_u16(Opcode::LoadVar, slot);
                } else {
                    let idx = self.module.global_index(target);
                    self.emit_u16(Opcode::LoadGlobal, idx);
                }
                self.compile_node(index)?;
                self.compile_node(value)?;
                self.emit(Opcode::SetIndex);
            }
            _ => self.emit(Opcode::LoadNull),
        }
        Ok(())
    }
}
