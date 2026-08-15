# Avelyn Compiler Architecture

This document describes the modern compiler pipeline for the **Avelyn** programming language.

The architecture takes strong architectural inspiration from modern multi-stage compilers (such as Swift's SIL pipeline), separating high-level language semantics, intermediate representation, optimization passes, and low-level code generation into clearly defined, modular stages.

---

## 1. High-Level Pipeline Overview

```
                          ┌────────────────────────┐
                          │   Avelyn Source Code   │ (.lyn)
                          └───────────┬────────────┘
                                      │
                                      ▼
                          ┌────────────────────────┐
                          │  Lexer (Indentation)   │ (lexer.rs)
                          └───────────┬────────────┘
                                      │ Tokens
                                      ▼
                          ┌────────────────────────┐
                          │  Recursive Parser      │ (parser.rs)
                          └───────────┬────────────┘
                                      │ ASTNode
                                      ▼
                          ┌────────────────────────┐
                          │ Semantic Analysis      │ (src/sema/)
                          │  - Diagnostics         │
                          │  - Name Resolution     │
                          │  - Type Checking       │
                          └───────────┬────────────┘
                                      │ TypedNode / Span
                                      ▼
                          ┌────────────────────────┐
                          │ AIRGen (AST Lowering)  │ (src/airgen/)
                          └───────────┬────────────┘
                                      │ Unoptimized AIR
                                      ▼
                          ┌────────────────────────┐
                          │ AIR Verifier           │ (src/air/verify.rs)
                          └───────────┬────────────┘
                                      │ Validated AIR
                                      ▼
                          ┌────────────────────────┐
                          │ AIR Optimization       │ (src/optimizer/)
                          │  - Const Fold / Prop   │
                          │  - DCE / Redundant     │
                          │  - Unreachable / BB    │
                          └───────────┬────────────┘
                                      │ Optimized AIR
                                      ▼
                          ┌────────────────────────┐
                          │ LLVM IRGen             │ (src/irgen/)
                          └───────────┬────────────┘
                                      │ LLVM IR (.ll)
                                      ▼
                          ┌────────────────────────┐
                          │ Target / Toolchain     │ (src/target/)
                          │ (Clang / MSVC / SDK)   │
                          └───────────┬────────────┘
                                      │
                                      ▼
                          ┌────────────────────────┐
                          │ Native Binary (.exe)   │
                          └────────────────────────┘
```

---

## 2. Compiler Pipeline Stages

### Stage 1: Lexical Analysis & Parsing (`src/lexer.rs`, `src/parser.rs`)
- **Lexer**: Handles Python-like whitespace/indentation (`INDENT` / `DEDENT`), string interpolation tokens, comments, operators, and literals.
- **Parser**: Recursive-descent parser producing an AST (`ASTNode`). Supports pattern matching, functions, expressions, destructuring, classes/structs, and exception handling.

### Stage 2: Semantic Analysis (`src/sema/`)
- **`diagnostics.rs`**: Standardized compiler diagnostics with severity levels (`Note`, `Warning`, `Error`, `Ice`), file-id tracking, source line pointers, and fix hints.
- **`name_resolver.rs`**: Builds lexically-scoped symbol tables to validate variable and function declarations, track immutability (`let` vs `var`), and resolve builtins.
- **`type_checker.rs`**: Annotates AST nodes with static `AvelynType` annotations (scalars, strings, heap containers, function signatures, `Any`).

### Stage 3: Avelyn Intermediate Representation (AIR) (`src/air/`)
AIR is an explicit SSA-like intermediate representation positioned between typed AST and LLVM IR:
- **`Value`**: Explicit SSA register identifiers (`%v0`, `%v1`, etc.).
- **`BlockId` & `BasicBlock`**: Explicit control flow graphs with predecessor and successor tracking.
- **`Inst`**: Flat, linear instruction set:
  - Constants (`ConstInt`, `ConstFloat`, `ConstBool`, `ConstStr`, `ConstNull`)
  - Memory operations (`Alloc`, `Load`, `Store`)
  - Explicit Runtime ABI invocations (`RuntimeCall(RuntimeFn, args)`)
  - First-class function calls (`Call(name, args)`)
  - CFG branches (`Branch`, `Jump`, `Return`, `Unreachable`)
  - Scalar arithmetic (`IAdd`, `ISub`, `IMul`, `ICmpEq`, `ICmpSlt`, `ICmpSle`)
  - Ownership primitives (`Retain`, `Release`, `Move`, `Copy`)
- **`verify.rs`**: Validates IR invariants (entry block rules, single terminators, defined-before-use value ordering, valid jump targets).
- **`printer.rs`**: Formats AIR into human-readable text for inspection.

### Stage 4: AIRGen (`src/airgen/`)
- Lowers AST nodes into SSA AIR instructions.
- Uses `AirBuilder` to handle lexical scopes, loop break/continue stacks, and stack allocation hoisting to the entry block.
- Uses `runtime_map.rs` as the single source of truth for runtime C-ABI function signatures.

### Stage 5: Optimization Pipeline (`src/optimizer/`)
Configurable optimization passes that run to a fixed point:
- **`const_fold.rs`**: Evaluates constant arithmetic and comparison operations at compile time.
- **`const_prop.rs`**: Propagates constants through local load/store slots.
- **`dce.rs`**: Dead code elimination for unused, side-effect-free instructions.
- **`unreachable_elim.rs`**: BFS reachability pass removing dead basic blocks.
- **`bb_simplify.rs`**: Merges straight-line basic blocks and removes jump trampolines.
- **`redundant_elim.rs`**: Deduplicates redundant allocas and debug markers.

| Level | Passes Executed |
|:-----:|:----------------|
| `-O0` | No optimization passes |
| `-O1` | `RedundantElim`, `DeadCodeElim`, `UnreachableElim` |
| `-O2` | `-O1` + `ConstFold`, `ConstProp`, `BbSimplify` |
| `-O3` | `-O2` (expanded inlining & loop optimization) |

### Stage 6: LLVM IRGen (`src/irgen/`)
- Translates verified and optimized `AirModule` into standard LLVM IR text.
- Fully decoupled from language semantics — only understands AIR instructions and target data layouts.
- Emits C-ABI declarations for the runtime engine (`sylvel_runtime.c`).

### Stage 7: Target Abstraction & Toolchain (`src/target/`)
- Target description including OS, architecture, ABI environment, triple, and data layout.
- **`windows_x64.rs`**: First-class Windows `x86_64-pc-windows-msvc` target support.
- **`probe_clang`**: Robust toolchain discovery looking up `AVELYN_LLVM_PATH`, `--llvm-path`, standard installation directories, and system PATH.

---

## 3. CLI Driver Usage

The `avelyn` CLI driver exposes granular flags to inspect and control every stage of the pipeline:

```bash
# Run via tree-walking interpreter
avelyn script.lyn

# Compile to native binary (.exe)
avelyn compile script.lyn -o app.exe

# Inspect the parsed AST
avelyn compile script.lyn --emit-ast

# Inspect unoptimized AIR
avelyn compile script.lyn --emit-air

# Inspect optimized AIR (-O2 by default)
avelyn compile script.lyn --emit-air-opt

# Emit LLVM IR (.ll)
avelyn compile script.lyn --emit-llvm -O3

# Target and toolchain customization
avelyn compile script.lyn --target x86_64-pc-windows-msvc --llvm-path "C:\Program Files\LLVM\bin"

# Run compiler unit & integration tests
cargo test
avelyn test Tests
```
