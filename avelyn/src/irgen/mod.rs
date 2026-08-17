#![allow(dead_code, unused_imports)]
// irgen/mod.rs — AIR → LLVM IR text translation
//
// This module has a single responsibility: translate a verified, optimized
// AirModule into LLVM IR text.  It contains NO Avelyn language semantics —
// all language-specific logic is in airgen/mod.rs.
//
// The LLVM IR format produced is identical in structure to what
// llvm_codegen.rs previously emitted, ensuring binary compatibility with
// the existing C runtime (sylvel_runtime.c).

pub mod runtime_lowering;

use std::collections::HashMap;
use crate::air::{
    AirFunction, AirModule, AirType, BasicBlock, BlockId, Inst, RuntimeFn, Value, VOID_VALUE,
};
use crate::target::Target;
use runtime_lowering::emit_declarations;

pub struct LlvmIrGen<'a> {
    module:    &'a AirModule,
    target:    &'a Target,
    temp_count: usize,
}

impl<'a> LlvmIrGen<'a> {
    pub fn new(module: &'a AirModule, target: &'a Target) -> Self {
        LlvmIrGen { module, target, temp_count: 0 }
    }

    fn fresh_temp(&mut self) -> String {
        self.temp_count += 1;
        format!("%_t{}", self.temp_count)
    }

    /// Translate the entire module to LLVM IR text.
    pub fn emit(&mut self) -> String {
        let mut ir = String::new();

        // ── Header ──────────────────────────────────────────────────────────
        ir.push_str(&format!("; ModuleID = '{}'\n", self.module.name));
        ir.push_str(&format!("target datalayout = \"{}\"\n", self.target.data_layout));
        ir.push_str(&format!("target triple = \"{}\"\n\n", self.target.triple));

        // ── Type declarations ────────────────────────────────────────────────
        ir.push_str("%SylvelVal = type { i32, i32, i64 }\n\n");

        // ── Runtime function declarations ────────────────────────────────────
        ir.push_str(&emit_declarations(self.module));

        // ── String constants ─────────────────────────────────────────────────
        for (i, s) in self.module.string_table.iter().enumerate() {
            let label = format!("@str.{}", i);
            let escaped = escape_string(s);
            let len = s.as_bytes().len() + 1;
            ir.push_str(&format!(
                "{} = private unnamed_addr constant [{} x i8] c\"{}\\00\", align 1\n",
                label, len, escaped
            ));
        }
        ir.push('\n');

        // ── Global variables ─────────────────────────────────────────────────
        for g in &self.module.globals {
            ir.push_str(&format!(
                "@lyn_var_{} = global %SylvelVal zeroinitializer, align 8\n",
                g.name
            ));
        }
        if !self.module.globals.is_empty() {
            ir.push('\n');
        }

        // ── Function definitions ─────────────────────────────────────────────
        // Emit all non-main functions first.
        for func in &self.module.functions {
            if func.name == "main" { continue; }
            let func_ir = self.emit_function(func);
            ir.push_str(&func_ir);
            ir.push('\n');
        }

        // Emit main last.
        for func in &self.module.functions {
            if func.name == "main" {
                let func_ir = self.emit_main(func);
                ir.push_str(&func_ir);
            }
        }

        ir
    }

    fn func_signature(&self, func: &AirFunction) -> String {
        if func.name == "main" {
            return "define i32 @main()".to_string();
        }
        let mut params: Vec<String> = vec!["%SylvelVal* %out".to_string()];
        for p in func.params.iter().skip(1) {
            params.push(format!("%SylvelVal* %param_{}", p.name));
        }
        format!("define void @lyn_fn_{}({})", func.name.trim_start_matches("lyn_fn_"), params.join(", "))
    }

