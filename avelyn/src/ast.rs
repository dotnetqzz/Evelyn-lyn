// ast.rs — Token and ASTNode definitions
// Ported from CoreInterpreter/AST.swift

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
    Indent, Dedent, Newline,
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
}

impl ASTNode {
    pub fn to_string_key(&self) -> String {
        match self {
            ASTNode::Str(s) => s.clone(),
            ASTNode::Var(s) => s.clone(),
            ASTNode::FuncCall { name, .. } => name.clone(),
            _ => format!("{:?}", self),
        }
    }
}
