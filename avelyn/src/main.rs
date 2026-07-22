// main.rs — CLI entry point for avelyn executable
// Ported from Sources/Sylvel/main.swift

mod ast;
mod lexer;
mod parser;
mod value;
mod env;
mod interpreter;
mod compiler;
mod stdlib_bundle;

use std::env as std_env;
use std::process::exit;
use std::io::{self, Write, Read, Seek};

use lexer::Lexer;
use parser::Parser;
use interpreter::Interpreter;
use compiler::{Compiler, writer::BytecodeWriter, loader::BytecodeLoader, verifier::BytecodeVerifier};

fn main() {
    // Increase stack size to 128MB for deep AST recursion resilience in production
    std::thread::Builder::new()
        .stack_size(128 * 1024 * 1024)
        .spawn(run_cli)
        .unwrap()
        .join()
        .unwrap();
}

fn run_cli() {
    let args: Vec<String> = std_env::args().collect();
    if args.len() < 2 {
        run_repl();
        return;
    }

    let first = &args[1];
    match first.as_str() {
        "--version" | "-v" => {
            println!("avelyn 2.5.7 (Rust)");
        }
        "--help" | "-h" => {
            print_usage();
        }
        "repl" => {
            run_repl();
        }
        "compile" => {
            if args.len() < 3 {
                eprintln!("Error: Missing input file for compile command.");
                exit(1);
            }
            let input_path = &args[2];
            let out_path = if args.len() >= 5 && args[3] == "-o" {
                args[4].clone()
            } else {
                format!("{}.lync", input_path.trim_end_matches(".lyn"))
            };
            compile_file(input_path, &out_path);
        }
        "bundle" => {
            if args.len() < 3 {
                eprintln!("Error: Missing entry file for bundle command.");
                exit(1);
            }
            let entry = &args[2];
            let out = if args.len() >= 5 && args[3] == "-o" {
                args[4].clone()
            } else {
                format!("{}.lynb", entry.trim_end_matches(".lyn"))
            };
            bundle_project(entry, &out);
        }
        "run-vm" => {
            if args.len() < 3 {
                eprintln!("Error: Missing input bytecode file for run-vm command.");
                exit(1);
            }
            run_vm_file(&args[2]);
        }
        "test" => {
            let path = if args.len() >= 3 { &args[2] } else { "." };
            run_test_runner(path);
        }
        "--sandbox" => {
            if args.len() < 3 {
                eprintln!("Error: Missing script for sandbox command.");
                exit(1);
            }
            run_sandboxed_file(&args[2]);
        }
        file_path => {
            if file_path.ends_with(".lync") || file_path.ends_with(".sbc") {
                run_vm_file(file_path);
            } else if file_path.ends_with(".lynb") {
                run_bundle_file(file_path);
            } else {
                run_interpreter_file(file_path);
            }
        }
    }
}

fn print_usage() {
    println!("avelyn 2.5.7 - Sylvel Programming Language Runtime");
    println!("Usage:");
    println!("  avelyn <file.lyn>              Run Sylvel source file");
    println!("  avelyn <file.lync>             Run Sylvel bytecode file");
    println!("  avelyn compile <file.lyn> [-o <out.lync>]  Compile source to bytecode");
    println!("  avelyn run-vm <file.lync>      Run bytecode file directly");
    println!("  avelyn --version               Show version");
    println!("  avelyn --help                  Show help");
}

fn run_interpreter_file(path: &str) {
    let lync_path = format!("{}.lync", path.trim_end_matches(".lyn"));

    // Check for cached bytecode — only use cache if mtime is available
    if let (Ok(src_meta), Ok(lync_meta)) = (std::fs::metadata(path), std::fs::metadata(&lync_path)) {
        if let (Ok(src_mtime), Ok(lync_mtime)) = (src_meta.modified(), lync_meta.modified()) {
            if lync_mtime > src_mtime {
                run_vm_file(&lync_path);
                return;
            }
        }
    }

    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", path, e);
            exit(1);
        }
    };

    let mut interp = Interpreter::new();
    interp.current_file = path.to_string();

    // Run stdlib init prelude if embedded
    if let Some(init_src) = stdlib_bundle::get_embedded_stdlib("init") {
        let mut stdlib_lexer = Lexer::new(init_src);
        let stdlib_tokens = stdlib_lexer.tokenize();
        let mut stdlib_parser = Parser::new(stdlib_tokens);
        let stdlib_ast = stdlib_parser.parse();
        let _ = interp.eval_ast(&stdlib_ast);
    }

    // Lex and Parse user file
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse();

    if let Err(err) = interp.eval_ast(&ast) {
        eprintln!("{}", err);
        exit(1);
    }
}

