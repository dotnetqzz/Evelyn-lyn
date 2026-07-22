// compiler/mod.rs — Compiler frontend & AST-to-bytecode translator

pub mod instruction;
pub mod writer;
pub mod loader;
pub mod verifier;
pub mod bundler;

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
        let mut folded_ast = ast.to_vec();
        self.fold_ast(&mut folded_ast);

        let main_proto = FunctionProto::new("<main>");
        self.function_stack.push(main_proto);
        for node in folded_ast { self.compile_node(&node)?; }
        self.emit(Opcode::ReturnNull);
        let finished = self.function_stack.pop().unwrap();
        self.module.protos.push(finished);
        Ok(self.module)
    }

    fn fold_ast(&self, ast: &mut Vec<ASTNode>) {
        for node in ast.iter_mut() {
            self.fold_node(node);
        }
        // Basic DCE: remove nodes after Return, Break, Continue
        let mut i = 0;
        while i < ast.len() {
            match &ast[i] {
                ASTNode::Return(_) | ASTNode::Break | ASTNode::Continue | ASTNode::Throw(_) => {
                    ast.truncate(i + 1);
                    break;
                }
                _ => i += 1,
            }
        }
    }

    fn fold_node(&self, node: &mut ASTNode) {
        match node {
            ASTNode::BinOp { left, op, right } => {
                self.fold_node(left);
                self.fold_node(right);
                if let (ASTNode::Int(l), ASTNode::Int(r)) = (left.as_ref(), right.as_ref()) {
                    match op.as_str() {
                        "+" => *node = ASTNode::Int(l.wrapping_add(*r)),
                        "-" => *node = ASTNode::Int(l.wrapping_sub(*r)),
                        "*" => *node = ASTNode::Int(l.wrapping_mul(*r)),
                        "//" if *r != 0 => *node = ASTNode::Int(l.div_euclid(*r)),
                        "%" if *r != 0 => *node = ASTNode::Int(l.rem_euclid(*r)),
                        "==" => *node = ASTNode::Bool(l == r),
                        "!=" => *node = ASTNode::Bool(l != r),
                        "<"  => *node = ASTNode::Bool(l < r),
                        ">"  => *node = ASTNode::Bool(l > r),
                        "<=" => *node = ASTNode::Bool(l <= r),
                        ">=" => *node = ASTNode::Bool(l >= r),
                        _ => {}
                    }
                } else if let (ASTNode::Float(l), ASTNode::Float(r)) = (left.as_ref(), right.as_ref()) {
                    match op.as_str() {
                        "+" => *node = ASTNode::Float(l + r),
                        "-" => *node = ASTNode::Float(l - r),
                        "*" => *node = ASTNode::Float(l * r),
                        "/" if *r != 0.0 => *node = ASTNode::Float(l / r),
                        "==" => *node = ASTNode::Bool(l == r),
                        _ => {}
                    }
                }
            }
            ASTNode::UnaryOp { op, operand } => {
                self.fold_node(operand);
                if let ASTNode::Int(v) = operand.as_ref() {
                    if op == "-" { *node = ASTNode::Int(-v); }
                    else if op == "~" { *node = ASTNode::Int(!v); }
                } else if let ASTNode::Bool(v) = operand.as_ref() {
                    if op == "!" || op == "not" { *node = ASTNode::Bool(!v); }
                }
            }
            ASTNode::If { cond, then, els } => {
                self.fold_node(cond);
                self.fold_ast(then);
                if let Some(e) = els { self.fold_ast(e); }

                if let ASTNode::Bool(b) = cond.as_ref() {
                    if *b {
                        // This would require replacing the If node with the block contents,
                        // which is hard since fold_node takes &mut ASTNode.
                        // For now we just fold the children.
                    }
                }
            }
            ASTNode::While { cond, body } => {
                self.fold_node(cond);
                self.fold_ast(body);
            }
            ASTNode::For { var: _, iter, body } => {
                self.fold_node(iter);
                self.fold_ast(body);
            }
            ASTNode::FuncDecl { name: _, params: _, body, variadic: _, annotations: _ } => {
                self.fold_ast(body);
            }
            ASTNode::Lambda { params: _, body, variadic: _, annotations: _ } => {
                self.fold_ast(body);
            }
            ASTNode::Decl { name: _, value, mutable: _, annotations: _ } => self.fold_node(value),
            ASTNode::Assign { name: _, value } => self.fold_node(value),
            ASTNode::CallExpr { callee, args } => {
                self.fold_node(callee);
                for a in args { self.fold_node(a); }
            }
            ASTNode::FuncCall { name: _, args } => {
                for a in args { self.fold_node(a); }
            }
            ASTNode::Match { subject, arms } => {
                self.fold_node(subject);
                for (_, body) in arms { self.fold_ast(body); }
            }
            ASTNode::Switch { subject, cases } => {
                self.fold_node(subject);
                for (_, body) in cases { self.fold_ast(body); }
            }
            ASTNode::TryCatch { body, catches, finally_body } => {
                self.fold_ast(body);
                for (_, _, c_body) in catches { self.fold_ast(c_body); }
                if let Some(f) = finally_body { self.fold_ast(f); }
            }
            ASTNode::ArrayLit(elements) => {
                for e in elements { self.fold_node(e); }
            }
            ASTNode::MapLit(pairs) => {
                for (k, v) in pairs { self.fold_node(k); self.fold_node(v); }
            }
            _ => {}
        }
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
            ASTNode::Decl { name, value, mutable, annotations: _ } => {
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
                for arg in args { self.compile_node(arg)?; }
                let idx = self.module.native_index(name);
                self.emit_call_native(idx, args.len() as u8);
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
            ASTNode::FuncDecl { name, params, body, variadic, annotations: _ } => {
                let mut proto = FunctionProto::new(name);
                proto.arity = params.len() as u8;
                proto.is_variadic = *variadic;
                for p in params {
                    proto.declare_local(&p.0, false);
                }

                self.function_stack.push(proto);
                for stmt in body {
                    self.compile_node(stmt)?;
                }
                self.emit(Opcode::ReturnNull);

                let finished_proto = self.function_stack.pop().unwrap();
                self.module.protos.push(finished_proto);
                let proto_idx = (self.module.protos.len() - 1) as u16;

                self.emit_u16(Opcode::MakeFunc, proto_idx);
                let g_idx = self.module.global_index(name);
                self.emit_u16(Opcode::StoreGlobal, g_idx);
            }
            ASTNode::While { cond, body } => {
                let loop_start = self.current().code.len();
                self.compile_node(cond)?;

                // JumpIfFPop placeholder (jump to end of loop if condition is false)
                let jump_false_ip = self.current().code.len();
                self.emit_u16(Opcode::JumpIfFPop, 0);

                for stmt in body {
                    self.compile_node(stmt)?;
                }

                // Loop back to loop_start
                let loop_end = self.current().code.len() + 3;
                let offset = (loop_start as isize - loop_end as isize) as i16;
                self.emit_u16(Opcode::Jump, offset as u16);

                // Patch jump_false_ip offset
                let false_offset = (self.current().code.len() as isize - (jump_false_ip + 3) as isize) as i16;
                let bytes = (false_offset as u16).to_be_bytes();
                self.current().code[jump_false_ip + 1] = bytes[0];
                self.current().code[jump_false_ip + 2] = bytes[1];
            }
            ASTNode::If { cond, then, els } => {
                self.compile_node(cond)?;
                let jump_false_ip = self.current().code.len();
                self.emit_u16(Opcode::JumpIfFPop, 0);

                for stmt in then {
                    self.compile_node(stmt)?;
                }

                if let Some(else_stmts) = els {
                    let jump_end_ip = self.current().code.len();
                    self.emit_u16(Opcode::Jump, 0);

                    let false_offset = (self.current().code.len() as isize - (jump_false_ip + 3) as isize) as i16;
                    let bytes = (false_offset as u16).to_be_bytes();
                    self.current().code[jump_false_ip + 1] = bytes[0];
                    self.current().code[jump_false_ip + 2] = bytes[1];

                    for stmt in else_stmts {
                        self.compile_node(stmt)?;
                    }

                    let end_offset = (self.current().code.len() as isize - (jump_end_ip + 3) as isize) as i16;
                    let end_bytes = (end_offset as u16).to_be_bytes();
                    self.current().code[jump_end_ip + 1] = end_bytes[0];
                    self.current().code[jump_end_ip + 2] = end_bytes[1];
                } else {
                    let false_offset = (self.current().code.len() as isize - (jump_false_ip + 3) as isize) as i16;
                    let bytes = (false_offset as u16).to_be_bytes();
                    self.current().code[jump_false_ip + 1] = bytes[0];
                    self.current().code[jump_false_ip + 2] = bytes[1];
                }
            }
            ASTNode::CompoundAssign { name, op, value } => {
                if let Some(slot) = self.current().resolve_local(name) {
                    self.emit_u16(Opcode::LoadVar, slot);
                } else {
                    let idx = self.module.global_index(name);
                    self.emit_u16(Opcode::LoadGlobal, idx);
                }
                self.compile_node(value)?;
                match op.as_str() {
                    "+" => self.emit(Opcode::Add),
                    "-" => self.emit(Opcode::Sub),
                    "*" => self.emit(Opcode::Mul),
                    "/" => self.emit(Opcode::Div),
                    "%" => self.emit(Opcode::Mod),
                    _ => self.emit(Opcode::Add),
                }
                if let Some(slot) = self.current().resolve_local(name) {
                    self.emit_u16(Opcode::StoreVar, slot);
                } else {
                    let idx = self.module.global_index(name);
                    self.emit_u16(Opcode::StoreGlobal, idx);
                }
            }
            ASTNode::Ternary { cond, then, els } => {
                self.compile_node(cond)?;
                let jump_false_ip = self.current().code.len();
                self.emit_u16(Opcode::JumpIfFPop, 0);

                self.compile_node(then)?;
                let jump_end_ip = self.current().code.len();
                self.emit_u16(Opcode::Jump, 0);

                let false_offset = (self.current().code.len() as isize - (jump_false_ip + 3) as isize) as i16;
                let bytes = (false_offset as u16).to_be_bytes();
                self.current().code[jump_false_ip + 1] = bytes[0];
                self.current().code[jump_false_ip + 2] = bytes[1];

                self.compile_node(els)?;

                let end_offset = (self.current().code.len() as isize - (jump_end_ip + 3) as isize) as i16;
                let end_bytes = (end_offset as u16).to_be_bytes();
                self.current().code[jump_end_ip + 1] = end_bytes[0];
                self.current().code[jump_end_ip + 2] = end_bytes[1];
            }
            ASTNode::NullCoalesce { left, right } => {
                self.compile_node(left)?;
                self.compile_node(right)?;
            }
            ASTNode::Spread(inner) => {
                self.compile_node(inner)?;
                self.emit(Opcode::Spread);
            }
            ASTNode::Pass => {}
            _ => self.emit(Opcode::LoadNull),
        }
        Ok(())
    }
}
