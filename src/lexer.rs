use crate::error::LexError;
use crate::token::{Delim, Keyword, Op, Token, TokenKind};

pub fn tokenize(source: &str) -> Result<Vec<Token>, LexError> {
    let mut lx = Lexer::new(source);
    lx.run()
}

struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    tokens: Vec<Token>,
    indent_stack: Vec<usize>,
    at_line_start: bool,
}

impl Lexer {
    fn new(source: &str) -> Self {
        Lexer {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            tokens: Vec::new(),
            indent_stack: vec![0],
            at_line_start: true,
        }
    }

    fn run(&mut self) -> Result<Vec<Token>, LexError> {
        while !self.eof() {
            if self.at_line_start {
                self.handle_line_start()?;
                if self.eof() {
                    break;
                }
            }
            self.skip_inline_space();
            if self.eof() {
                break;
            }
            let c = self.peek();
            if c == '#' {
                self.skip_comment();
                continue;
            }
            if c == '\n' {
                self.advance();
                self.at_line_start = true;
                self.emit_newline();
                continue;
            }
            self.lex_token()?;
        }
        self.finish()?;
        Ok(std::mem::take(&mut self.tokens))
    }

    fn eof(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn peek(&self) -> char {
        self.chars[self.pos]
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> char {
        let c = self.chars[self.pos];
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
        }
        c
    }

    fn skip_inline_space(&mut self) {
        while !self.eof() && (self.peek() == ' ' || self.peek() == '\t') {
            self.advance();
        }
    }

    fn skip_comment(&mut self) {
        while !self.eof() && self.peek() != '\n' {
            self.advance();
        }
    }

    fn handle_line_start(&mut self) -> Result<(), LexError> {
        let mut spaces = 0usize;
        let mut saw_tab = false;
        while !self.eof() {
            match self.peek() {
                ' ' => {
                    spaces += 1;
                    self.advance();
                }
                '\t' => {
                    saw_tab = true;
                    self.advance();
                }
                _ => break,
            }
        }
        if self.eof() {
            return Ok(());
        }
        if self.peek() == '\n' {
            self.advance();
            self.at_line_start = true;
            return Ok(());
        }
        if self.peek() == '#' {
            self.skip_comment();
            if !self.eof() && self.peek() == '\n' {
                self.advance();
            }
            self.at_line_start = true;
            return Ok(());
        }
        if saw_tab {
            return Err(LexError {
                line: self.line,
                message: "inconsistent indentation".to_string(),
            });
        }
        if spaces % 4 != 0 {
            return Err(LexError {
                line: self.line,
                message: "inconsistent indentation".to_string(),
            });
        }
        let level = spaces / 4;
        let current = *self.indent_stack.last().unwrap();
        if level > current {
            self.indent_stack.push(level);
            self.tokens.push(Token::new(TokenKind::Indent, String::new(), self.line));
        } else if level < current {
            while *self.indent_stack.last().unwrap() > level {
                self.indent_stack.pop();
                self.tokens.push(Token::new(TokenKind::Dedent, String::new(), self.line));
            }
            if *self.indent_stack.last().unwrap() != level {
                return Err(LexError {
                    line: self.line,
                    message: "inconsistent indentation".to_string(),
                });
            }
        }
        self.at_line_start = false;
        Ok(())
    }

    fn emit_newline(&mut self) {
        if let Some(last) = self.tokens.last() {
            if matches!(last.kind, TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent) {
                return;
            }
        } else {
            return;
        }
        self.tokens.push(Token::new(TokenKind::Newline, "\n".to_string(), self.line - 1));
    }

    fn finish(&mut self) -> Result<(), LexError> {
        if let Some(last) = self.tokens.last() {
            if !matches!(last.kind, TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent) {
                let line = last.line;
                self.tokens.push(Token::new(TokenKind::Newline, "\n".to_string(), line));
            }
        }
        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            self.tokens.push(Token::new(TokenKind::Dedent, String::new(), self.line));
        }
        self.tokens.push(Token::new(TokenKind::Eof, String::new(), self.line));
        Ok(())
    }