    fn emit_function(&self, func: &AirFunction) -> String {
        let mut out = String::new();
        let mut ctx = FuncEmitCtx::new(func, self.module);

        let sig = self.func_signature(func);
        out.push_str(&format!("{} {{\n", sig));
        out.push_str("entry:\n");

        // Emit Alloc instructions as LLVM allocas (excluding parameters).
        let param_vals: std::collections::HashSet<_> = func.params.iter().map(|p| p.value).collect();
        for block in &func.blocks {
            for inst in &block.insts {
                if let Inst::Alloc(v, ty) = inst {
                    if !param_vals.contains(v) {
                        let llvm_ty = air_type_to_llvm(ty);
                        out.push_str(&format!("  {} = alloca {}\n", ctx.val_name(*v), llvm_ty));
                    }
                }
            }
        }

        // Emit each block.
        for (bi, block) in func.blocks.iter().enumerate() {
            if bi > 0 {
                let lbl = ctx.block_label(block.id);
                out.push_str(&format!("\n{}:\n", lbl));
            }
            let mut terminated = false;
            for inst in &block.insts {
                self.emit_inst(inst, &mut out, &mut ctx);
                if inst.is_terminator() {
                    terminated = true;
                    break;
                }
            }
            if !terminated {
                out.push_str("  ret void\n");
            }
        }

        out.push_str("}\n");
        out
    }

    fn emit_main(&self, func: &AirFunction) -> String {
        let mut out = String::new();
        let mut ctx = FuncEmitCtx::new(func, self.module);

        out.push_str("define i32 @main() {\n");
        out.push_str("entry:\n");

        // Emit allocs.
        for block in &func.blocks {
            for inst in &block.insts {
                if let Inst::Alloc(v, ty) = inst {
                    let llvm_ty = air_type_to_llvm(ty);
                    out.push_str(&format!("  {} = alloca {}\n", ctx.val_name(*v), llvm_ty));
                }
            }
        }

        for (bi, block) in func.blocks.iter().enumerate() {
            if bi > 0 {
                let lbl = ctx.block_label(block.id);
                out.push_str(&format!("\n{}:\n", lbl));
            }
            let mut terminated = false;
            for inst in &block.insts {
                self.emit_inst(inst, &mut out, &mut ctx);
                if inst.is_terminator() {
                    terminated = true;
                    break;
                }
            }
            if !terminated {
                out.push_str("  ret i32 0\n");
            }
        }

        out.push_str("}\n");
        out
    }

