# Avelyn Architecture Description (ISO/IEC/IEEE 42010 Standard)

> **Document Version**: 2.5.7  
> **System**: Evelyn Language & Avelyn Compiler/Runtime Engine  
> **Classification**: Core System Architecture & Intercomponent Blueprint  

This document specifies the software architecture for the **Evelyn** programming language implementation (**Avelyn**). The architecture conforms to the **ISO/IEC/IEEE 42010** standard for systems and software engineering architecture descriptions, utilizing a modular **C4 Component Model** with multi-tier decoupling between the front-end language syntax, semantic reasoning, SSA intermediate representation, native compilation passes, tree-walking runtime, and standard library subsystems.

---

## 1. Landscape System Architecture Diagram

![Avelyn System Architecture](assets/architecture_excalidraw.jpg)

### Component Connectivity Matrix

```
                                  ┌─────────────────────────────────────────────────────────┐
                                  │                  COMPILATION PIPELINE                   │
                                  │   ┌─────────────────────────────────────────────────┐   │
                                  │   │  AIRGen (SSA Intermediate Representation Gen)   │   │
                                  │   │  • Lowers AST nodes to explicit SSA instructions│   │
                                  │   │  • Hoists stack allocas to entry basic block    │   │
                                  │   └───────────────────────┬─────────────────────────┘   │
                                  │                           │ AirModule                   │
                                  │                           ▼                             │
                                  │   ┌─────────────────────────────────────────────────┐   │
                                  │   │  AIR Verifier & SSA Consistency Auditor         │   │
                                  │   │  • Defined-before-use value ordering            │   │
                                  │   │  • CFG terminator invariants & dominance check  │   │
                                  │   └───────────────────────┬─────────────────────────┘   │
                                  │                           │ Validated AIR               │
                                  │                           ▼                             │
                                  │   ┌─────────────────────────────────────────────────┐   │
                                  │   │  AIR Optimizer Pipeline (-O0 to -O3)            │   │
                                  │   │  • Constant Folding & Propagation               │   │
                                  │   │  • Dead Code & Redundant Alloca Elimination     │   │
                                  │   │  • Unreachable Block & Trampoline Simplifier    │   │
                                  │   └───────────────────────┬─────────────────────────┘   │
                                  └───────────────────────────┼─────────────────────────────┘
                                                              │ Optimized AIR
                                                              ▼
┌───────────────────────────────────────┐         ┌──────────────────────────────┐         ┌───────────────────────────────────────┐
│           FRONTEND LAYER              │         │                              │         │            BACKEND LAYER              │
│ ┌───────────────────────────────────┐ │         │   ★ AVELYN CORE ENGINE ★     │         │ ┌───────────────────────────────────┐ │
│ │ Evelyn Source Code (*.lyn)        │ │         │   ────────────────────────   │         │ │ Native IR Generator (irgen/)      │ │
│ │ • Indentation / Braces            │ │         │   Central Execution Kernel   │         │ │ • Translates AIR to Target IR     │ │
│ └─────────────────┬─────────────────┘ │         │   & Driver Orchestrator      │         │ │ • Generates C-ABI declarations    │ │
│                   │ Raw Text          │         │                              │         │ └─────────────────┬─────────────────┘ │
│                   ▼                   │         │   • CLI Command Dispatcher   │         │                   │ Target IR (.ll)   │
│ ┌───────────────────────────────────┐ │         │   • Pipeline Phase Router    │         │                   ▼                   │
│ │ Indentation-Aware Lexer (lexer.rs)│ │ ──────► │   • Multi-Stage Coordinator  │ ──────► │ ┌───────────────────────────────────┐ │
│ │ • INDENT / DEDENT token synthesis │ │ ASTNode │   • Diagnostic Hub           │ Native  │ │ Target Toolchain & Linker Driver  │ │
│ │ • Radix literals & escapes        │ │         │   • Memory Lifecycle Guard   │ Binary  │ │ • Toolchain Discovery Probe       │ │
│ └─────────────────┬─────────────────┘ │         │                              │         │ │ • Platform Linker (MSVC / LLD)    │ │
│                   │ Token Stream      │         │                              │         │ └─────────────────┬─────────────────┘ │
│                   ▼                   │         │                              │         │                   │ Output Bin        │
│ ┌───────────────────────────────────┐ │         │                              │         │                   ▼                   │
│ │ Recursive Descent Parser          │ │         │                              │         │ ┌───────────────────────────────────┐ │
│ │ • Operator precedence ladder      │ │         │                              │         │ │ Standalone Native Binary (.exe)   │ │
│ │ • Pattern matching & expressions  │ │         │                              │         │ │ • Zero-dependency deployment      │ │
│ └─────────────────┬─────────────────┘ │         │                              │         │ └───────────────────────────────────┘ │
│                   │ AST Tree          │         │                              │         └───────────────────────────────────────┘
│                   ▼                   │         │                              │
│ ┌───────────────────────────────────┐ │         │                              │
│ │ Semantic Analyzer (sema/)         │ │         │                              │
│ │ • Symbol Tables & Name Resolution │ │         │                              │
│ │ • Immutability Guard (let vs var) │ │         │                              │
│ │ • Type Inference & Diagnostics    │ │         │                              │
│ └───────────────────────────────────┘ │         │                              │
└───────────────────────────────────────┘         └──────────────┬───────────────┘
                                                                 │
                                                                 │ AST & Prelude
                                                                 ▼
                                  ┌─────────────────────────────────────────────────────────┐
                                  │               RUNTIME & STDLIB SUBSYSTEM                │
                                  │   ┌─────────────────────────────────────────────────┐   │
                                  │   │  Tree-Walking Interpreter Engine (eval.rs)      │   │
                                  │   │  • Fast-path immediate execution & REPL         │   │
                                  │   │  • Call stack tracking & Python-style traceback │   │
                                  │   └───────────────────────┬─────────────────────────┘   │
                                  │                           │                             │
                                  │                           ▼                             │
                                  │   ┌─────────────────────────────────────────────────┐   │
                                  │   │  AvelynVal Memory Model & Scope Environments    │   │
                                  │   │  • Lexical closure captures & GC reference model│   │
                                  │   │  • Heterogeneous arrays, maps, structs & enums  │   │
                                  │   └───────────────────────┬─────────────────────────┘   │
                                  │                           │                             │
                                  │                           ▼                             │
                                  │   ┌─────────────────────────────────────────────────┐   │
                                  │   │  Embedded Standard Library & FFI Bridge         │   │
                                  │   │  • 30+ Built-in Modules (math, crypto, json...) │   │
                                  │   │  • System Capabilities & Security Sandbox       │   │
                                  │   └─────────────────────────────────────────────────┘   │
                                  └─────────────────────────────────────────────────────────┘
```

