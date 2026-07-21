# Implementation Plan - Phase 0: Stabilize the Base

This plan covers the implementation of foundational systems for the Avelyn language, specifically robust multi-file imports, a bytecode verifier, and version checking for compiled files.

## User Review Required

> [!IMPORTANT]
> The current interpreter evaluates imported files in the *global* environment. This means all variables declared in an imported file become globals. While Phase 2 will introduce explicit exports/requires for isolation, this Phase 0 implementation will focus on reliable loading and caching.

## Proposed Changes

### Module & Import System

Enhance the import mechanism to support caching and prevent circular dependencies.

#### [NEW] [module_manager.rs](file:///K:/Evelyn-lyn/avelyn/src/interpreter/module_manager.rs)
- Create a `ModuleManager` struct to track loaded files and their results.
- Implement path resolution logic (relative, stdlib, embedded).

#### [MODIFY] [mod.rs](file:///K:/Evelyn-lyn/avelyn/src/interpreter/mod.rs)
- Integrate `ModuleManager` into the `Interpreter` struct.
- Update `ASTNode::Import` handling to use the manager.

---

### Bytecode Security & Integrity

Implement verification to ensure bytecode files are valid and safe to execute.

#### [NEW] [verifier.rs](file:///K:/Evelyn-lyn/avelyn/src/compiler/verifier.rs)
- Implement `BytecodeVerifier` to validate `ModuleState`.
- Checks:
    - Opcodes are within the defined `Opcode` range.
    - Constant pool indices are valid.
    - Native function indices exist in the native table.
    - Jump targets are within the code bounds of the function.

#### [NEW] [loader.rs](file:///K:/Evelyn-lyn/avelyn/src/compiler/loader.rs)
- Implement `BytecodeLoader` to deserialize `.lync` files.
- Perform magic byte (`SYL\0`) and version (`1.0`) checks.

---

### CLI Integration

#### [MODIFY] [main.rs](file:///K:/Evelyn-lyn/avelyn/src/main.rs)
- Update `run_vm_file` to use the `BytecodeLoader` and `BytecodeVerifier` before execution.
- Improve error reporting for version mismatches or corrupted bytecode.

## Verification Plan

### Automated Tests
- Create a test script that performs a circular import and verifies it fails gracefully.
- Create a test script that imports the same file twice and verifies it's only evaluated once (e.g., via a print side-effect).
- Manually corrupt a `.lync` file (change magic bytes or version) and verify the loader rejects it.

### Manual Verification
- Run existing `Tests/*.lyn` files to ensure no regressions in basic functionality.
- Use `avelyn compile` and `avelyn run-vm` on a multi-file project.