fn run_repl() {
    println!("avelyn 2.5.7 REPL");
    println!("Type 'exit' to quit.");

    let mut interp = Interpreter::new();
    interp.current_file = "<repl>".to_string();

    // Run stdlib init prelude
    if let Some(init_src) = stdlib_bundle::get_embedded_stdlib("init") {
        let mut stdlib_lexer = Lexer::new(init_src);
        let stdlib_tokens = stdlib_lexer.tokenize();
        let mut stdlib_parser = Parser::new(stdlib_tokens);
        let stdlib_ast = stdlib_parser.parse();
        let _ = interp.eval_ast(&stdlib_ast);
    }

    let mut input = String::new();
    loop {
        print!(">>> ");
        io::stdout().flush().unwrap();

        input.clear();
        if io::stdin().read_line(&mut input).is_err() || input.trim() == "exit" {
            break;
        }

        if input.trim().is_empty() { continue; }

        let mut lexer = Lexer::new(&input);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();

        match interp.eval_ast(&ast) {
            Ok(val) => {
                if !val.is_null() {
                    println!("{}", val.format());
                }
            }
            Err(err) => {
                eprintln!("{}", err);
            }
        }
    }
}

fn bundle_project(entry_path: &str, out_path: &str) {
    use std::collections::HashMap;
    use compiler::bundler::Bundler;

    println!("Bundling project starting from {}", entry_path);

    let mut files_to_bundle = HashMap::new();
    let mut processed = std::collections::HashSet::new();
    let mut to_process = vec![entry_path.to_string()];
    let entry_dir = std::path::Path::new(entry_path).parent().unwrap_or_else(|| std::path::Path::new("."));

    while let Some(path) = to_process.pop() {
        if processed.contains(&path) { continue; }
        processed.insert(path.clone());

        let mut source = None;
        let mut actual_path = path.clone();

        let candidates = [
            path.clone(),
            format!("{}.lyn", path),
            entry_dir.join(&path).to_string_lossy().to_string(),
            entry_dir.join(format!("{}.lyn", path)).to_string_lossy().to_string(),
        ];

        for cand in candidates {
            if let Ok(s) = std::fs::read_to_string(&cand) {
                source = Some(s);
                actual_path = cand;
                break;
            }
        }

        let source = match source {
            Some(s) => s,
            None => {
                eprintln!("Bundle error: Could not read file '{}'", path);
                exit(1);
            }
        };

        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();

        for node in &ast {
            if let ast::ASTNode::Import(dep) | ast::ASTNode::Include(dep) = node {
                to_process.push(dep.clone());
            }
        }

        let compiler = Compiler::new();
        match compiler.compile(&ast) {
            Ok(module) => {
                let bytes = BytecodeWriter::serialize(&module);
                let bundle_name = std::path::Path::new(&actual_path)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or(&actual_path)
                    .to_string();
                files_to_bundle.insert(bundle_name, bytes);
            }
            Err(e) => {
                eprintln!("Error compiling {}: {}", actual_path, e);
                exit(1);
            }
        }
    }

    let entry_filename = std::path::Path::new(entry_path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(entry_path);

    if let Err(e) = Bundler::bundle(entry_filename, files_to_bundle, out_path) {
        eprintln!("Bundle error: {}", e);
        exit(1);
    }
    println!("Successfully created bundle: {}", out_path);
}

fn run_bundle_file(path: &str) {
    let bundle = match BytecodeLoader::load_bundle(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Failed to load bundle '{}': {}", path, e);
            exit(1);
        }
    };

    eprintln!("Note: Running .lynb requires VM integration. Orchestrating via temp files...");

    let temp_dir = std::env::temp_dir().join("avelyn_bundle");
    let _ = std::fs::create_dir_all(&temp_dir);

    for (name, data) in bundle {
        let mut p = temp_dir.join(name);
        if !p.to_string_lossy().ends_with(".lync") {
            p = p.with_extension("lync");
        }
        if let Some(parent) = p.parent() { let _ = std::fs::create_dir_all(parent); }
        if let Err(e) = std::fs::write(&p, data) {
            eprintln!("Bundle extraction error for '{}': {}", p.display(), e);
            exit(1);
        }
    }

    let entry_path = (|| -> Option<String> {
        let mut file = std::fs::File::open(path).ok()?;
        file.seek(std::io::SeekFrom::Start(6)).ok()?;
        let mut len_bytes = [0u8; 4];
        file.read_exact(&mut len_bytes).ok()?;
        let len = u32::from_be_bytes(len_bytes);
        let mut entry_bytes = vec![0u8; len as usize];
        file.read_exact(&mut entry_bytes).ok()?;
        String::from_utf8(entry_bytes).ok()
    })();

    let entry_path = match entry_path {
        Some(p) => p,
        None => {
            eprintln!("Failed to parse bundle entry point header from '{}'", path);
            exit(1);
        }
    };

    let file_stem = std::path::Path::new(&entry_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&entry_path);
    let entry_lync = format!("{}.lync", file_stem);
    run_vm_file(temp_dir.join(entry_lync).to_str().unwrap_or(&entry_path));
}