---

## 2. Interconnecting Components & Protocols

The Avelyn architecture enforces strict decoupling between pipeline stages through formalized data contracts:

### 2.1 The Core Driver (`main.rs`)
- **Role**: Central orchestrator and execution entry point.
- **Interconnections**:
  - Routes user requests from the CLI interface to either the **Fast-Path Interpreter** or the **AOT Compiler**.
  - Initializes the **Diagnostic Event Bus** and registers source files with unique `file_id` identifiers.
  - Spawns high-capacity execution stacks (128MB) to guarantee deep AST and recursive function resilience.

### 2.2 Frontend & Semantic Interconnections
1. **Source Text $\rightarrow$ Lexer (`lexer.rs`)**:
   - Emits a unified stream of `(Token, LineNumber)` tuples.
   - Synthesizes `INDENT` and `DEDENT` tokens using an internal indentation stack to transparently bridge indentation-style code with brace-style blocks.
2. **Tokens $\rightarrow$ Recursive Parser (`parser.rs`)**:
   - Constructs a rich Abstract Syntax Tree (`ASTNode`) with Pratt precedence parsing for binary operators.
3. **AST $\rightarrow$ Semantic Analyzer (`sema/`)**:
   - **Name Resolver (`name_resolver.rs`)**: Traverses AST nodes, checks variable declaration invariants, flags unauthorized reassignment of immutable `let` bindings, and builds symbol scope chains.
   - **Type Checker (`type_checker.rs`)**: Resolves static type annotations (`AvelynType`) and emits compile-time diagnostics.
   - **Diagnostic Emitter (`diagnostics.rs`)**: Emits structured errors and warnings with source spans (`file_id`, `line`, `col`), source line caret pointers, and actionable fix hints.

