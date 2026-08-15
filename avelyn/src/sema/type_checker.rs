// sema/type_checker.rs — Shallow type inference for the compiler pipeline
//
// The type checker annotates every AST node with an `AvelynType`.  In this
// initial implementation it is deliberately conservative:
//   • Integer / float / bool / string literals → concrete types
//   • All other expressions → AvelynType::Any
//   • Variables → type of their initializer, or Any if unknown
//
// This is enough for AIRGen to emit better-typed AIR instructions for scalars
// and to avoid redundant allocations.  A more precise type system (generics,
// optional typing, struct field typing) can be layered in later phases.

use std::collections::HashMap;
use crate::ast::{ASTNode, AvelynType, Span, TypedNode};
use super::diagnostics::DiagnosticEmitter;
use super::name_resolver::SymbolTable;

pub struct TypeChecker<'a> {
    diag:    &'a mut DiagnosticEmitter,
    /// Map from variable name → inferred AvelynType.
    env:     Vec<HashMap<String, AvelynType>>,
    #[allow(dead_code)]
    symbols: &'a SymbolTable,
}

impl<'a> TypeChecker<'a> {
    pub fn new(diag: &'a mut DiagnosticEmitter, symbols: &'a SymbolTable) -> Self {
        TypeChecker {
            diag,
            env: vec![HashMap::new()],
            symbols,
        }
    }

    fn push_env(&mut self) { self.env.push(HashMap::new()); }
    fn pop_env (&mut self) { self.env.pop(); }

    fn set_var(&mut self, name: &str, ty: AvelynType) {
        if let Some(top) = self.env.last_mut() {
            top.insert(name.to_string(), ty);
        }
    }

    fn lookup_var(&self, name: &str) -> AvelynType {
        for scope in self.env.iter().rev() {
            if let Some(t) = scope.get(name) { return t.clone(); }
        }
        AvelynType::Any   // conservative fallback
    }

    /// Infer the type of a single node (does not recurse into statements).
    pub fn infer(&mut self, node: &ASTNode) -> AvelynType {
        match node {
            ASTNode::Int(_)   => AvelynType::Int,
            ASTNode::Float(_) => AvelynType::Float,
            ASTNode::Bool(_)  => AvelynType::Bool,
            ASTNode::Null     => AvelynType::Null,
            ASTNode::Str(_)   => AvelynType::Str,
            ASTNode::ByteArray(_) => AvelynType::ByteArray,

            ASTNode::Var(name) => self.lookup_var(name),

            ASTNode::BinOp { left, op, right } => {
                let lt = self.infer(left);
                let rt = self.infer(right);
                self.infer_binop(op, &lt, &rt)
            }

            ASTNode::UnaryOp { op, operand } => {
                let ot = self.infer(operand);
                match op.as_str() {
                    "-" if ot == AvelynType::Int   => AvelynType::Int,
                    "-" if ot == AvelynType::Float => AvelynType::Float,
                    "!" | "not" => AvelynType::Bool,
                    _ => AvelynType::Any,
                }
            }

            ASTNode::Ternary { cond: _, then, els } => {
                let tt = self.infer(then);
                let et = self.infer(els);
                if tt == et { tt } else { AvelynType::Any }
            }

            ASTNode::NullCoalesce { left, right } => {
                let lt = self.infer(left);
                let rt = self.infer(right);
                if lt == AvelynType::Null { rt } else { lt }
            }

            ASTNode::ArrayLit(_)  => AvelynType::List(Box::new(AvelynType::Any)),
            ASTNode::MapLit(_)    => AvelynType::Map(Box::new(AvelynType::Any)),
            ASTNode::InterpStr(_) => AvelynType::Str,

            ASTNode::FuncCall { name, .. } | ASTNode::FuncDecl { name, .. } => {
                // For now, function calls return Any unless it's a known pure builtin.
                match name.as_str() {
                    "len" | "arrayLen" | "stringLen" => AvelynType::Int,
                    "toNumber" | "int" | "float"     => AvelynType::Float,
                    "toString" | "str"               => AvelynType::Str,
                    _ => AvelynType::Any,
                }
            }

            ASTNode::Lambda { .. } | ASTNode::CallExpr { .. } => AvelynType::Any,

            ASTNode::Subscript { target, .. } => {
                let tt = self.infer(target);
                match &tt {
                    AvelynType::List(elem) => *elem.clone(),
                    AvelynType::Str        => AvelynType::Str,
                    _                      => AvelynType::Any,
                }
            }

            ASTNode::Line(_, inner) => self.infer(inner),

            // Statements evaluated as expressions produce Null.
            ASTNode::While { .. } | ASTNode::For { .. } | ASTNode::ForRange { .. }
            | ASTNode::If { .. } | ASTNode::PrintCall(_) | ASTNode::Pass
            | ASTNode::Break | ASTNode::Continue => AvelynType::Null,

            ASTNode::Return(_) | ASTNode::Throw(_) => AvelynType::Never,

            _ => AvelynType::Any,
        }
    }

