// compiler/instruction.rs — SBC Bytecode Instructions
// Ported from SylvelCompiler/Instruction.swift

pub const SBC_MAGIC: &[u8; 4] = b"SYL\0";
pub const SBC_VERSION_MAJOR: u8 = 1;
pub const SBC_VERSION_MINOR: u8 = 0;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    LoadConst    = 0x01,
    LoadNull     = 0x02,
    LoadTrue     = 0x03,
    LoadFalse    = 0x04,
    LoadVar      = 0x05,
    StoreVar     = 0x06,
    LoadGlobal   = 0x07,
    StoreGlobal  = 0x08,
    DeleteVar    = 0x09,

    Pop          = 0x0A,
    Dup          = 0x0B,
    Swap         = 0x0C,

    Add          = 0x10,
    Sub          = 0x11,
    Mul          = 0x12,
    Div          = 0x13,
    Mod          = 0x14,
    Pow          = 0x15,
    FloorDiv     = 0x16,
    Neg          = 0x17,

    Eq           = 0x20,
    Neq          = 0x21,
    Lt           = 0x22,
    Gt           = 0x23,
    Lte          = 0x24,
    Gte          = 0x25,

    Not          = 0x30,
    And          = 0x31,
    Or           = 0x32,

    Band         = 0x38,
    Bor          = 0x39,
    Bxor         = 0x3A,
    Bnot         = 0x3B,
    Shl          = 0x3C,
    Shr          = 0x3D,
    Ushr         = 0x3E,

    Jump         = 0x40,
    JumpIfF      = 0x41,
    JumpIfT      = 0x42,
    JumpIfFPop   = 0x43,
    JumpIfTPop   = 0x44,

    MakeFunc     = 0x50,
    Call         = 0x51,
    CallNative   = 0x52,
    Return       = 0x53,
    ReturnNull   = 0x54,

    MakeList     = 0x60,
    MakeMap      = 0x61,
    GetIndex     = 0x62,
    SetIndex     = 0x63,
    Spread       = 0x64,

    GetAttr      = 0x68,
    SetAttr      = 0x69,

    Throw        = 0x70,
    TryBegin     = 0x71,
    TryEnd       = 0x72,
    CatchStore   = 0x73,

    StrConcat    = 0x78,
    FormatVal    = 0x79,

    Import       = 0x80,

    Line         = 0xFE,
    Nop          = 0xFF,
}

impl Opcode {
    pub fn has_operand(&self) -> bool {
        matches!(
            self,
            Opcode::LoadConst | Opcode::LoadVar | Opcode::StoreVar | Opcode::LoadGlobal | Opcode::StoreGlobal
            | Opcode::DeleteVar | Opcode::MakeFunc | Opcode::Call | Opcode::CallNative
            | Opcode::MakeList | Opcode::MakeMap | Opcode::GetAttr | Opcode::SetAttr
            | Opcode::Jump | Opcode::JumpIfF | Opcode::JumpIfT | Opcode::JumpIfFPop | Opcode::JumpIfTPop
            | Opcode::TryBegin | Opcode::CatchStore | Opcode::Import | Opcode::Line
        )
    }
}
