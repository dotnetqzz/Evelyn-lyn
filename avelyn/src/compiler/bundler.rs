// compiler/bundler.rs — Packages multiple .lync files into a .lynb bundle

use std::collections::HashMap;
use std::fs::File;
use std::io::{Write, Seek, SeekFrom};

pub const SYLB_MAGIC: &[u8; 4] = b"SYLB";
pub const SYLB_VERSION: u16 = 1;

pub struct Bundler;

impl Bundler {
    pub fn bundle(entry_module: &str, files: HashMap<String, Vec<u8>>, out_path: &str) -> Result<(), String> {
        let mut out = File::create(out_path).map_err(|e| format!("Failed to create bundle: {}", e))?;

        // 1. Header
        out.write_all(SYLB_MAGIC).map_err(|e| e.to_string())?;
        out.write_all(&SYLB_VERSION.to_be_bytes()).map_err(|e| e.to_string())?;

        // 2. Entry Point
        let entry_bytes = entry_module.as_bytes();
        out.write_all(&(entry_bytes.len() as u32).to_be_bytes()).map_err(|e| e.to_string())?;
        out.write_all(entry_bytes).map_err(|e| e.to_string())?;

        // 3. File Index
        let file_count = files.len() as u32;
        out.write_all(&file_count.to_be_bytes()).map_err(|e| e.to_string())?;

        // We'll write placeholders for offsets and fill them later
        let mut index_positions = Vec::new();
        for (name, _) in &files {
            let name_bytes = name.as_bytes();
            out.write_all(&(name_bytes.len() as u32).to_be_bytes()).map_err(|e| e.to_string())?;
            out.write_all(name_bytes).map_err(|e| e.to_string())?;

            let offset_pos = out.stream_position().map_err(|e| e.to_string())?;
            out.write_all(&0u64.to_be_bytes()).map_err(|e| e.to_string())?; // Offset placeholder
            out.write_all(&0u64.to_be_bytes()).map_err(|e| e.to_string())?; // Size placeholder
            index_positions.push((name.clone(), offset_pos));
        }

        // 4. Raw Data
        let mut final_index = Vec::new();
        for (name, pos) in index_positions {
            let data = files.get(&name).unwrap();
            let current_offset = out.stream_position().map_err(|e| e.to_string())?;
            out.write_all(data).map_err(|e| e.to_string())?;
            final_index.push((pos, current_offset, data.len() as u64));
        }

        // 5. Backfill Index
        for (pos, offset, size) in final_index {
            out.seek(SeekFrom::Start(pos)).map_err(|e| e.to_string())?;
            out.write_all(&offset.to_be_bytes()).map_err(|e| e.to_string())?;
            out.write_all(&size.to_be_bytes()).map_err(|e| e.to_string())?;
        }

        Ok(())
    }
}
