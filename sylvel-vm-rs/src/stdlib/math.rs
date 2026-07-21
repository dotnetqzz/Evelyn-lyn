// stdlib/math.rs — Math builtins

use crate::value::{SylError, SylVal};
use crate::vm::Vm;

fn arg(args: &[SylVal], i: usize) -> SylVal {
    args.get(i).cloned().unwrap_or(SylVal::Null)
}
fn to_f(v: &SylVal) -> f64 { v.as_f64() }

macro_rules! unary_math {
    ($name:ident, $func:expr) => {
        pub fn $name(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
            Ok(SylVal::Float($func(to_f(&arg(args, 0)))))
        }
    };
}

unary_math!(native_sqrt,  f64::sqrt);
unary_math!(native_floor, f64::floor);
unary_math!(native_ceil,  f64::ceil);
unary_math!(native_round, f64::round);
unary_math!(native_sin,   f64::sin);
unary_math!(native_cos,   f64::cos);
unary_math!(native_tan,   f64::tan);
unary_math!(native_log,   f64::ln);
unary_math!(native_log2,  f64::log2);
unary_math!(native_log10, f64::log10);
unary_math!(native_exp,   f64::exp);

pub fn native_abs(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    match arg(args, 0) {
        SylVal::Int(i) => Ok(SylVal::Int(i.abs())),
        v => Ok(SylVal::Float(v.as_f64().abs())),
    }
}

pub fn native_pow(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let a = to_f(&arg(args, 0));
    let b = to_f(&arg(args, 1));
    Ok(SylVal::Float(a.powf(b)))
}

pub fn native_min(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let a = to_f(&arg(args, 0));
    let b = to_f(&arg(args, 1));
    Ok(SylVal::Float(if a < b { a } else { b }))
}

pub fn native_max(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let a = to_f(&arg(args, 0));
    let b = to_f(&arg(args, 1));
    Ok(SylVal::Float(if a > b { a } else { b }))
}

pub fn native_pi(_vm: &mut Vm, _args: &[SylVal]) -> Result<SylVal, SylError> {
    Ok(SylVal::Float(std::f64::consts::PI))
}

pub fn native_clamp(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let v = to_f(&arg(args, 0));
    let lo = to_f(&arg(args, 1));
    let hi = to_f(&arg(args, 2));
    Ok(SylVal::Float(v.clamp(lo, hi)))
}

pub fn register(vm: &mut Vm) {
    // Short names
    vm.register_native("sqrt",       native_sqrt);
    vm.register_native("floor",      native_floor);
    vm.register_native("ceil",       native_ceil);
    vm.register_native("round",      native_round);
    vm.register_native("abs",        native_abs);
    vm.register_native("sin",        native_sin);
    vm.register_native("cos",        native_cos);
    vm.register_native("tan",        native_tan);
    vm.register_native("log",        native_log);
    vm.register_native("log2",       native_log2);
    vm.register_native("log10",      native_log10);
    vm.register_native("exp",        native_exp);
    vm.register_native("pow",        native_pow);
    vm.register_native("min",        native_min);
    vm.register_native("max",        native_max);
    vm.register_native("pi",         native_pi);
    vm.register_native("clamp",      native_clamp);
    // math.* prefix
    vm.register_native("math.sqrt",  native_sqrt);
    vm.register_native("math.floor", native_floor);
    vm.register_native("math.ceil",  native_ceil);
    vm.register_native("math.abs",   native_abs);
    vm.register_native("math.sin",   native_sin);
    vm.register_native("math.cos",   native_cos);
    vm.register_native("math.pow",   native_pow);
    vm.register_native("math.log",   native_log);
    vm.register_native("math.min",   native_min);
    vm.register_native("math.max",   native_max);
    // mathXxx camelCase prefix (used in .lyn tests)
    vm.register_native("mathSqrt",   native_sqrt);
    vm.register_native("mathFloor",  native_floor);
    vm.register_native("mathCeil",   native_ceil);
    vm.register_native("mathAbs",    native_abs);
    vm.register_native("mathRound",  native_round);
    vm.register_native("mathSin",    native_sin);
    vm.register_native("mathCos",    native_cos);
    vm.register_native("mathTan",    native_tan);
    vm.register_native("mathLog",    native_log);
    vm.register_native("mathLog2",   native_log2);
    vm.register_native("mathLog10",  native_log10);
    vm.register_native("mathExp",    native_exp);
    vm.register_native("mathPow",    native_pow);
    vm.register_native("mathMin",    native_min);
    vm.register_native("mathMax",    native_max);
    vm.register_native("mathPi",     native_pi);
    vm.register_native("mathClamp",  native_clamp);
}