    fn emit_inst(&self, inst: &Inst, out: &mut String, ctx: &mut FuncEmitCtx) {
        match inst {
            // Allocs are emitted in the prologue — skip here.
            Inst::Alloc(_, _) => {}

            Inst::ConstNull(_) | Inst::ConstBool(_, _) | Inst::ConstInt(_, _)
            | Inst::ConstFloat(_, _) | Inst::ConstStr(_, _) => {
                // Handled inside RuntimeCall emission — pure constant values
                // are immediately consumed.  Standalone constants become no-ops.
            }

            Inst::Store(val, ptr) => {
                let val_name = ctx.val_name(*val);
                let ptr_name = ctx.val_name(*ptr);
                let ptr_ty = ctx.alloc_types.get(ptr).cloned().unwrap_or(AirType::SylvelVal);
                match ptr_ty {
                    AirType::I64 => {
                        out.push_str(&format!("  store i64 {}, i64* {}\n", val_name, ptr_name));
                    }
                    AirType::Bool => {
                        out.push_str(&format!("  store i1 {}, i1* {}\n", val_name, ptr_name));
                    }
                    AirType::F64 => {
                        out.push_str(&format!("  store double {}, double* {}\n", val_name, ptr_name));
                    }
                    _ => {
                        let tmp = ctx.fresh();
                        out.push_str(&format!(
                            "  {} = load %SylvelVal, %SylvelVal* {}\n",
                            tmp, val_name
                        ));
                        out.push_str(&format!(
                            "  store %SylvelVal {}, %SylvelVal* {}\n",
                            tmp, ptr_name
                        ));
                    }
                }
            }

            Inst::Load(res, ptr) => {
                let ptr_name = ctx.val_name(*ptr);
                let ptr_ty = ctx.alloc_types.get(ptr).cloned().unwrap_or(AirType::SylvelVal);
                let res_name = ctx.val_name(*res);
                match ptr_ty {
                    AirType::I64 => {
                        out.push_str(&format!("  {} = load i64, i64* {}\n", res_name, ptr_name));
                    }
                    AirType::Bool => {
                        out.push_str(&format!("  {} = load i1, i1* {}\n", res_name, ptr_name));
                    }
                    AirType::F64 => {
                        out.push_str(&format!("  {} = load double, double* {}\n", res_name, ptr_name));
                    }
                    _ => {
                        out.push_str(&format!("  {} = load %SylvelVal, %SylvelVal* {}\n", res_name, ptr_name));
                    }
                }
            }

            Inst::RuntimeCall(res, rt_fn, args) => {
                self.emit_runtime_call(rt_fn, args, res, out, ctx);
            }

            Inst::Call(_, name, args) => {
                let target_name = if name.starts_with("lyn_fn_") {
                    name.clone()
                } else if name.starts_with('@') {
                    name.trim_start_matches('@').to_string()
                } else {
                    format!("lyn_fn_{}", name)
                };
                let fn_name = format!("@{}", target_name);
                let expected_arity = self.module.functions.iter()
                    .find(|f| f.name == target_name)
                    .map(|f| f.params.len())
                    .unwrap_or(args.len());

                let mut actual_args = args.to_vec();
                while actual_args.len() < expected_arity {
                    actual_args.push(args[0]);
                }
                actual_args.truncate(expected_arity);

                let arg_strs: Vec<String> = actual_args.iter()
                    .map(|a| format!("%SylvelVal* {}", ctx.val_name(*a)))
                    .collect();
                out.push_str(&format!("  call void {}({})\n", fn_name, arg_strs.join(", ")));
            }

            Inst::Branch(cond, then_bb, else_bb) => {
                let cond_name = ctx.val_name(*cond);
                out.push_str(&format!(
                    "  br i1 {}, label %{}, label %{}\n",
                    cond_name,
                    ctx.block_label(*then_bb),
                    ctx.block_label(*else_bb)
                ));
            }

            Inst::Jump(target) => {
                out.push_str(&format!("  br label %{}\n", ctx.block_label(*target)));
            }

            Inst::Return(v) => {
                if *v == VOID_VALUE {
                    out.push_str("  ret void\n");
                } else {
                    // Main returns i32.
                    let val_name = ctx.val_name(*v);
                    if let Ok(imm) = val_name.parse::<i64>() {
                        out.push_str(&format!("  ret i32 {}\n", imm as i32));
                    } else {
                        let tmp = ctx.fresh();
                        out.push_str(&format!(
                            "  {} = call i64 @sylvel_rt_to_int(%SylvelVal* {})\n",
                            tmp, val_name
                        ));
                        let tmp2 = ctx.fresh();
                        out.push_str(&format!(
                            "  {} = trunc i64 {} to i32\n", tmp2, tmp
                        ));
                        out.push_str(&format!("  ret i32 {}\n", tmp2));
                    }
                }
            }

            Inst::Unreachable => {
                out.push_str("  unreachable\n");
            }

            // High-level integer ops — these appear after constant folding.
            // Emit as LLVM arithmetic directly.
            Inst::IAdd(res, a, b) => {
                out.push_str(&format!(
                    "  {} = add i64 {}, {}\n",
                    ctx.val_name(*res), ctx.val_name(*a), ctx.val_name(*b)
                ));
            }
            Inst::ISub(res, a, b) => {
                out.push_str(&format!(
                    "  {} = sub i64 {}, {}\n",
                    ctx.val_name(*res), ctx.val_name(*a), ctx.val_name(*b)
                ));
            }
            Inst::IMul(res, a, b) => {
                out.push_str(&format!(
                    "  {} = mul i64 {}, {}\n",
                    ctx.val_name(*res), ctx.val_name(*a), ctx.val_name(*b)
                ));
            }
            Inst::ICmpEq(res, a, b) => {
                out.push_str(&format!(
                    "  {} = icmp eq i64 {}, {}\n",
                    ctx.val_name(*res), ctx.val_name(*a), ctx.val_name(*b)
                ));
            }
            Inst::ICmpSlt(res, a, b) => {
                out.push_str(&format!(
                    "  {} = icmp slt i64 {}, {}\n",
                    ctx.val_name(*res), ctx.val_name(*a), ctx.val_name(*b)
                ));
            }
            Inst::ICmpSle(res, a, b) => {
                out.push_str(&format!(
                    "  {} = icmp sle i64 {}, {}\n",
                    ctx.val_name(*res), ctx.val_name(*a), ctx.val_name(*b)
                ));
            }

            Inst::GepField(res, ptr, idx) => {
                out.push_str(&format!(
                    "  {} = getelementptr inbounds %SylvelVal, %SylvelVal* {}, i32 0, i32 {}\n",
                    ctx.val_name(*res), ctx.val_name(*ptr), idx
                ));
            }

            // Ownership instructions — currently no-ops in the LLVM backend.
            Inst::Retain(v) => {
                out.push_str(&format!("  call void @sylvel_rt_retain(%SylvelVal* {})\n", ctx.val_name(*v)));
            }
            Inst::Release(v) => {
                out.push_str(&format!("  call void @sylvel_rt_release(%SylvelVal* {})\n", ctx.val_name(*v)));
            }
            Inst::Move { dest, src } | Inst::Copy { dest, src } => {
                // Treated as a store/load copy for now.
                let tmp = ctx.fresh();
                out.push_str(&format!("  {} = load %SylvelVal, %SylvelVal* {}\n", tmp, ctx.val_name(*src)));
                out.push_str(&format!("  store %SylvelVal {}, %SylvelVal* {}\n", tmp, ctx.val_name(*dest)));
            }

            Inst::DebugLoc(_) => {} // Stripped in release.
        }
    }

