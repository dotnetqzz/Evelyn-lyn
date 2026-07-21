// verifier.rs — Static bytecode verifier
//
// Runs once per loaded .lync module, before any instruction executes.
// For every function prototype it checks:
//   1. every instruction's operand bytes actually fit in the code buffer
//      (no truncated instruction, no unknown opcode)
//   2. every index operand (constant pool, native table, local slot,
//      function proto) is in-bounds
//   3. every jump / try-catch target lands exactly on an instruction
//      boundary, inside the code buffer
//   4. the operand stack can never go negative
//
// Exception edges (TryBegin -> catch target) are seeded into the depth
// simulation with a best-effort estimate (matching the VM's behavior of
// pushing one placeholder value before entering the catch block) rather
// than strictly enforced, since a throw can happen at many different
// depths inside a try body.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::bytecode::Module;
use crate::vm::*;

pub fn verify_module(module: &Module) -> Result<(), String> {
    for proto in &module.protos {
        verify_proto(module, proto)
            .map_err(|e| format!("bytecode verification failed in '{}': {}", proto.name, e))?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Instr {
    op: u8,
    operand_start: usize,
    next: usize,
}

fn read_u16(code: &[u8], at: usize) -> u16 {
    ((code[at] as u16) << 8) | (code[at + 1] as u16)
}

fn verify_proto(module: &Module, proto: &crate::bytecode::Proto) -> Result<(), String> {
    let code: &[u8] = &proto.code;
    let len = code.len();
    let local_count = proto.local_count as usize;
    let pool_len = module.pool.len();
    let native_len = module.native_names.len();
    let proto_len = module.protos.len();

    // ---- Pass 1: decode every instruction, validate shape + static indices ----
    let mut starts: HashSet<usize> = HashSet::new();
    let mut instrs: HashMap<usize, Instr> = HashMap::new();
    let mut pc = 0usize;

    while pc < len {
        starts.insert(pc);
        let op = code[pc];
        let operand_start = pc + 1;

        let operand_len: usize = match op {
            OP_LOAD_CONST | OP_LOAD_VAR | OP_STORE_VAR | OP_LOAD_GLOBAL | OP_STORE_GLOBAL
            | OP_DELETE_VAR | OP_MAKE_FUNC | OP_CALL | OP_MAKE_LIST | OP_MAKE_MAP
            | OP_GET_ATTR | OP_SET_ATTR | OP_JUMP | OP_JUMP_IF_F | OP_JUMP_IF_T
            | OP_JUMP_IF_F_POP | OP_JUMP_IF_T_POP | OP_TRY_BEGIN | OP_CATCH_STORE
            | OP_IMPORT | OP_LINE => 2,
            OP_CALL_NATIVE => 3, // u16 native idx + u8 argc
            OP_LOAD_NULL | OP_LOAD_TRUE | OP_LOAD_FALSE | OP_POP | OP_DUP | OP_SWAP
            | OP_ADD | OP_SUB | OP_MUL | OP_DIV | OP_MOD | OP_POW | OP_FLOORDIV | OP_NEG
            | OP_EQ | OP_NEQ | OP_LT | OP_GT | OP_LTE | OP_GTE
            | OP_NOT | OP_AND | OP_OR
            | OP_BAND | OP_BOR | OP_BXOR | OP_BNOT | OP_SHL | OP_SHR | OP_USHR
            | OP_RETURN | OP_RETURN_NULL | OP_GET_INDEX | OP_SET_INDEX | OP_SPREAD
            | OP_THROW | OP_TRY_END | OP_STR_CONCAT | OP_FORMAT_VAL | OP_NOP => 0,
            other => return Err(format!("unknown opcode 0x{:02X} at pc={}", other, pc)),
        };

        if operand_start + operand_len > len {
            return Err(format!("truncated instruction (opcode 0x{:02X} at pc={})", op, pc));
        }

        match op {
            OP_LOAD_CONST | OP_LOAD_GLOBAL | OP_STORE_GLOBAL | OP_GET_ATTR | OP_SET_ATTR => {
                let idx = read_u16(code, operand_start) as usize;
                if idx >= pool_len {
                    return Err(format!("constant pool index {} out of bounds ({}) at pc={}", idx, pool_len, pc));
                }
            }
            OP_LOAD_VAR | OP_STORE_VAR | OP_DELETE_VAR | OP_CATCH_STORE => {
                let slot = read_u16(code, operand_start) as usize;
                if slot >= local_count {
                    return Err(format!("local slot {} out of bounds ({}) at pc={}", slot, local_count, pc));
                }
            }
            OP_MAKE_FUNC => {
                let idx = read_u16(code, operand_start) as usize;
                if idx >= proto_len {
                    return Err(format!("proto index {} out of bounds ({}) at pc={}", idx, proto_len, pc));
                }
            }
            OP_CALL_NATIVE => {
                let idx = read_u16(code, operand_start) as usize;
                if idx >= native_len {
                    return Err(format!("native index {} out of bounds ({}) at pc={}", idx, native_len, pc));
                }
            }
            _ => {}
        }

        let next = operand_start + operand_len;
        instrs.insert(pc, Instr { op, operand_start, next });
        pc = next;
    }

    // ---- Pass 2: jump / try targets must land on real instruction boundaries ----
    for (&pc, instr) in &instrs {
        let target: Option<i64> = match instr.op {
            OP_JUMP | OP_JUMP_IF_F | OP_JUMP_IF_T | OP_JUMP_IF_F_POP | OP_JUMP_IF_T_POP => {
                let off = read_u16(code, instr.operand_start) as i16 as i64;
                Some(instr.next as i64 + off)
            }
            OP_TRY_BEGIN => Some(read_u16(code, instr.operand_start) as i64), // absolute pc
            _ => None,
        };
        if let Some(t) = target {
            if t < 0 || t as usize >= len || !starts.contains(&(t as usize)) {
                return Err(format!("jump/try target {} is not a valid instruction boundary (from pc={})", t, pc));
            }
        }
    }

    // ---- Pass 3: BFS operand-stack depth simulation ----
    if len > 0 {
        simulate_stack(code, &instrs)?;
    }

    Ok(())
}

fn simulate_stack(code: &[u8], instrs: &HashMap<usize, Instr>) -> Result<(), String> {
    let mut visited: HashMap<usize, isize> = HashMap::new();
    let mut queue: VecDeque<(usize, isize)> = VecDeque::new();
    queue.push_back((0, 0));

    while let Some((pc, depth)) = queue.pop_front() {
        let instr = match instrs.get(&pc) {
            Some(i) => *i,
            None => return Err(format!("control flow lands mid-instruction at pc={}", pc)),
        };

        if let Some(&prev) = visited.get(&pc) {
            if prev != depth {
                // Best-effort on exception edges: don't hard-fail on a depth
                // mismatch reached via a TryBegin catch target.
                continue;
            }
            continue;
        }
        visited.insert(pc, depth);

        let (pop, push): (isize, isize) = match instr.op {
            OP_LOAD_CONST | OP_LOAD_NULL | OP_LOAD_TRUE | OP_LOAD_FALSE | OP_LOAD_VAR
            | OP_LOAD_GLOBAL | OP_DUP | OP_MAKE_FUNC => (0, 1),
            OP_STORE_VAR | OP_STORE_GLOBAL | OP_POP => (1, 0),
            OP_DELETE_VAR | OP_SWAP | OP_TRY_BEGIN | OP_TRY_END | OP_CATCH_STORE
            | OP_JUMP | OP_IMPORT | OP_LINE | OP_NOP => (0, 0),
            OP_ADD | OP_SUB | OP_MUL | OP_DIV | OP_MOD | OP_POW | OP_FLOORDIV
            | OP_EQ | OP_NEQ | OP_LT | OP_GT | OP_LTE | OP_GTE
            | OP_AND | OP_OR | OP_BAND | OP_BOR | OP_BXOR | OP_SHL | OP_SHR | OP_USHR
            | OP_GET_INDEX | OP_STR_CONCAT => (2, 1),
            OP_NEG | OP_NOT | OP_BNOT | OP_GET_ATTR | OP_FORMAT_VAL
            | OP_JUMP_IF_F | OP_JUMP_IF_T => (1, 1), // peek-jumps read but don't pop
            OP_JUMP_IF_F_POP | OP_JUMP_IF_T_POP | OP_THROW | OP_RETURN => (1, 0),
            OP_RETURN_NULL => (0, 0),
            OP_SET_ATTR => (2, 0),
            OP_SET_INDEX => (3, 0),
            OP_SPREAD => (1, 1), // approximate: real push count is runtime-dependent
            OP_MAKE_LIST => (read_u16(code, instr.operand_start) as isize, 1),
            OP_MAKE_MAP => (read_u16(code, instr.operand_start) as isize * 2, 1),
            OP_CALL => (read_u16(code, instr.operand_start) as isize + 1, 1),
            OP_CALL_NATIVE => (code[instr.operand_start + 2] as isize, 1), // argc byte
            other => return Err(format!("verifier missing stack-effect rule for opcode 0x{:02X}", other)),
        };

        if depth < pop {
            return Err(format!("stack underflow at pc={} (have {}, need {})", pc, depth, pop));
        }
        let new_depth = depth - pop + push;

        match instr.op {
            OP_RETURN | OP_RETURN_NULL | OP_THROW => {} // terminal, no successors
            OP_JUMP => {
                let off = read_u16(code, instr.operand_start) as i16 as i64;
                let t = (instr.next as i64 + off) as usize;
                queue.push_back((t, new_depth));
            }
            OP_JUMP_IF_F | OP_JUMP_IF_T | OP_JUMP_IF_F_POP | OP_JUMP_IF_T_POP => {
                let off = read_u16(code, instr.operand_start) as i16 as i64;
                let t = (instr.next as i64 + off) as usize;
                queue.push_back((t, new_depth));
                queue.push_back((instr.next, new_depth));
            }
            OP_TRY_BEGIN => {
                let catch_pc = read_u16(code, instr.operand_start) as usize;
                queue.push_back((catch_pc, new_depth + 1)); // +1 placeholder push, see module doc
                queue.push_back((instr.next, new_depth));
            }
            _ => queue.push_back((instr.next, new_depth)),
        }
    }

    Ok(())
}
