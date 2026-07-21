// compiler/verifier.rs — Validates bytecode integrity

use crate::compiler::{ModuleState, FunctionProto};
use crate::compiler::instruction::Opcode;

pub struct BytecodeVerifier;

impl BytecodeVerifier {
    pub fn verify(module: &ModuleState) -> Result<(), String> {
        for proto in &module.protos {
            Self::verify_proto(proto, module)?;
        }
        Ok(())
    }

    fn verify_proto(proto: &FunctionProto, module: &ModuleState) -> Result<(), String> {
        let mut pc = 0;
        let code = &proto.code;

        while pc < code.len() {
            let byte = code[pc];
            let opcode = Self::decode_opcode(byte)?;

            pc += 1;

            if opcode.has_operand() {
                if pc + 1 >= code.len() && opcode != Opcode::CallNative && opcode != Opcode::Line {
                     // Most operands are u16 (2 bytes)
                     // CallNative has 3 bytes total (u16 idx + u8 argc)
                     // Line has u32 (4 bytes)
                }

                match opcode {
                    Opcode::LoadConst | Opcode::LoadGlobal | Opcode::StoreGlobal => {
                        let idx = Self::read_u16(code, pc)?;
                        if idx as usize >= module.pool.entries.len() {
                            return Err(format!("VerifyError: Constant pool index {} out of bounds in proto '{}'", idx, proto.name));
                        }
                        pc += 2;
                    }
                    Opcode::LoadVar | Opcode::StoreVar | Opcode::DeleteVar => {
                        let slot = Self::read_u16(code, pc)?;
                        if slot >= proto.local_count {
                            return Err(format!("VerifyError: Local slot {} out of bounds in proto '{}'", slot, proto.name));
                        }
                        pc += 2;
                    }
                    Opcode::CallNative => {
                        let idx = Self::read_u16(code, pc)?;
                        if idx as usize >= module.native_table.len() {
                            return Err(format!("VerifyError: Native table index {} out of bounds in proto '{}'", idx, proto.name));
                        }
                        pc += 3; // u16 idx + u8 argc
                    }
                    Opcode::Jump | Opcode::JumpIfF | Opcode::JumpIfT | Opcode::JumpIfFPop | Opcode::JumpIfTPop => {
                        let offset = Self::read_u16(code, pc)?;
                        let target = offset as usize;
                        if target >= code.len() {
                            return Err(format!("VerifyError: Jump target {} out of bounds in proto '{}'", target, proto.name));
                        }
                        pc += 2;
                    }
                    Opcode::TryBegin => {
                        let handler_pc = Self::read_u16(code, pc)?;
                        if handler_pc as usize >= code.len() {
                            return Err(format!("VerifyError: Catch handler {} out of bounds in proto '{}'", handler_pc, proto.name));
                        }
                        pc += 2;
                    }
                    Opcode::Import => {
                        let idx = Self::read_u16(code, pc)?;
                        if idx as usize >= module.pool.entries.len() {
                             return Err(format!("VerifyError: Import path constant index {} out of bounds", idx));
                        }
                        pc += 2;
                    }
                    Opcode::Line => {
                        pc += 4; // u32 line
                    }
                    _ => {
                        // Other u16 operand instructions (Call, MakeFunc, MakeList, MakeMap, etc.)
                        pc += 2;
                    }
                }
            }
        }

        Ok(())
    }

    fn decode_opcode(byte: u8) -> Result<Opcode, String> {
        // This is a bit tedious without a proc-macro or something,
        // but we'll do it manually to ensure safety.
        match byte {
            0x01 => Ok(Opcode::LoadConst),
            0x02 => Ok(Opcode::LoadNull),
            0x03 => Ok(Opcode::LoadTrue),
            0x04 => Ok(Opcode::LoadFalse),
            0x05 => Ok(Opcode::LoadVar),
            0x06 => Ok(Opcode::StoreVar),
            0x07 => Ok(Opcode::LoadGlobal),
            0x08 => Ok(Opcode::StoreGlobal),
            0x09 => Ok(Opcode::DeleteVar),
            0x0A => Ok(Opcode::Pop),
            0x0B => Ok(Opcode::Dup),
            0x0C => Ok(Opcode::Swap),
            0x10 => Ok(Opcode::Add),
            0x11 => Ok(Opcode::Sub),
            0x12 => Ok(Opcode::Mul),
            0x13 => Ok(Opcode::Div),
            0x14 => Ok(Opcode::Mod),
            0x15 => Ok(Opcode::Pow),
            0x16 => Ok(Opcode::FloorDiv),
            0x17 => Ok(Opcode::Neg),
            0x20 => Ok(Opcode::Eq),
            0x21 => Ok(Opcode::Neq),
            0x22 => Ok(Opcode::Lt),
            0x23 => Ok(Opcode::Gt),
            0x24 => Ok(Opcode::Lte),
            0x25 => Ok(Opcode::Gte),
            0x30 => Ok(Opcode::Not),
            0x31 => Ok(Opcode::And),
            0x32 => Ok(Opcode::Or),
            0x38 => Ok(Opcode::Band),
            0x39 => Ok(Opcode::Bor),
            0x3A => Ok(Opcode::Bxor),
            0x3B => Ok(Opcode::Bnot),
            0x3C => Ok(Opcode::Shl),
            0x3D => Ok(Opcode::Shr),
            0x3E => Ok(Opcode::Ushr),
            0x40 => Ok(Opcode::Jump),
            0x41 => Ok(Opcode::JumpIfF),
            0x42 => Ok(Opcode::JumpIfT),
            0x43 => Ok(Opcode::JumpIfFPop),
            0x44 => Ok(Opcode::JumpIfTPop),
            0x50 => Ok(Opcode::MakeFunc),
            0x51 => Ok(Opcode::Call),
            0x52 => Ok(Opcode::CallNative),
            0x53 => Ok(Opcode::Return),
            0x54 => Ok(Opcode::ReturnNull),
            0x60 => Ok(Opcode::MakeList),
            0x61 => Ok(Opcode::MakeMap),
            0x62 => Ok(Opcode::GetIndex),
            0x63 => Ok(Opcode::SetIndex),
            0x64 => Ok(Opcode::Spread),
            0x68 => Ok(Opcode::GetAttr),
            0x69 => Ok(Opcode::SetAttr),
            0x70 => Ok(Opcode::Throw),
            0x71 => Ok(Opcode::TryBegin),
            0x72 => Ok(Opcode::TryEnd),
            0x73 => Ok(Opcode::CatchStore),
            0x78 => Ok(Opcode::StrConcat),
            0x79 => Ok(Opcode::FormatVal),
            0x80 => Ok(Opcode::Import),
            0xFE => Ok(Opcode::Line),
            0xFF => Ok(Opcode::Nop),
            _ => Err(format!("VerifyError: Unknown opcode 0x{:02X}", byte)),
        }
    }

    fn read_u16(code: &[u8], pc: usize) -> Result<u16, String> {
        if pc + 1 >= code.len() { return Err("VerifyError: Truncated operand".to_string()); }
        Ok(u16::from_be_bytes([code[pc], code[pc+1]]))
    }
}
