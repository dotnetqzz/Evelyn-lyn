// irgen/runtime_lowering.rs — AIR RuntimeFn → LLVM IR text declarations
//
// This module knows only about LLVM IR text syntax and the C ABI.
// It has no knowledge of Avelyn language semantics.

use std::collections::{HashMap, HashSet};
use crate::air::{AirModule, RuntimeFn};
use crate::airgen::runtime_map::llvm_signature;

/// Collect all RuntimeFn variants referenced in the module and emit LLVM
/// `declare` statements for each.
pub fn emit_declarations(module: &AirModule) -> String {
    let mut used_builtin_arities: HashMap<String, usize> = HashMap::new();
    let mut used_core: HashSet<String> = HashSet::new();

    for func in &module.functions {
        for block in &func.blocks {
            for inst in &block.insts {
                if let crate::air::Inst::RuntimeCall(_, rt_fn, args) = inst {
                    let c_name = rt_fn.c_name();
                    if c_name.starts_with("sylvel_rt_builtin_") {
                        used_builtin_arities.entry(c_name).or_insert(args.len());
                    } else {
                        used_core.insert(c_name);
                    }
                }
            }
        }
    }

    for (c_name, arity) in &module.extern_fns {
        if c_name.starts_with("sylvel_rt_builtin_") {
            used_builtin_arities.entry(c_name.clone()).or_insert(arity + 1);
        }
    }

    let mut out = String::new();

    let mut declared: HashSet<String> = HashSet::new();

    // Core runtime declarations (always emitted if referenced).
    let core_fns: &[RuntimeFn] = &[
        RuntimeFn::MakeNull, RuntimeFn::MakeBool, RuntimeFn::MakeInt,
        RuntimeFn::MakeFloat, RuntimeFn::AllocString, RuntimeFn::AllocStringLen,
        RuntimeFn::AllocList, RuntimeFn::AllocMap,
        RuntimeFn::ToBool, RuntimeFn::ToInt, RuntimeFn::ToFloat,
        RuntimeFn::Retain, RuntimeFn::Release,
        RuntimeFn::BinOp, RuntimeFn::UnaryOp, RuntimeFn::Print, RuntimeFn::Len,
        RuntimeFn::ListPush, RuntimeFn::ListGet, RuntimeFn::ListSet,
        RuntimeFn::MapGet, RuntimeFn::MapSet,
        RuntimeFn::SubscriptGet, RuntimeFn::SubscriptSet,
        RuntimeFn::CallExpr,
        RuntimeFn::EnterTry, RuntimeFn::ExitTry,
        RuntimeFn::HasError, RuntimeFn::ClearError,
        RuntimeFn::RaiseError, RuntimeFn::ThrowVal,
        RuntimeFn::BuiltinAssert,
    ];

    for rt_fn in core_fns {
        let c_name = rt_fn.c_name();
        if declared.contains(&c_name) { continue; }
        if !used_core.contains(&c_name) && !is_always_declared(&c_name) { continue; }
        declared.insert(c_name.clone());
        let (ret, params) = llvm_signature(rt_fn);
        out.push_str(&format!("declare {} @{}({})\n",
            ret, c_name, params.join(", ")));
    }

    // Builtin declarations
    for (c_name, param_count) in &used_builtin_arities {
        if declared.contains(c_name) { continue; }
        declared.insert(c_name.clone());
        let sylvel_ptr = "%SylvelVal*";
        let count = if *param_count == 0 { 1 } else { *param_count };
        let params: Vec<&str> = vec![sylvel_ptr; count];
        out.push_str(&format!("declare void @{}({})\n",
            c_name, params.join(", ")));
    }

    out.push('\n');
    out
}

fn is_always_declared(name: &str) -> bool {
    // These are always emitted regardless of usage (minimal runtime baseline).
    matches!(name,
        "sylvel_rt_make_null" | "sylvel_rt_make_int" | "sylvel_rt_make_float"
        | "sylvel_rt_make_bool" | "sylvel_rt_alloc_string"
        | "sylvel_rt_print" | "sylvel_rt_bin_op" | "sylvel_rt_unary_op"
        | "sylvel_rt_to_bool" | "sylvel_rt_to_int" | "sylvel_rt_to_float"
        | "sylvel_rt_retain" | "sylvel_rt_release"
        | "sylvel_rt_raise_error" | "sylvel_rt_throw_val"
        | "sylvel_rt_has_error" | "sylvel_rt_clear_error"
        | "sylvel_rt_enter_try" | "sylvel_rt_exit_try"
        | "sylvel_rt_len" | "sylvel_rt_alloc_list" | "sylvel_rt_alloc_map"
        | "sylvel_rt_list_push" | "sylvel_rt_list_get" | "sylvel_rt_list_set"
        | "sylvel_rt_map_get" | "sylvel_rt_map_set"
        | "sylvel_rt_subscript_get" | "sylvel_rt_subscript_set"
        | "sylvel_rt_call_expr" | "sylvel_rt_builtin_assert"
    )
}