fn run_sandboxed_file(path: &str) {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", path, e);
            exit(1);
        }
    };

    let mut interp = Interpreter::new();
    interp.current_file = path.to_string();
    interp.capabilities = interpreter::capabilities::Capabilities::none();

    // Lex and Parse user file
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse();

    if let Err(err) = interp.eval_ast(&ast) {
        eprintln!("{}", err);
        exit(1);
    }
}

fn run_test_runner(path: &str) {
    use rayon::prelude::*;
    println!("avelyn Test Runner");
    println!("Scanning: {}", path);

    let mut files = Vec::new();
    let meta = std::fs::metadata(path).unwrap();
    if meta.is_file() {
        files.push(path.to_string());
    } else if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().map(|e| e == "lyn").unwrap_or(false) {
                files.push(p.to_string_lossy().to_string());
            }
        }
    }
    files.sort();

    // Configure Rayon worker threads with 128MB stack size
    let pool = rayon::ThreadPoolBuilder::new()
        .stack_size(128 * 1024 * 1024)
        .build()
        .unwrap();

    let results: Vec<(String, bool, String)> = pool.install(|| {
        files.par_iter().map(|f| {
            let source = match std::fs::read_to_string(f) {
                Ok(s) => s,
                Err(e) => return (f.clone(), false, format!("IOError: {}", e)),
            };
            let mut interp = Interpreter::new();
            interp.current_file = f.clone();

            let mut lexer = Lexer::new(&source);
            let tokens = lexer.tokenize();
            let mut parser = Parser::new(tokens);
            let ast = parser.parse();

            match interp.eval_ast(&ast) {
                Ok(_) => (f.clone(), true, String::new()),
                Err(e) => (f.clone(), false, format!("{}", e)),
            }
        }).collect()
    });

    let mut passed = 0;
    let mut failed = 0;

    for (f, ok, msg) in results {
        print!("Test {} ... ", f);
        if ok {
            println!("PASSED");
            passed += 1;
        } else {
            println!("FAILED: {}", msg);
            failed += 1;
        }
    }

    println!("\nTest Results: {} passed, {} failed", passed, failed);
    if failed > 0 { exit(1); }
}

fn compile_file(input_path: &str, out_path: &str) {
    let source = match std::fs::read_to_string(input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", input_path, e);
            exit(1);
        }
    };

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse();

    let compiler = Compiler::new();
    match compiler.compile(&ast) {
        Ok(module) => {
            if let Err(e) = BytecodeWriter::write(&module, out_path) {
                eprintln!("Compilation error: {}", e);
                exit(1);
            }
            println!("Compiled {}  {}", input_path, out_path);
        }
        Err(e) => {
            eprintln!("Compile error: {}", e);
            exit(1);
        }
    }
}

fn run_vm_file(path: &str) {
    // 1. Validate bytecode integrity before running
    match BytecodeLoader::load(path) {
        Ok(module) => {
            if let Err(e) = BytecodeVerifier::verify(&module) {
                eprintln!("Bytecode Verification Error in '{}': {}", path, e);
                exit(1);
            }
        }
        Err(e) => {
            eprintln!("Failed to load bytecode '{}': {}", path, e);
            if !path.ends_with(".lyn") { exit(1); }
        }
    }

    // 2. Shell out to standalone VM binary
    let vm_bin = if cfg!(windows) { "sylvel-vm.exe" } else { "sylvel-vm" };
    let status = std::process::Command::new(vm_bin)
        .arg(path)
        .status();

    match status {
        Ok(code) => {
            if !code.success() {
                exit(code.code().unwrap_or(1));
            }
        }
        Err(_) => {
            // If VM is missing, we can't run bytecode since the interpreter is tree-walk only
            eprintln!("Error: 'sylvel-vm' not found. Bytecode execution requires the VM.");
            exit(1);
        }
    }
}
