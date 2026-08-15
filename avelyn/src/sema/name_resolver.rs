// sema/name_resolver.rs — Symbol table and name resolution
//
// The name resolver walks the flat AST produced by the parser, builds a
// lexically-scoped symbol table, and annotates each name reference with the
// definition it resolves to.  Import resolution is delegated to the existing
// ModuleManager so the module system is preserved exactly.
//
// Design note: We produce a `SymbolTable` rather than rewriting the AST.
// AIRGen queries the table when it needs to know whether a name refers to a
// local variable, a function, a builtin, or an imported module member.

use std::collections::HashMap;
use crate::ast::{ASTNode, Span};
use super::diagnostics::DiagnosticEmitter;

// ─── Symbol kinds ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    /// A local variable declared with `let` (immutable) or `var`.
    Local { mutable: bool },
    /// A user-defined function at the top level or nested.
    Function { arity: usize },
    /// A native runtime builtin (resolved at runtime via the ABI).
    Builtin,
    /// An imported module binding.
    Module,
    /// A struct type declaration.
    StructType,
    /// An enum type declaration.
    EnumType,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name:   String,
    pub kind:   SymbolKind,
    pub span:   Span,
    /// Depth in the scope stack at which this symbol was defined.
    pub depth:  usize,
}

// ─── SymbolTable ─────────────────────────────────────────────────────────────

/// A flat map from resolved symbol name → Symbol, built by `NameResolver`.
/// Entries are keyed by `"<depth>:<name>"` to handle shadowing correctly.
#[derive(Debug, Default)]
pub struct SymbolTable {
    /// All resolved symbols in definition order.
    pub symbols: Vec<Symbol>,
    /// Fast name→symbol-index lookup for the current scope chain.
    /// After resolution this holds the final (innermost) binding for each name.
    pub resolved: HashMap<String, usize>,
}

impl SymbolTable {
    pub fn new() -> Self { SymbolTable::default() }

    pub fn define(&mut self, sym: Symbol) {
        let name = sym.name.clone();
        let idx  = self.symbols.len();
        self.symbols.push(sym);
        self.resolved.insert(name, idx);
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.resolved.get(name).map(|&i| &self.symbols[i])
    }

    pub fn is_function(&self, name: &str) -> bool {
        self.lookup(name).map(|s| matches!(s.kind, SymbolKind::Function { .. })).unwrap_or(false)
    }

    pub fn is_mutable(&self, name: &str) -> bool {
        self.lookup(name).map(|s| matches!(s.kind, SymbolKind::Local { mutable: true })).unwrap_or(true)
    }
}

// ─── NameResolver ─────────────────────────────────────────────────────────────

/// Known builtins that are always in scope (subset — extend as needed).
const KNOWN_BUILTINS: &[&str] = &[
    "print", "len", "range", "toString", "toNumber", "int", "float",
    "str", "bool", "type", "assert", "time", "input",
    "arrayAppend", "arrayPush", "arrayPop", "arraySlice", "arrayIndexOf",
    "arrayContains", "arrayRemove", "arrayLen",
    "stringLen", "stringConcat", "stringSplit", "stringSub",
    "stringUpper", "stringLower", "stringTrim", "stringReplace",
    "stringContains", "stringStartsWith", "stringEndsWith", "stringReverse",
    "mapGet", "mapSet", "mapHas", "mapKeys", "mapValues",
    "mathSqrt", "mathRound", "mathPow", "mathAbs", "mathFloor", "mathCeil",
    "random", "randint", "choice", "sha256", "md5", "sha1",
    "b64encode", "b64decode", "base64Encode", "base64Decode",
    "hexEncode", "hexDecode", "jsonStringify",
    "fileRead", "fileWrite", "numCpus", "timeSec", "dateNow",
    "spawnWorkers", "square", "double", "cube",
    "isNumber", "charFromCode", "charCodeAt", "tokenHex",
    "Queue", "Stack", "Set",
];

pub struct NameResolver<'a> {
    diag:        &'a mut DiagnosticEmitter,
    scopes:      Vec<HashMap<String, Symbol>>,
    pub table:   SymbolTable,
    current_depth: usize,
}

