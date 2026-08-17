// air/mod.rs — Avelyn Intermediate Representation (AIR)
//
// AIR is positioned between the typed AST (output of Sema) and LLVM IR.
// It is the language-specific optimization IR, inspired by Swift SIL:
//
//   Typed AST  →  AIRGen  →  AIR  →  Optimizer  →  LLVM IRGen  →  LLVM IR
//
// AIR is a linear, SSA-like IR with explicit basic blocks and control-flow
// edges.  Key design choices:
//
//   • Every definition is a `Value` — a typed SSA register identified by u32.
//   • Instructions are flat and immutable after construction.
//   • `RuntimeCall` makes C ABI calls explicit before LLVM lowering.
//   • `Retain`/`Release`/`Move`/`Copy` mark ownership — initially a no-op
//     placeholder that the future ownership optimizer will fill in.
//   • `DebugLoc` carries source location through the pipeline.

#![allow(dead_code)]
pub mod verify;
pub mod printer;

use std::collections::HashMap;
use crate::ast::Span;

// ─── Value ───────────────────────────────────────────────────────────────────

/// An SSA value — the "result" of an instruction.
/// Values are identified by a monotonically increasing u32 within a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Value(pub u32);

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "%v{}", self.0)
    }
}

/// Sentinal value: no result (void instructions).
pub const VOID_VALUE: Value = Value(u32::MAX);

// ─── Block IDs ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

impl std::fmt::Display for BlockId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

// ─── Function IDs ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FuncId(pub u32);

// ─── AIR Type System ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AirType {
    /// No return value.
    Void,
    /// i1 — boolean.
    Bool,
    /// i64 — 64-bit signed integer.
    I64,
    /// double — 64-bit float.
    F64,
    /// i8* — raw byte pointer.
    Ptr,
    /// Opaque `SylvelVal` — the dynamic runtime value struct.
    /// All runtime ABI calls use `*SylvelVal` pointers.
    SylvelVal,
    /// A pointer-to-SylvelVal (the normal allocation representation).
    SylvelValPtr,
    /// Function reference: parameter types → return type.
    FnRef(Vec<AirType>, Box<AirType>),
    /// Aggregate of types (for future struct lowering).
    Aggregate(Vec<AirType>),
}

impl std::fmt::Display for AirType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AirType::Void         => write!(f, "void"),
            AirType::Bool         => write!(f, "i1"),
            AirType::I64          => write!(f, "i64"),
            AirType::F64          => write!(f, "f64"),
            AirType::Ptr          => write!(f, "ptr"),
            AirType::SylvelVal    => write!(f, "SylvelVal"),
            AirType::SylvelValPtr => write!(f, "*SylvelVal"),
            AirType::FnRef(ps, r) => {
                let ps: Vec<_> = ps.iter().map(|t| t.to_string()).collect();
                write!(f, "fn({}) -> {}", ps.join(", "), r)
            }
            AirType::Aggregate(ts) => {
                let ts: Vec<_> = ts.iter().map(|t| t.to_string()).collect();
                write!(f, "({})", ts.join(", "))
            }
        }
    }
}

// ─── Runtime Function Enum ───────────────────────────────────────────────────

/// Every call to the C runtime goes through this enum.
/// This is the single source of truth for the ABI — `runtime_map.rs` in
/// airgen converts logical operations to these variants; the LLVM IRGen maps
/// them to `@sylvel_rt_*` declarations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RuntimeFn {
    // Constructors
    MakeNull,
    MakeBool,
    MakeInt,
    MakeFloat,
    AllocString,
    AllocStringLen,
    AllocList,
    AllocMap,
    // Extractors
    ToBool,
    ToInt,
    ToFloat,
    // Memory management (ARC)
    Retain,
    Release,
    // Operations
    BinOp,
    UnaryOp,
    Print,
    Len,
    // Collection operations
    ListPush,
    ListGet,
    ListSet,
    MapGet,
    MapSet,
    SubscriptGet,
    SubscriptSet,
    // Function call dispatch
    CallExpr,
    // Error handling
    EnterTry,
    ExitTry,
    HasError,
    ClearError,
    RaiseError,
    ThrowVal,
    GetErrorVal,
    // Assertion
    BuiltinAssert,
    // Named builtins (dynamic dispatch via @sylvel_rt_builtin_<name>)
    Builtin(String),
}

