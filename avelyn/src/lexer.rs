// lexer.rs — Indentation-aware tokenizer
// Ported from CoreInterpreter/Lexer.swift

use crate::ast::Token;

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    pub line: u32,
    /// Non-fatal warnings collected during tokenisation (line, message)
    pub warnings: Vec<(u32, String)>,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer { chars: input.chars().collect(), pos: 0, line: 1, warnings: Vec::new() }
    }

    fn cur(&self) -> Option<char> { self.chars.get(self.pos).copied() }
    fn peek(&self) -> Option<char> { self.chars.get(self.pos + 1).copied() }
    fn peek2(&self) -> Option<char> { self.chars.get(self.pos + 2).copied() }
    fn at_end(&self) -> bool { self.pos >= self.chars.len() }

    fn advance(&mut self) {
        if self.pos < self.chars.len() {
            if self.chars[self.pos] == '\n' { self.line += 1; }
            self.pos += 1;
        }
    }

    pub fn tokenize(&mut self) -> Vec<(Token, u32)> {
        let mut tokens: Vec<(Token, u32)> = Vec::new();
        let mut indent_stack: Vec<usize> = vec![0];
        let mut at_line_start = true;
        let mut current_indent = 0usize;
        let mut bracket_depth = 0i32;

        while !self.at_end() {
            let ch = match self.cur() { Some(c) => c, None => break };

            // Newlines
            if ch == '\n' || ch == '\r' {
                if ch == '\r' && self.peek() == Some('\n') { self.advance(); }
                self.advance();
                at_line_start = true;
                current_indent = 0;
                continue;
            }

            // Measure indentation
            if at_line_start {
                if ch == ' ' || ch == '\t' {
                    current_indent += if ch == '\t' { 4 } else { 1 };
                    self.advance();
                    continue;
                }
                at_line_start = false;

                let is_comment = ch == '#' || (ch == '/' && self.peek() == Some('*'));
                if !is_comment && bracket_depth == 0 {
                    let top = *indent_stack.last().unwrap();
                    if current_indent > top {
                        indent_stack.push(current_indent);
                        tokens.push((Token::Indent, self.line));
                    } else if current_indent < top {
                        while indent_stack.last().copied().unwrap_or(0) > current_indent {
                            indent_stack.pop();
                            tokens.push((Token::Dedent, self.line));
                        }
                    }
                }
            }

            let ch = match self.cur() { Some(c) => c, None => break };
            let tok_line = self.line;

            // Single-line comment
            if ch == '#' {
                while !self.at_end() && self.cur() != Some('\n') { self.advance(); }
                continue;
            }
            // Multi-line comment /* ... */
            if ch == '/' && self.peek() == Some('*') {
                self.advance(); self.advance();
                while !self.at_end() {
                    if self.cur() == Some('*') && self.peek() == Some('/') {
                        self.advance(); self.advance(); break;
                    }
                    self.advance();
                }
                continue;
            }

            if ch == ' ' || ch == '\t' { self.advance(); continue; }

            // Triple-quote strings """..."""
            if ch == '"' && self.peek() == Some('"') && self.peek2() == Some('"') {
                self.advance(); self.advance(); self.advance();
                let mut seg = String::new();
                while !self.at_end() {
                    if self.cur() == Some('"') && self.peek() == Some('"') && self.peek2() == Some('"') {
                        self.advance(); self.advance(); self.advance(); break;
                    }
                    if self.cur() == Some('\n') { seg.push('\n'); self.advance(); continue; }
                    seg.push(self.cur().unwrap()); self.advance();
                }
                if seg.starts_with("\r\n") { seg.drain(0..2); }
                else if seg.starts_with('\n') || seg.starts_with('\r') { seg.remove(0); }
                tokens.push((Token::Str(seg), tok_line));
                continue;
            }

            // Byte string b"..." or b'...'
            if (ch == 'b' || ch == 'B') && (self.peek() == Some('"') || self.peek() == Some('\'')) {
                self.advance();
                let quote = self.cur().unwrap();
                self.advance();
                let mut bytes: Vec<u8> = Vec::new();
                while !self.at_end() && self.cur() != Some(quote) {
                    if self.cur() == Some('\\') {
                        self.advance();
                        match self.cur() {
                            Some('x') | Some('X') => {
                                self.advance();
                                let mut hex = String::new();
                                if self.cur().map(|c| c.is_ascii_hexdigit()).unwrap_or(false) {
                                    hex.push(self.cur().unwrap()); self.advance();
                                }
                                if self.cur().map(|c| c.is_ascii_hexdigit()).unwrap_or(false) {
                                    hex.push(self.cur().unwrap()); self.advance();
                                }
                                if let Ok(b) = u8::from_str_radix(&hex, 16) { bytes.push(b); }
                            }
                            Some('n') => { bytes.push(0x0A); self.advance(); }
                            Some('t') => { bytes.push(0x09); self.advance(); }
                            Some('r') => { bytes.push(0x0D); self.advance(); }
                            Some('0') => { bytes.push(0x00); self.advance(); }
                            Some('\\') => { bytes.push(0x5C); self.advance(); }
                            Some(c) => { bytes.push(c as u8); self.advance(); }
                            None => {}
                        }
                    } else {
                        bytes.push(self.cur().unwrap() as u8); self.advance();
                    }
                }
                if !self.at_end() { self.advance(); }
                tokens.push((Token::ByteStr(bytes), tok_line));
                continue;
            }

            // .. and ...
            if ch == '.' && self.peek() == Some('.') {
                self.advance(); self.advance();
                if self.cur() == Some('.') { self.advance(); tokens.push((Token::DotDotDot, tok_line)); }
                else { tokens.push((Token::DotDot, tok_line)); }
                continue;
            }

            // =>
            if ch == '=' && self.peek() == Some('>') {
                self.advance(); self.advance(); tokens.push((Token::Arrow, tok_line)); continue;
            }

            // << <<=
            if ch == '<' && self.peek() == Some('<') {
                self.advance(); self.advance();
                if self.cur() == Some('=') { self.advance(); tokens.push((Token::LtltEq, tok_line)); }
                else { tokens.push((Token::Ltlt, tok_line)); }
                continue;
            }
            // >>> >> >>=
            if ch == '>' && self.peek() == Some('>') {
                self.advance(); self.advance();
                if self.cur() == Some('>') { self.advance(); tokens.push((Token::Gtgtgt, tok_line)); }
                else if self.cur() == Some('=') { self.advance(); tokens.push((Token::GtgtEq, tok_line)); }
                else { tokens.push((Token::Gtgt, tok_line)); }
                continue;
            }

            // ?? |>
            if ch == '?' && self.peek() == Some('?') { self.advance(); self.advance(); tokens.push((Token::QQ, tok_line)); continue; }
            if ch == '|' && self.peek() == Some('>') { self.advance(); self.advance(); tokens.push((Token::PipeArrow, tok_line)); continue; }

            // Two-char operators
            macro_rules! two {
                ($a:expr, $b:expr, $tok:expr) => {
                    if ch == $a && self.peek() == Some($b) {
                        self.advance(); self.advance(); tokens.push(($tok, tok_line)); continue;
                    }
                }
            }
            two!('=','=', Token::EqEq);   two!('!','=', Token::BangEq);
            two!('<','=', Token::LtEq);   two!('>','=', Token::GtEq);
            two!('&','&', Token::AndAnd); two!('|','|', Token::OrOr);
            two!('+','=', Token::PlusEq); two!('-','=', Token::MinusEq);
            two!('*','=', Token::StarEq); two!('/','=', Token::SlashEq);
            two!('%','=', Token::PercentEq);
            two!('*','*', Token::StarStar); two!('/','/', Token::SlashSlash);
            two!('&','=', Token::AmpEq);  two!('|','=', Token::PipeEq);
            two!('^','=', Token::CaretEq);

            // f/r string prefix
            if (ch == 'f' || ch == 'F' || ch == 'r' || ch == 'R') && (self.peek() == Some('"') || self.peek() == Some('\'')) {
                self.advance(); // skip prefix, fall through
            }

            let ch = match self.cur() { Some(c) => c, None => break };

            // Single-char operators
            macro_rules! one {
                ($c:expr, $tok:expr) => {
                    if ch == $c { self.advance(); tokens.push(($tok, tok_line)); continue; }
                }
            }
            one!('=', Token::Eq);  one!('+', Token::Plus);  one!('-', Token::Minus);
            one!('*', Token::Star); one!('/', Token::Slash); one!('%', Token::Percent);
            one!('<', Token::Lt);   one!('>', Token::Gt);    one!('!', Token::Bang);
            one!(',', Token::Comma); one!(':', Token::Colon); one!(';', Token::Semi);
            one!('.', Token::Dot);   one!('&', Token::Amp);   one!('|', Token::Pipe);
            one!('^', Token::Caret); one!('~', Token::Tilde); one!('?', Token::Quest);
            one!('@', Token::At);
            if ch == '(' { bracket_depth += 1; self.advance(); tokens.push((Token::LParen, tok_line)); continue; }
            if ch == ')' { bracket_depth -= 1; self.advance(); tokens.push((Token::RParen, tok_line)); continue; }
            if ch == '{' { bracket_depth += 1; self.advance(); tokens.push((Token::LBrace, tok_line)); continue; }
            if ch == '}' { bracket_depth -= 1; self.advance(); tokens.push((Token::RBrace, tok_line)); continue; }
            if ch == '[' { bracket_depth += 1; self.advance(); tokens.push((Token::LBracket, tok_line)); continue; }
            if ch == ']' { bracket_depth -= 1; self.advance(); tokens.push((Token::RBracket, tok_line)); continue; }

            let ch = match self.cur() { Some(c) => c, None => break };

            // Single-quote strings
            if ch == '\'' {
                self.advance();
                let mut seg = String::new();
                while !self.at_end() && self.cur() != Some('\'') {
                    if self.cur() == Some('\\') { self.advance(); seg.push(self.escape_char()); }
                    else { seg.push(self.cur().unwrap()); self.advance(); }
                }
                self.advance(); // closing '
                tokens.push((Token::Str(seg), tok_line));
                continue;
            }

            // Double-quote strings with \(expr) interpolation
            if ch == '"' {
                self.advance();
                let mut seg = String::new();
                while !self.at_end() && self.cur() != Some('"') {
                    if self.cur() == Some('\\') {
                        self.advance();
                        if self.cur() == Some('(') {
                            // string interpolation: collect inner source
                            if !seg.is_empty() { tokens.push((Token::Str(seg.clone()), tok_line)); seg.clear(); }
                            self.advance(); // skip (
                            tokens.push((Token::Ident("__interp_start__".into()), tok_line));
                            let mut inner = String::new();
                            let mut depth = 1;
                            while !self.at_end() && depth > 0 {
                                let ic = self.cur().unwrap();
                                if ic == '(' { depth += 1; }
                                if ic == ')' { depth -= 1; if depth == 0 { self.advance(); break; } }
                                inner.push(ic); self.advance();
                            }
                            // Re-lex inner
                            let mut inner_lexer = Lexer::new(&inner);
                            let inner_toks = inner_lexer.tokenize();
                            for (t, _) in inner_toks { if t != Token::Eof { tokens.push((t, tok_line)); } }
                            tokens.push((Token::Ident("__interp_end__".into()), tok_line));
                            continue;
                        }
                        seg.push(self.escape_char());
                        continue;
                    }
                    seg.push(self.cur().unwrap()); self.advance();
                }
                self.advance(); // closing "
                tokens.push((Token::Str(seg), tok_line));
                continue;
            }

            let ch = match self.cur() { Some(c) => c, None => break };

            // Numbers
            if ch.is_ascii_digit() {
                let tok = self.lex_number();
                tokens.push((tok, tok_line));
                continue;
            }

            // Keywords / identifiers
            if ch.is_alphabetic() || ch == '_' {
                let mut word = String::new();
                while !self.at_end() {
                    let c = match self.cur() { Some(c) => c, None => break };
                    if c.is_alphanumeric() || c == '_' { word.push(c); self.advance(); }
                    else { break; }
                }
                let tok = Self::keyword_or_ident(word);
                tokens.push((tok, tok_line));
                continue;
            }

            self.advance(); // skip unknown
        }

        // Close remaining indents
        while indent_stack.len() > 1 {
            indent_stack.pop();
            tokens.push((Token::Dedent, self.line));
        }
        tokens.push((Token::Eof, self.line));
        tokens
    }

    fn escape_char(&mut self) -> char {
        let c = self.cur().unwrap_or('\\');
        self.advance();
        match c {
            'n' => '\n', 't' => '\t', 'r' => '\r', '0' => '\0',
            '\\' => '\\', '\'' => '\'', '"' => '"',
            _ => c,
        }
    }

    fn lex_number(&mut self) -> Token {
        let mut s = String::new();
        let mut is_float = false;

        // Hex / bin / octal prefix
        if self.cur() == Some('0') {
            s.push('0'); self.advance();
            match self.cur() {
        // Hex
                Some('x') | Some('X') => {
                    self.advance();
                    let mut hex = String::new();
                    while let Some(c) = self.cur() {
                        if c.is_ascii_hexdigit() { hex.push(c); self.advance(); }
                        else if c == '_' { self.advance(); }
                        else { break; }
                    }
                    match i64::from_str_radix(&hex, 16) {
                        Ok(v) => return Token::Int(v),
                        Err(_) => {
                            match u64::from_str_radix(&hex, 16) {
                                Ok(v) => {
                                    self.warnings.push((self.line, format!("IntegerOverflow: hex literal 0x{} overflows i64; truncated", hex)));
                                    return Token::Int(v as i64);
                                }
                                Err(_) => {
                                    self.warnings.push((self.line, format!("IntegerOverflow: hex literal 0x{} is too large", hex)));
                                    return Token::Int(i64::MAX);
                                }
                            }
                        }
                    }
                }
                // Binary
                Some('b') | Some('B') => {
                    self.advance();
                    let mut bin = String::new();
                    while let Some(c) = self.cur() {
                        if c == '0' || c == '1' { bin.push(c); self.advance(); }
                        else if c == '_' { self.advance(); }
                        else { break; }
                    }
                    match i64::from_str_radix(&bin, 2) {
                        Ok(v) => return Token::Int(v),
                        Err(_) => {
                            self.warnings.push((self.line, format!("IntegerOverflow: binary literal 0b{} overflows i64", bin)));
                            return Token::Int(i64::MAX);
                        }
                    }
                }
                // Octal
                Some('o') | Some('O') => {
                    self.advance();
                    let mut oct = String::new();
                    while let Some(c) = self.cur() {
                        if ('0'..='7').contains(&c) { oct.push(c); self.advance(); }
                        else if c == '_' { self.advance(); }
                        else { break; }
                    }
                    match i64::from_str_radix(&oct, 8) {
                        Ok(v) => return Token::Int(v),
                        Err(_) => {
                            self.warnings.push((self.line, format!("IntegerOverflow: octal literal 0o{} overflows i64", oct)));
                            return Token::Int(i64::MAX);
                        }
                    }
                }
                _ => {}
            }
        }

        while let Some(c) = self.cur() {
            if c == '_' { self.advance(); continue; }
            if c == '.' {
                if self.peek() == Some('.') { break; }
                if is_float { break; }
                is_float = true;
            } else if c == 'e' || c == 'E' {
                is_float = true; s.push(c); self.advance();
                if self.cur() == Some('+') || self.cur() == Some('-') {
                    s.push(self.cur().unwrap()); self.advance();
                }
                continue;
            } else if !c.is_ascii_digit() { break; }
            s.push(c); self.advance();
        }

        if is_float {
            Token::Float(s.parse().unwrap_or(f64::NAN))
        } else {
            // Decimal integer — promote to Float on overflow rather than returning 0
            match s.parse::<i64>() {
                Ok(v) => Token::Int(v),
                Err(_) => {
                    // Try as f64 representation of a large integer
                    match s.parse::<f64>() {
                        Ok(f) => {
                            self.warnings.push((self.line, format!("IntegerOverflow: decimal literal {} overflows i64; using float", s)));
                            Token::Float(f)
                        }
                        Err(_) => {
                            self.warnings.push((self.line, format!("IntegerOverflow: decimal literal {} is too large", s)));
                            Token::Int(i64::MAX)
                        }
                    }
                }
            }
        }
    }

    fn keyword_or_ident(word: String) -> Token {
        match word.as_str() {
            "let"                       => Token::Let,
            "var"                       => Token::Var,
            "print"                     => Token::Print,
            "while"                     => Token::While,
            "for"                       => Token::For,
            "in"                        => Token::In,
            "if"                        => Token::If,
            "elif"                      => Token::Elif,
            "else"                      => Token::Else,
            "def" | "fn"               => Token::Def,
            "return"                    => Token::Return,
            "break"                     => Token::Break,
            "continue"                  => Token::Continue,
            "pass"                      => Token::Pass,
            "true"  | "True"            => Token::True,
            "false" | "False"           => Token::False,
            "null"  | "None" | "nil"    => Token::Null,
            "import"                    => Token::Import,
            "export"                    => Token::Export,
            "try"                       => Token::Try,
            "catch" | "except"          => Token::Catch,
            "throw" | "raise"           => Token::Throw,
            "finally"                   => Token::Finally,
            "switch"                    => Token::Switch,
            "match"                     => Token::Match,
            "case"                      => Token::Case,
            "default"                   => Token::Default,
            "not"                       => Token::Not,
            "and"                       => Token::And,
            "or"                        => Token::Or,
            "as"                        => Token::As,
            "is"                        => Token::EqEq, // Python identity → equality
            "struct"                    => Token::Struct,
            "enum"                      => Token::Enum,
            "lambda"                    => Token::Def,
            "assert"                    => Token::Assert,
            _                           => Token::Ident(word),
        }
    }
}
