// optimizer/passes/dce.rs — Dead Code Elimination pass
//
// Removes instructions whose defined Values are never used anywhere in the
// function.  Only eliminates pure, side-effect-free instructions (constants,
// allocs, loads, arithmetic).  RuntimeCalls, Stores, and terminators are
// never removed.

use std::collections::HashSet;
use crate::air::{AirModule, AirFunction, Inst, Value, VOID_VALUE};

pub struct DeadCodeElim;

impl DeadCodeElim {
    pub fn run(module: &mut AirModule) -> bool {
        let mut changed = false;
        for func in &mut module.functions {
            changed |= elim_function(func);
        }
        changed
    }
}

fn elim_function(func: &mut AirFunction) -> bool {
    // Collect all values that appear as inputs anywhere in the function.
    let mut used: HashSet<Value> = HashSet::new();

    // Also count function parameters as used.
    for param in &func.params {
        used.insert(param.value);
    }

    for block in &func.blocks {
        for inst in &block.insts {
            for v in inst.used_values() {
                used.insert(v);
            }
        }
    }

    let mut changed = false;
    for block in &mut func.blocks {
        let before_len = block.insts.len();
        block.insts.retain(|inst| {
            // Check if this instruction is purely dead.
            if let Some(def) = inst.defined_value() {
                if def != VOID_VALUE && !used.contains(&def) && is_pure(inst) {
                    return false; // remove
                }
            }
            true
        });
        if block.insts.len() != before_len {
            changed = true;
        }
    }
    changed
}

/// Returns true if the instruction has no observable side effects and can be
/// safely removed if its result is unused.
fn is_pure(inst: &Inst) -> bool {
    matches!(inst,
        Inst::ConstNull(_) | Inst::ConstBool(_, _) | Inst::ConstInt(_, _)
        | Inst::ConstFloat(_, _) | Inst::ConstStr(_, _)
        | Inst::Load(_, _)
        | Inst::IAdd(_, _, _) | Inst::ISub(_, _, _) | Inst::IMul(_, _, _)
        | Inst::ICmpEq(_, _, _) | Inst::ICmpSlt(_, _, _) | Inst::ICmpSle(_, _, _)
        | Inst::GepField(_, _, _)
    )
}
