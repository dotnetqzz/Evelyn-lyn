// parser.rs — Recursive descent parser
// Ported from CoreInterpreter/Parser.swift

use crate::ast::{ASTNode, Param, Token};

pub struct Parser {
    tokens: Vec<Token>,
    #[allow(dead_code)]
    lines:  Vec<u32>,
    pos:    usize,
}

#[derive(Clone, Copy, PartialEq)]
enum BlockKind { Indented, Braced, SingleLine }

impl Parser {
    pub fn new(pairs: Vec<(Token, u32)>) -> Self {
        let tokens: Vec<Token> = pairs.iter().map(|(t,_)| t.clone()).collect();
        let lines:  Vec<u32>   = pairs.iter().map(|(_,l)| *l).collect();
        Parser { tokens, lines, pos: 0 }
    }

    fn cur(&self) -> &Token { self.tokens.get(self.pos).unwrap_or(&Token::Eof) }
    fn peek(&self, offset: usize) -> &Token {
        self.tokens.get(self.pos + offset).unwrap_or(&Token::Eof)
    }
    fn advance(&mut self) { if self.pos < self.tokens.len() { self.pos += 1; } }
    #[allow(dead_code)]
    fn consume(&mut self, expected: &Token) -> bool {
        if std::mem::discriminant(self.cur()) == std::mem::discriminant(expected) {
            self.advance(); true
        } else { false }
    }
    fn consume_exact(&mut self, expected: &Token) -> bool {
        if self.cur() == expected { self.advance(); true } else { false }
    }

    fn ident_name(&self) -> Option<String> {
        match self.cur() {
            Token::Ident(s) => Some(s.clone()),
            Token::Var => Some("var".into()),
            Token::Let => Some("let".into()),
            _ => None,
        }
    }

    // ── Public entry ──────────────────────────────────────────────────────────

    pub fn parse(&mut self) -> Vec<ASTNode> {
        let mut stmts = Vec::new();
        while self.cur() != &Token::Eof {
            let start = self.pos;
            if let Some(s) = self.parse_stmt() { stmts.push(s); }
            if self.pos == start { self.advance(); }
        }
        stmts
    }

    fn parse_annotations(&mut self) -> Vec<ASTNode> {
        let mut annotations = Vec::new();
        while self.cur() == &Token::At {
            self.advance();
            // Match @Name or @Name(Args)
            if let Some(name) = self.ident_name() {
                self.advance();
                if self.cur() == &Token::LParen {
                    self.advance();
                    let args = self.parse_arg_list();
                    self.consume_exact(&Token::RParen);
                    annotations.push(ASTNode::FuncCall { name, args });
                } else {
                    annotations.push(ASTNode::Str(name));
                }
            }
        }
        annotations
    }

    // ── Block helpers ────────────────────────────────────────────────────────

    fn open_block(&mut self) -> BlockKind {
        if self.cur() == &Token::Colon {
            self.advance();
            if self.cur() == &Token::Indent { self.advance(); return BlockKind::Indented; }
            return BlockKind::SingleLine;
        }
        self.consume_exact(&Token::LBrace);
        BlockKind::Braced
    }

    fn parse_block(&mut self, kind: BlockKind) -> Vec<ASTNode> {
        if kind == BlockKind::SingleLine {
            return if let Some(s) = self.parse_stmt() { vec![s] } else { vec![] };
        }
        let stop = if kind == BlockKind::Indented { &Token::Dedent } else { &Token::RBrace };
        let mut stmts = Vec::new();
        while self.cur() != stop && self.cur() != &Token::Eof {
            if self.cur() == &Token::Indent { self.advance(); continue; }
            let start = self.pos;
            if let Some(s) = self.parse_stmt() { stmts.push(s); }
            if self.pos == start { self.advance(); }
        }
        stmts
    }

    fn close_block(&mut self, kind: BlockKind) {
        match kind {
            BlockKind::Indented   => { self.consume_exact(&Token::Dedent); }
            BlockKind::Braced     => { self.consume_exact(&Token::RBrace); }
            BlockKind::SingleLine => {}
        }
    }

    // ── Arguments / parameters ────────────────────────────────────────────────

    fn parse_arg_list(&mut self) -> Vec<ASTNode> {
        let mut args = Vec::new();
        if self.cur() == &Token::RParen { return args; }
        args.push(self.parse_one_arg());
        while self.cur() == &Token::Comma {
            self.advance();
            if self.cur() == &Token::RParen { break; }
            args.push(self.parse_one_arg());
        }
        args
    }

