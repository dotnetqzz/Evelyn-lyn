// airgen/runtime_map.rs — Single source of truth for the C runtime ABI
//
// Maps logical compiler operations to RuntimeFn enum variants and records
// each external function's arity so the LLVM IRGen can emit the correct
// `declare` statements.
//
// The `BinOpCode` and `UnaryOpCode` integer constants exactly match the
// dispatch table in `sylvel_runtime.c`.

use crate::air::{RuntimeFn, BinOpCode, UnaryOpCode, AirType};

/// Description of a runtime function: its logical name and parameter count
/// (not including the output pointer, which is always first for void-ret fns).
pub struct RuntimeDesc {
    pub fn_variant: RuntimeFn,
    pub param_count: usize,  // number of SylvelVal* args (excluding out ptr)
}

/// Look up the RuntimeFn for a named builtin (e.g. "len", "arrayPush").
/// Returns None if the function should be dispatched through the interpreter
/// path and is not available in the native runtime.
pub fn builtin_runtime_fn(name: &str) -> RuntimeFn {
    // Map interpreter builtin names → runtime variants
    RuntimeFn::Builtin(name.to_string())
}

/// Convert a binary operator string to its integer op-code for the runtime.
pub fn binop_code(op: &str) -> i32 {
    BinOpCode::from_str(op) as i32
}

/// Convert a unary operator string to its integer op-code for the runtime.
pub fn unaryop_code(op: &str) -> i32 {
    UnaryOpCode::from_str(op) as i32
}

/// Return the LLVM `declare` signature for a RuntimeFn.
/// Format: `(return_type, Vec<param_llvm_types>)`
pub fn llvm_signature(rt_fn: &RuntimeFn) -> (String, Vec<String>) {
    let sylvel_ptr = "%SylvelVal*".to_string();
    let void       = "void".to_string();
    let i32_ty     = "i32".to_string();
    let i64_ty     = "i64".to_string();
    let i1_ty      = "i1".to_string();
    let double_ty  = "double".to_string();
    let i8_ptr     = "i8*".to_string();

    match rt_fn {
        RuntimeFn::MakeNull      => (void.clone(),    vec![sylvel_ptr.clone()]),
        RuntimeFn::MakeBool      => (void.clone(),    vec![sylvel_ptr.clone(), i32_ty.clone()]),
        RuntimeFn::MakeInt       => (void.clone(),    vec![sylvel_ptr.clone(), i64_ty.clone()]),
        RuntimeFn::MakeFloat     => (void.clone(),    vec![sylvel_ptr.clone(), double_ty.clone()]),
        RuntimeFn::AllocString   => (void.clone(),    vec![sylvel_ptr.clone(), i8_ptr.clone()]),
        RuntimeFn::AllocStringLen => (void.clone(),   vec![sylvel_ptr.clone(), i8_ptr.clone(), i64_ty.clone()]),
        RuntimeFn::AllocList     => (void.clone(),    vec![sylvel_ptr.clone(), i64_ty.clone()]),
        RuntimeFn::AllocMap      => (void.clone(),    vec![sylvel_ptr.clone(), i64_ty.clone()]),
        RuntimeFn::ToBool        => (i1_ty.clone(),   vec![sylvel_ptr.clone()]),
        RuntimeFn::ToInt         => (i64_ty.clone(),  vec![sylvel_ptr.clone()]),
        RuntimeFn::ToFloat       => (double_ty.clone(),vec![sylvel_ptr.clone()]),
        RuntimeFn::Retain        => (void.clone(),    vec![sylvel_ptr.clone()]),
        RuntimeFn::Release       => (void.clone(),    vec![sylvel_ptr.clone()]),
        RuntimeFn::BinOp         => (void.clone(),    vec![sylvel_ptr.clone(), sylvel_ptr.clone(), i32_ty.clone(), sylvel_ptr.clone()]),
        RuntimeFn::UnaryOp       => (void.clone(),    vec![sylvel_ptr.clone(), i32_ty.clone(), sylvel_ptr.clone()]),
        RuntimeFn::Print         => (void.clone(),    vec![sylvel_ptr.clone()]),
        RuntimeFn::Len           => (i64_ty.clone(),  vec![sylvel_ptr.clone()]),
        RuntimeFn::ListPush      => (void.clone(),    vec![sylvel_ptr.clone(), sylvel_ptr.clone()]),
        RuntimeFn::ListGet       => (void.clone(),    vec![sylvel_ptr.clone(), sylvel_ptr.clone(), i64_ty.clone()]),
        RuntimeFn::ListSet       => (void.clone(),    vec![sylvel_ptr.clone(), i64_ty.clone(), sylvel_ptr.clone()]),
        RuntimeFn::MapGet        => (void.clone(),    vec![sylvel_ptr.clone(), sylvel_ptr.clone(), sylvel_ptr.clone()]),
        RuntimeFn::MapSet        => (void.clone(),    vec![sylvel_ptr.clone(), sylvel_ptr.clone(), sylvel_ptr.clone()]),
        RuntimeFn::SubscriptGet  => (void.clone(),    vec![sylvel_ptr.clone(), sylvel_ptr.clone(), sylvel_ptr.clone()]),
        RuntimeFn::SubscriptSet  => (void.clone(),    vec![sylvel_ptr.clone(), sylvel_ptr.clone(), sylvel_ptr.clone()]),
        RuntimeFn::CallExpr      => (void.clone(),    vec![sylvel_ptr.clone(), sylvel_ptr.clone(), sylvel_ptr.clone(), sylvel_ptr.clone()]),
        RuntimeFn::EnterTry      => (void.clone(),    vec![]),
        RuntimeFn::ExitTry       => (void.clone(),    vec![]),
        RuntimeFn::HasError      => (i64_ty.clone(),  vec![]),
        RuntimeFn::ClearError    => (void.clone(),    vec![]),
        RuntimeFn::RaiseError    => (void.clone(),    vec![i8_ptr.clone()]),
        RuntimeFn::ThrowVal      => (void.clone(),    vec![sylvel_ptr.clone()]),
        RuntimeFn::BuiltinAssert => (void.clone(),    vec![sylvel_ptr.clone(), sylvel_ptr.clone(), sylvel_ptr.clone()]),
        RuntimeFn::Builtin(_)    => {
            // Generic builtin: up to 4 SylvelVal* args + 1 out ptr = 5; we
            // emit the actual arity from the AirModule::extern_fns table.
            (void.clone(), vec![])
        }
    }
}