    fn emit_runtime_call(
        &self,
        rt_fn: &RuntimeFn,
        args: &[Value],
        result: &Value,
        out: &mut String,
        ctx: &mut FuncEmitCtx,
    ) {
        let c_name = rt_fn.c_name();

        match rt_fn {
            // ── Constructors (first arg = out ptr, rest = value args) ────────

            RuntimeFn::MakeNull => {
                if !args.is_empty() {
                    let dst = ctx.val_name(args[0]);
                    out.push_str(&format!("  call void @sylvel_rt_make_null(%SylvelVal* {})\n", dst));
                }
            }
            RuntimeFn::MakeBool => {
                if args.len() >= 2 {
                    let dst = ctx.val_name(args[0]);
                    let bval = ctx.val_name(args[1]);
                    out.push_str(&format!("  call void @sylvel_rt_make_bool(%SylvelVal* {}, i32 {})\n", dst, bval));
                }
            }
            RuntimeFn::MakeInt => {
                if args.len() >= 2 {
                    let dst = ctx.val_name(args[0]);
                    let ival = ctx.val_name(args[1]);
                    out.push_str(&format!("  call void @sylvel_rt_make_int(%SylvelVal* {}, i64 {})\n", dst, ival));
                }
            }
            RuntimeFn::MakeFloat => {
                if args.len() >= 2 {
                    let dst = ctx.val_name(args[0]);
                    let fval = ctx.val_name(args[1]);
                    out.push_str(&format!("  call void @sylvel_rt_make_float(%SylvelVal* {}, double {})\n", dst, fval));
                }
            }
            RuntimeFn::AllocString => {
                if args.len() >= 2 {
                    let dst = ctx.val_name(args[0]);
                    let str_ptr = ctx.resolve_str_ptr(&args[1], out);
                    out.push_str(&format!("  call void @sylvel_rt_alloc_string(%SylvelVal* {}, i8* {})\n", dst, str_ptr));
                }
            }
            RuntimeFn::AllocList => {
                if args.len() >= 2 {
                    let dst = ctx.val_name(args[0]);
                    let cap = ctx.val_name(args[1]);
                    out.push_str(&format!("  call void @sylvel_rt_alloc_list(%SylvelVal* {}, i64 {})\n", dst, cap));
                }
            }
            RuntimeFn::AllocMap => {
                if args.len() >= 2 {
                    let dst = ctx.val_name(args[0]);
                    let cap = ctx.val_name(args[1]);
                    out.push_str(&format!("  call void @sylvel_rt_alloc_map(%SylvelVal* {}, i64 {})\n", dst, cap));
                }
            }

            // ── Extractors ────────────────────────────────────────────────────

            RuntimeFn::ToBool => {
                if !args.is_empty() {
                    let src = ctx.val_name(args[0]);
                    let res = ctx.val_name(*result);
                    out.push_str(&format!("  {} = call i1 @sylvel_rt_to_bool(%SylvelVal* {})\n", res, src));
                }
            }
            RuntimeFn::ToInt => {
                if !args.is_empty() {
                    let src = ctx.val_name(args[0]);
                    let res = ctx.val_name(*result);
                    out.push_str(&format!("  {} = call i64 @sylvel_rt_to_int(%SylvelVal* {})\n", res, src));
                }
            }
            RuntimeFn::ToFloat => {
                if !args.is_empty() {
                    let src = ctx.val_name(args[0]);
                    let res = ctx.val_name(*result);
                    out.push_str(&format!("  {} = call double @sylvel_rt_to_float(%SylvelVal* {})\n", res, src));
                }
            }

            // ── Operations ────────────────────────────────────────────────────

            RuntimeFn::BinOp => {
                if args.len() >= 4 {
                    let dst  = ctx.val_name(args[0]);
                    let left = ctx.val_name(args[1]);
                    let op   = ctx.val_name(args[2]);
                    let right = ctx.val_name(args[3]);
                    out.push_str(&format!(
                        "  call void @sylvel_rt_bin_op(%SylvelVal* {}, %SylvelVal* {}, i32 {}, %SylvelVal* {})\n",
                        dst, left, op, right
                    ));
                }
            }
            RuntimeFn::UnaryOp => {
                let dst = ctx.val_name(args[0]);
                let op  = ctx.val_name(args[1]);
                let src = ctx.val_name(args[2]);
                out.push_str(&format!(
                    "  call void @sylvel_rt_unary_op(%SylvelVal* {}, i32 {}, %SylvelVal* {})\n",
                    dst, op, src
                ));
            }
            RuntimeFn::Print => {
                let val = ctx.val_name(args[0]);
                out.push_str(&format!("  call void @sylvel_rt_print(%SylvelVal* {})\n", val));
            }
            RuntimeFn::Len => {
                let val = ctx.val_name(args[0]);
                let res = ctx.val_name(*result);
                out.push_str(&format!("  {} = call i64 @sylvel_rt_len(%SylvelVal* {})\n", res, val));
            }
            RuntimeFn::ListPush => {
                let list = ctx.val_name(args[0]);
                let item = ctx.val_name(args[1]);
                out.push_str(&format!("  call void @sylvel_rt_list_push(%SylvelVal* {}, %SylvelVal* {})\n", list, item));
            }
            RuntimeFn::ListGet => {
                let dst  = ctx.val_name(args[0]);
                let list = ctx.val_name(args[1]);
                let idx  = ctx.val_name(args[2]);
                out.push_str(&format!("  call void @sylvel_rt_list_get(%SylvelVal* {}, %SylvelVal* {}, i64 {})\n", dst, list, idx));
            }
            RuntimeFn::ListSet => {
                let list = ctx.val_name(args[0]);
                let idx  = ctx.val_name(args[1]);
                let val  = ctx.val_name(args[2]);
                out.push_str(&format!("  call void @sylvel_rt_list_set(%SylvelVal* {}, i64 {}, %SylvelVal* {})\n", list, idx, val));
            }
            RuntimeFn::MapGet => {
                let dst = ctx.val_name(args[0]);
                let map = ctx.val_name(args[1]);
                let key = ctx.val_name(args[2]);
                out.push_str(&format!("  call void @sylvel_rt_map_get(%SylvelVal* {}, %SylvelVal* {}, %SylvelVal* {})\n", dst, map, key));
            }
            RuntimeFn::MapSet => {
                let map = ctx.val_name(args[0]);
                let key = ctx.val_name(args[1]);
                let val = ctx.val_name(args[2]);
                out.push_str(&format!("  call void @sylvel_rt_map_set(%SylvelVal* {}, %SylvelVal* {}, %SylvelVal* {})\n", map, key, val));
            }
            RuntimeFn::SubscriptGet => {
                let dst    = ctx.val_name(args[0]);
                let target = ctx.val_name(args[1]);
                let idx    = ctx.val_name(args[2]);
                out.push_str(&format!("  call void @sylvel_rt_subscript_get(%SylvelVal* {}, %SylvelVal* {}, %SylvelVal* {})\n", dst, target, idx));
            }
            RuntimeFn::SubscriptSet => {
                let target = ctx.val_name(args[0]);
                let idx    = ctx.val_name(args[1]);
                let val    = ctx.val_name(args[2]);
                out.push_str(&format!("  call void @sylvel_rt_subscript_set(%SylvelVal* {}, %SylvelVal* {}, %SylvelVal* {})\n", target, idx, val));
            }
            RuntimeFn::CallExpr => {
                let dst     = ctx.val_name(args[0]);
                let callee  = ctx.val_name(args[1]);
                let arg1    = ctx.val_name(args[2]);
                let arg2    = ctx.val_name(args[3]);
                out.push_str(&format!("  call void @sylvel_rt_call_expr(%SylvelVal* {}, %SylvelVal* {}, %SylvelVal* {}, %SylvelVal* {})\n", dst, callee, arg1, arg2));
            }

            // ── Error handling ────────────────────────────────────────────────

            RuntimeFn::EnterTry  => out.push_str("  call void @sylvel_rt_enter_try()\n"),
            RuntimeFn::ExitTry   => out.push_str("  call void @sylvel_rt_exit_try()\n"),
            RuntimeFn::ClearError => out.push_str("  call void @sylvel_rt_clear_error()\n"),
            RuntimeFn::HasError  => {
                let res = ctx.val_name(*result);
                out.push_str(&format!("  {} = call i64 @sylvel_rt_has_error()\n", res));
            }
            RuntimeFn::RaiseError => {
                let msg = ctx.resolve_str_ptr(&args[0], out);
                out.push_str(&format!("  call void @sylvel_rt_raise_error(i8* {})\n", msg));
            }
            RuntimeFn::ThrowVal => {
                let val = ctx.val_name(args[0]);
                out.push_str(&format!("  call void @sylvel_rt_throw_val(%SylvelVal* {})\n", val));
            }
            RuntimeFn::GetErrorVal => {
                let val = ctx.val_name(args[0]);
                out.push_str(&format!("  call void @sylvel_rt_get_error_val(%SylvelVal* {})\n", val));
            }

            // ── Assert ────────────────────────────────────────────────────────

            RuntimeFn::BuiltinAssert => {
                let dst  = ctx.val_name(args[0]);
                let cond = ctx.val_name(args[1]);
                let msg  = ctx.val_name(args[2]);
                out.push_str(&format!("  call void @sylvel_rt_builtin_assert(%SylvelVal* {}, %SylvelVal* {}, %SylvelVal* {})\n", dst, cond, msg));
            }

            // ── ARC ───────────────────────────────────────────────────────────

            RuntimeFn::Retain  => { out.push_str(&format!("  call void @sylvel_rt_retain(%SylvelVal* {})\n", ctx.val_name(args[0]))); }
            RuntimeFn::Release => { out.push_str(&format!("  call void @sylvel_rt_release(%SylvelVal* {})\n", ctx.val_name(args[0]))); }

            // ── Named builtins ────────────────────────────────────────────────

            RuntimeFn::Builtin(name) => {
                let c_fn = format!("sylvel_rt_builtin_{}", name);
                let arg_strs: Vec<String> = args.iter()
                    .map(|a| format!("%SylvelVal* {}", ctx.val_name(*a)))
                    .collect();
                out.push_str(&format!("  call void @{}({})\n", c_fn, arg_strs.join(", ")));
            }

            // Fallback for any new variants not yet matched.
            _ => {
                out.push_str(&format!("  ; unimplemented runtime call: {}\n", c_name));
            }
        }
    }
}