    fn parse_one_arg(&mut self) -> ASTNode {
        // Named arg: ident followed by colon
        if let Token::Ident(name) = self.cur().clone() {
            if self.peek(1) == &Token::Colon {
                let n = name.clone(); self.advance(); self.advance();
                let val = self.parse_expr();
                return ASTNode::NamedArg { name: n, value: Box::new(val) };
            }
        }
        if self.cur() == &Token::Star { self.advance(); return ASTNode::Spread(Box::new(self.parse_expr())); }
        self.parse_expr()
    }

    fn parse_param_list(&mut self) -> (Vec<Param>, bool) {
        let mut params: Vec<Param> = Vec::new();
        let mut variadic = false;
        if self.cur() == &Token::RParen { return (params, variadic); }

        if self.cur() == &Token::DotDotDot {
            self.advance();
            if let Some(name) = self.ident_name() { self.advance(); params.push((name, None)); }
            return (params, true);
        }

        if let Some(name) = self.ident_name() {
            self.advance();
            let default = if self.cur() == &Token::Eq { self.advance(); Some(Box::new(self.parse_expr())) } else { None };
            params.push((name, default));
            while self.cur() == &Token::Comma {
                self.advance();
                if self.cur() == &Token::DotDotDot {
                    self.advance();
                    if let Some(vp) = self.ident_name() { self.advance(); params.push((vp, None)); }
                    variadic = true; break;
                }
                if let Some(np) = self.ident_name() {
                    self.advance();
                    let dv = if self.cur() == &Token::Eq { self.advance(); Some(Box::new(self.parse_expr())) } else { None };
                    params.push((np, dv));
                }
            }
        }
        (params, variadic)
    }

    fn parse_pattern(&mut self) -> crate::ast::Pattern {
        if self.cur() == &Token::Ident("_".into()) { self.advance(); return crate::ast::Pattern::Wildcard; }

        // Enum or Struct pattern
        if let Token::Ident(name) = self.cur().clone() {
            if self.peek(1) == &Token::Dot {
                self.advance(); self.advance(); // Type.
                if let Token::Ident(vname) = self.cur().clone() {
                    self.advance();
                    let mut args = Vec::new();
                    if self.cur() == &Token::LParen {
                        self.advance();
                        while self.cur() != &Token::RParen && self.cur() != &Token::Eof {
                            args.push(self.parse_pattern());
                            if self.cur() == &Token::Comma { self.advance(); }
                        }
                        self.consume_exact(&Token::RParen);
                    }
                    return crate::ast::Pattern::Enum { type_name: name, variant: vname, args };
                }
            } else if self.peek(1) == &Token::LParen {
                self.advance(); self.advance(); // Struct(
                let mut fields = Vec::new();
                while self.cur() != &Token::RParen && self.cur() != &Token::Eof {
                    if let Token::Ident(fname) = self.cur().clone() {
                        if self.peek(1) == &Token::Colon {
                            self.advance(); self.advance();
                            fields.push((fname, self.parse_pattern()));
                        } else {
                            fields.push((fname.clone(), crate::ast::Pattern::Var(fname)));
                            self.advance();
                        }
                    }
                    if self.cur() == &Token::Comma { self.advance(); }
                }
                self.consume_exact(&Token::RParen);
                return crate::ast::Pattern::Struct { name, fields };
            }
        }

        match self.cur().clone() {
            Token::Int(_) | Token::Float(_) | Token::Str(_) | Token::True | Token::False | Token::Null => {
                let expr = self.parse_expr();
                crate::ast::Pattern::Literal(expr)
            }
            Token::Ident(name) => { self.advance(); crate::ast::Pattern::Var(name) }
            Token::LBracket => {
                self.advance();
                let mut elements = Vec::new();
                while self.cur() != &Token::RBracket && self.cur() != &Token::Eof {
                    elements.push(self.parse_pattern());
                    if self.cur() == &Token::Comma { self.advance(); }
                }
                self.consume_exact(&Token::RBracket);
                crate::ast::Pattern::List(elements)
            }
            _ => { self.advance(); crate::ast::Pattern::Wildcard }
        }
    }

    // ── Statements ────────────────────────────────────────────────────────────

