#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Op(OpKind),
    Atom(AtomKind),
    SemiColon,
    Fn,
    For,
    Let,
    Nil,
    Print,
    Return,
    While,
    Eof,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpKind {
    Plus,
    Minus,
    Mul,
    Div,
    Equal,
    DoubleEqual,
    OpenParen,
    CloseParen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtomKind {
    Ident,
    Number,
    String,
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
}