### 2.3 Intermediate Representation & Optimization Interconnections
1. **AST $\rightarrow$ AIRGen (`airgen/`)**:
   - Lowers high-level AST constructs into **AIR** (Avelyn Intermediate Representation) — a linear, SSA-form three-address code representation.
   - Normalizes complex control flow into explicit `BasicBlock` graphs with single-terminator exits (`Jump`, `Branch`, `Return`).
   - Uses `runtime_map.rs` as the single source of truth for runtime C-ABI function signatures.
2. **AIR $\rightarrow$ AIR Verifier (`air/verify.rs`)**:
   - Validates dominance and defined-before-use properties for all SSA registers (`%v0`, `%v1`, ...).
   - Verifies jump targets, basic block connectivity, and parameter types before optimization passes execute.
3. **AIR $\rightarrow$ AIR Optimizer (`optimizer/`)**:
   - Executes multi-pass fixed-point optimizations configured by optimization levels:
     - **`-O0`**: Unoptimized debug generation.
     - **`-O1`**: Dead Code Elimination (`dce.rs`), Redundant Alloca Removal (`redundant_elim.rs`), Unreachable Block Elimination (`unreachable_elim.rs`).
     - **`-O2`**: `-O1` + Compile-Time Constant Folding (`const_fold.rs`), Local Constant Propagation (`const_prop.rs`), Trampoline & Block Simplification (`bb_simplify.rs`).
     - **`-O3`**: `-O2` + aggressive loop unrolling and function inline candidates.

### 2.4 Native Code Generation Interconnections
1. **Optimized AIR $\rightarrow$ Native IRGen (`irgen/`)**:
   - Translates verified and optimized `AirModule` instructions into standard intermediate representation text (`.ll`).
   - Declares and links runtime C-ABI signatures (`sylvel_runtime.c` / `sylvel_runtime.h`).
2. **Target Toolchain Linker (`target/`)**:
   - Discovers platform toolchains via `probe_clang` and environment paths (`AVELYN_LLVM_PATH`, system PATH).
   - Directs the platform linker to produce standalone, statically linked machine executables (`.exe` / ELF).

### 2.5 Dynamic Runtime & Standard Library Interconnections
1. **AST $\rightarrow$ Tree-Walking Interpreter (`interpreter/eval.rs`)**:
   - Evaluates AST nodes directly in memory for instant feedback and REPL sessions.
   - Maintains execution state in `Interpreter` struct with lexical scope environments (`Env`), call stack frames, and recursion depth guards.
2. **Value Representation (`value.rs`)**:
   - Encapsulates dynamic values inside `AvelynVal` enum (Scalars, Strings, Lists, Maps, Structs, Enums, Native Function Pointers).
   - Manages memory through reference counting (`Rc<RefCell<...>>`) and copy-on-write semantics for mutable collections.
3. **Embedded Standard Library (`stdlib_bundle.rs`)**:
   - Embeds 30+ core library modules directly inside the compiler binary.
   - Loads the standard prelude (`init.lyn`) upon engine startup to provide immediate access to math, strings, crypto, hashing, collections, and I/O.
4. **Security & Capabilities Manager (`capabilities.rs`)**:
   - Enforces sandboxed security boundaries (`--sandbox`), restricting filesystem, network, and process spawning when running untrusted scripts.

---

## 3. Dual Execution Pipeline Data Flow

