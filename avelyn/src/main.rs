// main.rs — CLI entry point for avelyn executable

mod ast;
mod lexer;
mod parser;
mod value;
mod env;
mod interpreter;
mod compiler;
mod stdlib_bundle;
mod sema;
mod air;
mod airgen;
mod optimizer;
mod irgen;
mod target;
#[cfg(test)]
mod pipeline_tests;

use std::env as std_env;
use std::process::exit;
use std::io::{self, Write};

use lexer::Lexer;
use parser::Parser;
use interpreter::Interpreter;
use compiler::Compiler;

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
            println!("avelyn 2.5.7 (Rust + LLVM Native)");
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
            compile_file_with_args(input_path, &args[3..]);
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
            run_interpreter_file(file_path);
        }
    }
}

fn print_usage() {
    println!("avelyn 2.5.7 — Avelyn Programming Language Compiler & Runtime");
    println!();
    println!("USAGE:");
    println!("  avelyn <file.lyn>                          Run file via interpreter");
    println!("  avelyn compile <file.lyn> [OPTIONS]        Compile to native binary");
    println!("  avelyn repl                                Start interactive REPL");
    println!("  avelyn test [path]                         Run test suite");
    println!("  avelyn --sandbox <file.lyn>                Run in sandboxed mode");
    println!("  avelyn --version                           Show version");
    println!("  avelyn --help                              Show this help");
    println!();
    println!("COMPILE OPTIONS:");
    println!("  -o <output>            Output file path (default: <input>.exe)");
    println!("  --emit-ast             Print AST and exit");
    println!("  --emit-air             Print unoptimized AIR and exit");
    println!("  --emit-air-opt         Print optimized AIR and exit");
    println!("  --emit-llvm            Emit LLVM IR (.ll file) and exit");
    println!("  --emit-object          Compile to object file, do not link");
    println!("  --emit-asm             Compile to assembly (.s file)");
    println!("  -O0 / -O1 / -O2 / -O3 Optimization level (default: -O2)");
    println!("  --target <triple>      Override target triple");
    println!("  --llvm-path <dir>      Path to LLVM bin directory");
    println!("  --verify               Run AIR verifier (default: on)");
    println!("  --no-verify            Skip AIR verifier");
    println!("  --verbose              Print each pipeline stage");
    println!();
    println!("EXAMPLES:");
    println!("  avelyn hello.lyn");
    println!("  avelyn compile hello.lyn -o hello.exe");
    println!("  avelyn compile hello.lyn --emit-air");
    println!("  avelyn compile hello.lyn --emit-llvm -O3");
    println!("  avelyn compile hello.lyn --llvm-path \"C:\\Program Files\\LLVM\\bin\"");
}

// ──────────────── Interpreter path (completely unchanged) ─────────────────────────

fn run_interpreter_file(path: &str) {
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

// ──────────────── New compiler driver ──────────────────────────────────────────

fn compile_file_with_args(input_path: &str, extra_args: &[String]) {
    use compiler::{CompilerOptions, EmitStage};
    use crate::optimizer::OptLevel;
    use crate::target::Target;

    let source = match std::fs::read_to_string(input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", input_path, e);
            exit(1);
        }
    };

    let mut opts = CompilerOptions {
        out_path: if cfg!(windows) {
            format!("{}.exe", input_path.trim_end_matches(".lyn"))
        } else {
            input_path.trim_end_matches(".lyn").to_string()
        },
        ..Default::default()
    };

    let mut i = 0;
    while i < extra_args.len() {
        match extra_args[i].as_str() {
            "-o" if i + 1 < extra_args.len() => {
                opts.out_path = extra_args[i + 1].clone();
                i += 2;
            }
            "--emit-ast"     => { opts.emit = EmitStage::Ast;     i += 1; }
            "--emit-air"     => { opts.emit = EmitStage::Air;     i += 1; }
            "--emit-air-opt" => { opts.emit = EmitStage::AirOpt;  i += 1; }
            "--emit-llvm"    => { opts.emit = EmitStage::Llvm;    i += 1; }
            "--emit-object"  => { opts.emit = EmitStage::Object;  i += 1; }
            "--emit-asm"     => { opts.emit = EmitStage::Asm;     i += 1; }
            "-O0"            => { opts.opt_level = OptLevel::O0;  i += 1; }
            "-O1"            => { opts.opt_level = OptLevel::O1;  i += 1; }
            "-O2"            => { opts.opt_level = OptLevel::O2;  i += 1; }
            "-O3"            => { opts.opt_level = OptLevel::O3;  i += 1; }
            "--verbose"      => { opts.verbose = true;            i += 1; }
            "--verify"       => { opts.verify_air = true;         i += 1; }
            "--no-verify"    => { opts.verify_air = false;        i += 1; }
            "--target" if i + 1 < extra_args.len() => {
                match Target::from_triple(&extra_args[i + 1]) {
                    Ok(t) => opts.target = t,
                    Err(e) => { eprintln!("Error: {}", e); exit(1); }
                }
                i += 2;
            }
            "--llvm-path" if i + 1 < extra_args.len() => {
                opts.llvm_path = Some(extra_args[i + 1].clone());
                i += 2;
            }
            _ => { i += 1; }
        }
    }

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();
    let mut parser_inst = Parser::new(tokens);
    let ast = parser_inst.parse();

    if opts.verbose {
        eprintln!("[avelyn] Parsed {} top-level nodes from '{}'", ast.len(), input_path);
    }

    let compiler = Compiler::new();
    match compiler.compile_with_options(&ast, input_path, &opts) {
        Ok(_) => {
            if opts.emit == EmitStage::Executable {
                println!("Successfully compiled {} -> {}", input_path, opts.out_path);
            }
        }
        Err(e) => {
            eprintln!("{}", e);
            exit(1);
        }
    }
}

// ──────────────── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::capabilities::Capabilities;
    use crate::value::AvelynVal;

    #[test]
    fn test_worker_sandbox_propagation() {
        let interp = Interpreter::new_with_capabilities(Capabilities::none());
        assert!(interp.capabilities.check_fs_write().is_err());
        let caps = interp.capabilities.clone();
        let worker_interp = Interpreter::new_with_capabilities(caps);
        assert!(worker_interp.capabilities.check_fs_write().is_err());
    }

    #[test]
    fn test_unmarshal_dos_protection() {
        let mut payload = vec![5u8];
        payload.extend_from_slice(&0xFFFF_FFFFu32.to_be_bytes());
        let (val, _pos) = AvelynVal::unmarshal(&payload);
        assert!(matches!(val, AvelynVal::List(_)));
    }

    #[test]
    fn test_relational_type_safety() {
        let interp = Interpreter::new();
        let err = interp.eval_bin_op(&AvelynVal::Null, "<", &AvelynVal::Int(5));
        assert!(err.is_err());
    }

    #[test]
    fn test_integer_overflow_error() {
        let interp = Interpreter::new();
        let err = interp.eval_bin_op(&AvelynVal::Int(i64::MAX), "+", &AvelynVal::Int(1));
        assert!(err.is_err());
    }

    #[test]
    fn test_substring_negative_index() {
        let mut interp = Interpreter::new();
        let s = AvelynVal::str("hello world");
        let start = AvelynVal::Int(-5);
        let res = crate::interpreter::builtins::native_substring(&mut interp, vec![s, start]).unwrap();
        assert_eq!(res.as_str(), "world");
    }
}
