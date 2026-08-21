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
use crate::value::AvelynError;

fn main() {
    // Standard 8MB stack size matching industry standard runtimes (Python, GCC/Clang, JVM)
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
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
            println!("Avelyn 2.5.7");
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
    println!("Avelyn 2.5.7 Compiler & Runtime");
    println!();
    println!("USAGE:");
    println!("  Avelyn <file.lyn>                          Run file via interpreter");
    println!("  Avelyn compile <file.lyn> [OPTIONS]        Compile to native binary");
    println!("  Avelyn repl                                Start interactive REPL");
    println!("  Avelyn test [path]                         Run test suite");
    println!("  Avelyn --sandbox <file.lyn>                Run in sandboxed mode");
    println!("  Avelyn --version                           Show version");
    println!("  Avelyn --help                              Show this help");
    println!();
    println!("COMPILE OPTIONS:");
    println!("  -o <output>            Output file path (default: <input>.exe)");
    println!("  --emit-ast             Print AST and exit");
    println!("  --emit-air             Print unoptimized AIR and exit");
    println!("  --emit-air-opt         Print optimized AIR and exit");
    println!("  --emit-llvm            Emit intermediate IR (.ll file) and exit");
    println!("  --emit-object          Compile to object file, do not link");
    println!("  --emit-asm             Compile to assembly (.s file)");
    println!("  -O0 / -O1 / -O2 / -O3 Optimization level (default: -O2)");
    println!("  --target <triple>      Override target triple");
    println!("  --llvm-path <dir>      Path to compiler backend bin directory");
    println!("  --verify               Run AIR verifier (default: on)");
    println!("  --no-verify            Skip AIR verifier");
    println!("  --verbose              Print each pipeline stage");
    println!();
    println!("EXAMPLES:");
    println!("  Avelyn hello.lyn");
    println!("  Avelyn compile hello.lyn -o hello.exe");
    println!("  Avelyn compile hello.lyn --emit-air");
    println!("  Avelyn compile hello.lyn -o hello.exe -O3");
}

// ──────────────── Interpreter path (completely unchanged) ─────────────────────────

fn print_formatted_runtime_error(source: &str, file_path: &str, interp: &Interpreter, err: &AvelynError) {
    let source_lines: Vec<&str> = source.lines().collect();
    
    eprintln!("Traceback (most recent call last):");
    
    if !interp.call_stack.is_empty() {
        for (func_name, fpath, line) in &interp.call_stack {
            let line_num = *line;
            let display_file = if fpath.is_empty() { file_path } else { fpath.as_str() };
            eprintln!("  File \"{}\", line {}, in {}", display_file, line_num, func_name);
            if display_file == file_path && line_num > 0 {
                let idx = (line_num - 1) as usize;
                if let Some(src_line) = source_lines.get(idx) {
                    eprintln!("    {}", src_line.trim());
                }
            }
        }
    }

    let final_line = if err.line > 0 { err.line } else if interp.current_line > 0 { interp.current_line } else { 1 };
    let final_file = if !err.file.is_empty() { err.file.as_str() } else { file_path };
    
    if interp.call_stack.is_empty() {
        eprintln!("  File \"{}\", line {}, in <main>", final_file, final_line);
        if final_file == file_path && final_line > 0 {
            let idx = (final_line - 1) as usize;
            if let Some(src_line) = source_lines.get(idx) {
                eprintln!("    {}", src_line.trim());
            }
        }
    }

    let err_str = err.val.format();
    let trimmed = err_str.trim();

    if trimmed.starts_with("AssertionError:") || trimmed.contains("assertion failed") || trimmed.starts_with("assert ") {
        let msg = trimmed.trim_start_matches("AssertionError:").trim();
        eprintln!("AssertionError: {}", msg);
    } else if trimmed.starts_with("Uncaught throw:") {
        eprintln!("UncaughtException: {}", trimmed.trim_start_matches("Uncaught throw:").trim());
    } else if trimmed.starts_with("NameError:") || trimmed.contains("is not defined") || trimmed.contains("undefined variable") || trimmed.contains("unknown variable") {
        let msg = trimmed.trim_start_matches("NameError:").trim();
        eprintln!("NameError: {}", msg);
    } else if trimmed.starts_with("TypeError:") || trimmed.contains("cannot apply") || trimmed.contains("mismatched type") {
        let msg = trimmed.trim_start_matches("TypeError:").trim();
        eprintln!("TypeError: {}", msg);
    } else if trimmed.starts_with("IndexError:") || trimmed.contains("index out of") || trimmed.contains("out of bounds") {
        let msg = trimmed.trim_start_matches("IndexError:").trim();
        eprintln!("IndexError: {}", msg);
    } else if trimmed.starts_with("KeyError:") || trimmed.contains("key not found") || trimmed.contains("missing key") {
        let msg = trimmed.trim_start_matches("KeyError:").trim();
        eprintln!("KeyError: {}", msg);
    } else if trimmed.starts_with("ZeroDivisionError:") || trimmed.contains("division by zero") {
        let msg = trimmed.trim_start_matches("ZeroDivisionError:").trim();
        eprintln!("ZeroDivisionError: {}", msg);
    } else if trimmed.starts_with("SyntaxError:") || trimmed.contains("syntax error") {
        let msg = trimmed.trim_start_matches("SyntaxError:").trim();
        eprintln!("SyntaxError: {}", msg);
    } else if trimmed.starts_with("MutabilityError:") || trimmed.contains("immutable") || trimmed.contains("Cannot assign to immutable") {
        let msg = trimmed.trim_start_matches("MutabilityError:").trim();
        eprintln!("MutabilityError: {}", msg);
    } else {
        let msg = trimmed.trim_start_matches("RuntimeError:").trim();
        eprintln!("RuntimeError: {}", msg);
    }
}

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
        print_formatted_runtime_error(&source, path, &interp, &err);
        exit(1);
    }
}

fn run_repl() {
    println!("Avelyn 2.5.7 REPL");
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
                print_formatted_runtime_error(&input, "<repl>", &interp, &err);
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
    println!("Avelyn Test Runner");
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
        eprintln!("[Avelyn] Parsed {} top-level nodes from '{}'", ast.len(), input_path);
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