impl RuntimeFn {
    /// The exact C function name in the ABI.
    pub fn c_name(&self) -> String {
        match self {
            RuntimeFn::MakeNull      => "sylvel_rt_make_null".to_string(),
            RuntimeFn::MakeBool      => "sylvel_rt_make_bool".to_string(),
            RuntimeFn::MakeInt       => "sylvel_rt_make_int".to_string(),
            RuntimeFn::MakeFloat     => "sylvel_rt_make_float".to_string(),
            RuntimeFn::AllocString   => "sylvel_rt_alloc_string".to_string(),
            RuntimeFn::AllocStringLen => "sylvel_rt_alloc_string_len".to_string(),
            RuntimeFn::AllocList     => "sylvel_rt_alloc_list".to_string(),
            RuntimeFn::AllocMap      => "sylvel_rt_alloc_map".to_string(),
            RuntimeFn::ToBool        => "sylvel_rt_to_bool".to_string(),
            RuntimeFn::ToInt         => "sylvel_rt_to_int".to_string(),
            RuntimeFn::ToFloat       => "sylvel_rt_to_float".to_string(),
            RuntimeFn::Retain        => "sylvel_rt_retain".to_string(),
            RuntimeFn::Release       => "sylvel_rt_release".to_string(),
            RuntimeFn::BinOp         => "sylvel_rt_bin_op".to_string(),
            RuntimeFn::UnaryOp       => "sylvel_rt_unary_op".to_string(),
            RuntimeFn::Print         => "sylvel_rt_print".to_string(),
            RuntimeFn::Len           => "sylvel_rt_len".to_string(),
            RuntimeFn::ListPush      => "sylvel_rt_list_push".to_string(),
            RuntimeFn::ListGet       => "sylvel_rt_list_get".to_string(),
            RuntimeFn::ListSet       => "sylvel_rt_list_set".to_string(),
            RuntimeFn::MapGet        => "sylvel_rt_map_get".to_string(),
            RuntimeFn::MapSet        => "sylvel_rt_map_set".to_string(),
            RuntimeFn::SubscriptGet  => "sylvel_rt_subscript_get".to_string(),
            RuntimeFn::SubscriptSet  => "sylvel_rt_subscript_set".to_string(),
            RuntimeFn::CallExpr      => "sylvel_rt_call_expr".to_string(),
            RuntimeFn::EnterTry      => "sylvel_rt_enter_try".to_string(),
            RuntimeFn::ExitTry       => "sylvel_rt_exit_try".to_string(),
            RuntimeFn::HasError      => "sylvel_rt_has_error".to_string(),
            RuntimeFn::ClearError    => "sylvel_rt_clear_error".to_string(),
            RuntimeFn::RaiseError    => "sylvel_rt_raise_error".to_string(),
            RuntimeFn::ThrowVal      => "sylvel_rt_throw_val".to_string(),
            RuntimeFn::GetErrorVal   => "sylvel_rt_get_error_val".to_string(),
            RuntimeFn::BuiltinAssert => "sylvel_rt_builtin_assert".to_string(),
            RuntimeFn::Builtin(name) => format!("sylvel_rt_builtin_{}", name),
        }
    }

    /// The AIR return type of this runtime function.
    pub fn return_type(&self) -> AirType {
        match self {
            RuntimeFn::ToBool   => AirType::Bool,
            RuntimeFn::ToInt    => AirType::I64,
            RuntimeFn::ToFloat  => AirType::F64,
            RuntimeFn::Len      => AirType::I64,
            RuntimeFn::HasError => AirType::I64,
            _                   => AirType::Void,
        }
    }
}

// ─── Binary / Unary op codes ─────────────────────────────────────────────────

/// Matches the `op_type` constants in `sylvel_runtime.c`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum BinOpCode {
    Add     = 1, Sub    = 2,  Mul = 3,  Div = 4,  Mod = 5,
    Eq      = 6, Ne     = 7,  Lt  = 8,  Le  = 9,  Gt  = 10, Ge = 11,
    BitAnd  = 12, BitOr = 13, Xor = 14, Shl = 15, Shr = 16,
    And     = 17, Or    = 18,
    FloorDiv = 19, Pow   = 20,
}

