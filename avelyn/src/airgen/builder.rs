// airgen/builder.rs — AIR instruction builder
//
// AirBuilder provides a high-level API for constructing AIR instructions.
// It wraps an AirFunction under construction and ensures:
//   • Values are monotonically allocated.
//   • Instructions are appended to the correct basic block.
//   • The current insertion point is tracked.
//   • Immutable variable set is maintained for assignment checks.

use std::collections::{HashMap, HashSet};
use crate::air::{
    AirFunction, AirModule, AirParam, AirType, BasicBlock, BlockId,
    Inst, RuntimeFn, Value, VOID_VALUE,
};
use crate::ast::Span;

pub struct AirBuilder {
    /// The function being built.
    pub func: AirFunction,
    /// Current insertion block.
    pub current_block: BlockId,
    /// Scope stack: maps variable names to their SylvelVal* alloc Values.
    scopes: Vec<HashMap<String, Value>>,
    /// Names declared with `let` (immutable).
    pub immutable_vars: HashSet<String>,
    /// Stack of (break_target, continue_target) for loops.
    pub loop_targets: Vec<(BlockId, BlockId)>,
    /// Saved alloca values accumulated for the entry block pre-alloca pass.
    /// All Alloc instructions go here; they are flushed into blocks[0] at
    /// `finalize()`.
    pub entry_allocs: Vec<Inst>,
}

impl AirBuilder {
    pub fn new(name: impl Into<String>, span: Span) -> Self {
        let mut func = AirFunction::new(name, span);
        // Create the entry block immediately.
        let entry = func.fresh_block("entry");
        AirBuilder {
            func,
            current_block: entry,
            scopes: vec![HashMap::new()],
            immutable_vars: HashSet::new(),
            loop_targets: Vec::new(),
            entry_allocs: Vec::new(),
        }
    }

    // ── Value allocation ────────────────────────────────────────────────────

    pub fn fresh_value(&mut self) -> Value {
        self.func.fresh_value()
    }

    // ── Block management ────────────────────────────────────────────────────

    pub fn new_block(&mut self, label: impl Into<String>) -> BlockId {
        self.func.fresh_block(label)
    }

    pub fn switch_to(&mut self, block: BlockId) {
        self.current_block = block;
    }

    pub fn is_terminated(&self) -> bool {
        self.func.block(self.current_block)
            .map(|b| b.is_terminated())
            .unwrap_or(false)
    }

    // ── Instruction emission ────────────────────────────────────────────────

    pub fn emit(&mut self, inst: Inst) {
        let block = self.current_block;
        self.func.push_to(block, inst);
    }

    /// Emit a stack allocation for a SylvelVal.
    /// All allocs are immediately hoisted to entry_allocs so that LLVM can
    /// see them in the entry block (avoids dynamic alloca issues).
    pub fn emit_alloc(&mut self, ty: AirType) -> Value {
        let v = self.fresh_value();
        self.entry_allocs.push(Inst::Alloc(v, ty));
        v
    }

    pub fn emit_const_null(&mut self) -> Value {
        let v = self.fresh_value();
        let res = self.fresh_value(); // alloc slot
        self.entry_allocs.push(Inst::Alloc(res, AirType::SylvelVal));
        self.emit(Inst::RuntimeCall(VOID_VALUE, RuntimeFn::MakeNull, vec![res]));
        res
    }

    pub fn emit_const_bool(&mut self, b: bool, module: &mut AirModule) -> Value {
        let res = self.fresh_value();
        self.entry_allocs.push(Inst::Alloc(res, AirType::SylvelVal));
        // We'll encode the bool as a const int for the runtime call.
        let bval = self.fresh_value();
        self.emit(Inst::ConstBool(bval, b));
        self.emit(Inst::RuntimeCall(VOID_VALUE, RuntimeFn::MakeBool, vec![res, bval]));
        res
    }

    pub fn emit_const_int(&mut self, i: i64) -> Value {
        let res = self.fresh_value();
        self.entry_allocs.push(Inst::Alloc(res, AirType::SylvelVal));
        let iv = self.fresh_value();
        self.emit(Inst::ConstInt(iv, i));
        self.emit(Inst::RuntimeCall(VOID_VALUE, RuntimeFn::MakeInt, vec![res, iv]));
        res
    }

