// optimizer/passes/redundant_elim.rs — Redundant instruction elimination
//
// Removes duplicate Alloc instructions for the same value ID (can arise
// when multiple codegen paths emit the same pre-allocation).
// Also deduplicates consecutive identical pure instructions within a block.

use std::collections::HashSet;
use crate::air::{AirModule, AirFunction, BasicBlock, Inst, Value};

pub struct RedundantElim;

impl RedundantElim {
    pub fn run(module: &mut AirModule) -> bool {
        let mut changed = false;
        for func in &mut module.functions {
            changed |= elim_function(func);
        }
        changed
    }
}

fn elim_function(func: &mut AirFunction) -> bool {
    let mut changed = false;

    // Deduplicate Alloc instructions: keep only the first Alloc for each Value.
    let mut seen_allocs: HashSet<Value> = HashSet::new();
    for block in &mut func.blocks {
        let before = block.insts.len();
        block.insts.retain(|inst| {
            if let Inst::Alloc(v, _) = inst {
                if seen_allocs.contains(v) {
                    return false; // duplicate alloc
                }
                seen_allocs.insert(*v);
            }
            true
        });
        if block.insts.len() != before { changed = true; }
    }

    // Remove consecutive identical DebugLoc instructions.
    for block in &mut func.blocks {
        let before = block.insts.len();
        let mut last_debug_span: Option<crate::ast::Span> = None;
        block.insts.retain(|inst| {
            if let Inst::DebugLoc(span) = inst {
                if last_debug_span.as_ref() == Some(span) {
                    return false; // duplicate debug loc
                }
                last_debug_span = Some(*span);
            } else {
                // Non-debug instructions don't reset the span tracker;
                // consecutive identical debug locs on either side of an
                // instruction would still be deduplicated.
            }
            true
        });
        if block.insts.len() != before { changed = true; }
    }

    changed
}
