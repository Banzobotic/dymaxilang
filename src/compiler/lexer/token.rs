#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Op(OpKind),
    Atom(AtomKind),
    SemiColon,
    OpenBrace,
    CloseBrace,
    Fn,
    For,
    In,
    Let,
    Print,
    Return,
    While,
    Eof,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpKind {
    Bang,
    Plus,
    Minus,
    Mul,
    Div,
    Equal,
    DoubleEqual,
    BangEqual,
    GreaterEqual,
    LessEqual,
    Greater,
    Less,
    And,
    Or,
    OpenParen,
    CloseParen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtomKind {
    Ident,
    Number,
    String,
    True,
    False,
    Null,
}

#[derive(Clone, Copy)]
pub struct Token {
    pub kind: TokenKind,
    pub line: u32,
    pub start: usize,
    pub end: usize,
}

impl Token {
    pub fn new(kind: TokenKind, line: u32, start: usize, end: usize) -> Self {
        Self {
            kind,
            line,
            start,
            end,
        }
    }

    pub fn lexeme_str<'a>(&self, program: &'a str) -> &'a str {
        &program[self.start..self.end]
    }
}
