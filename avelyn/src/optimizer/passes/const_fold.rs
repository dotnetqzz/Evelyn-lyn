// optimizer/passes/const_fold.rs — Constant folding pass
//
// Replaces arithmetic/comparison instructions whose operands are both
// compile-time constants with a single ConstInt/ConstBool/ConstFloat.
// Operates block-by-block in a single forward pass.

use std::collections::HashMap;
use crate::air::{AirModule, AirFunction, BasicBlock, Inst, Value};

pub struct ConstFold;

impl ConstFold {
    pub fn run(module: &mut AirModule) -> bool {
        let mut changed = false;
        for func in &mut module.functions {
            changed |= fold_function(func);
        }
        changed
    }
}

fn fold_function(func: &mut AirFunction) -> bool {
    let mut changed = false;
    for block in &mut func.blocks {
        changed |= fold_block(block);
    }
    changed
}

fn fold_block(block: &mut BasicBlock) -> bool {
    let mut const_ints: HashMap<Value, i64>  = HashMap::new();
    let mut const_floats: HashMap<Value, f64> = HashMap::new();
    let mut changed = false;
    let mut replacements: Vec<(usize, Inst)> = Vec::new();

    for (idx, inst) in block.insts.iter().enumerate() {
        match inst {
            Inst::ConstInt(v, i) => { const_ints.insert(*v, *i); }
            Inst::ConstFloat(v, f) => { const_floats.insert(*v, *f); }

            Inst::IAdd(res, a, b) => {
                if let (Some(&av), Some(&bv)) = (const_ints.get(a), const_ints.get(b)) {
                    if let Some(folded) = av.checked_add(bv) {
                        let r = *res;
                        replacements.push((idx, Inst::ConstInt(r, folded)));
                        const_ints.insert(r, folded);
                    }
                }
            }
            Inst::ISub(res, a, b) => {
                if let (Some(&av), Some(&bv)) = (const_ints.get(a), const_ints.get(b)) {
                    if let Some(folded) = av.checked_sub(bv) {
                        let r = *res;
                        replacements.push((idx, Inst::ConstInt(r, folded)));
                        const_ints.insert(r, folded);
                    }
                }
            }
            Inst::IMul(res, a, b) => {
                if let (Some(&av), Some(&bv)) = (const_ints.get(a), const_ints.get(b)) {
                    if let Some(folded) = av.checked_mul(bv) {
                        let r = *res;
                        replacements.push((idx, Inst::ConstInt(r, folded)));
                        const_ints.insert(r, folded);
                    }
                }
            }
            Inst::ICmpEq(res, a, b) => {
                if let (Some(&av), Some(&bv)) = (const_ints.get(a), const_ints.get(b)) {
                    let r = *res;
                    replacements.push((idx, Inst::ConstBool(r, av == bv)));
                }
            }
            Inst::ICmpSlt(res, a, b) => {
                if let (Some(&av), Some(&bv)) = (const_ints.get(a), const_ints.get(b)) {
                    let r = *res;
                    replacements.push((idx, Inst::ConstBool(r, av < bv)));
                }
            }
            Inst::ICmpSle(res, a, b) => {
                if let (Some(&av), Some(&bv)) = (const_ints.get(a), const_ints.get(b)) {
                    let r = *res;
                    replacements.push((idx, Inst::ConstBool(r, av <= bv)));
                }
            }
            _ => {}
        }
    }

    for (idx, new_inst) in replacements {
        block.insts[idx] = new_inst;
        changed = true;
    }
    changed
}