    fn parse_stmt(&mut self) -> Option<ASTNode> {
        let annotations = self.parse_annotations();
        match self.cur().clone() {

            Token::Export => {
                self.advance();
                let inner = self.parse_stmt()?;
                Some(ASTNode::Export(Box::new(inner)))
            }

            Token::Import => {
                self.advance();
                let path = match self.cur().clone() {
                    Token::Str(s) => { self.advance(); s }
                    Token::Ident(s) => { self.advance(); s }
                    _ => return None,
                };
                Some(ASTNode::Include(path))
            }

            Token::Struct => {
                self.advance();
                let name = self.ident_name()?; self.advance();
                self.consume_exact(&Token::LBrace);
                let mut fields = Vec::new();
                while self.cur() != &Token::RBrace && self.cur() != &Token::Eof {
                    if let Some(f) = self.ident_name() {
                        fields.push(f); self.advance();
                        if self.cur() == &Token::Comma { self.advance(); }
                    } else { self.advance(); }
                }
                self.consume_exact(&Token::RBrace);
                Some(ASTNode::StructDecl { name, fields, annotations })
            }

            Token::Enum => {
                self.advance();
                let name = self.ident_name()?; self.advance();
                self.consume_exact(&Token::LBrace);
                let mut variants = Vec::new();
                while self.cur() != &Token::RBrace && self.cur() != &Token::Eof {
                    if let Some(vname) = self.ident_name() {
                        self.advance();
                        let mut vfields = Vec::new();
                        let mut varity = 0;
                        if self.cur() == &Token::LParen {
                            self.advance();
                            while self.cur() != &Token::RParen && self.cur() != &Token::Eof {
                                if let Some(f) = self.ident_name() {
                                    vfields.push(f); self.advance();
                                    varity += 1;
                                    if self.cur() == &Token::Comma { self.advance(); }
                                } else { self.advance(); }
                            }
                            self.consume_exact(&Token::RParen);
                        }
                        variants.push(crate::ast::EnumVariant { name: vname, fields: vfields, arity: varity });
                        if self.cur() == &Token::Comma { self.advance(); }
                    } else { self.advance(); }
                }
                self.consume_exact(&Token::RBrace);
                Some(ASTNode::EnumDecl { name, variants, annotations })
            }

            Token::Match => {
                self.advance();
                let subject = self.parse_expr();
                let has_brace = self.cur() == &Token::LBrace;
                if has_brace { self.advance(); }
                let mut arms = Vec::new();
                while self.cur() != &Token::Eof {
                    if has_brace && self.cur() == &Token::RBrace { break; }
                    if self.cur() == &Token::Case {
                        self.advance();
                        let pat = self.parse_pattern();
                        if self.cur() == &Token::Colon || self.cur() == &Token::Arrow { self.advance(); }
                        let mut body = Vec::new();
                        while !matches!(self.cur(), Token::Case | Token::Default | Token::RBrace | Token::Eof | Token::Dedent) {
                            let s = self.pos;
                            if let Some(st) = self.parse_stmt() { body.push(st); }
                            if self.pos == s { self.advance(); }
                        }
                        arms.push((pat, body));
                    } else if self.cur() == &Token::Default {
                        self.advance();
                        if self.cur() == &Token::Colon || self.cur() == &Token::Arrow { self.advance(); }
                        let mut body = Vec::new();
                        while !matches!(self.cur(), Token::Case | Token::Default | Token::RBrace | Token::Eof | Token::Dedent) {
                            let s = self.pos;
                            if let Some(st) = self.parse_stmt() { body.push(st); }
                            if self.pos == s { self.advance(); }
                        }
                        arms.push((crate::ast::Pattern::Wildcard, body));
                    } else {
                        let pat = self.parse_pattern();
                        if self.cur() == &Token::Arrow || self.cur() == &Token::Colon { self.advance(); }
                        let mut body = Vec::new();
                        if self.cur() == &Token::LBrace {
                            self.advance();
                            while self.cur() != &Token::RBrace && self.cur() != &Token::Eof {
                                if let Some(s) = self.parse_stmt() { body.push(s); }
                            }
                            self.consume_exact(&Token::RBrace);
                        } else {
                            if let Some(s) = self.parse_stmt() { body.push(s); }
                        }
                        arms.push((pat, body));
                    }
                    if self.cur() == &Token::Comma { self.advance(); }
                }
                if has_brace { self.consume_exact(&Token::RBrace); }
                Some(ASTNode::Match { subject: Box::new(subject), arms })
            }

            Token::Let | Token::Var => {
                let mutable = self.cur() == &Token::Var;
                // Destructure array: let [a, b] = ...
                if self.peek(1) == &Token::LBracket {
                    self.advance(); self.advance();
                    let mut names: Vec<Option<String>> = Vec::new();
                    while self.cur() != &Token::RBracket && self.cur() != &Token::Eof {
                        if self.cur() == &Token::Comma { self.advance(); continue; }
                        if let Some(n) = self.ident_name() { names.push(Some(n)); self.advance(); }
                        else { names.push(None); self.advance(); }
                    }
                    self.consume_exact(&Token::RBracket);
                    self.consume_exact(&Token::Eq);
                    let val = self.parse_expr();
                    return Some(ASTNode::DestructureArray { names, value: Box::new(val), mutable });
                }
                // Destructure map: let {"key": var} = ...
                if self.peek(1) == &Token::LBrace {
                    self.advance(); self.advance();
                    let mut keys: Vec<(String, Option<String>)> = Vec::new();
                    while self.cur() != &Token::RBrace && self.cur() != &Token::Eof {
                        if self.cur() == &Token::Comma { self.advance(); continue; }
                        if let Token::Str(k) = self.cur().clone() {
                            self.advance(); self.consume_exact(&Token::Colon);
                            let alias = self.ident_name().map(|n| { self.advance(); n });
                            keys.push((k, alias));
                        } else { self.advance(); }
                    }
                    self.consume_exact(&Token::RBrace);
                    self.consume_exact(&Token::Eq);
                    let val = self.parse_expr();
                    return Some(ASTNode::DestructureMap { keys, value: Box::new(val), mutable });
                }
                // Normal declaration
                let name: String;
                if self.peek(1) == &Token::Eq {
                    name = if mutable { "var".into() } else { "let".into() };
                    self.advance();
                } else {
                    self.advance();
                    name = self.ident_name()?; self.advance();
                }
                self.consume_exact(&Token::Eq);
                Some(ASTNode::Decl { name, value: Box::new(self.parse_expr()), mutable, annotations })
            }

            Token::Print => {
                self.advance();
                self.consume_exact(&Token::LParen);
                let arg = self.parse_expr();
                self.consume_exact(&Token::RParen);
                Some(ASTNode::PrintCall(Box::new(arg)))
            }

            Token::Assert => {
                self.advance();
                let cond = self.parse_expr();
                let msg = if self.cur() == &Token::Comma { self.advance(); Some(Box::new(self.parse_expr())) } else { None };
                Some(ASTNode::Assert { cond: Box::new(cond), msg })
            }

            Token::While => {
                self.advance();
                let cond = self.parse_expr();
                let kind = self.open_block();
                let body = self.parse_block(kind);
                self.close_block(kind);
                Some(ASTNode::While { cond: Box::new(cond), body })
            }

            Token::For => {
                self.advance();
                let var = self.ident_name()?; self.advance();
                self.consume_exact(&Token::In);
                let iter = self.parse_expr();
                let kind = self.open_block();
                let body = self.parse_block(kind);
                self.close_block(kind);
                // Detect range sugar: for x in a..b
                if let ASTNode::BinOp { left, op, right } = &iter {
                    if op == ".." {
                        return Some(ASTNode::ForRange { var, from: left.clone(), to: right.clone(), inclusive: false, body });
                    }
                    if op == "..." {
                        return Some(ASTNode::ForRange { var, from: left.clone(), to: right.clone(), inclusive: true, body });
                    }
                }
                Some(ASTNode::For { var, iter: Box::new(iter), body })
            }

            Token::If => {
                self.advance();
                let cond = self.parse_expr();
                let kind = self.open_block();
                let then = self.parse_block(kind);
                self.close_block(kind);
                let els = if self.cur() == &Token::Elif {
                    self.parse_elif_chain().map(|n| vec![n])
                } else if self.cur() == &Token::Else {
                    self.advance();
                    if self.cur() == &Token::If {
                        self.parse_stmt().map(|n| vec![n])
                    } else {
                        let ek = self.open_block();
                        let eb = self.parse_block(ek);
                        self.close_block(ek);
                        Some(eb)
                    }
                } else { None };
                Some(ASTNode::If { cond: Box::new(cond), then, els })
            }

            Token::Switch => {
                self.advance();
                let subject = self.parse_expr();
                self.consume_exact(&Token::LBrace);
                let mut cases: Vec<(Option<ASTNode>, Vec<ASTNode>)> = Vec::new();
                while self.cur() != &Token::RBrace && self.cur() != &Token::Eof {
                    if self.cur() == &Token::Case {
                        self.advance();
                        let pattern = self.parse_expr();
                        self.consume_exact(&Token::Colon);
                        let mut body = Vec::new();
                        while !matches!(self.cur(), Token::Case | Token::Default | Token::RBrace | Token::Eof) {
                            let s = self.pos;
                            if let Some(st) = self.parse_stmt() { body.push(st); }
                            if self.pos == s { self.advance(); }
                        }
                        cases.push((Some(pattern), body));
                    } else if self.cur() == &Token::Default {
                        self.advance(); self.consume_exact(&Token::Colon);
                        let mut body = Vec::new();
                        while !matches!(self.cur(), Token::Case | Token::RBrace | Token::Eof) {
                            let s = self.pos;
                            if let Some(st) = self.parse_stmt() { body.push(st); }
                            if self.pos == s { self.advance(); }
                        }
                        cases.push((None, body));
                    } else { self.advance(); }
                }
                self.consume_exact(&Token::RBrace);
                Some(ASTNode::Switch { subject: Box::new(subject), cases })
            }

            Token::Def => {
                self.advance();
                if self.cur() == &Token::LParen {
                    self.consume_exact(&Token::LParen);
                    let (params, variadic) = self.parse_param_list();
                    self.consume_exact(&Token::RParen);
                    if self.cur() == &Token::Arrow {
                        self.advance();
                        let expr = self.parse_expr();
                        return Some(ASTNode::Lambda { params, body: vec![ASTNode::Return(Box::new(expr))], variadic, annotations });
                    }
                    let kind = self.open_block();
                    let body = self.parse_block(kind);
                    self.close_block(kind);
                    Some(ASTNode::Lambda { params, body, variadic, annotations })
                } else {
                    let name = match self.ident_name() {
                        Some(n) => n,
                        None => return None,
                    };
                    self.advance();
                    self.consume_exact(&Token::LParen);
                    let (params, variadic) = self.parse_param_list();
                    self.consume_exact(&Token::RParen);
                    let kind = self.open_block();
                    let body = self.parse_block(kind);
                    self.close_block(kind);
                    Some(ASTNode::FuncDecl { name, params, body, variadic, annotations })
                }
            }

            Token::Return => {
                self.advance();
                if matches!(self.cur(), Token::RBrace | Token::Eof | Token::Dedent) {
                    return Some(ASTNode::Return(Box::new(ASTNode::Null)));
                }
                Some(ASTNode::Return(Box::new(self.parse_expr())))
            }

            Token::Break    => { self.advance(); Some(ASTNode::Break) }
            Token::Continue => { self.advance(); Some(ASTNode::Continue) }
            Token::Pass     => { self.advance(); Some(ASTNode::Pass) }

            Token::Throw => {
                self.advance();
                Some(ASTNode::Throw(Box::new(self.parse_expr())))
            }

            Token::Try => {
                self.advance();
                let tk = self.open_block();
                let try_body = self.parse_block(tk);
                self.close_block(tk);

                let mut catches = Vec::new();
                while self.cur() == &Token::Catch {
                    self.advance();
                    let mut type_filter = None;
                    let mut catch_var = "_err".to_string();

                    if let Some(name) = self.ident_name() {
                        self.advance();
                        if self.cur() == &Token::As {
                            self.advance();
                            type_filter = Some(name);
                            if let Some(v) = self.ident_name() { catch_var = v; self.advance(); }
                        } else {
                            catch_var = name;
                        }
                    }

                    let ck = self.open_block();
                    let catch_body = self.parse_block(ck);
                    self.close_block(ck);
                    catches.push((type_filter, catch_var, catch_body));
                }

                let mut finally_body = None;
                if self.cur() == &Token::Finally {
                    self.advance();
                    let fk = self.open_block();
                    finally_body = Some(self.parse_block(fk));
                    self.close_block(fk);
                }
                Some(ASTNode::TryCatch { body: try_body, catches, finally_body })
            }

            Token::Ident(name) => {
                let name = name.clone();
                self.parse_ident_or_expr(name)
            }

            _ => Some(self.parse_expr()),
        }
    }

