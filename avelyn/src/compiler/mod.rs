// compiler/mod.rs — Native LLVM compiler pipeline

pub mod llvm_codegen;

use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashSet;
use crate::ast::ASTNode;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::interpreter::module_manager::{ModuleManager, ModuleSource};
use llvm_codegen::LLVMCodeGen;

pub struct Compiler;

impl Compiler {
    pub fn new() -> Self {
        Compiler
    }

    fn expand_imports(&self, ast: &[ASTNode], current_file: &str, loaded: &mut HashSet<String>) -> Result<Vec<ASTNode>, String> {
        let mut expanded = Vec::new();
        let mm = ModuleManager::new();

        for node in ast {
            if let ASTNode::Import(path_str) = node {
                let clean_name = path_str.trim_end_matches(".lyn").to_string();
                if loaded.contains(&clean_name) {
                    continue;
                }
                loaded.insert(clean_name.clone());

                if let Ok(source) = mm.resolve(path_str, current_file) {
                    let (content, res_path) = match source {
                        ModuleSource::File(p) => (fs::read_to_string(&p).map_err(|e| e.to_string())?, p.to_string_lossy().to_string()),
                        ModuleSource::Embedded(c) => (c, format!("embedded://{}", path_str)),
                    };

                    let mut lexer = Lexer::new(&content);
                    let tokens = lexer.tokenize();
                    let mut parser = Parser::new(tokens);
                    let mod_ast = parser.parse();

                    let child_nodes = self.expand_imports(&mod_ast, &res_path, loaded)?;
                    expanded.extend(child_nodes);
                }
            } else {
                expanded.push(node.clone());
            }
        }
        Ok(expanded)
    }

    pub fn compile_to_native(&self, ast: &[ASTNode], out_path: &str, emit_llvm: bool) -> Result<(), String> {
        let mut loaded = HashSet::new();
        let full_ast = self.expand_imports(ast, out_path, &mut loaded)?;

        let mut codegen = LLVMCodeGen::new();
        let ir = codegen.generate(&full_ast)?;

        let temp_dir = std::env::temp_dir().join("avelyn_llvm_build");
        let _ = fs::create_dir_all(&temp_dir);

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let pid = std::process::id();

        let ir_filename = format!("module_{}_{}.ll", pid, nanos);
        let ir_path = temp_dir.join(&ir_filename);
        if let Err(e) = fs::write(&ir_path, &ir) {
            return Err(format!("Failed to write LLVM IR: {}", e));
        }

        if emit_llvm {
            let llvm_out_path = format!("{}.ll", out_path.trim_end_matches(".exe"));
            let _ = fs::write(&llvm_out_path, &ir);
            println!("Emitted LLVM IR: {}", llvm_out_path);
        }

        // Write embedded runtime C source files
        let runtime_h_content = include_str!("sylvel_runtime.h");
        let runtime_c_content = include_str!("sylvel_runtime.c");

        let rt_h_path = temp_dir.join("sylvel_runtime.h");
        let rt_c_path = temp_dir.join("sylvel_runtime.c");

        let _ = fs::write(&rt_h_path, runtime_h_content);
        let _ = fs::write(&rt_c_path, runtime_c_content);

        // Invoke clang to compile and link LLVM IR and C runtime into native platform executable
        let status = Command::new("clang")
            .arg("-O2")
            .arg(&ir_path)
            .arg(&rt_c_path)
            .arg("-o")
            .arg(out_path)
            .status();

        // Cleanup temp ir file
        let _ = fs::remove_file(&ir_path);

        match status {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(format!("clang compilation failed with exit code {:?}", s.code())),
            Err(e) => Err(format!("Failed to execute clang (LLVM toolchain): {}. Ensure LLVM/clang is installed.", e)),
        }
    }
}
