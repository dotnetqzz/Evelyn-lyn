// compiler/writer.rs — Serializes ModuleState into .sbc binary file
// Ported from SylvelCompiler/BytecodeWriter.swift

use std::fs::File;
use std::io::Write;
use crate::compiler::{FunctionProto, ModuleState};
use crate::compiler::instruction::{SBC_MAGIC, SBC_VERSION_MAJOR, SBC_VERSION_MINOR};

pub struct BytecodeWriter;

impl BytecodeWriter {
    pub fn write(module: &ModuleState, path: &str) -> Result<(), String> {
        let bytes = Self::serialize(module);
        let mut file = File::create(path).map_err(|e| format!("Failed to create file '{}': {}", path, e))?;
        file.write_all(&bytes).map_err(|e| format!("Failed to write bytecode to '{}': {}", path, e))?;
        Ok(())
    }

    pub fn serialize(module: &ModuleState) -> Vec<u8> {
        let mut out = Vec::new();

        // Header
        out.extend_from_slice(SBC_MAGIC);
        out.push(SBC_VERSION_MAJOR);
        out.push(SBC_VERSION_MINOR);
        out.push(0x00); // reserved
        out.push(0x00);

        // Constant Pool
        out.extend(module.pool.serialize());

        // Native Table
        let native_count = module.native_table.len() as u32;
        out.extend_from_slice(&native_count.to_be_bytes());
        for name in &module.native_table {
            let bytes = name.as_bytes();
            out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
            out.extend_from_slice(bytes);
        }

        // Function Prototypes
        let proto_count = module.protos.len() as u32;
        out.extend_from_slice(&proto_count.to_be_bytes());
        for proto in &module.protos {
            out.extend(Self::serialize_proto(proto));
        }

        out
    }

    fn serialize_proto(proto: &FunctionProto) -> Vec<u8> {
        let mut out = Vec::new();
        let name_bytes = proto.name.as_bytes();
        out.extend_from_slice(&(name_bytes.len() as u32).to_be_bytes());
        out.extend_from_slice(name_bytes);

        out.push(proto.arity);
        out.push(if proto.is_variadic { 1 } else { 0 });
        out.extend_from_slice(&proto.local_count.to_be_bytes());

        out.extend_from_slice(&(proto.code.len() as u32).to_be_bytes());
        out.extend_from_slice(&proto.code);

        out.extend_from_slice(&(proto.line_map.len() as u32).to_be_bytes());
        for (pc, line) in &proto.line_map {
            out.extend_from_slice(&pc.to_be_bytes());
            out.extend_from_slice(&line.to_be_bytes());
        }

        out
    }
}
