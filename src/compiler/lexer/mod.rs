pub use token::{AtomKind, OpKind, Token, TokenKind};

mod token;

pub struct Lexer {
    program: String,
    start: usize,
    position: usize,
    line: u32,
}

impl Lexer {
    pub fn new(program: String) -> Self {
        Self {
            program,
            start: 0,
            position: 0,
            line: 1,
        }
    }

    pub fn program(&self) -> &str {
        &self.program
    }

    fn is_alpha(c: char) -> bool {
        matches!(c, 'A'..='Z' | 'a'..='z' | '_')
    }

    fn is_numeric(c: char) -> bool {
        c.is_ascii_digit()
    }

    fn is_alphanumeric(c: char) -> bool {
        Self::is_alpha(c) || Self::is_numeric(c)
    }

    fn peek(&mut self) -> char {
        self.program[self.position..].chars().next().unwrap_or('\0')
    }

    fn advance(&mut self) -> char {
        let c = self.peek();
        self.position += c.len_utf8();
        c
    }

    fn check(&mut self, c: char) -> bool {
        if c == self.peek() {
            self.position += c.len_utf8();
            true
        } else {
            false
        }
    }

    fn make_token(&mut self, kind: TokenKind) -> Token {
        Token::new(kind, self.line, self.start, self.position)
    }

    fn identifier_type(&self) -> TokenKind {
        let identifier = &self.program[self.start..self.position];

        let check_keyword = |start, rest, kind| {
            if &identifier[start..] == rest {
                kind
            } else {
                TokenKind::Atom(AtomKind::Ident)
            }
        };

        let mut cs = identifier.chars();
        match cs.next().unwrap() {
            'f' => match cs.next().unwrap() {
                'n' => TokenKind::Fn,
                'o' => check_keyword(2, "r", TokenKind::For),
                _ => TokenKind::Atom(AtomKind::Ident),
            },
            'l' => check_keyword(1, "et", TokenKind::Let),
            'n' => check_keyword(1, "il", TokenKind::Nil),
            'p' => check_keyword(1, "rint", TokenKind::Print),
            'r' => check_keyword(1, "eturn", TokenKind::Return),
            'w' => check_keyword(1, "hile", TokenKind::While),
            _ => TokenKind::Atom(AtomKind::Ident),
        }
    }

    fn identifier(&mut self) -> Token {
        while Self::is_alphanumeric(self.peek()) {
            self.advance();
        }

        self.make_token(self.identifier_type())
    }

    fn number(&mut self) -> Token {
        while Self::is_numeric(self.peek()) {
            self.advance();
        }

        if self.check('.') {
            while Self::is_numeric(self.peek()) {
                self.advance();
            }
        }

        self.make_token(TokenKind::Atom(AtomKind::Number))
    }

    fn string(&mut self) -> Token {
        while self.peek() != '"' {
            if self.advance() == '\n' {
                self.line += 1;
            }
        }

        self.make_token(TokenKind::Atom(AtomKind::String))
    }

    pub fn next_token(&mut self) -> Token {
        loop {
            self.start = self.position;
            match self.advance() {
                '+' => return self.make_token(TokenKind::Op(OpKind::Plus)),
                '-' => return self.make_token(TokenKind::Op(OpKind::Minus)),
                '*' => return self.make_token(TokenKind::Op(OpKind::Mul)),
                '/' => {
                    if self.check('/') {
                        while self.peek() != '\n' {
                            self.advance();
                        }
                    } else {
                        return self.make_token(TokenKind::Op(OpKind::Div));
                    }
                }
                '=' => {
                    if self.check('=') {
                        return self.make_token(TokenKind::Op(OpKind::DoubleEqual));
                    } else {
                        return self.make_token(TokenKind::Op(OpKind::Equal));
                    }
                }
                '{' => return self.make_token(TokenKind::OpenBrace),
                '}' => return self.make_token(TokenKind::CloseBrace),
                '(' => return self.make_token(TokenKind::Op(OpKind::OpenParen)),
                ')' => return self.make_token(TokenKind::Op(OpKind::CloseParen)),
                'a'..='z' | 'A'..='Z' | '_' => return self.identifier(),
                '0'..='9' => return self.number(),
                '"' => return self.string(),
                ';' => return self.make_token(TokenKind::SemiColon),
                '\n' => {
                    self.line += 1;
                }
                '\0' => return self.make_token(TokenKind::Eof),
                c if c.is_whitespace() => (),
                _ => panic!("Unrecognised token"),
            }
        }
    }

    pub fn get_token_string(&self, token: &Token) -> &str {
        &self.program[token.start..token.end]
    }
}
