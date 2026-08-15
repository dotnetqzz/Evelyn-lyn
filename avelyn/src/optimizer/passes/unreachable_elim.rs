// optimizer/passes/unreachable_elim.rs — Unreachable block elimination
//
// Removes basic blocks that have no predecessors (except for the entry block).
// Such blocks are dead code: they can never execute.
// After removal, CFG edges are rebuilt.

use std::collections::HashSet;
use crate::air::{AirModule, AirFunction, BlockId};

pub struct UnreachableElim;

impl UnreachableElim {
    pub fn run(module: &mut AirModule) -> bool {
        let mut changed = false;
        for func in &mut module.functions {
            changed |= elim_function(func);
        }
        changed
    }
}

fn elim_function(func: &mut AirFunction) -> bool {
    if func.blocks.is_empty() { return false; }

    // Collect reachable block IDs via BFS from the entry block.
    let entry_id = func.blocks[0].id;
    let mut reachable: HashSet<BlockId> = HashSet::new();
    let mut worklist: Vec<BlockId> = vec![entry_id];

    while let Some(id) = worklist.pop() {
        if !reachable.insert(id) { continue; }
        if let Some(block) = func.block(id) {
            for &succ in &block.succs {
                worklist.push(succ);
            }
        }
    }

    // Remove blocks that are not reachable.
    let before_len = func.blocks.len();
    func.blocks.retain(|b| reachable.contains(&b.id));
    let removed = before_len - func.blocks.len();

    if removed > 0 {
        // Rebuild CFG edges after removal.
        func.rebuild_cfg();
        true
    } else {
        false
    }
}
