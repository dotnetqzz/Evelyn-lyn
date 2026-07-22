// main.rs — Sylvel VM (Rust) entry point
//
// Usage:
//   sylvel-vm <file.lync>      Run a compiled Sylvel bytecode file
//   sylvel-vm --version        Print version and exit
//   sylvel-vm --help           Show help

mod value;
mod bytecode;
mod verifier;
mod vm;
mod stdlib;

use vm::Vm;

const VERSION: &str = "1.0.0";

fn print_usage(prog: &str) {
    println!("Usage:");
    println!("  {} <file.lync>    Run a compiled Sylvel bytecode file", prog);
    println!("  {} --version      Print version and exit", prog);
    println!("  {} --help         Show this help message", prog);
    println!();
    println!("Pipeline:");
    println!("  1. Compile:  sylvel compile hello.lyn   → hello.lync");
    println!("  2. Run:      sylvel-vm hello.lync        → executes");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let prog = args.first().map(|s| s.as_str()).unwrap_or("sylvel-vm");

    if args.len() < 2 {
        println!("Sylvel VM v{} — Stack Machine Runtime", VERSION);
        print_usage(prog);
        std::process::exit(1);
    }

    let flag = &args[1];

    if flag == "--version" || flag == "-v" {
        println!("Sylvel VM {}", VERSION);
        return;
    }

    if flag == "--help" || flag == "-h" {
        print_usage(prog);
        return;
    }

    let path = flag.as_str();

    // Warn if .lyn source passed instead of .lync bytecode
    if path.ends_with(".lyn") {
        eprintln!("sylvel-vm: '{}' is a source file.", path);
        eprintln!("Compile it first:  sylvel compile {}", path);
        eprintln!("Then run:          sylvel-vm {}", path.replace(".lyn", ".lync"));
        std::process::exit(1);
    }

    let path_str = path.to_string();
    let builder = std::thread::Builder::new()
        .name("sylvel-vm-main".to_string())
        .stack_size(128 * 1024 * 1024);

    let handler = builder.spawn(move || {
        let module = match bytecode::load_file(&path_str) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("sylvel-vm: cannot load '{}': {}", path_str, e);
                std::process::exit(1);
            }
        };
        let mut vm = Vm::new();
        stdlib::register_all(&mut vm);
        if let Err(e) = vm.run_module(&module) {
            eprintln!("Runtime error: {}", e);
            std::process::exit(1);
        }
    }).unwrap();

    handler.join().unwrap();
}
