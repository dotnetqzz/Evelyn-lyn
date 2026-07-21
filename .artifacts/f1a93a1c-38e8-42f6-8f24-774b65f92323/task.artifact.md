# Tasks - Phase 0: Stabilize the base

- `[ ]` Implement `ModuleManager` for multi-file imports
    - `[ ]` Create `avelyn/src/interpreter/module_manager.rs`
    - `[ ]` Integrate into `avelyn/src/interpreter/mod.rs`
- `[ ]` Implement Bytecode Loader and Verifier
    - `[ ]` Create `avelyn/src/compiler/loader.rs`
    - `[ ]` Create `avelyn/src/compiler/verifier.rs`
    - `[ ]` Expose modules in `avelyn/src/compiler/mod.rs`
- `[ ]` Update CLI in `avelyn/src/main.rs`
    - `[ ]` Use loader/verifier in `run-vm`
- `[ ]` Verification
    - `[ ]` Verify circular import protection
    - `[ ]` Verify bytecode version check
    - `[ ]` Run regression tests