impl<'a> NameResolver<'a> {
    pub fn new(diag: &'a mut DiagnosticEmitter) -> Self {
        let mut r = NameResolver {
            diag,
            scopes: vec![HashMap::new()],
            table: SymbolTable::new(),
            current_depth: 0,
        };
        // Pre-populate builtins in the root scope.
        for &name in KNOWN_BUILTINS {
            r.define_sym(Symbol {
                name:  name.to_string(),
                kind:  SymbolKind::Builtin,
                span:  Span::UNKNOWN,
                depth: 0,
            });
        }
        r
    }

    fn push_scope(&mut self) {
        self.current_depth += 1;
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.current_depth -= 1;
        self.scopes.pop();
    }

    fn define_sym(&mut self, sym: Symbol) {
        let name = sym.name.clone();
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.clone(), sym.clone());
        }
        self.table.define(sym);
    }

    fn resolve_name(&self, name: &str) -> bool {
        for scope in self.scopes.iter().rev() {
            if scope.contains_key(name) { return true; }
        }
        false
    }

    /// Walk the top-level AST, registering top-level function and variable
    /// names in the first pass so forward calls work correctly.
    fn pre_register_top_level(&mut self, nodes: &[ASTNode]) {
        for node in nodes {
            match node {
                ASTNode::FuncDecl { name, params, .. } => {
                    self.define_sym(Symbol {
                        name:  name.clone(),
                        kind:  SymbolKind::Function { arity: params.len() },
                        span:  Span::UNKNOWN,
                        depth: 0,
                    });
                }
                ASTNode::Decl { name, mutable, .. } => {
                    self.define_sym(Symbol {
                        name:  name.clone(),
                        kind:  SymbolKind::Local { mutable: *mutable },
                        span:  Span::UNKNOWN,
                        depth: 0,
                    });
                }
                ASTNode::StructDecl { name, .. } => {
                    self.define_sym(Symbol {
                        name:  name.clone(),
                        kind:  SymbolKind::StructType,
                        span:  Span::UNKNOWN,
                        depth: 0,
                    });
                }
                ASTNode::EnumDecl { name, .. } => {
                    self.define_sym(Symbol {
                        name:  name.clone(),
                        kind:  SymbolKind::EnumType,
                        span:  Span::UNKNOWN,
                        depth: 0,
                    });
                }
                ASTNode::DestructureArray { names, mutable, .. } => {
                    for n in names.iter().flatten() {
                        self.define_sym(Symbol {
                            name:  n.clone(),
                            kind:  SymbolKind::Local { mutable: *mutable },
                            span:  Span::UNKNOWN,
                            depth: 0,
                        });
                    }
                }
                ASTNode::DestructureMap { keys, mutable, .. } => {
                    for (k, alias) in keys {
                        let name = alias.as_ref().unwrap_or(k);
                        self.define_sym(Symbol {
                            name:  name.clone(),
                            kind:  SymbolKind::Local { mutable: *mutable },
                            span:  Span::UNKNOWN,
                            depth: 0,
                        });
                    }
                }
                ASTNode::Line(_, inner) => {
                    // Unwrap Line wrappers and recurse for pre-registration
                    self.pre_register_top_level(std::slice::from_ref(inner.as_ref()));
                }
                _ => {}
            }
        }
    }

    /// Resolve all names in a program, emitting warnings for unknowns.
    pub fn resolve(&mut self, nodes: &[ASTNode]) {
        self.pre_register_top_level(nodes);
        for node in nodes {
            self.resolve_node(node);
        }
    }

    fn resolve_node(&mut self, node: &ASTNode) {
        match node {
            ASTNode::Var(name) => {
                if !self.resolve_name(name) {
                    // Unknown names are warnings (not errors) — the runtime
                    // handles late binding for imported modules, etc.
                    self.diag.warning(
                        Span::UNKNOWN,
                        format!("use of undeclared identifier '{}'", name),
                    );
                }
            }
            ASTNode::Decl { name, value, mutable, .. } => {
                self.resolve_node(value);
                self.define_sym(Symbol {
                    name:  name.clone(),
                    kind:  SymbolKind::Local { mutable: *mutable },
                    span:  Span::UNKNOWN,
                    depth: self.current_depth,
                });
            }
            ASTNode::DestructureArray { names, value, mutable } => {
                self.resolve_node(value);
                for n in names.iter().flatten() {
                    self.define_sym(Symbol {
                        name:  n.clone(),
                        kind:  SymbolKind::Local { mutable: *mutable },
                        span:  Span::UNKNOWN,
                        depth: self.current_depth,
                    });
                }
            }
            ASTNode::DestructureMap { keys, value, mutable } => {
                self.resolve_node(value);
                for (k, alias) in keys {
                    let name = alias.as_ref().unwrap_or(k);
                    self.define_sym(Symbol {
                        name:  name.clone(),
                        kind:  SymbolKind::Local { mutable: *mutable },
                        span:  Span::UNKNOWN,
                        depth: self.current_depth,
                    });
                }
            }
            ASTNode::Assign { name, value } => {
                if !self.resolve_name(name) {
                    self.diag.warning(Span::UNKNOWN,
                        format!("assignment to undeclared variable '{}'", name));
                }
                self.resolve_node(value);
            }
            ASTNode::FuncDecl { name, params, body, .. } => {
                // Register function name (may already be pre-registered)
                if !self.resolve_name(name) {
                    self.define_sym(Symbol {
                        name:  name.clone(),
                        kind:  SymbolKind::Function { arity: params.len() },
                        span:  Span::UNKNOWN,
                        depth: self.current_depth,
                    });
                }
                self.push_scope();
                for (pname, default) in params {
                    self.define_sym(Symbol {
                        name:  pname.clone(),
                        kind:  SymbolKind::Local { mutable: true },
                        span:  Span::UNKNOWN,
                        depth: self.current_depth,
                    });
                    if let Some(d) = default { self.resolve_node(d); }
                }
                for stmt in body { self.resolve_node(stmt); }
                self.pop_scope();
            }
            ASTNode::Lambda { params, body, .. } => {
                self.push_scope();
                for (pname, default) in params {
                    self.define_sym(Symbol {
                        name:  pname.clone(),
                        kind:  SymbolKind::Local { mutable: true },
                        span:  Span::UNKNOWN,
                        depth: self.current_depth,
                    });
                    if let Some(d) = default { self.resolve_node(d); }
                }
                for stmt in body { self.resolve_node(stmt); }
                self.pop_scope();
            }
            ASTNode::FuncCall { name, args } => {
                if !self.resolve_name(name) {
                    // Could be an interpreter builtin — emit note, not error.
                    self.diag.note(Span::UNKNOWN,
                        format!("call to unknown function '{}' — will resolve at runtime", name));
                }
                for a in args { self.resolve_node(a); }
            }
            ASTNode::CallExpr { callee, args } => {
                self.resolve_node(callee);
                for a in args { self.resolve_node(a); }
            }
            ASTNode::If { cond, then, els } => {
                self.resolve_node(cond);
                self.push_scope();
                for s in then { self.resolve_node(s); }
                self.pop_scope();
                if let Some(e) = els {
                    self.push_scope();
                    for s in e { self.resolve_node(s); }
                    self.pop_scope();
                }
            }
            ASTNode::While { cond, body } => {
                self.resolve_node(cond);
                self.push_scope();
                for s in body { self.resolve_node(s); }
                self.pop_scope();
            }
            ASTNode::For { var, iter, body } => {
                self.resolve_node(iter);
                self.push_scope();
                self.define_sym(Symbol {
                    name:  var.clone(),
                    kind:  SymbolKind::Local { mutable: true },
                    span:  Span::UNKNOWN,
                    depth: self.current_depth,
                });
                for s in body { self.resolve_node(s); }
                self.pop_scope();
            }
            ASTNode::ForRange { var, from, to, body, .. } => {
                self.resolve_node(from);
                self.resolve_node(to);
                self.push_scope();
                self.define_sym(Symbol {
                    name:  var.clone(),
                    kind:  SymbolKind::Local { mutable: true },
                    span:  Span::UNKNOWN,
                    depth: self.current_depth,
                });
                for s in body { self.resolve_node(s); }
                self.pop_scope();
            }
            ASTNode::BinOp { left, right, .. } => {
                self.resolve_node(left);
                self.resolve_node(right);
            }
            ASTNode::UnaryOp { operand, .. } => { self.resolve_node(operand); }
            ASTNode::Return(e)   => { self.resolve_node(e); }
            ASTNode::Throw(e)    => { self.resolve_node(e); }
            ASTNode::PrintCall(e) => { self.resolve_node(e); }
            ASTNode::Spread(e)   => { self.resolve_node(e); }
            ASTNode::Export(e)   => { self.resolve_node(e); }
            ASTNode::Assert { cond, msg } => {
                self.resolve_node(cond);
                if let Some(m) = msg { self.resolve_node(m); }
            }
            ASTNode::Ternary { cond, then, els } => {
                self.resolve_node(cond);
                self.resolve_node(then);
                self.resolve_node(els);
            }
            ASTNode::NullCoalesce { left, right } => {
                self.resolve_node(left);
                self.resolve_node(right);
            }
            ASTNode::Subscript { target, index } => {
                self.resolve_node(target);
                self.resolve_node(index);
            }
            ASTNode::IndexAssign { target: _, index, value } => {
                self.resolve_node(index);
                self.resolve_node(value);
            }
            ASTNode::CompoundAssign { name, value, .. } => {
                if !self.resolve_name(name) {
                    self.diag.warning(Span::UNKNOWN,
                        format!("compound assignment to undeclared '{}'", name));
                }
                self.resolve_node(value);
            }
            ASTNode::ArrayLit(items) => {
                for i in items { self.resolve_node(i); }
            }
            ASTNode::MapLit(pairs) => {
                for (k, v) in pairs { self.resolve_node(k); self.resolve_node(v); }
            }
            ASTNode::InterpStr(parts) => {
                for p in parts { self.resolve_node(p); }
            }
            ASTNode::TryCatch { body, catches, finally_body } => {
                self.push_scope();
                for s in body { self.resolve_node(s); }
                self.pop_scope();
                for (_, var, stmts) in catches {
                    self.push_scope();
                    self.define_sym(Symbol {
                        name:  var.clone(),
                        kind:  SymbolKind::Local { mutable: true },
                        span:  Span::UNKNOWN,
                        depth: self.current_depth,
                    });
                    for s in stmts { self.resolve_node(s); }
                    self.pop_scope();
                }
                if let Some(fin) = finally_body {
                    for s in fin { self.resolve_node(s); }
                }
            }
            ASTNode::Switch { subject, cases } => {
                self.resolve_node(subject);
                for (cond, body) in cases {
                    if let Some(c) = cond { self.resolve_node(c); }
                    for s in body { self.resolve_node(s); }
                }
            }
            ASTNode::Match { subject, arms } => {
                self.resolve_node(subject);
                for (_, body) in arms {
                    for s in body { self.resolve_node(s); }
                }
            }
            ASTNode::DestructureArray { value, .. } => { self.resolve_node(value); }
            ASTNode::DestructureMap  { value, .. } => { self.resolve_node(value); }
            ASTNode::NamedArg { value, .. } => { self.resolve_node(value); }
            ASTNode::Line(line, inner) => {
                // Propagate line info — AIRGen will pick up the span.
                let _ = line;
                self.resolve_node(inner);
            }
            // Terminals with no sub-nodes
            ASTNode::Int(_) | ASTNode::Float(_) | ASTNode::Str(_)
            | ASTNode::Bool(_) | ASTNode::Null | ASTNode::ByteArray(_)
            | ASTNode::Break | ASTNode::Continue | ASTNode::Pass
            | ASTNode::TimeCall | ASTNode::Import(_) | ASTNode::Include(_)
            | ASTNode::StructDecl { .. } | ASTNode::EnumDecl { .. } => {}
        }
    }
}
