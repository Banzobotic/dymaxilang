pub use token::{AtomKind, OpKind, Token, TokenKind};

mod token;

pub struct Lexer {
    program: String,
    start: usize,
    position: usize,
    line: u32,
    pub lines: Vec<usize>,
}

impl Lexer {
    pub fn new(program: String) -> Self {
        Self {
            program,
            start: 0,
            position: 0,
            line: 1,
            lines: vec![0],
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
        self.program
            .as_bytes()
            .get(self.position)
            .copied()
            .unwrap_or(b'\0') as char
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

    fn make_token(&mut self, kind: TokenKind) -> Result<Token, String> {
        Ok(Token::new(kind, self.line, self.start, self.position))
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
            'e' => check_keyword(1, "lse", TokenKind::Else),
            'f' => match cs.next().unwrap_or('\0') {
                'a' => check_keyword(2, "lse", TokenKind::Atom(AtomKind::False)),
                'n' => TokenKind::Atom(AtomKind::Fn),
                'o' => check_keyword(2, "r", TokenKind::For),
                _ => TokenKind::Atom(AtomKind::Ident),
            },
            'i' => match cs.next().unwrap_or('\0') {
                'f' => TokenKind::If,
                'n' => TokenKind::In,
                _ => TokenKind::Atom(AtomKind::Ident),
            },
            'l' => check_keyword(1, "et", TokenKind::Let),
            'n' => check_keyword(1, "ull", TokenKind::Atom(AtomKind::Null)),
            'r' => check_keyword(1, "eturn", TokenKind::Return),
            't' => check_keyword(1, "rue", TokenKind::Atom(AtomKind::True)),
            'w' => check_keyword(1, "hile", TokenKind::While),
            _ => TokenKind::Atom(AtomKind::Ident),
        }
    }

    fn identifier(&mut self) -> Result<Token, String> {
        while Self::is_alphanumeric(self.peek()) {
            self.advance();
        }

        self.make_token(self.identifier_type())
    }

    fn number(&mut self) -> Result<Token, String> {
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

    fn string(&mut self) -> Result<Token, String> {
        while self.peek() != '"' {
            if self.peek() == '\0' {
                return Err("string not closed".to_owned());
            }

            if self.advance() == '\n' {
                self.line += 1;
            }
        }
        self.advance();

        self.make_token(TokenKind::Atom(AtomKind::String))
    }

    pub fn next_token(&mut self) -> Result<Token, String> {
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
                '!' => {
                    if self.check('=') {
                        return self.make_token(TokenKind::Op(OpKind::BangEqual));
                    } else {
                        return self.make_token(TokenKind::Op(OpKind::Bang));
                    }
                }
                '>' => {
                    if self.check('=') {
                        return self.make_token(TokenKind::Op(OpKind::GreaterEqual));
                    } else {
                        return self.make_token(TokenKind::Op(OpKind::Greater));
                    }
                }
                '<' => {
                    if self.check('=') {
                        return self.make_token(TokenKind::Op(OpKind::LessEqual));
                    } else {
                        return self.make_token(TokenKind::Op(OpKind::Less));
                    }
                }
                '&' => {
                    if self.advance() != '&' {
                        return Err("use '&&' not '&'".to_owned());
                    }

                    return self.make_token(TokenKind::Op(OpKind::And));
                }
                '|' => {
                    if self.advance() != '|' {
                        return Err("use '||' not '|'".to_owned());
                    }

                    return self.make_token(TokenKind::Op(OpKind::Or));
                }
                '(' => return self.make_token(TokenKind::Op(OpKind::OpenParen)),
                ')' => return self.make_token(TokenKind::Op(OpKind::CloseParen)),
                '[' => return self.make_token(TokenKind::Op(OpKind::OpenSquare)),
                ']' => return self.make_token(TokenKind::Op(OpKind::CloseSquare)),
                '{' => return self.make_token(TokenKind::OpenBrace),
                '}' => return self.make_token(TokenKind::CloseBrace),
                'a'..='z' | 'A'..='Z' | '_' => return self.identifier(),
                '0'..='9' => return self.number(),
                '"' => return self.string(),
                ';' => return self.make_token(TokenKind::SemiColon),
                ',' => return self.make_token(TokenKind::Comma),
                '\n' => {
                    self.line += 1;
                    self.lines.push(self.position);
                }
                '\0' => return self.make_token(TokenKind::Eof),
                c if c.is_whitespace() => (),
                _ => return Err("unrecognised token".to_owned()),
            }
        }
    }

    pub fn get_token_string(&self, token: &Token) -> &str {
        &self.program[token.start..token.end]
    }
}
