// air/printer.rs — Human-readable AIR text printer
//
// Produces output for `--emit-air` and `--emit-air-opt` driver flags.
// The format is modelled on LLVM IR / Swift SIL: one instruction per line,
// blocks labelled, values prefixed with `%v`.

use std::fmt::Write;
use super::{AirModule, AirFunction, BasicBlock, Inst, Value, VOID_VALUE, RuntimeFn};

pub struct AirPrinter;

impl AirPrinter {
    pub fn print_module(module: &AirModule) -> String {
        let mut out = String::new();
        writeln!(out, "; AIR Module: {}", module.name).ok();
        writeln!(out, "; String table ({} entries)", module.string_table.len()).ok();
        for (i, s) in module.string_table.iter().enumerate() {
            writeln!(out, ";   [{}] {:?}", i, s).ok();
        }
        writeln!(out).ok();

        for func in &module.functions {
            Self::print_function(&mut out, func);
            writeln!(out).ok();
        }
        out
    }

    fn print_function(out: &mut String, func: &AirFunction) {
        // Signature line
        write!(out, "fn @{}(", func.name).ok();
        let params: Vec<String> = func.params.iter()
            .map(|p| format!("{}: {}", p.value, p.ty))
            .collect();
        writeln!(out, "{}) -> {} {{", params.join(", "), func.ret_ty).ok();

        for block in &func.blocks {
            Self::print_block(out, block);
        }
        writeln!(out, "}}").ok();
    }

    fn print_block(out: &mut String, block: &BasicBlock) {
        // Predecessors comment
        if !block.preds.is_empty() {
            let preds: Vec<String> = block.preds.iter().map(|b| b.to_string()).collect();
            writeln!(out, "  ; preds: {}", preds.join(", ")).ok();
        }
        writeln!(out, "{}:", block.label).ok();

        for inst in &block.insts {
            write!(out, "  ").ok();
            Self::print_inst(out, inst);
            writeln!(out).ok();
        }
    }

    fn print_inst(out: &mut String, inst: &Inst) {
        match inst {
            Inst::ConstNull(v)       => write!(out, "{} = const_null", v).ok(),
            Inst::ConstBool(v, b)    => write!(out, "{} = const_bool {}", v, b).ok(),
            Inst::ConstInt(v, i)     => write!(out, "{} = const_int {}", v, i).ok(),
            Inst::ConstFloat(v, f)   => write!(out, "{} = const_float {}", v, f).ok(),
            Inst::ConstStr(v, idx)   => write!(out, "{} = const_str [{}]", v, idx).ok(),

            Inst::Alloc(v, ty)       => write!(out, "{} = alloc {}", v, ty).ok(),
            Inst::Load(v, ptr)       => write!(out, "{} = load {}", v, ptr).ok(),
            Inst::Store(val, ptr)    => write!(out, "store {} -> {}", val, ptr).ok(),

            Inst::RuntimeCall(res, rt_fn, args) => {
                let args_str: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                if *res == VOID_VALUE {
                    write!(out, "runtime_call @{}({})", rt_fn.c_name(), args_str.join(", ")).ok()
                } else {
                    write!(out, "{} = runtime_call @{}({})", res, rt_fn.c_name(), args_str.join(", ")).ok()
                }
            }

            Inst::Call(res, name, args) => {
                let args_str: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                if *res == VOID_VALUE {
                    write!(out, "call @{}({})", name, args_str.join(", ")).ok()
                } else {
                    write!(out, "{} = call @{}({})", res, name, args_str.join(", ")).ok()
                }
            }

            Inst::Branch(cond, t, e) => write!(out, "branch {} ? {} : {}", cond, t, e).ok(),
            Inst::Jump(t)            => write!(out, "jump {}", t).ok(),
            Inst::Return(v) if *v == VOID_VALUE => write!(out, "return").ok(),
            Inst::Return(v)          => write!(out, "return {}", v).ok(),
            Inst::Unreachable        => write!(out, "unreachable").ok(),

            Inst::IAdd(v, a, b)      => write!(out, "{} = iadd {}, {}", v, a, b).ok(),
            Inst::ISub(v, a, b)      => write!(out, "{} = isub {}, {}", v, a, b).ok(),
            Inst::IMul(v, a, b)      => write!(out, "{} = imul {}, {}", v, a, b).ok(),
            Inst::ICmpEq(v, a, b)    => write!(out, "{} = icmp_eq {}, {}", v, a, b).ok(),
            Inst::ICmpSlt(v, a, b)   => write!(out, "{} = icmp_slt {}, {}", v, a, b).ok(),
            Inst::ICmpSle(v, a, b)   => write!(out, "{} = icmp_sle {}, {}", v, a, b).ok(),

            Inst::Retain(v)          => write!(out, "retain {}", v).ok(),
            Inst::Release(v)         => write!(out, "release {}", v).ok(),
            Inst::Move { dest, src } => write!(out, "{} = move {}", dest, src).ok(),
            Inst::Copy { dest, src } => write!(out, "{} = copy {}", dest, src).ok(),

            Inst::DebugLoc(span) => write!(out, "; loc {}", span).ok(),

            Inst::GepField(v, ptr, idx) => write!(out, "{} = gep_field {}, {}", v, ptr, idx).ok(),
        };
    }
}
