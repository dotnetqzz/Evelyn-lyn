// ast.rs — Token and ASTNode definitions
// Ported from CoreInterpreter/AST.swift
//
// ─── Compiler pipeline additions ──────────────────────────────────────────────
// The types below (Span, AvelynType, TypedNode) are used exclusively by the
// compiler backend (Sema → AIRGen → Optimizer → IRGen).  The interpreter,
// parser, lexer, and all stdlib code are completely unaffected.

// ─── Source location ─────────────────────────────────────────────────────────

/// A compact, copy-able source location.  `file_id` indexes into a file-name
/// table held by the compiler driver; 0 = unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub file_id: u32,
    pub line: u32,
    pub col: u32,
}

impl Span {
    pub const UNKNOWN: Span = Span { file_id: 0, line: 0, col: 0 };

    pub fn new(file_id: u32, line: u32, col: u32) -> Self {
        Span { file_id, line, col }
    }

    pub fn from_line(line: u32) -> Self {
        Span { file_id: 0, line, col: 0 }
    }

    pub fn is_known(&self) -> bool {
        self.line > 0
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.col > 0 {
            write!(f, "{}:{}", self.line, self.col)
        } else {
            write!(f, "line {}", self.line)
        }
    }
}

// ─── Type system (compiler-only) ──────────────────────────────────────────────

/// Avelyn's static type annotation used during semantic analysis.
/// Initially conservative: most expressions resolve to `Any` until the type
/// checker is made more precise in future phases.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AvelynType {
    /// Type is not yet known (resolved by type-checker, defaults to Any).
    Unknown,
    /// Any dynamic value — the fallback for un-typed positions.
    Any,
    Null,
    Bool,
    Int,
    Float,
    Str,
    ByteArray,
    /// Homogeneous list (element type may be Any).
    List(Box<AvelynType>),
    /// String-keyed map (value type may be Any).
    Map(Box<AvelynType>),
    /// Function: (param types, return type)
    Func(Vec<AvelynType>, Box<AvelynType>),
    /// Named struct / enum variant.
    Named(String),
    /// Internal: a type that can never be produced (bottom type, after error).
    Never,
}

impl AvelynType {
    /// Returns true if values of this type are guaranteed to be scalars
    /// (no heap allocation needed in the runtime representation).
    pub fn is_scalar(&self) -> bool {
        matches!(self, AvelynType::Null | AvelynType::Bool | AvelynType::Int | AvelynType::Float)
    }

    /// Returns true if the type is definitely reference-counted at runtime.
    pub fn is_heap(&self) -> bool {
        matches!(self, AvelynType::Str | AvelynType::ByteArray
            | AvelynType::List(_) | AvelynType::Map(_)
            | AvelynType::Func(..) | AvelynType::Named(_))
    }

    pub fn is_unknown(&self) -> bool { matches!(self, AvelynType::Unknown) }
    pub fn is_any(&self) -> bool     { matches!(self, AvelynType::Any) }
}

impl std::fmt::Display for AvelynType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AvelynType::Unknown => write!(f, "?"),
            AvelynType::Any     => write!(f, "any"),
            AvelynType::Null    => write!(f, "null"),
            AvelynType::Bool    => write!(f, "bool"),
            AvelynType::Int     => write!(f, "int"),
            AvelynType::Float   => write!(f, "float"),
            AvelynType::Str     => write!(f, "str"),
            AvelynType::ByteArray => write!(f, "bytes"),
            AvelynType::List(t) => write!(f, "list[{}]", t),
            AvelynType::Map(t)  => write!(f, "map[{}]", t),
            AvelynType::Func(ps, r) => {
                let ps: Vec<String> = ps.iter().map(|t| t.to_string()).collect();
                write!(f, "fn({}) -> {}", ps.join(", "), r)
            }
            AvelynType::Named(n) => write!(f, "{}", n),
            AvelynType::Never   => write!(f, "never"),
        }
    }
}

// ─── Typed AST node (compiler-only) ───────────────────────────────────────────

/// A wrapper around `ASTNode` that carries the inferred type and source span.
/// Used by the compiler backend; never constructed by the parser/interpreter.
#[derive(Debug, Clone)]
pub struct TypedNode {
    pub node: ASTNode,
    pub ty:   AvelynType,
    pub span: Span,
}

impl TypedNode {
    pub fn new(node: ASTNode, ty: AvelynType, span: Span) -> Self {
        TypedNode { node, ty, span }
    }

