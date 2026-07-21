// interpreter/module_manager.rs — Manages module resolution and caching

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use crate::stdlib_bundle;

pub struct ModuleManager {
    loaded_paths: HashSet<PathBuf>,
    // Stack of files being currently loaded to detect circular imports
    loading_stack: Vec<PathBuf>,
}

impl ModuleManager {
    pub fn new() -> Self {
        ModuleManager {
            loaded_paths: HashSet::new(),
            loading_stack: Vec::new(),
        }
    }

    pub fn is_loaded(&self, path: &Path) -> bool {
        self.loaded_paths.contains(path)
    }

    pub fn mark_loaded(&mut self, path: PathBuf) {
        self.loaded_paths.insert(path);
    }

    pub fn enter_loading(&mut self, path: PathBuf) -> Result<(), String> {
        if self.loading_stack.contains(&path) {
            return Err(format!("ImportError: Circular import detected: {}", path.display()));
        }
        self.loading_stack.push(path);
        Ok(())
    }

    pub fn exit_loading(&mut self) {
        self.loading_stack.pop();
    }

    pub fn resolve(&self, path_str: &str, current_file: &str) -> Result<ModuleSource, String> {
        let clean_path = if path_str.ends_with(".lyn") { path_str.to_string() } else { format!("{}.lyn", path_str) };

        // 1. Check embedded stdlib first (highest priority)
        if let Some(content) = stdlib_bundle::get_embedded_stdlib(path_str) {
            return Ok(ModuleSource::Embedded(content.to_string()));
        }

        let mut candidates = Vec::new();

        // 2. Relative to currently executing file
        if let Some(parent) = Path::new(current_file).parent() {
            candidates.push(parent.join(&clean_path));
            candidates.push(parent.join("stdlib").join(&clean_path));
        }

        // 3. Direct path (relative to CWD)
        candidates.push(PathBuf::from(&clean_path));

        // 4. stdlib/ directory relative to workspace root (assumed CWD or nearby)
        candidates.push(PathBuf::from("stdlib").join(&clean_path));

        for cand in candidates {
            if cand.exists() && cand.is_file() {
                return Ok(ModuleSource::File(cand));
            }
        }

        Err(format!("ImportError: Module '{}' not found. Searched in candidates.", path_str))
    }
}

pub enum ModuleSource {
    File(PathBuf),
    Embedded(String),
}
