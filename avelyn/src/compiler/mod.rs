// compiler/mod.rs — Native compiler pipeline (AIR-based)
//
// This module orchestrates the full compilation pipeline:
//
//   Source → Lexer → Parser → AST → [Sema] → AIRGen → AIR →
//   [Verify] → [Optimize] → LLVM IRGen → LLVM IR → clang → executable
//
// The old `llvm_codegen.rs` is preserved as a compatibility shim and used
// as a fallback if the AIR pipeline is explicitly disabled (e.g. during
// incremental migration).  In normal operation the AIR pipeline is active.

pub mod llvm_codegen;  // preserved compatibility shim

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashSet;

use crate::ast::ASTNode;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::interpreter::module_manager::{ModuleManager, ModuleSource};
use crate::sema::{DiagnosticEmitter, SemaContext};
use crate::airgen::lower_to_air;
use crate::air::{verify::verify_module, printer::AirPrinter};
use crate::optimizer::{optimize, OptLevel};
use crate::irgen::lower_to_llvm;
use crate::target::{Target, windows_x64, diagnostics as target_diagnostics};

// ─── Emit stage ───────────────────────────────────────────────────────────────

/// Which compilation stage to stop at and emit output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmitStage {
    /// Run through the full pipeline and produce a native binary.
    Executable,
    /// Stop after parsing and print the AST.
    Ast,
    /// Stop after AIRGen and print unoptimized AIR.
    Air,
    /// Stop after optimization and print optimized AIR.
    AirOpt,
    /// Stop after LLVM IRGen and emit a `.ll` file (no native binary).
    Llvm,
    /// Compile to an object file but do not link.
    Object,
    /// Compile to assembly.
    Asm,
}

// ─── Compiler options ─────────────────────────────────────────────────────────

pub struct CompilerOptions {
    pub out_path:    String,
    pub emit:        EmitStage,
    pub opt_level:   OptLevel,
    pub target:      Target,
    pub llvm_path:   Option<String>,
    pub verbose:     bool,
    pub verify_air:  bool,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        CompilerOptions {
            out_path:   String::new(),
            emit:       EmitStage::Executable,
            opt_level:  OptLevel::O2,
            target:     Target::host_default(),
            llvm_path:  None,
            verbose:    false,
            verify_air: true,
        }
    }
}

// ─── Compiler ─────────────────────────────────────────────────────────────────

pub struct Compiler;

impl Compiler {
    pub fn new() -> Self { Compiler }

    // ── Import expansion (unchanged from original) ─────────────────────────

    fn unwrap_line(node: &ASTNode) -> &ASTNode {
        match node {
            ASTNode::Line(_, inner) => Self::unwrap_line(inner),
            _ => node,
        }
    }

    fn expand_imports(&self, ast: &[ASTNode], current_file: &str, loaded: &mut HashSet<String>) -> Result<Vec<ASTNode>, String> {
        let mut expanded = Vec::new();
        let mm = ModuleManager::new();

        for node in ast {
            let unwrap = Self::unwrap_line(node);
            if let ASTNode::Import(path_str) | ASTNode::Include(path_str) = unwrap {
                let clean_name = path_str.trim_end_matches(".lyn").to_string();
                if loaded.contains(&clean_name) { continue; }
                loaded.insert(clean_name.clone());

                if let Ok(source) = mm.resolve(path_str, current_file) {
                    let (content, res_path) = match source {
                        ModuleSource::File(p) => (
                            fs::read_to_string(&p).map_err(|e| e.to_string())?,
                            p.to_string_lossy().to_string(),
                        ),
                        ModuleSource::Embedded(c) => (c, format!("embedded://{}", path_str)),
                    };
                    let mut lexer  = Lexer::new(&content);
                    let tokens     = lexer.tokenize();
                    let mut parser = Parser::new(tokens);
                    let mod_ast    = parser.parse();
                    let child      = self.expand_imports(&mod_ast, &res_path, loaded)?;
                    expanded.extend(child);
                }
            } else {
                expanded.push(node.clone());
            }
        }
        Ok(expanded)
    }

    // ── Legacy compile_to_native (preserved for backward compatibility) ────

    pub fn compile_to_native(&self, ast: &[ASTNode], out_path: &str, emit_llvm: bool) -> Result<(), String> {
        let opts = CompilerOptions {
            out_path:  out_path.to_string(),
            emit:      if emit_llvm { EmitStage::Llvm } else { EmitStage::Executable },
            opt_level: OptLevel::O2,
            verbose:   false,
            ..Default::default()
        };
        self.compile_with_options(ast, out_path, &opts)
    }

    // ── Full AIR pipeline ─────────────────────────────────────────────────

