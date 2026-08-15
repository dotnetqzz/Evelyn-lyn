#![allow(dead_code, unused_imports)]
// sema/mod.rs — Semantic analysis stage entry point

pub mod diagnostics;
pub mod name_resolver;
pub mod type_checker;

pub use diagnostics::{Diagnostic, DiagnosticEmitter, Severity};
pub use name_resolver::{NameResolver, Symbol, SymbolKind, SymbolTable};
pub use type_checker::{SemaContext, TypeChecker};