impl BinOpCode {
    pub fn from_str(op: &str) -> Self {
        match op {
            "+"   => BinOpCode::Add,  "-"  => BinOpCode::Sub,
            "*"   => BinOpCode::Mul,  "/"  => BinOpCode::Div,
            "%"   => BinOpCode::Mod,  "==" => BinOpCode::Eq,
            "!="  => BinOpCode::Ne,   "<"  => BinOpCode::Lt,
            "<="  => BinOpCode::Le,   ">"  => BinOpCode::Gt,
            ">="  => BinOpCode::Ge,   "&"  => BinOpCode::BitAnd,
            "|"   => BinOpCode::BitOr,"^"  => BinOpCode::Xor,
            "<<"  => BinOpCode::Shl,  ">>" => BinOpCode::Shr,
            "and" | "&&" => BinOpCode::And,
            "or"  | "||" => BinOpCode::Or,
            "//"  => BinOpCode::FloorDiv,
            "**"  => BinOpCode::Pow,
            _     => BinOpCode::Add, // fallback
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum UnaryOpCode {
    Neg    = 1,
    Not    = 2,
    BitNot = 3,
}

impl UnaryOpCode {
    pub fn from_str(op: &str) -> Self {
        match op {
            "-" => UnaryOpCode::Neg,
            "~" => UnaryOpCode::BitNot,
            _   => UnaryOpCode::Not,
        }
    }
}

// ─── Instruction ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Inst {
    // ── Constants ────────────────────────────────────────────────────────────
    /// %result = const_null
    ConstNull(Value),
    /// %result = const_bool <b>
    ConstBool(Value, bool),
    /// %result = const_int <i>
    ConstInt(Value, i64),
    /// %result = const_float <f>
    ConstFloat(Value, f64),
    /// %result = const_str <s>  — interned index into AirModule::string_table
    ConstStr(Value, u32),

    // ── Memory ───────────────────────────────────────────────────────────────
    /// %result = alloc <ty>  — stack slot
    Alloc(Value, AirType),
    /// %result = load %ptr
    Load(Value, Value),
    /// store %val -> %ptr  (void)
    Store(Value, Value),

    // ── Runtime calls ─────────────────────────────────────────────────────
    /// %result = runtime_call <fn> (%args...)
    /// The result Value is VOID_VALUE for void calls.
    RuntimeCall(Value, RuntimeFn, Vec<Value>),

    // ── User function calls ───────────────────────────────────────────────
    /// %result = call @<func_id>(%args...)
    Call(Value, String, Vec<Value>),

    // ── Control flow ─────────────────────────────────────────────────────
    /// branch %cond, bb_then, bb_else
    Branch(Value, BlockId, BlockId),
    /// jump bb_target
    Jump(BlockId),
    /// return %val (VOID_VALUE for void return)
    Return(Value),
    /// Marks unreachable code (after unconditional break/continue).
    Unreachable,

    // ── Arithmetic / logic ────────────────────────────────────────────────
    // (high-level AIR; not always lowered — optimizer may constant-fold)
    /// %result = iadd %a, %b
    IAdd(Value, Value, Value),
    /// %result = isub %a, %b
    ISub(Value, Value, Value),
    /// %result = imul %a, %b
    IMul(Value, Value, Value),
    /// %result = icmp_eq %a, %b → i1
    ICmpEq(Value, Value, Value),
    /// %result = icmp_slt %a, %b → i1
    ICmpSlt(Value, Value, Value),
    /// %result = icmp_sle %a, %b → i1
    ICmpSle(Value, Value, Value),

    // ── Ownership (future) ────────────────────────────────────────────────
    /// Increment the ref-count of a heap value.
    Retain(Value),
    /// Decrement the ref-count (may dealloc) of a heap value.
    Release(Value),
    /// Transfer ownership (move semantics — invalidates source).
    Move { dest: Value, src: Value },
    /// Shallow copy (increment ref-count of any heap parts).
    Copy { dest: Value, src: Value },

    // ── Source location ───────────────────────────────────────────────────
    /// Carries source location; stripped in release builds.
    DebugLoc(Span),

    // ── GEP-like helper for extracting SylvelVal fields ───────────────────
    /// %result = gep_field %ptr, <field_index>
    GepField(Value, Value, u32),
}

impl Inst {
    /// Returns the Value defined by this instruction, if any.
    pub fn defined_value(&self) -> Option<Value> {
        match self {
            Inst::ConstNull(v) | Inst::ConstBool(v, _) | Inst::ConstInt(v, _)
            | Inst::ConstFloat(v, _) | Inst::ConstStr(v, _)
            | Inst::Alloc(v, _) | Inst::Load(v, _)
            | Inst::IAdd(v, _, _) | Inst::ISub(v, _, _) | Inst::IMul(v, _, _)
            | Inst::ICmpEq(v, _, _) | Inst::ICmpSlt(v, _, _) | Inst::ICmpSle(v, _, _)
            | Inst::GepField(v, _, _)
              => Some(*v),

            Inst::RuntimeCall(v, _, _) if *v != VOID_VALUE => Some(*v),
            Inst::Call(v, _, _) if *v != VOID_VALUE         => Some(*v),

            Inst::Move { dest, .. } | Inst::Copy { dest, .. } => Some(*dest),

            _ => None,
        }
    }

    /// Returns true if this instruction terminates its basic block.
    pub fn is_terminator(&self) -> bool {
        matches!(self, Inst::Branch(..) | Inst::Jump(..) | Inst::Return(..) | Inst::Unreachable)
    }

    /// Collect all Values used as inputs by this instruction.
    pub fn used_values(&self) -> Vec<Value> {
        match self {
            Inst::Store(val, ptr)  => vec![*val, *ptr],
            Inst::Load(_, ptr)     => vec![*ptr],
            Inst::RuntimeCall(_, _, args) => args.clone(),
            Inst::Call(_, _, args)        => args.clone(),
            Inst::Branch(cond, _, _)      => vec![*cond],
            Inst::Return(v) if *v != VOID_VALUE => vec![*v],
            Inst::IAdd(_, a, b) | Inst::ISub(_, a, b) | Inst::IMul(_, a, b)
            | Inst::ICmpEq(_, a, b) | Inst::ICmpSlt(_, a, b) | Inst::ICmpSle(_, a, b)
              => vec![*a, *b],
            Inst::GepField(_, ptr, _) => vec![*ptr],
            Inst::Retain(v) | Inst::Release(v) => vec![*v],
            Inst::Move { src, .. } | Inst::Copy { src, .. } => vec![*src],
            _ => vec![],
        }
    }
}

// ─── BasicBlock ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id:     BlockId,
    pub label:  String,
    pub insts:  Vec<Inst>,
    pub preds:  Vec<BlockId>,
    pub succs:  Vec<BlockId>,
}

impl BasicBlock {
    pub fn new(id: BlockId, label: impl Into<String>) -> Self {
        BasicBlock {
            id,
            label: label.into(),
            insts: Vec::new(),
            preds: Vec::new(),
            succs: Vec::new(),
        }
    }

