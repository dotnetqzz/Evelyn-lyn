#![allow(dead_code, unused_imports)]
// optimizer/mod.rs — Optimization pipeline
//
// The OptimizationPipeline runs a sequence of passes over an AirModule.
// Each pass implements the `OptPass` trait.  The pipeline iterates until
// a fixed point is reached or the maximum iteration count is hit.
//
// Optimization levels:
//   O0 — no passes
//   O1 — redundant_elim, dce, unreachable_elim
//   O2 — O1 + const_fold, const_prop, bb_simplify
//   O3 — O2 (future: inlining, specialization, escape analysis)

pub mod passes;

use crate::air::AirModule;
use passes::{
    const_fold::ConstFold,
    const_prop::ConstProp,
    dce::DeadCodeElim,
    unreachable_elim::UnreachableElim,
    bb_simplify::BbSimplify,
    redundant_elim::RedundantElim,
};

/// Optimization level — mirrors the `-O` flag in the driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptLevel {
    O0 = 0,
    O1 = 1,
    O2 = 2,
    O3 = 3,
}

impl OptLevel {
    pub fn from_u8(n: u8) -> Self {
        match n {
            0 => OptLevel::O0,
            1 => OptLevel::O1,
            2 => OptLevel::O2,
            _ => OptLevel::O3,
        }
    }
}

impl std::fmt::Display for OptLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "O{}", *self as u8)
    }
}

/// Maximum number of optimizer iterations before giving up.
const MAX_ITERATIONS: usize = 20;

/// Run the full optimization pipeline at the given level.
/// Returns the number of passes that made changes.
pub fn optimize(module: &mut AirModule, level: OptLevel) -> usize {
    if level == OptLevel::O0 {
        return 0;
    }

    let mut total_changed = 0;

    for _ in 0..MAX_ITERATIONS {
        let mut round_changed = false;

        // Always run these at O1+
        round_changed |= RedundantElim::run(module);
        round_changed |= DeadCodeElim::run(module);
        round_changed |= UnreachableElim::run(module);

        // O2+ passes
        if level >= OptLevel::O2 {
            round_changed |= ConstFold::run(module);
            round_changed |= ConstProp::run(module);
            round_changed |= BbSimplify::run(module);
        }

        if round_changed {
            total_changed += 1;
        } else {
            break; // Fixed point reached.
        }
    }

    total_changed
}

/// Run just the cleanup passes (redundant + DCE) — used after verification
/// even at O0 to ensure the IR is minimal.
pub fn cleanup(module: &mut AirModule) {
    RedundantElim::run(module);
    DeadCodeElim::run(module);
}