    fn parse_elif_chain(&mut self) -> Option<ASTNode> {
        if self.cur() != &Token::Elif { return None; }
        self.advance();
        let cond = self.parse_expr();
        let kind = self.open_block();
        let then = self.parse_block(kind);
        self.close_block(kind);
        let els = if self.cur() == &Token::Elif {
            self.parse_elif_chain().map(|n| vec![n])
        } else if self.cur() == &Token::Else {
            self.advance();
            let ek = self.open_block();
            let eb = self.parse_block(ek);
            self.close_block(ek);
            Some(eb)
        } else { None };
        Some(ASTNode::If { cond: Box::new(cond), then, els })
    }

    fn parse_ident_or_expr(&mut self, name: String) -> Option<ASTNode> {
        // compound assignment: x += ...
        let compound = match self.peek(1) {
            Token::PlusEq    => Some("+"),  Token::MinusEq  => Some("-"),
            Token::StarEq    => Some("*"),  Token::SlashEq  => Some("/"),
            Token::PercentEq => Some("%"),  Token::AmpEq    => Some("&"),
            Token::PipeEq    => Some("|"),  Token::CaretEq  => Some("^"),
            Token::LtltEq    => Some("<<"), Token::GtgtEq   => Some(">>"),
            _ => None,
        };
        if let Some(op) = compound {
            self.advance(); self.advance();
            return Some(ASTNode::CompoundAssign { name, op: op.into(), value: Box::new(self.parse_expr()) });
        }
        // assignment: x = ...
        if self.peek(1) == &Token::Eq {
            self.advance(); self.advance();
            return Some(ASTNode::Assign { name, value: Box::new(self.parse_expr()) });
        }
        // index assignment: x[i] = ...
        if self.peek(1) == &Token::LBracket {
            let saved = self.pos;
            self.advance(); self.advance();
            let idx = self.parse_expr();
            self.consume_exact(&Token::RBracket);
            if self.cur() == &Token::Eq {
                self.advance();
                return Some(ASTNode::IndexAssign { target: name, index: Box::new(idx), value: Box::new(self.parse_expr()) });
            }
            self.pos = saved;
        }
        Some(self.parse_expr())
    }

