// optimizer/passes/bb_simplify.rs — Basic block simplification pass
//
// Merges blocks where one has a single unconditional successor that has a
// single predecessor.  Also removes empty blocks that only contain a Jump.
// This reduces the number of basic blocks and improves code locality.

use crate::air::{AirModule, AirFunction, Inst, BlockId};

pub struct BbSimplify;

impl BbSimplify {
    pub fn run(module: &mut AirModule) -> bool {
        let mut changed = false;
        for func in &mut module.functions {
            changed |= simplify_function(func);
        }
        changed
    }
}

fn simplify_function(func: &mut AirFunction) -> bool {
    let mut changed = false;

    // Pass 1: Merge blocks where A ends with Jump(B) and B has exactly one pred.
    loop {
        let merge = find_merge_candidate(func);
        if let Some((src_id, dst_id)) = merge {
            merge_blocks(func, src_id, dst_id);
            func.rebuild_cfg();
            changed = true;
        } else {
            break;
        }
    }

    // Pass 2: Remove blocks that consist solely of Jump(target) and redirect
    // all predecessors to jump directly to target.
    loop {
        let passthrough = find_passthrough(func);
        if let Some((empty_id, target_id)) = passthrough {
            redirect_predecessors(func, empty_id, target_id);
            func.blocks.retain(|b| b.id != empty_id);
            func.rebuild_cfg();
            changed = true;
        } else {
            break;
        }
    }

    changed
}

/// Find a (src, dst) pair where src ends with Jump(dst) and dst has
/// exactly one predecessor.
fn find_merge_candidate(func: &AirFunction) -> Option<(BlockId, BlockId)> {
    let entry_id = func.blocks.first().map(|b| b.id)?;
    for block in &func.blocks {
        if let Some(Inst::Jump(dst)) = block.insts.last() {
            let dst_id = *dst;
            if dst_id == entry_id { continue; }  // don't merge entry
            if let Some(dst_block) = func.block(dst_id) {
                if dst_block.preds.len() == 1 {
                    return Some((block.id, dst_id));
                }
            }
        }
    }
    None
}

fn merge_blocks(func: &mut AirFunction, src_id: BlockId, dst_id: BlockId) {
    // Remove the trailing Jump from src.
    if let Some(src) = func.blocks.iter_mut().find(|b| b.id == src_id) {
        if let Some(Inst::Jump(_)) = src.insts.last() {
            src.insts.pop();
        }
    }

    // Drain dst's instructions into src.
    let dst_insts = func.blocks.iter_mut()
        .find(|b| b.id == dst_id)
        .map(|b| std::mem::take(&mut b.insts))
        .unwrap_or_default();

    if let Some(src) = func.blocks.iter_mut().find(|b| b.id == src_id) {
        src.insts.extend(dst_insts);
    }

    // Remove dst.
    func.blocks.retain(|b| b.id != dst_id);
}

/// Find a block that contains only a single Jump instruction.
fn find_passthrough(func: &AirFunction) -> Option<(BlockId, BlockId)> {
    let entry_id = func.blocks.first().map(|b| b.id)?;
    for block in &func.blocks {
        if block.id == entry_id { continue; }
        if block.insts.len() == 1 {
            if let Some(Inst::Jump(dst)) = block.insts.first() {
                if *dst != block.id {
                    return Some((block.id, *dst));
                }
            }
        }
    }
    None
}

fn redirect_predecessors(func: &mut AirFunction, empty_id: BlockId, target_id: BlockId) {
    for block in &mut func.blocks {
        for inst in &mut block.insts {
            match inst {
                Inst::Jump(dst) if *dst == empty_id => { *dst = target_id; }
                Inst::Branch(_, t, e) => {
                    if *t == empty_id { *t = target_id; }
                    if *e == empty_id { *e = target_id; }
                }
                _ => {}
            }
        }
    }
}