    fn lex_token(&mut self) -> Result<(), LexError> {
        let c = self.peek();
        if c.is_ascii_digit() {
            return self.lex_number();
        }
        if c == '"' || c == '\'' {
            return self.lex_string(c);
        }
        if c.is_alphabetic() || c == '_' {
            return self.lex_ident_or_keyword();
        }
        self.lex_operator_or_delim()
    }

    fn lex_number(&mut self) -> Result<(), LexError> {
        let start = self.pos;
        let line = self.line;
        while !self.eof() && self.peek().is_ascii_digit() {
            self.advance();
        }
        let mut is_float = false;
        if !self.eof() && self.peek() == '.' {
            if let Some(next) = self.peek_at(1) {
                if next.is_ascii_digit() {
                    is_float = true;
                    self.advance();
                    while !self.eof() && self.peek().is_ascii_digit() {
                        self.advance();
                    }
                }
            }
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        if is_float {
            let v: f64 = text.parse().map_err(|_| LexError {
                line,
                message: format!("invalid float literal '{}'", text),
            })?;
            self.tokens.push(Token::new(TokenKind::Float(v), text, line));
        } else {
            let v: i64 = text.parse().map_err(|_| LexError {
                line,
                message: format!("invalid integer literal '{}'", text),
            })?;
            self.tokens.push(Token::new(TokenKind::Int(v), text, line));
        }
        Ok(())
    }

    fn lex_string(&mut self, quote: char) -> Result<(), LexError> {
        let line = self.line;
        self.advance();
        let mut buf = String::new();
        loop {
            if self.eof() {
                return Err(LexError {
                    line,
                    message: "unterminated string literal".to_string(),
                });
            }
            let c = self.peek();
            if c == '\n' {
                return Err(LexError {
                    line,
                    message: "unterminated string literal".to_string(),
                });
            }
            if c == quote {
                self.advance();
                break;
            }
            if c == '\\' {
                self.advance();
                if self.eof() {
                    return Err(LexError {
                        line,
                        message: "unterminated string literal".to_string(),
                    });
                }
                let esc = self.advance();
                match esc {
                    'n' => buf.push('\n'),
                    't' => buf.push('\t'),
                    '\\' => buf.push('\\'),
                    '"' => buf.push('"'),
                    '\'' => buf.push('\''),
                    other => {
                        return Err(LexError {
                            line: self.line,
                            message: format!("invalid escape sequence '\\{}'", other),
                        });
                    }
                }
            } else {
                buf.push(c);
                self.advance();
            }
        }
        self.tokens.push(Token::new(TokenKind::Str(buf.clone()), buf, line));
        Ok(())
    }

    fn lex_ident_or_keyword(&mut self) -> Result<(), LexError> {
        let start = self.pos;
        let line = self.line;
        while !self.eof() && (self.peek().is_alphanumeric() || self.peek() == '_') {
            self.advance();
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        if let Some(kw) = keyword_of(&text) {
            self.tokens.push(Token::new(TokenKind::Keyword(kw), text, line));
        } else {
            self.tokens.push(Token::new(TokenKind::Ident(text.clone()), text, line));
        }
        Ok(())
    }

    fn lex_operator_or_delim(&mut self) -> Result<(), LexError> {
        let line = self.line;
        let c = self.advance();
        let (kind, text) = match c {
            '+' => self.match_char('=', Op::PlusAssign, Op::Plus),
            '-' => {
                if self.peek_or_none() == Some('>') {
                    self.advance();
                    (TokenKind::Op(Op::Arrow), "->".to_string())
                } else {
                    self.match_char('=', Op::MinusAssign, Op::Minus)
                }
            }
            '*' => {
                if self.peek_or_none() == Some('*') {
                    self.advance();
                    (TokenKind::Op(Op::Pow), "**".to_string())
                } else {
                    self.match_char('=', Op::StarAssign, Op::Star)
                }
            }
            '/' => {
                if self.peek_or_none() == Some('/') {
                    self.advance();
                    (TokenKind::Op(Op::FloorDiv), "//".to_string())
                } else {
                    self.match_char('=', Op::SlashAssign, Op::Slash)
                }
            }
            '%' => (TokenKind::Op(Op::Percent), "%".to_string()),
            '=' => {
                if self.peek_or_none() == Some('=') {
                    self.advance();
                    (TokenKind::Op(Op::EqEq), "==".to_string())
                } else {
                    (TokenKind::Op(Op::Assign), "=".to_string())
                }
            }
            '!' => {
                if self.peek_or_none() == Some('=') {
                    self.advance();
                    (TokenKind::Op(Op::NotEq), "!=".to_string())
                } else {
                    return Err(LexError {
                        line,
                        message: "unexpected character '!'".to_string(),
                    });
                }
            }
            '<' => {
                if self.peek_or_none() == Some('=') {
                    self.advance();
                    (TokenKind::Op(Op::Le), "<=".to_string())
                } else {
                    (TokenKind::Op(Op::Lt), "<".to_string())
                }
            }
            '>' => {
                if self.peek_or_none() == Some('=') {
                    self.advance();
                    (TokenKind::Op(Op::Ge), ">=".to_string())
                } else {
                    (TokenKind::Op(Op::Gt), ">".to_string())
                }
            }
            '(' => (TokenKind::Delim(Delim::LParen), "(".to_string()),
            ')' => (TokenKind::Delim(Delim::RParen), ")".to_string()),
            '[' => (TokenKind::Delim(Delim::LBracket), "[".to_string()),
            ']' => (TokenKind::Delim(Delim::RBracket), "]".to_string()),
            '{' => (TokenKind::Delim(Delim::LBrace), "{".to_string()),
            '}' => (TokenKind::Delim(Delim::RBrace), "}".to_string()),
            ',' => (TokenKind::Delim(Delim::Comma), ",".to_string()),
            ':' => (TokenKind::Delim(Delim::Colon), ":".to_string()),
            '.' => (TokenKind::Delim(Delim::Dot), ".".to_string()),
            other => {
                return Err(LexError {
                    line,
                    message: format!("unexpected character '{}'", other),
                });
            }
        };
        self.tokens.push(Token::new(kind, text, line));
        Ok(())
    }

    fn peek_or_none(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn match_char(&mut self, expected: char, with_eq: Op, without: Op) -> (TokenKind, String) {
        if self.peek_or_none() == Some(expected) {
            self.advance();
            let text = format!("{}{}", op_symbol(without), expected);
            (TokenKind::Op(with_eq), text)
        } else {
            (TokenKind::Op(without), op_symbol(without).to_string())
        }
    }
}

fn op_symbol(op: Op) -> &'static str {
    match op {
        Op::Plus => "+",
        Op::Minus => "-",
        Op::Star => "*",
        Op::Slash => "/",
        Op::FloorDiv => "//",
        Op::Percent => "%",
        Op::Pow => "**",
        Op::EqEq => "==",
        Op::NotEq => "!=",
        Op::Lt => "<",
        Op::Gt => ">",
        Op::Le => "<=",
        Op::Ge => ">=",
        Op::Assign => "=",
        Op::PlusAssign => "+=",
        Op::MinusAssign => "-=",
        Op::StarAssign => "*=",
        Op::SlashAssign => "/=",
        Op::Arrow => "->",
    }
}

fn keyword_of(text: &str) -> Option<Keyword> {
    match text {
        "if" => Some(Keyword::If),
        "else" => Some(Keyword::Else),
        "elif" => Some(Keyword::Elif),
        "while" => Some(Keyword::While),
        "for" => Some(Keyword::For),
        "in" => Some(Keyword::In),
        "def" => Some(Keyword::Def),
        "return" => Some(Keyword::Return),
        "break" => Some(Keyword::Break),
        "continue" => Some(Keyword::Continue),
        "and" => Some(Keyword::And),
        "or" => Some(Keyword::Or),
        "not" => Some(Keyword::Not),
        "true" => Some(Keyword::True),
        "false" => Some(Keyword::False),
        "none" => Some(Keyword::None),
        _ => None,
    }
}