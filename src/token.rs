#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Keyword(Keyword),
    Ident(String),
    Int(i64),
    Float(f64),
    Str(String),
    Op(Op),
    Delim(Delim),
    Newline,
    Indent,
    Dedent,
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    If,
    Else,
    Elif,
    While,
    For,
    In,
    Def,
    Return,
    Break,
    Continue,
    And,
    Or,
    Not,
    True,
    False,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Plus,
    Minus,
    Star,
    Slash,
    FloorDiv,
    Percent,
    Pow,
    EqEq,
    NotEq,
    Lt,
    Gt,
    Le,
    Ge,
    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    Arrow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delim {
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Colon,
    Dot,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub line: usize,
}

impl Token {
    pub fn new(kind: TokenKind, text: String, line: usize) -> Self {
        Token { kind, text, line }
    }
}