    pub fn terminator(&self) -> Option<&Inst> {
        self.insts.last().filter(|i| i.is_terminator())
    }

    pub fn is_terminated(&self) -> bool {
        self.insts.last().map(|i| i.is_terminator()).unwrap_or(false)
    }

    pub fn push(&mut self, inst: Inst) {
        if !self.is_terminated() {
            self.insts.push(inst);
        }
    }
}

// ─── AirParam ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AirParam {
    pub name:  String,
    pub value: Value,
    pub ty:    AirType,
}

// ─── AirFunction ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AirFunction {
    /// The mangled function name (e.g. `lyn_fn_fibonacci`).
    pub name:      String,
    /// Parameters, including the implicit `%out` pointer for non-void returns.
    pub params:    Vec<AirParam>,
    /// Return type of the function in AIR terms (usually Void for user fns).
    pub ret_ty:    AirType,
    /// Ordered list of basic blocks. First block is the entry block.
    pub blocks:    Vec<BasicBlock>,
    /// Next value counter (monotonic).
    next_val:      u32,
    /// Next block counter (monotonic).
    next_block:    u32,
    /// Source span of the function declaration.
    pub span:      Span,
    /// Whether this is a variadic function.
    pub variadic:  bool,
    /// Maps Value ID -> global variable symbol string (e.g. "@lyn_var_math").
    pub global_val_map: HashMap<Value, String>,
}