    pub fn emit_const_float(&mut self, f: f64) -> Value {
        let res = self.fresh_value();
        self.entry_allocs.push(Inst::Alloc(res, AirType::SylvelVal));
        let fv = self.fresh_value();
        self.emit(Inst::ConstFloat(fv, f));
        self.emit(Inst::RuntimeCall(VOID_VALUE, RuntimeFn::MakeFloat, vec![res, fv]));
        res
    }

    pub fn emit_const_str(&mut self, s: &str, module: &mut AirModule) -> Value {
        let idx = module.intern_string(s);
        let res = self.fresh_value();
        self.entry_allocs.push(Inst::Alloc(res, AirType::SylvelVal));
        let sv = self.fresh_value();
        self.emit(Inst::ConstStr(sv, idx));
        self.emit(Inst::RuntimeCall(VOID_VALUE, RuntimeFn::AllocString, vec![res, sv]));
        res
    }

    pub fn emit_runtime_call_void(&mut self, rt_fn: RuntimeFn, args: Vec<Value>) {
        self.emit(Inst::RuntimeCall(VOID_VALUE, rt_fn, args));
    }

    pub fn emit_runtime_call(&mut self, rt_fn: RuntimeFn, args: Vec<Value>, result_ty: AirType) -> Value {
        let res = self.fresh_value();
        self.emit(Inst::RuntimeCall(res, rt_fn, args));
        res
    }

    pub fn emit_call(&mut self, name: &str, args: Vec<Value>) -> Value {
        let res = self.fresh_value();
        // The output slot (first arg) is pre-allocated by caller.
        self.emit(Inst::Call(VOID_VALUE, name.to_string(), args));
        res
    }

    pub fn emit_branch(&mut self, cond: Value, then_bb: BlockId, else_bb: BlockId) {
        self.emit(Inst::Branch(cond, then_bb, else_bb));
    }

    pub fn emit_jump(&mut self, target: BlockId) {
        if !self.is_terminated() {
            self.emit(Inst::Jump(target));
        }
    }

    pub fn emit_return(&mut self, val: Value) {
        if !self.is_terminated() {
            self.emit(Inst::Return(val));
        }
    }

    pub fn emit_return_void(&mut self) {
        if !self.is_terminated() {
            self.emit(Inst::Return(VOID_VALUE));
        }
    }

    pub fn emit_debug_loc(&mut self, span: Span) {
        if span.is_known() {
            self.emit(Inst::DebugLoc(span));
        }
    }

    // ── Scope management ────────────────────────────────────────────────────

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn set_var(&mut self, name: &str, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), value);
        }
    }

    pub fn lookup_var(&self, name: &str) -> Option<Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(&v) = scope.get(name) { return Some(v); }
        }
        None
    }

    pub fn is_immutable(&self, name: &str) -> bool {
        self.immutable_vars.contains(name)
    }

    pub fn mark_immutable(&mut self, name: &str) {
        self.immutable_vars.insert(name.to_string());
    }

    // ── Loop stack ──────────────────────────────────────────────────────────

    pub fn push_loop(&mut self, break_bb: BlockId, continue_bb: BlockId) {
        self.loop_targets.push((break_bb, continue_bb));
    }

    pub fn pop_loop(&mut self) {
        self.loop_targets.pop();
    }

    pub fn loop_break_target(&self) -> Option<BlockId> {
        self.loop_targets.last().map(|(b, _)| *b)
    }

    pub fn loop_continue_target(&self) -> Option<BlockId> {
        self.loop_targets.last().map(|(_, c)| *c)
    }

    // ── Finalization ─────────────────────────────────────────────────────────

    /// Flush accumulated entry_allocs into blocks[0] and rebuild CFG edges.
    pub fn finalize(mut self) -> AirFunction {
        if !self.entry_allocs.is_empty() {
            if let Some(entry) = self.func.blocks.get_mut(0) {
                // Prepend allocs before all other instructions.
                let mut new_insts = self.entry_allocs;
                new_insts.extend(std::mem::take(&mut entry.insts));
                entry.insts = new_insts;
            }
        }
        self.func.rebuild_cfg();
        self.func
    }
}
