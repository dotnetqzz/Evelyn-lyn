// optimizer/passes/const_prop.rs — Constant propagation pass

use std::collections::HashMap;
use crate::air::{AirModule, AirFunction, BasicBlock, Inst, Value};

pub struct ConstProp;

impl ConstProp {
    pub fn run(module: &mut AirModule) -> bool {
        let mut changed = false;
        for func in &mut module.functions {
            changed |= prop_function(func);
        }
        changed
    }
}

fn prop_function(func: &mut AirFunction) -> bool {
    let mut changed = false;
    for block in &mut func.blocks {
        changed |= prop_block(block);
    }
    changed
}

fn prop_block(block: &mut BasicBlock) -> bool {
    #[derive(Clone, Debug)]
    enum KnownConst {
        Int(i64),
        Bool(bool),
        Float(f64),
    }

    let mut store_consts: HashMap<Value, KnownConst> = HashMap::new();
    let mut val_consts:   HashMap<Value, KnownConst> = HashMap::new();
    let mut replacements: Vec<(usize, Inst)> = Vec::new();

    for (idx, inst) in block.insts.iter().enumerate() {
        match inst {
            Inst::ConstInt(v, i) => {
                val_consts.insert(*v, KnownConst::Int(*i));
            }
            Inst::ConstBool(v, b) => {
                val_consts.insert(*v, KnownConst::Bool(*b));
            }
            Inst::ConstFloat(v, f) => {
                val_consts.insert(*v, KnownConst::Float(*f));
            }
            Inst::Store(val, ptr) => {
                if let Some(kc) = val_consts.get(val).cloned() {
                    store_consts.insert(*ptr, kc);
                } else {
                    store_consts.remove(ptr);
                }
            }
            Inst::Load(res, ptr) => {
                if let Some(kc) = store_consts.get(ptr).cloned() {
                    let r = *res;
                    match kc.clone() {
                        KnownConst::Int(i) => {
                            val_consts.insert(r, KnownConst::Int(i));
                            replacements.push((idx, Inst::ConstInt(r, i)));
                        }
                        KnownConst::Bool(b) => {
                            val_consts.insert(r, KnownConst::Bool(b));
                            replacements.push((idx, Inst::ConstBool(r, b)));
                        }
                        KnownConst::Float(f) => {
                            val_consts.insert(r, KnownConst::Float(f));
                            replacements.push((idx, Inst::ConstFloat(r, f)));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let changed = !replacements.is_empty();
    for (idx, new_inst) in replacements {
        block.insts[idx] = new_inst;
    }
    changed
}