```
                           ┌──────────────────────────┐
                           │   Evelyn Source (*.lyn)  │
                           └─────────────┬────────────┘
                                         │
                                         ▼
                           ┌──────────────────────────┐
                           │  Lexer & Parser Frontend │
                           └─────────────┬────────────┘
                                         │
                                         ▼
                           ┌──────────────────────────┐
                           │   ASTNode Syntax Tree    │
                           └─────────────┬────────────┘
                                         │
                    ┌────────────────────┴────────────────────┐
                    │                                         │
                    ▼ [Fast-Path Mode]                        ▼ [Native AOT Mode]
     ┌─────────────────────────────┐           ┌─────────────────────────────┐
     │  Tree-Walking Interpreter   │           │    Semantic Analyzer (Sema) │
     │  (eval.rs + Environment)    │           │    (NameResolver + TyCheck) │
     └──────────────┬──────────────┘           └──────────────┬──────────────┘
                    │                                         │
                    ▼                                         ▼
     ┌─────────────────────────────┐           ┌─────────────────────────────┐
     │   Value Memory Model        │           │    AIR SSA Generator        │
     │   (AvelynVal + Native FFI)  │           │    (airgen/ + Builder)      │
     └──────────────┬──────────────┘           └──────────────┬──────────────┘
                    │                                         │
                    ▼                                         ▼
     ┌─────────────────────────────┐           ┌─────────────────────────────┐
     │  Instant Program Output     │           │    AIR SSA Optimizer        │
     │  • REPL / Script Execution  │           │    (-O0, -O1, -O2, -O3)     │
     │  • Python-Style Traceback   │           └──────────────┬──────────────┘
     └─────────────────────────────┘                          │
                                                              ▼
                                               ┌─────────────────────────────┐
                                               │    Native IR Generator      │
                                               │    (irgen/ + Runtime ABI)   │
                                               └──────────────┬──────────────┘
                                                              │
                                                              ▼
                                               ┌─────────────────────────────┐
                                               │  Target Linker & Toolchain  │
                                               │  (MSVC / Clang / LLD)       │
                                               └──────────────┬──────────────┘
                                                              │
                                                              ▼
                                               ┌─────────────────────────────┐
                                               │ Standalone Machine Binary   │
                                               │ (.exe / ELF / Mach-O)       │
                                               └─────────────────────────────┘
```

---

## 4. Diagnostics & Error Traceback Protocol

Avelyn implements a dual diagnostic and error reporting architecture:

### 4.1 Compile-Time Diagnostic Protocol
During compilation passes, the diagnostic engine produces formatted location indicators:

```text
tests/error_sample.lyn:14:9: ERROR: Cannot assign to immutable binding 'maxCount'
  | let maxCount = 100
  | maxCount = 200
  | ^^^^^^^^
  hint: declared as immutable with 'let' here: tests/error_sample.lyn:14:5
  fix:  change 'let' to 'var' to allow reassignment
```

### 4.2 Runtime Traceback Protocol
During dynamic interpretation, unhandled exceptions produce a Python-style call stack traceback:

```text
Traceback (most recent call last):
  File "math_pipeline.lyn", line 8, in computeRatio
    let ratio = numerator / denominator
  File "main.lyn", line 32, in <main>
    computeRatio(100, 0)
ZeroDivisionError: division by zero
```

---

## 5. Security & Isolation Model

The Avelyn runtime incorporates a multi-layer isolation model:
- **Default Mode**: Full system access for local tooling, scripting, and application execution.
- **Sandboxed Mode (`--sandbox`)**:
  - Restricts filesystem write and deletion operations.
  - Disables arbitrary dynamic library loading (`libloading` plugins).
  - Isolates external process execution (`std_process_spawn`).
  - Guards against runaway recursion with a configurable `call_depth` limit.

---

## 6. Verification & Conformance Standard

Avelyn's implementation is continuously validated against a comprehensive deterministic test suite located in [`Tests/`](Tests):
- **Lexical & Syntax Invariants**: Operator precedence, multi-line continuations, indentation boundaries.
- **Semantic & Scoping Rules**: Lexical block isolation, immutability guarantees, closures.
- **Compiler Parity**: Parallel validation ensuring both the Interpreter and Native Compiler produce identical normalized outputs for all deterministic algorithms.

Run the global verification suite:
```bash
# Run all tests sequentially line-by-line
python run_all_tests.py

# Run with dual-mode parity verification
python run_all_tests.py --mode dual
```