    // ── Expression hierarchy (precedence: low → high) ─────────────────────────

    fn parse_expr(&mut self) -> ASTNode { self.parse_pipe() }

    fn parse_pipe(&mut self) -> ASTNode {
        let mut left = self.parse_null_coalesce();
        while self.cur() == &Token::PipeArrow {
            self.advance();
            let right = self.parse_null_coalesce();
            left = ASTNode::CallExpr { callee: Box::new(right), args: vec![left] };
        }
        left
    }

    fn parse_null_coalesce(&mut self) -> ASTNode {
        let mut left = self.parse_ternary();
        while self.cur() == &Token::QQ {
            self.advance();
            let right = self.parse_ternary();
            left = ASTNode::NullCoalesce { left: Box::new(left), right: Box::new(right) };
        }
        left
    }

    fn parse_ternary(&mut self) -> ASTNode {
        let cond = self.parse_or();
        if self.cur() == &Token::Quest {
            self.advance();
            let then = self.parse_ternary();
            self.consume_exact(&Token::Colon);
            let els = self.parse_ternary();
            return ASTNode::Ternary { cond: Box::new(cond), then: Box::new(then), els: Box::new(els) };
        }
        cond
    }

    fn parse_or(&mut self) -> ASTNode {
        let mut left = self.parse_and();
        while matches!(self.cur(), Token::OrOr | Token::Or) {
            self.advance();
            left = ASTNode::BinOp { left: Box::new(left), op: "||".into(), right: Box::new(self.parse_and()) };
        }
        left
    }