    /// Infer the result type of a binary operation given concrete operand types.
    fn infer_binop(&self, op: &str, lt: &AvelynType, rt: &AvelynType) -> AvelynType {
        // Comparison and logical ops always produce Bool.
        match op {
            "==" | "!=" | "<" | "<=" | ">" | ">=" | "and" | "or" | "&&" | "||" => {
                return AvelynType::Bool;
            }
            _ => {}
        }
        // Arithmetic: preserve precision.
        match op {
            "+" | "-" | "*" | "/" | "%" | "**" => {
                match (lt, rt) {
                    (AvelynType::Int,   AvelynType::Int)   => AvelynType::Int,
                    (AvelynType::Float, _)
                    | (_, AvelynType::Float)               => AvelynType::Float,
                    (AvelynType::Str,   AvelynType::Str) if op == "+" => AvelynType::Str,
                    _ => AvelynType::Any,
                }
            }
            // Bitwise
            "&" | "|" | "^" | "<<" | ">>" | ">>>" => AvelynType::Int,
            _ => AvelynType::Any,
        }
    }

    /// Walk a statement list, updating the type environment and returning
    /// the list of `TypedNode` wrappers.
    pub fn check_stmts(&mut self, nodes: &[ASTNode]) -> Vec<TypedNode> {
        let mut out = Vec::with_capacity(nodes.len());
        for node in nodes {
            let tn = self.check_node(node);
            out.push(tn);
        }
        out
    }

    /// Infer + annotate a single node recursively.
    pub fn check_node(&mut self, node: &ASTNode) -> TypedNode {
        let span = match node {
            ASTNode::Line(line, _) => Span::from_line(*line),
            _ => Span::UNKNOWN,
        };
        let ty = self.infer(node);

        // For declarations, update the type environment.
        match node {
            ASTNode::Decl { name, value, .. } => {
                let vt = self.infer(value);
                self.set_var(name, vt);
            }
            ASTNode::FuncDecl { name, .. } => {
                // Mark the function as callable (returns Any for now).
                self.set_var(name, AvelynType::Func(vec![], Box::new(AvelynType::Any)));
            }
            ASTNode::For { var, .. } | ASTNode::ForRange { var, .. } => {
                self.set_var(var, AvelynType::Any);
            }
            _ => {}
        }

        // Recurse into scoped blocks.
        match node {
            ASTNode::If { cond: _, then, els } => {
                self.push_env();
                for s in then { self.check_node(s); }
                self.pop_env();
                if let Some(e) = els {
                    self.push_env();
                    for s in e { self.check_node(s); }
                    self.pop_env();
                }
            }
            ASTNode::While { body, .. } | ASTNode::For { body, .. }
            | ASTNode::ForRange { body, .. } => {
                self.push_env();
                for s in body { self.check_node(s); }
                self.pop_env();
            }
            ASTNode::FuncDecl { body, .. } | ASTNode::Lambda { body, .. } => {
                self.push_env();
                for s in body { self.check_node(s); }
                self.pop_env();
            }
            ASTNode::TryCatch { body, catches, finally_body } => {
                self.push_env();
                for s in body { self.check_node(s); }
                self.pop_env();
                for (_, var, stmts) in catches {
                    self.push_env();
                    self.set_var(var, AvelynType::Any);
                    for s in stmts { self.check_node(s); }
                    self.pop_env();
                }
                if let Some(fin) = finally_body {
                    for s in fin { self.check_node(s); }
                }
            }
            _ => {}
        }

        TypedNode::new(node.clone(), ty, span)
    }

    /// Full type-check pass: resolve types for all top-level nodes.
    pub fn check_program(&mut self, nodes: &[ASTNode]) -> Vec<TypedNode> {
        // Pre-declare top-level function types so forward calls type-check.
        for node in nodes {
            if let ASTNode::FuncDecl { name, params, .. } = node {
                let param_tys = vec![AvelynType::Any; params.len()];
                self.set_var(name, AvelynType::Func(param_tys, Box::new(AvelynType::Any)));
            }
        }
        self.check_stmts(nodes)
    }
}

// ─── SemaContext (public entry-point) ─────────────────────────────────────────

/// Convenience wrapper that runs name resolution + type checking in sequence
/// and returns typed nodes alongside any diagnostics.
pub struct SemaContext {
    pub diag: DiagnosticEmitter,
}

impl SemaContext {
    pub fn new() -> Self {
        SemaContext { diag: DiagnosticEmitter::new() }
    }

    /// Run the full semantic analysis pass on a parsed program.
    pub fn analyse(&mut self, nodes: &[ASTNode]) -> Vec<TypedNode> {
        use super::name_resolver::NameResolver;
        let mut resolver = NameResolver::new(&mut self.diag);
        resolver.resolve(nodes);
        let table = resolver.table;

        let mut checker = TypeChecker::new(&mut self.diag, &table);
        checker.check_program(nodes)
    }
}