    /// Wrap a node with unknown type and unknown span (to be filled by Sema).
    pub fn untyped(node: ASTNode) -> Self {
        TypedNode { node, ty: AvelynType::Unknown, span: Span::UNKNOWN }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    pub fn with_type(mut self, ty: AvelynType) -> Self {
        self.ty = ty;
        self
    }
}

// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Let, Var, Print, While, For, In, If, Else, Elif,
    Def, Return, Break, Continue, Pass, True, False, Null,
    Import, Try, Catch, Throw, Finally, Switch, Match,
    Case, Default, Not, And, Or, As, Assert,
    Struct, Enum, Export,
    // Literals
    Ident(String),
    Int(i64),
    Float(f64),
    Str(String),
    ByteStr(Vec<u8>),
    // Operators
    Eq, Plus, Minus, Star, Slash, Percent, Lt, Gt,
    Comma, Bang, Dot, Colon, Semi, At,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    // Bitwise
    Amp, Pipe, Caret, Tilde, Ltlt, Gtgt, Gtgtgt,
    AmpEq, PipeEq, CaretEq, LtltEq, GtgtEq,
    // Pipe arrow
    PipeArrow,
    // Multi-char
    EqEq, BangEq, LtEq, GtEq, AndAnd, OrOr,
    DotDot, DotDotDot,
    PlusEq, MinusEq, StarEq, SlashEq, PercentEq,
    StarStar, SlashSlash,
    Arrow, // =>
    // Null coalescing / ternary
    QQ,    // ??
    Quest, // ?
    // Indentation
    Indent, Dedent,
    Eof,
}

pub type Param = (String, Option<Box<ASTNode>>);

#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard,
    Literal(ASTNode),
    Var(String),
    Struct { name: String, fields: Vec<(String, Pattern)> },
    Enum { type_name: String, variant: String, args: Vec<Pattern> },
    List(Vec<Pattern>),
    #[allow(dead_code)]
    Or(Vec<Pattern>),
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<String>, // if any, for named fields in variant
    pub arity: usize,        // for positional
}

#[derive(Debug, Clone)]
pub enum ASTNode {
    // Literals
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Null,
    ByteArray(Vec<u8>),
    // Collections
    Var(String),
    ArrayLit(Vec<ASTNode>),
    MapLit(Vec<(ASTNode, ASTNode)>),
    InterpStr(Vec<ASTNode>),
    // Declarations
    Decl { name: String, value: Box<ASTNode>, mutable: bool, annotations: Vec<ASTNode> },
    StructDecl { name: String, fields: Vec<String>, annotations: Vec<ASTNode> },
    EnumDecl { name: String, variants: Vec<EnumVariant>, annotations: Vec<ASTNode> },
    Assign { name: String, value: Box<ASTNode> },
    CompoundAssign { name: String, op: String, value: Box<ASTNode> },
    IndexAssign { target: String, index: Box<ASTNode>, value: Box<ASTNode> },
    DestructureArray { names: Vec<Option<String>>, value: Box<ASTNode>, mutable: bool },
    DestructureMap { keys: Vec<(String, Option<String>)>, value: Box<ASTNode>, mutable: bool },
    // Expressions
    BinOp { left: Box<ASTNode>, op: String, right: Box<ASTNode> },
    UnaryOp { op: String, operand: Box<ASTNode> },
    Subscript { target: Box<ASTNode>, index: Box<ASTNode> },
    Ternary { cond: Box<ASTNode>, then: Box<ASTNode>, els: Box<ASTNode> },
    NullCoalesce { left: Box<ASTNode>, right: Box<ASTNode> },
    Spread(Box<ASTNode>),
    NamedArg { name: String, value: Box<ASTNode> },
    // Functions
    FuncDecl { name: String, params: Vec<Param>, body: Vec<ASTNode>, variadic: bool, annotations: Vec<ASTNode> },
    FuncCall { name: String, args: Vec<ASTNode> },
    Lambda { params: Vec<Param>, body: Vec<ASTNode>, variadic: bool, annotations: Vec<ASTNode> },
    CallExpr { callee: Box<ASTNode>, args: Vec<ASTNode> },
    // Builtins
    PrintCall(Box<ASTNode>),
    TimeCall,
    // Control flow
    While { cond: Box<ASTNode>, body: Vec<ASTNode> },
    For { var: String, iter: Box<ASTNode>, body: Vec<ASTNode> },
    ForRange { var: String, from: Box<ASTNode>, to: Box<ASTNode>, inclusive: bool, body: Vec<ASTNode> },
    If { cond: Box<ASTNode>, then: Vec<ASTNode>, els: Option<Vec<ASTNode>> },
    Switch { subject: Box<ASTNode>, cases: Vec<(Option<ASTNode>, Vec<ASTNode>)> },
    Match { subject: Box<ASTNode>, arms: Vec<(Pattern, Vec<ASTNode>)> },
    Return(Box<ASTNode>),
    Break,
    Continue,
    Pass,
    Throw(Box<ASTNode>),
    TryCatch {
        body: Vec<ASTNode>,
        catches: Vec<(Option<String>, String, Vec<ASTNode>)>, // (type_filter, var_name, body)
        finally_body: Option<Vec<ASTNode>>,
    },
    Assert { cond: Box<ASTNode>, msg: Option<Box<ASTNode>> },
    Import(String),
    Include(String),
    Export(Box<ASTNode>),
    Line(u32, Box<ASTNode>),
}
 
impl ASTNode {
    pub fn to_string_key(&self) -> String {
        match self {
            ASTNode::Str(s) => s.clone(),
            ASTNode::Var(s) => s.clone(),
            ASTNode::FuncCall { name, .. } => name.clone(),
            ASTNode::Line(_, inner) => inner.to_string_key(),
            _ => format!("{:?}", self),
        }
    }
}