    fn parse_and(&mut self) -> ASTNode {
        let mut left = self.parse_equality();
        while matches!(self.cur(), Token::AndAnd | Token::And) {
            self.advance();
            left = ASTNode::BinOp { left: Box::new(left), op: "&&".into(), right: Box::new(self.parse_equality()) };
        }
        left
    }

    fn parse_equality(&mut self) -> ASTNode {
        let mut left = self.parse_comparison();
        while matches!(self.cur(), Token::EqEq | Token::BangEq) {
            let op = if self.cur() == &Token::EqEq { "==" } else { "!=" };
            self.advance();
            left = ASTNode::BinOp { left: Box::new(left), op: op.into(), right: Box::new(self.parse_comparison()) };
        }
        left
    }

    fn parse_comparison(&mut self) -> ASTNode {
        let mut left = self.parse_range();
        loop {
            let op = match self.cur() {
                Token::Lt   => "<",  Token::Gt  => ">",
                Token::LtEq => "<=", Token::GtEq => ">=",
                _ => break,
            };
            self.advance();
            left = ASTNode::BinOp { left: Box::new(left), op: op.into(), right: Box::new(self.parse_range()) };
        }
        left
    }

    fn parse_range(&mut self) -> ASTNode {
        let left = self.parse_bitor();
        if self.cur() == &Token::DotDot { self.advance(); return ASTNode::BinOp { left: Box::new(left), op: "..".into(), right: Box::new(self.parse_bitor()) }; }
        if self.cur() == &Token::DotDotDot { self.advance(); return ASTNode::BinOp { left: Box::new(left), op: "...".into(), right: Box::new(self.parse_bitor()) }; }
        left
    }

