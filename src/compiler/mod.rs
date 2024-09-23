use lexer::{AtomKind, Lexer, OpKind, Token, TokenKind};

use crate::vm::{
    chunk::{Chunk, OpCode},
    value::Value,
};

mod lexer;

struct Parser {
    lexer: lexer::Lexer,
    previous: Option<Token>,
    current: Token,
}

impl Parser {
    pub fn new(program: String) -> Self {
        let mut lexer = Lexer::new(program);
        let current = lexer.next_token();
        println!("Token: {:?}", current.kind);
        Parser {
            lexer,
            previous: None,
            current,
        }
    }

    pub fn previous(&mut self) -> Token {
        self.previous
            .expect("Can't access previous before advancing parser")
    }

    pub fn current(&mut self) -> Token {
        self.current
    }

    pub fn advance(&mut self) {
        self.previous = Some(self.current);

        self.current = self.lexer.next_token();
        println!("Token: {:?}", self.current.kind);
    }

    pub fn consume(&mut self, kind: TokenKind) {
        if self.current.kind != kind {
            panic!("Expected {:?}", kind)
        }

        self.advance();
    }

    pub fn compare_next(&mut self, kind: TokenKind) -> bool {
        self.current.kind == kind
    }

    pub fn check(&mut self, kind: TokenKind) -> bool {
        if !self.compare_next(kind) {
            return false;
        }
        self.advance();
        true
    }
}

pub struct Compiler {
    parser: Parser,
    chunk: Chunk,
}

impl Compiler {
    pub fn new(program: String) -> Self {
        Self {
            parser: Parser::new(program),
            chunk: Chunk::new(),
        }
    }

    fn number(&mut self) {
        let token = self.parser.previous();
        let value = self.parser.lexer.get_token_string(&token).parse().unwrap();
        self.chunk.push_constant(Value::float(value));
    }

    fn identifier(&mut self) {
        let token = self.parser.previous();
        let name = &self.parser.lexer.program()[token.start..token.end];
        let global_idx = self.chunk.get_global_idx(name);

        if self.parser.check(TokenKind::Op(OpKind::Equal)) {
            self.expression();
            self.chunk.push_opcode(OpCode::SetGlobal);
        } else {
            self.chunk.push_opcode(OpCode::GetGlobal);
            self.chunk.push_byte(global_idx);
        }
    }

    fn expression_bp(&mut self, min_bp: u8) {
        fn prefix_bp(op: OpKind) -> ((), u8) {
            match op {
                OpKind::Minus => ((), 15),
                _ => panic!("Can't use {:?} here", op),
            }
        }

        fn infix_bp(op: OpKind) -> Option<(u8, u8)> {
            let ret = match op {
                OpKind::Plus | OpKind::Minus => (11, 12),
                OpKind::Mul | OpKind::Div => (13, 14),
                _ => return None,
            };
            Some(ret)
        }

        self.parser.advance();
        match self.parser.previous().kind {
            TokenKind::Atom(it) => match it {
                AtomKind::Number => self.number(),
                AtomKind::Ident => self.identifier(),
                _ => unimplemented!(),
            },
            TokenKind::Op(OpKind::OpenParen) => {
                self.expression_bp(0);
                assert!(self.parser.check(TokenKind::Op(OpKind::CloseParen)));
            }
            TokenKind::Op(op) => {
                let ((), r_bp) = prefix_bp(op);
                self.expression_bp(r_bp);

                match op {
                    OpKind::Minus => self.chunk.push_opcode(OpCode::Negate),
                    _ => unreachable!("Error handled in prefix_bp call"),
                }
            }
            token => panic!("Unexpected token: {:?}", token),
        }

        while let TokenKind::Op(op) = self.parser.current().kind {
            if let Some((l_bp, r_bp)) = infix_bp(op) {
                if l_bp < min_bp {
                    break;
                }
                self.parser.advance();

                self.expression_bp(r_bp);

                match op {
                    OpKind::Plus => self.chunk.push_opcode(OpCode::Add),
                    OpKind::Minus => self.chunk.push_opcode(OpCode::Sub),
                    OpKind::Mul => self.chunk.push_opcode(OpCode::Mul),
                    OpKind::Div => self.chunk.push_opcode(OpCode::Div),
                    token => panic!("Unexpected token: {:?}", token),
                }

                continue;
            }

            break;
        }
    }

    fn expression(&mut self) {
        self.expression_bp(0);
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.chunk.push_opcode(OpCode::Pop);
    }

    fn print_statement(&mut self) {
        self.expression();
        self.chunk.push_opcode(OpCode::Print);
        self.parser.consume(TokenKind::SemiColon)
    }

    fn parse_variable(&mut self, error_message: &str) -> u8 {
        self.parser.consume(TokenKind::Atom(AtomKind::Ident));

        let ident = self.parser.previous();
        self.chunk
            .get_global_idx(&self.parser.lexer.program()[ident.start..ident.end])
    }

    fn define_variable(&mut self, global_idx: u8) {
        self.chunk.push_opcode(OpCode::DefineGlobal);
        self.chunk.push_byte(global_idx);
    }

    fn var_decl(&mut self) {
        let global_idx = self.parse_variable("Expect variable name");

        if self.parser.check(TokenKind::Op(OpKind::Equal)) {
            self.expression();
        } else {
            self.chunk.push_opcode(OpCode::Nil);
        }

        self.parser.consume(TokenKind::SemiColon);

        self.define_variable(global_idx);
    }

    fn statement(&mut self) {
        if self.parser.check(TokenKind::Let) {
            self.var_decl();
        } else if self.parser.check(TokenKind::Print) {
            self.print_statement();
        } else {
            self.expression_statement();
        }
    }

    pub fn compile(&mut self) -> Chunk {
        while !self.parser.compare_next(TokenKind::Eof) {
            self.statement();
        }

        self.chunk.push_opcode(OpCode::Return);

        self.chunk.clone()
    }
}