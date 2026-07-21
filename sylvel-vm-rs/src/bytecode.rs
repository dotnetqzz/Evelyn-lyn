// bytecode.rs — .lync file loader
// Binary format produced by Sylvel Swift compiler (BytecodeWriter.swift)

use std::fs;
use std::io;
use std::rc::Rc;

use crate::value::SylVal;

// ---------------------------------------------------------------------------
// Magic bytes: SYL\0
// ---------------------------------------------------------------------------
const MAGIC: [u8; 4] = [0x53, 0x59, 0x4C, 0x00];

// ---------------------------------------------------------------------------
// Structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Proto {
    pub name: String,
    pub arity: u8,
    pub is_variadic: bool,
    pub local_count: u16,
    pub code: Vec<u8>,
    pub debug_pcs: Vec<u32>,
    pub debug_lines: Vec<u32>,
}

#[derive(Debug)]
pub struct Module {
    pub pool: Vec<SylVal>,
    pub native_names: Vec<String>,
    pub protos: Vec<Rc<Proto>>,
}

// ---------------------------------------------------------------------------
// Reader (cursor over a byte slice)
// ---------------------------------------------------------------------------

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self { Reader { data, pos: 0 } }

    fn u8(&mut self) -> Result<u8, String> {
        if self.pos >= self.data.len() {
            return Err("unexpected EOF".to_string());
        }
        let b = self.data[self.pos];
        self.pos += 1;
        Ok(b)
    }

    fn u16(&mut self) -> Result<u16, String> {
        let hi = self.u8()? as u16;
        let lo = self.u8()? as u16;
        Ok((hi << 8) | lo)
    }

    fn u32(&mut self) -> Result<u32, String> {
        let a = self.u8()? as u32;
        let b = self.u8()? as u32;
        let c = self.u8()? as u32;
        let d = self.u8()? as u32;
        Ok((a << 24) | (b << 16) | (c << 8) | d)
    }

    fn u64(&mut self) -> Result<u64, String> {
        let hi = self.u32()? as u64;
        let lo = self.u32()? as u64;
        Ok((hi << 32) | lo)
    }

    fn i64(&mut self) -> Result<i64, String> {
        Ok(self.u64()? as i64)
    }

    fn f64(&mut self) -> Result<f64, String> {
        let bits = self.u64()?;
        Ok(f64::from_bits(bits))
    }

    fn string(&mut self) -> Result<String, String> {
        let len = self.u32()? as usize;
        if self.pos + len > self.data.len() {
            return Err("string overflows buffer".to_string());
        }
        let s = std::str::from_utf8(&self.data[self.pos..self.pos + len])
            .map_err(|e| e.to_string())?
            .to_string();
        self.pos += len;
        Ok(s)
    }

    fn bytes(&mut self, n: usize) -> Result<Vec<u8>, String> {
        if self.pos + n > self.data.len() {
            return Err("not enough bytes".to_string());
        }
        let v = self.data[self.pos..self.pos + n].to_vec();
        self.pos += n;
        Ok(v)
    }
}

// ---------------------------------------------------------------------------
// Loader
// ---------------------------------------------------------------------------

fn load(data: &[u8]) -> Result<Module, String> {
    let mut r = Reader::new(data);

    // Header
    if data.len() < 8 {
        return Err("file too small".to_string());
    }
    for expected in MAGIC.iter() {
        let b = r.u8()?;
        if b != *expected {
            return Err("invalid magic bytes (not a .lync file)".to_string());
        }
    }
    let ver_major = r.u8()?;
    let _ver_minor = r.u8()?;
    r.u8()?; r.u8()?; // reserved

    if ver_major > 1 {
        return Err(format!("unsupported .lync version {}", ver_major));
    }

    // Constant Pool
    let pool_count = r.u32()? as usize;
    let mut pool = Vec::with_capacity(pool_count);
    for _ in 0..pool_count {
        let tag = r.u8()?;
        let val = match tag {
            0x00 => SylVal::Null,
            0x01 => SylVal::Bool(r.u8()? != 0),
            0x02 => SylVal::Int(r.i64()?),
            0x03 => SylVal::Float(r.f64()?),
            0x04 => {
                let slen = r.u32()? as usize;
                let bytes = r.bytes(slen)?;
                let s = String::from_utf8(bytes).map_err(|e| e.to_string())?;
                SylVal::Str(Rc::new(s))
            }
            _ => return Err(format!("unknown constant tag 0x{:02X}", tag)),
        };
        pool.push(val);
    }

    // Native Table
    let native_count = r.u32()? as usize;
    let mut native_names = Vec::with_capacity(native_count);
    for _ in 0..native_count {
        native_names.push(r.string()?);
    }

    // Function Prototypes
    let proto_count = r.u32()? as usize;
    let mut protos = Vec::with_capacity(proto_count);
    for _ in 0..proto_count {
        let name = r.string()?;
        let arity = r.u8()?;
        let is_variadic = r.u8()? != 0;
        let local_count = r.u16()?;

        let code_len = r.u32()? as usize;
        let code = r.bytes(code_len)?;

        let debug_len = r.u32()? as usize;
        let mut debug_pcs = Vec::with_capacity(debug_len);
        let mut debug_lines = Vec::with_capacity(debug_len);
        for _ in 0..debug_len {
            debug_pcs.push(r.u32()?);
            debug_lines.push(r.u32()?);
        }

        protos.push(Rc::new(Proto { name, arity, is_variadic, local_count, code, debug_pcs, debug_lines }));
    }

    Ok(Module { pool, native_names, protos })
}

pub fn load_bytes(data: &[u8]) -> Result<Module, String> {
    load(data)
}

pub fn load_file(path: &str) -> Result<Module, io::Error> {
    let data = fs::read(path)?;
    load(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