// ─── Function emission context ────────────────────────────────────────────────

struct FuncEmitCtx<'a> {
    func:      &'a AirFunction,
    module:    &'a AirModule,
    /// Maps Value → inline constant string for ConstInt/ConstFloat/ConstBool
    /// so they can be emitted inline rather than as separate instructions.
    const_map: HashMap<Value, String>,
    alloc_types: HashMap<Value, AirType>,
    temp_seq:  usize,
    /// Maps BlockId → label string.
    block_labels: HashMap<BlockId, String>,
}

impl<'a> FuncEmitCtx<'a> {
    fn new(func: &'a AirFunction, module: &'a AirModule) -> Self {
        let mut const_map: HashMap<Value, String> = HashMap::new();
        let mut alloc_types: HashMap<Value, AirType> = HashMap::new();
        let mut block_labels: HashMap<BlockId, String> = HashMap::new();

        for block in &func.blocks {
            let unique_lbl = format!("{}_{}", block.label, block.id.0);
            block_labels.insert(block.id, unique_lbl);
            for inst in &block.insts {
                match inst {
                    Inst::Alloc(v, ty)     => { alloc_types.insert(*v, ty.clone()); }
                    Inst::ConstInt(v, i)   => { const_map.insert(*v, i.to_string()); }
                    Inst::ConstBool(v, b)  => { const_map.insert(*v, if *b { "1".to_string() } else { "0".to_string() }); }
                    Inst::ConstFloat(v, f) => {
                        let s = if f.fract() == 0.0 { format!("{:.1}", f) } else { format!("{}", f) };
                        const_map.insert(*v, s);
                    }
                    Inst::ConstStr(v, idx) => { const_map.insert(*v, idx.to_string()); }
                    _ => {}
                }
            }
        }

        // Register parameter values.
        for (i, param) in func.params.iter().enumerate() {
            if i == 0 {
                const_map.insert(param.value, "%out".to_string());
            } else {
                const_map.insert(param.value, format!("%param_{}", param.name));
            }
        }

        // Register global variable references.
        for (v, sym) in &func.global_val_map {
            const_map.insert(*v, sym.clone());
        }

        FuncEmitCtx { func, module, const_map, alloc_types, temp_seq: 0, block_labels }
    }

