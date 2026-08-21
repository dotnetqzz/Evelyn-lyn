# Evelyn Programming Language (Avelyn v2.5.7)

[![Release](https://img.shields.io/badge/release-v2.5.7-blue.svg)](https://github.com/kiwikiwicq/Evelyn)
[![Language](https://img.shields.io/badge/language-Evelyn-purple.svg)](https://github.com/kiwikiwicq/Evelyn)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

**Evelyn** (and its high-performance compiler/runtime **Avelyn**) is a modern, expressive, indentation-aware programming language engineered with a clean, intuitive syntax, rich functional and object-oriented paradigms, an extensive standard library, and dual execution backends:
1. **Instant Tree-Walking Interpreter** — for rapid prototyping, interactive scripting, and REPL.
2. **Ahead-of-Time (AOT) Native Compiler** — compiles `.lyn` source code through an SSA Intermediate Representation (**AIR**) directly into optimized, standalone native machine binaries (`.exe` / ELF / Mach-O).

---

## Key Highlights

- **Clean, Expressive Syntax**: Indentation-based scoping, intuitive control structures, and zero boilerplate.
- **Dual Execution Engine**: Run immediately with the interpreter or compile to optimized machine code.
- **Explicit SSA Compiler Pipeline (AIR)**: Multi-stage pipeline featuring semantic analysis, AST lowering, AIR verification, constant folding, dead-code elimination, and native code generation.
- **Immutable by Default**: Strict distinction between immutable bindings (`let`) and mutable variables (`var`).
- **Pattern Matching & Algebraic Data Types**: Powerful `match` expressions supporting literal, range, enum variant, and wildcard patterns.
- **First-Class Functions & Closures**: Higher-order functions, lambda expressions (`def(x) => x * 2`), currying, function composition, and the forward pipe operator (`|>`).
- **Extensive Standard Library**: 30+ built-in modules covering strings, arrays, maps, crypto, hashing (MD5, SHA-256, HMAC, AES), JSON, regex, async event loops, file I/O, networking, and system diagnostics.
- **Single Unified Global Test Suite**: Comprehensive, deterministic test suite verified across both interpreter and native modes.

---

## Quick Start

### 1. Installation & Building

```bash
# Clone the repository
git clone https://github.com/kiwikiwicq/Evelyn.git
cd Evelyn

# Build the release binary
cargo build --release --manifest-path avelyn/Cargo.toml
```

The compiled binary will be located at `Avelyn/target/release/Avelyn` (or `Avelyn.exe` on Windows).

---

### 2. Running Evelyn Code

#### Mode A: Immediate Interpretation
```bash
avelyn hello.lyn
```

#### Mode B: Ahead-of-Time (AOT) Native Compilation
```bash
# Compile to an optimized native executable
avelyn compile hello.lyn -o hello.exe -O3

# Run the native binary directly
./hello.exe
```

---

## Hello World Example

Create a file named `hello.lyn`:

```lyn
# hello.lyn
let greeting = "Hello, Evelyn!"
print(greeting)

# High-order function & forward pipe
fn double(x): return x * 2
fn addTen(x): return x + 10

let result = 5 |> double |> addTen
print("Result: " + toString(result))
```

Run it:
```bash
avelyn hello.lyn
# Output:
# Hello, Evelyn!
# Result: 20
```

---

## Language Tour

### 1. Variables & Mutability
```lyn
let pi = 3.14159       # Immutable binding (cannot be reassigned)
var counter = 0        # Mutable variable

counter += 1           # Compound operators: +=, -=, *=, /=, %=, &=, |=, ^=, <<=, >>=
```

### 2. Collections & Destructuring
```lyn
# Arrays & Maps
let numbers = [10, 20, 30, 40]
let config = {"host": "localhost", "port": 8080, "debug": true}

# Destructuring
let [first, second] = numbers
let {"host": hostname, "port": portNum} = config
```

### 3. Pattern Matching & Enums
```lyn
enum Status {
    Pending,
    Active(user),
    Failed(reason)
}

fn handleStatus(s):
    match s {
        case Status.Pending:
            print("Job is pending...")
        case Status.Active(user):
            print("Active for user: " + user)
        case Status.Failed(err):
            print("Failed with error: " + err)
        default:
            print("Unknown status")
    }
```

### 4. Error Handling
```lyn
try {
    throw "DatabaseConnectionTimeout"
} catch err {
    print("Caught error: " + toString(err))
} finally {
    print("Cleanup resources executed")
}
```

---

## Compiler Architecture

![Avelyn System Architecture](assets/architecture_excalidraw.jpg)

The Avelyn compiler follows a modern, multi-stage architecture with an explicit SSA Intermediate Representation:

```
[ .lyn Source Code ]
        │
        ▼
   Lexer & Parser       --> Produces ASTNode tree
        │
        ▼
  Semantic Analysis     --> Name resolution, type checking & diagnostics
        │
        ▼
       AIRGen           --> Lowers AST to Avelyn Intermediate Representation (AIR SSA)
        │
        ▼
    AIR Verifier        --> Ensures SSA CFG invariants & defined-before-use ordering
        │
        ▼
    AIR Optimizer       --> Constant folding/propagation, DCE, CFG simplification
        │
        ▼
     LLVM IRGen         --> Emits optimized LLVM IR (.ll)
        │
        ▼
 Clang / LLD Toolchain  --> Generates standalone native machine binary (.exe / ELF)
```

For full architectural details, see [ARCHITECTURE.md](ARCHITECTURE.md).

---

## Running the Global Test Suite

Evelyn comes with a single unified global test runner:

```bash
# Run all tests sequentially line-by-line (Interpreter mode, 5-minute timeout per test)
python run_all_tests.py

# Run all tests compiled to native machine binaries via LLVM
python run_all_tests.py --mode native

# Run dual parity checks (verifies interpreter and native compiler produce identical results)
python run_all_tests.py --mode dual

# Run tests in parallel with multi-threading
python run_all_tests.py --jobs 8

# Filter tests by module or category
python run_all_tests.py --filter algo
python run_all_tests.py --filter crypto
```

---

## Documentation

For full documentation covering all language constructs, 30+ standard library modules, built-in functions, and example programs, please refer to [DOCS.md](DOCS.md).

---

## License

This project is licensed under the MIT License.
