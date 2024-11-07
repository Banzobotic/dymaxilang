use lexer::{AtomKind, Lexer, OpKind, Token, TokenKind};

use crate::vm::{
    chunk::OpCode, object::ObjString, value::Value, VM
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

struct Local {
    name: String,
    depth: Option<u32>,
}

pub struct Compiler {
    parser: Parser,
    locals: Vec<Local>,
    scope_depth: u32,
    vm: VM,
}

impl Compiler {
    pub fn new(program: String) -> Self {
        Self {
            parser: Parser::new(program),
            locals: Vec::new(),
            scope_depth: 0,
            vm: VM::new(),
        }
    }

    fn number(&mut self) {
        let token = self.parser.previous();
        let value = self.parser.lexer.get_token_string(&token).parse().unwrap();
        self.vm.chunk.push_constant(Value::float(value));
    }

    fn string(&mut self) {
        let token = self.parser.previous();
        let value = self.parser.lexer.get_token_string(&token);
        let obj = ObjString::new(&value[1..value.len() - 1]);
        let obj = self.vm.alloc(obj);
        self.vm.chunk.push_constant(Value::obj(obj));
    }

    fn resolve_local(&self, name: &str) -> Option<u8> {
        for (i, local) in self.locals.iter().enumerate().rev() {
            if name == local.name {
                if local.depth.is_none() {
                    panic!("Can't reference local in its own initialiser");
                }

                return Some(i as u8);
            }
        }

        None
    }

    fn identifier(&mut self) {
        let (get_op, set_op);
        let name = &self
            .parser
            .previous()
            .lexeme_str(self.parser.lexer.program());
        let mut arg = self.resolve_local(name);

        match arg {
            Some(_) => {
                get_op = OpCode::GetLocal;
                set_op = OpCode::SetLocal;
            }
            None => {
                arg = Some(self.vm.chunk.get_global_idx(name));
                get_op = OpCode::GetGlobal;
                set_op = OpCode::SetGlobal;
            }
        }

        if self.parser.check(TokenKind::Op(OpKind::Equal)) {
            self.expression();
            self.vm.chunk.push_opcode(set_op);
            self.vm.chunk.push_byte(arg.unwrap());
        } else {
            self.vm.chunk.push_opcode(get_op);
            self.vm.chunk.push_byte(arg.unwrap());
        }
    }

    fn expression_bp(&mut self, min_bp: u8) {
        fn prefix_bp(op: OpKind) -> ((), u8) {
            match op {
                OpKind::Bang => ((), 15),
                OpKind::Minus => ((), 15),
                _ => panic!("Can't use {:?} here", op),
            }
        }

        fn infix_bp(op: OpKind) -> Option<(u8, u8)> {
            let ret = match op {
                OpKind::DoubleEqual | OpKind::BangEqual => (7, 8),
                OpKind::Greater | OpKind::GreaterEqual | OpKind::Less | OpKind::LessEqual => {
                    (9, 10)
                }
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
                AtomKind::String => self.string(),
                AtomKind::Ident => self.identifier(),
                AtomKind::True => self.vm.chunk.push_constant(Value::TRUE),
                AtomKind::False => self.vm.chunk.push_constant(Value::FALSE),
                AtomKind::Null => self.vm.chunk.push_constant(Value::NULL),
            },
            TokenKind::Op(OpKind::OpenParen) => {
                self.expression_bp(0);
                assert!(self.parser.check(TokenKind::Op(OpKind::CloseParen)));
            }
            TokenKind::Op(op) => {
                let ((), r_bp) = prefix_bp(op);
                self.expression_bp(r_bp);

                match op {
                    OpKind::Bang => self.vm.chunk.push_opcode(OpCode::Not),
                    OpKind::Minus => self.vm.chunk.push_opcode(OpCode::Negate),
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
                    OpKind::Plus => self.vm.chunk.push_opcode(OpCode::Add),
                    OpKind::Minus => self.vm.chunk.push_opcode(OpCode::Sub),
                    OpKind::Mul => self.vm.chunk.push_opcode(OpCode::Mul),
                    OpKind::Div => self.vm.chunk.push_opcode(OpCode::Div),
                    OpKind::DoubleEqual => self.vm.chunk.push_opcode(OpCode::Equal),
                    OpKind::BangEqual => self.vm.chunk.push_opcode(OpCode::NotEqual),
                    OpKind::Greater => self.vm.chunk.push_opcode(OpCode::Greater),
                    OpKind::GreaterEqual => self.vm.chunk.push_opcode(OpCode::GreaterEqual),
                    OpKind::Less => self.vm.chunk.push_opcode(OpCode::Less),
                    OpKind::LessEqual => self.vm.chunk.push_opcode(OpCode::LessEqual),
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
        self.parser.consume(TokenKind::SemiColon);
        self.vm.chunk.push_opcode(OpCode::Pop);
    }

    fn print_statement(&mut self) {
        self.expression();
        self.vm.chunk.push_opcode(OpCode::Print);
        self.parser.consume(TokenKind::SemiColon)
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.scope_depth -= 1;

        while let Some(local) = self.locals.last() {
            if local.depth.unwrap() <= self.scope_depth {
                break;
            }

            self.vm.chunk.push_opcode(OpCode::Pop);
            self.locals.pop();
        }
    }

    fn block(&mut self) {
        while !self.parser.compare_next(TokenKind::CloseBrace)
            && !self.parser.compare_next(TokenKind::Eof)
        {
            self.statement();
        }

        self.parser.consume(TokenKind::CloseBrace)
    }

    fn add_local(&mut self, name: String) {
        if self.locals.len() == 256 {
            panic!("Too many local variables");
        }

        self.locals.push(Local { name, depth: None });
    }

    fn declare_variable(&mut self) {
        if self.scope_depth == 0 {
            return;
        }

        let name = self
            .parser
            .previous()
            .lexeme_str(self.parser.lexer.program());

        for local in self.locals.iter().rev() {
            if local.depth.unwrap() < self.scope_depth {
                break;
            }

            if name == local.name {
                panic!("Already a variable with this name in this scope.");
            }
        }

        self.add_local(name.to_owned());
    }

    fn parse_variable(&mut self, error_message: &str) -> u8 {
        self.parser.consume(TokenKind::Atom(AtomKind::Ident));

        self.declare_variable();
        if self.scope_depth > 0 {
            return 0;
        }

        self.vm.chunk.get_global_idx(
            self.parser
                .previous()
                .lexeme_str(self.parser.lexer.program()),
        )
    }

    fn mark_initialised(&mut self) {
        self.locals.last_mut().unwrap().depth = Some(self.scope_depth);
    }

    fn define_variable(&mut self, global_idx: u8) {
        if self.scope_depth > 0 {
            self.mark_initialised();
            return;
        }

        self.vm.chunk.push_opcode(OpCode::DefineGlobal);
        self.vm.chunk.push_byte(global_idx);
    }

    fn var_decl(&mut self) {
        let global_idx = self.parse_variable("Expect variable name");

        if self.parser.check(TokenKind::Op(OpKind::Equal)) {
            self.expression();
        } else {
            self.vm.chunk.push_opcode(OpCode::Nil);
        }

        self.parser.consume(TokenKind::SemiColon);

        self.define_variable(global_idx);
    }

    fn statement(&mut self) {
        if self.parser.check(TokenKind::Let) {
            self.var_decl();
        } else if self.parser.check(TokenKind::Print) {
            self.print_statement();
        } else if self.parser.check(TokenKind::OpenBrace) {
            self.begin_scope();
            self.block();
            self.end_scope();
        } else {
            self.expression_statement();
        }
    }

    pub fn compile(mut self) -> VM {
        while !self.parser.compare_next(TokenKind::Eof) {
            self.statement();
        }

        self.vm.chunk.push_opcode(OpCode::Return);

        self.vm
    }
}
