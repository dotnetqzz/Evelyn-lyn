// compiler/loader.rs — Deserializes .lync binary files

use std::fs::File;
use std::io::{Read, Cursor};
use crate::compiler::{Constant, ConstantPool, FunctionProto, ModuleState};
use crate::compiler::instruction::{SBC_MAGIC, SBC_VERSION_MAJOR, SBC_VERSION_MINOR};

pub struct BytecodeLoader;

impl BytecodeLoader {
    pub fn load(path: &str) -> Result<ModuleState, String> {
        let mut file = File::open(path).map_err(|e| format!("Failed to open file '{}': {}", path, e))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).map_err(|e| format!("Failed to read file '{}': {}", path, e))?;

        Self::deserialize(&buffer)
    }

    pub fn deserialize(bytes: &[u8]) -> Result<ModuleState, String> {
        let mut cursor = Cursor::new(bytes);

        // Header
        let mut magic = [0u8; 4];
        cursor.read_exact(&mut magic).map_err(|_| "Invalid header")?;
        if &magic != SBC_MAGIC {
            return Err("Not a valid Sylvel bytecode file (magic mismatch)".to_string());
        }

        let mut version = [0u8; 4];
        cursor.read_exact(&mut version).map_err(|_| "Invalid version header")?;
        if version[0] != SBC_VERSION_MAJOR || version[1] != SBC_VERSION_MINOR {
            return Err(format!("Version mismatch: expected {}.{}, found {}.{}",
                SBC_VERSION_MAJOR, SBC_VERSION_MINOR, version[0], version[1]));
        }

        let mut module = ModuleState::new();

        // Constant Pool
        let pool_count = Self::read_u32(&mut cursor)?;
        for _ in 0..pool_count {
            let tag = Self::read_u8(&mut cursor)?;
            match tag {
                0x00 => module.pool.entries.push(Constant::Null),
                0x01 => {
                    let b = Self::read_u8(&mut cursor)? != 0;
                    module.pool.entries.push(Constant::Bool(b));
                }
                0x02 => {
                    let i = Self::read_i64(&mut cursor)?;
                    module.pool.entries.push(Constant::Int(i));
                }
                0x03 => {
                    let f = Self::read_f64(&mut cursor)?;
                    module.pool.entries.push(Constant::Float(f));
                }
                0x04 => {
                    let s = Self::read_string(&mut cursor)?;
                    module.pool.entries.push(Constant::Str(s));
                }
                _ => return Err(format!("Unknown constant tag: 0x{:02X}", tag)),
            }
        }

        // Native Table
        let native_count = Self::read_u32(&mut cursor)?;
        for _ in 0..native_count {
            module.native_table.push(Self::read_string(&mut cursor)?);
        }

        // Function Prototypes
        let proto_count = Self::read_u32(&mut cursor)?;
        for _ in 0..proto_count {
            module.protos.push(Self::deserialize_proto(&mut cursor)?);
        }

        Ok(module)
    }

    fn deserialize_proto(cursor: &mut Cursor<&[u8]>) -> Result<FunctionProto, String> {
        let name = Self::read_string(cursor)?;
        let mut proto = FunctionProto::new(name);

        proto.arity = Self::read_u8(cursor)?;
        proto.is_variadic = Self::read_u8(cursor)? != 0;
        proto.local_count = Self::read_u16(cursor)?;

        let code_len = Self::read_u32(cursor)?;
        let mut code = vec![0u8; code_len as usize];
        cursor.read_exact(&mut code).map_err(|_| "Failed to read bytecode")?;
        proto.code = code;

        let map_len = Self::read_u32(cursor)?;
        for _ in 0..map_len {
            let pc = Self::read_u32(cursor)?;
            let line = Self::read_u32(cursor)?;
            proto.line_map.push((pc, line));
        }

        Ok(proto)
    }

    fn read_u8(cursor: &mut Cursor<&[u8]>) -> Result<u8, String> {
        let mut b = [0u8; 1];
        cursor.read_exact(&mut b).map_err(|_| "Unexpected EOF")?;
        Ok(b[0])
    }

    fn read_u16(cursor: &mut Cursor<&[u8]>) -> Result<u16, String> {
        let mut b = [0u8; 2];
        cursor.read_exact(&mut b).map_err(|_| "Unexpected EOF")?;
        Ok(u16::from_be_bytes(b))
    }

    fn read_u32(cursor: &mut Cursor<&[u8]>) -> Result<u32, String> {
        let mut b = [0u8; 4];
        cursor.read_exact(&mut b).map_err(|_| "Unexpected EOF")?;
        Ok(u32::from_be_bytes(b))
    }

    fn read_i64(cursor: &mut Cursor<&[u8]>) -> Result<i64, String> {
        let mut b = [0u8; 8];
        cursor.read_exact(&mut b).map_err(|_| "Unexpected EOF")?;
        Ok(i64::from_be_bytes(b))
    }

    fn read_f64(cursor: &mut Cursor<&[u8]>) -> Result<f64, String> {
        let mut b = [0u8; 8];
        cursor.read_exact(&mut b).map_err(|_| "Unexpected EOF")?;
        Ok(f64::from_be_bytes(b))
    }

    fn read_string(cursor: &mut Cursor<&[u8]>) -> Result<String, String> {
        let len = Self::read_u32(cursor)?;
        let mut b = vec![0u8; len as usize];
        cursor.read_exact(&mut b).map_err(|_| "Unexpected EOF reading string")?;
        String::from_utf8(b).map_err(|_| "Invalid UTF-8 string".to_string())
    }
}