    fn fresh(&mut self) -> String {
        self.temp_seq += 1;
        format!("%_tmp{}", self.temp_seq)
    }

    fn val_name(&self, v: Value) -> String {
        if let Some(c) = self.const_map.get(&v) {
            return c.clone();
        }
        format!("%t{}", v.0)
    }

    fn block_label(&self, id: BlockId) -> String {
        self.block_labels.get(&id).cloned().unwrap_or_else(|| format!("bb{}", id.0))
    }

    /// Resolve a ConstStr Value to an LLVM `i8*` GEP expression, emitting
    /// the GEP instruction into `out`.
    fn resolve_str_ptr(&mut self, val: &Value, out: &mut String) -> String {
        // Check if it's inline-mapped as an integer index.
        if let Some(idx_str) = self.const_map.get(val) {
            if let Ok(idx) = idx_str.parse::<usize>() {
                let str_content = self.module.string_table.get(idx).cloned().unwrap_or_default();
                let len = str_content.as_bytes().len() + 1;
                let label = format!("@str.{}", idx);
                let tmp = self.fresh();
                out.push_str(&format!(
                    "  {} = getelementptr inbounds [{} x i8], [{} x i8]* {}, i64 0, i64 0\n",
                    tmp, len, len, label
                ));
                return tmp;
            }
        }
        // Fallback: treat val as a SylvelVal* containing a string (dynamic case).
        format!("%t{}", val.0)
    }
}

// ─── AIR type to LLVM type string ─────────────────────────────────────────────

fn air_type_to_llvm(ty: &AirType) -> &'static str {
    match ty {
        AirType::Void         => "void",
        AirType::Bool         => "i1",
        AirType::I64          => "i64",
        AirType::F64          => "double",
        AirType::Ptr          => "i8*",
        AirType::SylvelVal    => "%SylvelVal",
        AirType::SylvelValPtr => "%SylvelVal*",
        AirType::FnRef(..)    => "i8*",
        AirType::Aggregate(_) => "%SylvelVal", // simplified
    }
}

// ─── String escaping ─────────────────────────────────────────────────────────

fn escape_string(s: &str) -> String {
    s.bytes().map(|b| match b {
        b'\n' => "\\0A".to_string(),
        b'\r' => "\\0D".to_string(),
        b'\t' => "\\09".to_string(),
        b'"' | b'\\' => format!("\\{:02X}", b),
        32..=126 => (b as char).to_string(),
        _ => format!("\\{:02X}", b),
    }).collect()
}

/// Public entry-point for the LLVM IR generation stage.
pub fn lower_to_llvm(module: &AirModule, target: &Target) -> String {
    let mut gen = LlvmIrGen::new(module, target);
    gen.emit()
}