impl AirFunction {
    pub fn new(name: impl Into<String>, span: Span) -> Self {
        AirFunction {
            name: name.into(),
            params: Vec::new(),
            ret_ty: AirType::Void,
            blocks: Vec::new(),
            next_val: 0,
            next_block: 0,
            span,
            variadic: false,
            global_val_map: HashMap::new(),
        }
    }

    /// Allocate a fresh SSA value ID.
    pub fn fresh_value(&mut self) -> Value {
        let v = Value(self.next_val);
        self.next_val += 1;
        v
    }

    /// Allocate a fresh basic block.
    pub fn fresh_block(&mut self, label: impl Into<String>) -> BlockId {
        let id = BlockId(self.next_block);
        self.next_block += 1;
        let block = BasicBlock::new(id, label);
        self.blocks.push(block);
        id
    }

    /// Append an instruction to the given block.
    pub fn push_to(&mut self, block: BlockId, inst: Inst) {
        if let Some(b) = self.blocks.iter_mut().find(|b| b.id == block) {
            b.push(inst);
        }
    }

    /// Get a mutable reference to the block with the given ID.
    pub fn block_mut(&mut self, id: BlockId) -> Option<&mut BasicBlock> {
        self.blocks.iter_mut().find(|b| b.id == id)
    }

    pub fn block(&self, id: BlockId) -> Option<&BasicBlock> {
        self.blocks.iter().find(|b| b.id == id)
    }

    /// Rebuild the predecessor/successor edge lists from branch/jump instructions.
    pub fn rebuild_cfg(&mut self) {
        // Clear existing edges
        for b in &mut self.blocks { b.preds.clear(); b.succs.clear(); }

        // Collect edges
        let mut edges: Vec<(BlockId, BlockId)> = Vec::new();
        for b in &self.blocks {
            match b.terminator() {
                Some(Inst::Branch(_, then_bb, else_bb)) => {
                    edges.push((b.id, *then_bb));
                    edges.push((b.id, *else_bb));
                }
                Some(Inst::Jump(target)) => {
                    edges.push((b.id, *target));
                }
                _ => {}
            }
        }

        // Apply edges
        for (src, dst) in edges {
            if let Some(b) = self.blocks.iter_mut().find(|b| b.id == src) {
                if !b.succs.contains(&dst) { b.succs.push(dst); }
            }
            if let Some(b) = self.blocks.iter_mut().find(|b| b.id == dst) {
                if !b.preds.contains(&src) { b.preds.push(src); }
            }
        }
    }
}

// ─── AirGlobal ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AirGlobal {
    pub name: String,
    pub ty:   AirType,
    /// Optional constant initializer (string index).
    pub init: Option<u32>,
}

// ─── AirModule ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AirModule {
    pub name:         String,
    pub functions:    Vec<AirFunction>,
    pub globals:      Vec<AirGlobal>,
    /// Interned string constants.  Index = ConstStr value.
    pub string_table: Vec<String>,
    /// Set of user-defined function names (for CallExpr dispatch).
    pub user_fn_names: std::collections::HashSet<String>,
    /// Declared external function arities (for LLVM `declare` emission).
    pub extern_fns:   HashMap<String, usize>,
}

impl AirModule {
    pub fn new(name: impl Into<String>) -> Self {
        AirModule {
            name: name.into(),
            functions: Vec::new(),
            globals: Vec::new(),
            string_table: Vec::new(),
            user_fn_names: std::collections::HashSet::new(),
            extern_fns: HashMap::new(),
        }
    }

    /// Intern a string constant and return its index.
    pub fn intern_string(&mut self, s: &str) -> u32 {
        if let Some(idx) = self.string_table.iter().position(|x| x == s) {
            return idx as u32;
        }
        let idx = self.string_table.len() as u32;
        self.string_table.push(s.to_string());
        idx
    }

    pub fn add_function(&mut self, f: AirFunction) {
        self.user_fn_names.insert(f.name.clone());
        self.functions.push(f);
    }

    pub fn function_by_name(&self, name: &str) -> Option<&AirFunction> {
        self.functions.iter().find(|f| f.name == name)
    }

    /// Rebuild all CFG edges in every function.
    pub fn rebuild_all_cfgs(&mut self) {
        for f in &mut self.functions {
            f.rebuild_cfg();
        }
    }
}