    fn parse_bitor(&mut self) -> ASTNode {
        let mut left = self.parse_bitxor();
        while self.cur() == &Token::Pipe { self.advance(); left = ASTNode::BinOp { left: Box::new(left), op: "|".into(), right: Box::new(self.parse_bitxor()) }; }
        left
    }

    fn parse_bitxor(&mut self) -> ASTNode {
        let mut left = self.parse_bitand();
        while self.cur() == &Token::Caret { self.advance(); left = ASTNode::BinOp { left: Box::new(left), op: "^".into(), right: Box::new(self.parse_bitand()) }; }
        left
    }

    fn parse_bitand(&mut self) -> ASTNode {
        let mut left = self.parse_shift();
        while self.cur() == &Token::Amp { self.advance(); left = ASTNode::BinOp { left: Box::new(left), op: "&".into(), right: Box::new(self.parse_shift()) }; }
        left
    }

    fn parse_shift(&mut self) -> ASTNode {
        let mut left = self.parse_add_sub();
        loop {
            let op = match self.cur() { Token::Ltlt => "<<", Token::Gtgt => ">>", Token::Gtgtgt => ">>>", _ => break };
            self.advance();
            left = ASTNode::BinOp { left: Box::new(left), op: op.into(), right: Box::new(self.parse_add_sub()) };
        }
        left
    }

    fn parse_add_sub(&mut self) -> ASTNode {
        let mut left = self.parse_mul_div();
        while matches!(self.cur(), Token::Plus | Token::Minus) {
            let op = if self.cur() == &Token::Plus { "+" } else { "-" };
            self.advance();
            left = ASTNode::BinOp { left: Box::new(left), op: op.into(), right: Box::new(self.parse_mul_div()) };
        }
        left
    }

    fn parse_mul_div(&mut self) -> ASTNode {
        let mut left = self.parse_pow();
        loop {
            let op = match self.cur() {
                Token::Star => "*", Token::Slash => "/",
                Token::Percent => "%", Token::SlashSlash => "//",
                _ => break,
            };
            self.advance();
            left = ASTNode::BinOp { left: Box::new(left), op: op.into(), right: Box::new(self.parse_pow()) };
        }
        left
    }

    fn parse_pow(&mut self) -> ASTNode {
        let base = self.parse_unary();
        if self.cur() == &Token::StarStar { self.advance(); return ASTNode::BinOp { left: Box::new(base), op: "**".into(), right: Box::new(self.parse_pow()) }; }
        base
    }

