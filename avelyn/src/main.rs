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

use lexer::Lexer;
use parser::Parser;
use interpreter::Interpreter;
use compiler::{Compiler, writer::BytecodeWriter};

fn main() {
    let args: Vec<String> = std_env::args().collect();
    if args.len() < 2 {
        print_usage();
        exit(1);
    }

    let first = &args[1];
    match first.as_str() {
        "--version" | "-v" => {
            println!("avelyn 2.5.7 (Rust)");
        }
        "--help" | "-h" => {
            print_usage();
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
        "run-vm" => {
            if args.len() < 3 {
                eprintln!("Error: Missing input bytecode file for run-vm command.");
                exit(1);
            }
            run_vm_file(&args[2]);
        }
        file_path => {
            if file_path.ends_with(".lync") || file_path.ends_with(".sbc") {
                run_vm_file(file_path);
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
            println!("Compiled {} -> {}", input_path, out_path);
        }
        Err(e) => {
            eprintln!("Compile error: {}", e);
            exit(1);
        }
    }
}

fn run_vm_file(path: &str) {
    // Shell out to standalone VM binary or delegate
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
            // Fall back to built-in interpreter
            run_interpreter_file(path);
        }
    }
}