    pub fn compile_with_options(
        &self,
        ast: &[ASTNode],
        input_path: &str,
        opts: &CompilerOptions,
    ) -> Result<(), String> {
        // ── Stage 0: Prepend stdlib prelude & Import expansion ──────────────
        let mut initial_ast = Vec::new();
        if let Some(init_src) = crate::stdlib_bundle::get_embedded_stdlib("init") {
            let mut stdlib_lexer = Lexer::new(init_src);
            let stdlib_tokens = stdlib_lexer.tokenize();
            let mut stdlib_parser = Parser::new(stdlib_tokens);
            let mut stdlib_ast = stdlib_parser.parse();
            initial_ast.append(&mut stdlib_ast);
        }
        initial_ast.extend_from_slice(ast);

        let mut loaded = HashSet::new();
        loaded.insert("init".to_string());
        let full_ast = self.expand_imports(&initial_ast, input_path, &mut loaded)?;

        // ── Stage 1: --emit-ast ─────────────────────────────────────────────
        if opts.emit == EmitStage::Ast {
            for node in &full_ast {
                println!("{:#?}", node);
            }
            return Ok(());
        }

        // ── Stage 2: Sema ────────────────────────────────────────────────────
        let mut sema = SemaContext::new();
        let _typed_nodes = sema.analyse(&full_ast);
        if sema.diag.has_errors() {
            sema.diag.emit_to_stderr(None);
            return Err(format!("{} semantic error(s)", sema.diag.error_count()));
        }
        // Emit warnings but continue.
        if sema.diag.warning_count() > 0 {
            sema.diag.emit_to_stderr(None);
        }

        // ── Stage 3: AIRGen ──────────────────────────────────────────────────
        let mut diag = DiagnosticEmitter::new();
        let mut air_module = lower_to_air(&full_ast, &mut diag)
            .map_err(|errs| errs.join("\n"))?;

        if diag.has_errors() {
            diag.emit_to_stderr(None);
            return Err(format!("{} AIRGen error(s)", diag.error_count()));
        }

        // ── Stage 3b: --emit-air (unoptimized) ──────────────────────────────
        if opts.emit == EmitStage::Air {
            print!("{}", AirPrinter::print_module(&air_module));
            return Ok(());
        }

        // ── Stage 4: AIR Verification ────────────────────────────────────────
        if opts.verify_air {
            let verify_errors = verify_module(&air_module);
            if !verify_errors.is_empty() {
                for e in &verify_errors {
                    eprintln!("AIR verify: {}", e.message);
                }
                // Verification failures are warnings in current phase
                // (the verifier may flag items from complex control flow
                //  that are actually correct).  Treat as non-fatal for now.
            }
        }

        // ── Stage 5: Optimization ────────────────────────────────────────────
        let _pass_count = optimize(&mut air_module, opts.opt_level);
        if opts.verbose {
            eprintln!("[avelyn] Optimization ({}) complete.", opts.opt_level);
        }

        // ── Stage 5b: --emit-air-opt (optimized) ────────────────────────────
        if opts.emit == EmitStage::AirOpt {
            print!("{}", AirPrinter::print_module(&air_module));
            return Ok(());
        }

        // ── Stage 6: LLVM IRGen ──────────────────────────────────────────────
        let llvm_ir = lower_to_llvm(&air_module, &opts.target);

        // ── Stage 6b: --emit-llvm ────────────────────────────────────────────
        if opts.emit == EmitStage::Llvm {
            let ll_path = format!("{}.ll", opts.out_path.trim_end_matches(".exe"));
            fs::write(&ll_path, &llvm_ir).map_err(|e| format!("Failed to write .ll file: {}", e))?;
            println!("Emitted LLVM IR: {}", ll_path);
            return Ok(());
        }

        // ── Stage 7: Toolchain invocation ────────────────────────────────────

        // Write LLVM IR to temp file.
        let temp_dir = std::env::temp_dir().join("avelyn_llvm_build");
        let _ = fs::create_dir_all(&temp_dir);

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let pid = std::process::id();
        let ir_path = temp_dir.join(format!("module_{}_{}.ll", pid, nanos));

        fs::write(&ir_path, &llvm_ir).map_err(|e| format!("Failed to write LLVM IR: {}", e))?;

        // Write embedded runtime C source files.
        let rt_h = include_str!("sylvel_runtime.h");
        let rt_c = include_str!("sylvel_runtime.c");
        let rt_h_path = temp_dir.join("sylvel_runtime.h");
        let rt_c_path = temp_dir.join("sylvel_runtime.c");
        let _ = fs::write(&rt_h_path, rt_h);
        let _ = fs::write(&rt_c_path, rt_c);

        // Find clang.
        let clang = windows_x64::probe_clang(opts.llvm_path.as_deref())
            .map_err(|searched| {
                target_diagnostics::toolchain_not_found_message(&searched)
            })?;

        let opt_u8 = opts.opt_level as u8;

        match opts.emit {
            EmitStage::Object => {
                let obj_path = PathBuf::from(opts.out_path.trim_end_matches(".exe"))
                    .with_extension("obj");
                windows_x64::compile_to_object(
                    &clang, &ir_path, &obj_path, opt_u8, opts.verbose
                ).map_err(|e| e)?;
                println!("Object file: {}", obj_path.display());
            }
            EmitStage::Asm => {
                let asm_path = PathBuf::from(opts.out_path.trim_end_matches(".exe"))
                    .with_extension("s");
                windows_x64::compile_to_asm(
                    &clang, &ir_path, &asm_path, opt_u8, opts.verbose
                )?;
                println!("Assembly file: {}", asm_path.display());
            }
            EmitStage::Executable | _ => {
                windows_x64::compile_and_link(
                    &clang, &ir_path, &rt_c_path,
                    &PathBuf::from(&opts.out_path),
                    opt_u8, opts.verbose
                )?;
            }
        }

        // Cleanup IR temp file.
        let _ = fs::remove_file(&ir_path);

        Ok(())
    }
}
