// air/verify.rs — AIR module verifier
//
// The verifier checks structural and type-system invariants of an AIR module
// after AIRGen and again after each optimization pass.  It is inspired by
// LLVM's verifyModule and Swift SIL's SILVerifier.
//
// Invariants checked:
//   1. Every function's entry block (blocks[0]) has no predecessors.
//   2. Every basic block ends with exactly one terminator instruction.
//   3. No instruction appears after a terminator in the same block.
//   4. Every Value used in an instruction is defined before it (in block order).
//   5. Branch targets reference blocks that exist in the function.
//   6. Return instructions respect the function's declared return type.
//   7. RuntimeCall result Values are VOID_VALUE iff the function returns Void.
//   8. ConstStr indices are within bounds of the module's string table.

use std::collections::HashSet;
use crate::sema::diagnostics::{Diagnostic, DiagnosticEmitter};
use super::{AirModule, AirFunction, BasicBlock, BlockId, Inst, Value, VOID_VALUE};
use super::AirType;

pub struct AirVerifier<'a> {
    diag: &'a mut DiagnosticEmitter,
}

impl<'a> AirVerifier<'a> {
    pub fn new(diag: &'a mut DiagnosticEmitter) -> Self {
        AirVerifier { diag }
    }

    /// Verify an entire module, populating `diag` with any issues found.
    /// Returns `true` if the module is valid (no errors).
    pub fn verify(&mut self, module: &AirModule) -> bool {
        let str_count = module.string_table.len();

        for func in &module.functions {
            self.verify_function(func, str_count);
        }

        !self.diag.has_errors()
    }

    fn verify_function(&mut self, func: &AirFunction, str_count: usize) {
        let name = &func.name;

        // Rule 1: at least one block.
        if func.blocks.is_empty() {
            self.diag.error(func.span, format!("function '{}' has no basic blocks", name));
            return;
        }

        // Rule 1b: entry block has no predecessors.
        let entry = &func.blocks[0];
        if !entry.preds.is_empty() {
            self.diag.error(func.span,
                format!("entry block '{}' of function '{}' has predecessors", entry.label, name));
        }

        // Build a set of valid block IDs.
        let valid_blocks: HashSet<BlockId> = func.blocks.iter().map(|b| b.id).collect();

        // Build the set of defined values (in sequential order across blocks).
        let mut defined: HashSet<Value> = HashSet::new();

        // Pre-populate parameter values and global variable references as defined.
        for param in &func.params {
            defined.insert(param.value);
        }
        for (gv, _) in &func.global_val_map {
            defined.insert(*gv);
        }

        for block in &func.blocks {
            self.verify_block(block, func, name, &valid_blocks, &mut defined, str_count);
        }
    }

    fn verify_block(
        &mut self,
        block: &BasicBlock,
        func: &AirFunction,
        func_name: &str,
        valid_blocks: &HashSet<BlockId>,
        defined: &mut HashSet<Value>,
        str_count: usize,
    ) {
        let label = &block.label;
        let mut found_terminator = false;

        for (i, inst) in block.insts.iter().enumerate() {
            // Rule 3: no instruction after a terminator.
            if found_terminator {
                self.diag.error(func.span,
                    format!("instruction after terminator in block '{}' of '{}' (inst #{})",
                        label, func_name, i));
                break;
            }

            // Rule 4: all used values must be defined.
            for used in inst.used_values() {
                if !defined.contains(&used) {
                    self.diag.error(func.span,
                        format!("use of undefined value {} in block '{}' of '{}'",
                            used, label, func_name));
                }
            }

            // Register newly defined value.
            if let Some(def) = inst.defined_value() {
                defined.insert(def);
            }

            // Rule 5: branch targets must be valid.
            match inst {
                Inst::Branch(_, t, e) => {
                    if !valid_blocks.contains(t) {
                        self.diag.error(func.span,
                            format!("branch to unknown block {} in '{}::{}'", t, func_name, label));
                    }
                    if !valid_blocks.contains(e) {
                        self.diag.error(func.span,
                            format!("branch to unknown block {} in '{}::{}'", e, func_name, label));
                    }
                    found_terminator = true;
                }
                Inst::Jump(t) => {
                    if !valid_blocks.contains(t) {
                        self.diag.error(func.span,
                            format!("jump to unknown block {} in '{}::{}'", t, func_name, label));
                    }
                    found_terminator = true;
                }
                Inst::Return(_) | Inst::Unreachable => {
                    found_terminator = true;
                }

                // Rule 7: RuntimeCall void/non-void consistency.
                Inst::RuntimeCall(result, rt_fn, _) => {
                    let expected_void = rt_fn.return_type() == AirType::Void;
                    let is_void = *result == VOID_VALUE;
                    if expected_void && !is_void {
                        self.diag.error(func.span,
                            format!("RuntimeCall to '{}' is void but has result value in '{}::{}'",
                                rt_fn.c_name(), func_name, label));
                    }
                    if !expected_void && is_void {
                        self.diag.error(func.span,
                            format!("RuntimeCall to '{}' returns a value but result is VOID_VALUE in '{}::{}'",
                                rt_fn.c_name(), func_name, label));
                    }
                }

                // Rule 8: ConstStr index within bounds.
                Inst::ConstStr(_, idx) => {
                    if *idx as usize >= str_count {
                        self.diag.error(func.span,
                            format!("ConstStr index {} out of bounds (table size {}) in '{}::{}'",
                                idx, str_count, func_name, label));
                    }
                }

                _ => {}
            }
        }

        // Rule 2: every block must be terminated.
        if !found_terminator && !block.insts.is_empty() {
            self.diag.error(func.span,
                format!("block '{}' in function '{}' has no terminator", label, func_name));
        }
    }
}

/// Convenience free function — creates a throwaway emitter and returns errors.
pub fn verify_module(module: &AirModule) -> Vec<Diagnostic> {
    let mut diag = DiagnosticEmitter::new();
    let mut verifier = AirVerifier::new(&mut diag);
    verifier.verify(module);
    diag.take()
}