    fn parse_unary(&mut self) -> ASTNode {
        if self.cur() == &Token::Minus { self.advance(); return ASTNode::UnaryOp { op: "-".into(), operand: Box::new(self.parse_unary()) }; }
        if matches!(self.cur(), Token::Bang | Token::Not) { self.advance(); return ASTNode::UnaryOp { op: "!".into(), operand: Box::new(self.parse_unary()) }; }
        if self.cur() == &Token::Tilde { self.advance(); return ASTNode::UnaryOp { op: "~".into(), operand: Box::new(self.parse_unary()) }; }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> ASTNode {
        let mut node = self.parse_primary();
        loop {
            if self.cur() == &Token::LBracket {
                self.advance();
                let idx = self.parse_expr();
                self.consume_exact(&Token::RBracket);
                node = ASTNode::Subscript { target: Box::new(node), index: Box::new(idx) };
            } else if self.cur() == &Token::LParen {
                self.advance();
                let args = self.parse_arg_list();
                self.consume_exact(&Token::RParen);
                node = ASTNode::CallExpr { callee: Box::new(node), args };
            } else if self.cur() == &Token::Dot {
                self.advance();
                if let Some(field) = self.ident_name() {
                    self.advance();
                    // member access or method call
                    if self.cur() == &Token::LParen {
                        self.advance();
                        let args = self.parse_arg_list();
                        self.consume_exact(&Token::RParen);

                        // Instead of desugaring to FuncCall immediately,
                        // let's use CallExpr on a Subscript for better flexibility
                        node = ASTNode::CallExpr {
                            callee: Box::new(ASTNode::Subscript {
                                target: Box::new(node),
                                index: Box::new(ASTNode::Str(field)),
                            }),
                            args,
                        };
                    } else {
                        // Field access: obj.field → obj["field"]
                        node = ASTNode::Subscript {
                            target: Box::new(node),
                            index: Box::new(ASTNode::Str(field)),
                        };
                    }
                } else { break; }
            } else { break; }
        }
        node
    }

    fn parse_primary(&mut self) -> ASTNode {
        let annotations = self.parse_annotations();
        match self.cur().clone() {
            Token::Int(v)   => { self.advance(); ASTNode::Int(v) }
            Token::Float(v) => { self.advance(); ASTNode::Float(v) }
            Token::True     => { self.advance(); ASTNode::Bool(true) }
            Token::False    => { self.advance(); ASTNode::Bool(false) }
            Token::Null     => { self.advance(); ASTNode::Null }
            Token::ByteStr(b) => { self.advance(); ASTNode::ByteArray(b) }

            Token::Str(val) => {
                self.advance();
                let mut parts = vec![ASTNode::Str(val)];
                while self.cur() == &Token::Ident("__interp_start__".into()) {
                    self.advance();
                    let mut inner: Vec<(Token, u32)> = Vec::new();
                    while self.cur() != &Token::Ident("__interp_end__".into()) && self.cur() != &Token::Eof {
                        inner.push((self.cur().clone(), 0)); self.advance();
                    }
                    self.advance(); // __interp_end__
                    inner.push((Token::Eof, 0));
                    let mut p2 = Parser::new(inner);
                    parts.push(p2.parse_expr());
                    if let Token::Str(next) = self.cur().clone() { parts.push(ASTNode::Str(next)); self.advance(); }
                }
                if parts.len() == 1 { parts.remove(0) } else { ASTNode::InterpStr(parts) }
            }

            Token::LBracket => {
                self.advance();
                let mut elements = Vec::new();
                if self.cur() != &Token::RBracket {
                    if self.cur() == &Token::Star { self.advance(); elements.push(ASTNode::Spread(Box::new(self.parse_expr()))); }
                    else { elements.push(self.parse_expr()); }
                    while self.cur() == &Token::Comma {
                        self.advance();
                        if self.cur() == &Token::RBracket { break; }
                        if self.cur() == &Token::Star { self.advance(); elements.push(ASTNode::Spread(Box::new(self.parse_expr()))); }
                        else { elements.push(self.parse_expr()); }
                    }
                }
                self.consume_exact(&Token::RBracket);
                ASTNode::ArrayLit(elements)
            }

            Token::LBrace => {
                self.advance();
                let mut pairs = Vec::new();
                if self.cur() != &Token::RBrace {
                    let k = self.parse_expr(); self.consume_exact(&Token::Colon); let v = self.parse_expr();
                    pairs.push((k, v));
                    while self.cur() == &Token::Comma {
                        self.advance();
                        if self.cur() == &Token::RBrace { break; }
                        let k2 = self.parse_expr(); self.consume_exact(&Token::Colon); let v2 = self.parse_expr();
                        pairs.push((k2, v2));
                    }
                }
                self.consume_exact(&Token::RBrace);
                ASTNode::MapLit(pairs)
            }

            Token::LParen => {
                self.advance();
                let expr = self.parse_expr();
                self.consume_exact(&Token::RParen);
                expr
            }

            // Anonymous function: def(params) { body } or def(params) => expr
            Token::Def => {
                self.advance();
                self.consume_exact(&Token::LParen);
                let (params, variadic) = self.parse_param_list();
                self.consume_exact(&Token::RParen);
                if self.cur() == &Token::Arrow {
                    self.advance();
                    let expr = self.parse_expr();
                    return ASTNode::Lambda { params, body: vec![ASTNode::Return(Box::new(expr))], variadic, annotations };
                }
                let kind = self.open_block();
                let body = self.parse_block(kind);
                self.close_block(kind);
                ASTNode::Lambda { params, body, variadic, annotations }
            }

            Token::Import => {
                self.advance();
                let path = match self.cur().clone() {
                    Token::Str(s) => { self.advance(); s }
                    Token::Ident(s) => { self.advance(); s }
                    _ => return ASTNode::Null,
                };
                ASTNode::Import(path)
            }

            Token::Ident(name) => {
                let name = name.clone(); self.advance();
                if name == "now" {
                    if self.cur() == &Token::LParen { self.advance(); self.consume_exact(&Token::RParen); }
                    return ASTNode::TimeCall;
                }
                if self.cur() == &Token::LParen {
                    self.advance();
                    let args = self.parse_arg_list();
                    self.consume_exact(&Token::RParen);
                    return ASTNode::FuncCall { name, args };
                }
                ASTNode::Var(name)
            }

            Token::Let | Token::Var => {
                if let Some(name) = self.ident_name() { self.advance(); return ASTNode::Var(name); }
                self.advance(); ASTNode::Null
            }

            _ => { self.advance(); ASTNode::Null }
        }
    }
}
