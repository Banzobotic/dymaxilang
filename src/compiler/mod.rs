use std::{ptr::NonNull, time::SystemTime};

use lexer::{AtomKind, Lexer, OpKind, Token, TokenKind};

use crate::vm::{
    chunk::{Chunk, OpCode},
    object::{NativeFn, ObjFunction, ObjNative, ObjString},
    value::Value,
    VM,
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

struct CompilingFunction {
    function: ObjFunction,
    locals: Vec<Local>,
    scope_depth: u32,
    current_stack_effect: u32,
    peak_stack_effect: u32,
    is_function: bool,
}

impl CompilingFunction {
    pub fn new(is_function: bool) -> Self {
        Self {
            function: ObjFunction::new(),
            locals: Vec::new(),
            scope_depth: 0,
            current_stack_effect: 10,
            peak_stack_effect: 10,
            is_function,
        }
    }
}

pub struct Compiler {
    vm: VM,
    parser: Parser,
    function_stack: Vec<CompilingFunction>,
}

impl Compiler {
    pub fn new(program: String) -> Self {
        Self {
            vm: VM::new(),
            parser: Parser::new(program),
            function_stack: vec![CompilingFunction::new(false)],
        }
    }

    fn push_fn(&mut self) {
        self.function_stack.push(CompilingFunction::new(true));
    }

    fn pop_fn(&mut self) {
        self.chunk_mut().push_opcode(OpCode::Null);
        self.chunk_mut().push_opcode(OpCode::Return);
        let stack_effect = self.function_stack.last().unwrap().peak_stack_effect;
        let mut func = self.function_stack.pop().unwrap().function;
        func.stack_effect = stack_effect;
        let func = self.vm.alloc(func);
        self.chunk_mut().push_constant(Value::obj(func));
    }

    fn add_stack_effect(&mut self, effect: u32) {
        let function = self.function_stack.last_mut().unwrap();
        function.current_stack_effect += effect;
        function.peak_stack_effect =
            u32::max(function.current_stack_effect, function.peak_stack_effect);
    }

    fn remove_stack_effect(&mut self, effect: u32) {
        let function = self.function_stack.last_mut().unwrap();
        function.current_stack_effect -= effect;
    }

    fn locals(&self) -> &Vec<Local> {
        &self.function_stack.last().unwrap().locals
    }

    fn locals_mut(&mut self) -> &mut Vec<Local> {
        &mut self.function_stack.last_mut().unwrap().locals
    }

    fn scope_depth(&self) -> u32 {
        self.function_stack.last().unwrap().scope_depth
    }

    fn current(&mut self) -> &mut ObjFunction {
        &mut self.function_stack.last_mut().unwrap().function
    }

    fn chunk_mut(&mut self) -> &mut Chunk {
        &mut self.current().chunk
    }

    fn integer(&mut self) {
        let token = self.parser.previous();
        let value: f64 = self.parser.lexer.get_token_string(&token).parse().unwrap();
        if value != value.round() {
            panic!("Number must be an integer");
        }
        self.chunk_mut().push_constant(Value::float(value))
    }

    fn number(&mut self) {
        let token = self.parser.previous();
        let value = self.parser.lexer.get_token_string(&token).parse().unwrap();
        self.chunk_mut().push_constant(Value::float(value));
    }

    fn string(&mut self) {
        let token = self.parser.previous();
        let value = self.parser.lexer.get_token_string(&token);
        let obj = ObjString::new(&value[1..value.len() - 1]);
        let obj = self.vm.alloc(obj);
        self.chunk_mut().push_constant(Value::obj(obj));
    }

    fn resolve_local(&self, name: &str) -> Option<u8> {
        for (i, local) in self.locals().iter().enumerate().rev() {
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
                arg = Some(self.vm.globals.get_global_idx(name));
                get_op = OpCode::GetGlobal;
                set_op = OpCode::SetGlobal;
            }
        }

        if self.parser.check(TokenKind::Op(OpKind::Equal)) {
            self.expression();
            self.chunk_mut().push_opcode(set_op);
            self.chunk_mut().push_byte(arg.unwrap());
        } else {
            self.chunk_mut().push_opcode(get_op);
            self.chunk_mut().push_byte(arg.unwrap());
        }
    }

    fn function(&mut self) {
        self.push_fn();
        self.begin_scope();

        self.parser.consume(TokenKind::Op(OpKind::OpenParen));
        if !self.parser.compare_next(TokenKind::Op(OpKind::CloseParen)) {
            loop {
                self.current().arity += 1;
                if self.current().arity > 255 {
                    panic!("Can't have more than 255 parameters");
                }
                self.parse_variable("Expected parameter");
                self.mark_initialised();

                if !self.parser.check(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.parser.consume(TokenKind::Op(OpKind::CloseParen));
        self.parser.consume(TokenKind::OpenBrace);
        self.block();

        self.pop_fn();
    }

    fn call(&mut self) {
        let mut arg_count = 0;
        if !self.parser.compare_next(TokenKind::Op(OpKind::CloseParen)) {
            loop {
                if arg_count == u8::MAX {
                    panic!("Can't have more than 255 arguments");
                }
                arg_count += 1;

                self.expression();

                if !self.parser.check(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.parser.consume(TokenKind::Op(OpKind::CloseParen));

        self.chunk_mut().push_opcode(OpCode::Call);
        self.chunk_mut().push_byte(arg_count);
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
                OpKind::Or => (3, 4),
                OpKind::And => (5, 6),
                OpKind::DoubleEqual | OpKind::BangEqual => (7, 8),
                OpKind::Greater | OpKind::GreaterEqual | OpKind::Less | OpKind::LessEqual => {
                    (9, 10)
                }
                OpKind::Plus | OpKind::Minus => (11, 12),
                OpKind::Mul | OpKind::Div => (13, 14),
                OpKind::OpenParen => (15, 16),
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
                AtomKind::True => self.chunk_mut().push_constant(Value::TRUE),
                AtomKind::False => self.chunk_mut().push_constant(Value::FALSE),
                AtomKind::Null => self.chunk_mut().push_opcode(OpCode::Null),
                AtomKind::Fn => self.function(),
            },
            TokenKind::Op(OpKind::OpenParen) => {
                self.expression_bp(0);
                assert!(self.parser.check(TokenKind::Op(OpKind::CloseParen)));
            }
            TokenKind::Op(op) => {
                let ((), r_bp) = prefix_bp(op);
                self.expression_bp(r_bp);

                match op {
                    OpKind::Bang => self.chunk_mut().push_opcode(OpCode::Not),
                    OpKind::Minus => self.chunk_mut().push_opcode(OpCode::Negate),
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

                if op == OpKind::And {
                    let jump = self.chunk_mut().push_jump(OpCode::JumpIfFalseNoPop);
                    self.chunk_mut().push_opcode(OpCode::Pop);
                    self.expression_bp(r_bp);
                    self.chunk_mut().patch_jump(jump);
                    continue;
                } else if op == OpKind::Or {
                    let jump = self.chunk_mut().push_jump(OpCode::JumpIfTrueNoPop);
                    self.chunk_mut().push_opcode(OpCode::Pop);
                    self.expression_bp(r_bp);
                    self.chunk_mut().patch_jump(jump);
                    continue;
                } else if op == OpKind::OpenParen {
                    self.call();
                    continue;
                }

                self.expression_bp(r_bp);

                match op {
                    OpKind::Plus => self.chunk_mut().push_opcode(OpCode::Add),
                    OpKind::Minus => self.chunk_mut().push_opcode(OpCode::Sub),
                    OpKind::Mul => self.chunk_mut().push_opcode(OpCode::Mul),
                    OpKind::Div => self.chunk_mut().push_opcode(OpCode::Div),
                    OpKind::DoubleEqual => self.chunk_mut().push_opcode(OpCode::Equal),
                    OpKind::BangEqual => self.chunk_mut().push_opcode(OpCode::NotEqual),
                    OpKind::Greater => self.chunk_mut().push_opcode(OpCode::Greater),
                    OpKind::GreaterEqual => self.chunk_mut().push_opcode(OpCode::GreaterEqual),
                    OpKind::Less => self.chunk_mut().push_opcode(OpCode::Less),
                    OpKind::LessEqual => self.chunk_mut().push_opcode(OpCode::LessEqual),
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
        self.chunk_mut().push_opcode(OpCode::Pop);
    }

    fn return_statement(&mut self) {
        if !self.function_stack.last().unwrap().is_function {
            panic!("Can only return from functions");
        }

        if self.parser.compare_next(TokenKind::SemiColon) {
            self.chunk_mut().push_opcode(OpCode::Null);
        } else {
            self.expression();
        }
        self.chunk_mut().push_opcode(OpCode::Return);
        self.parser.consume(TokenKind::SemiColon);
    }

    fn begin_scope(&mut self) {
        self.function_stack.last_mut().unwrap().scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.function_stack.last_mut().unwrap().scope_depth += 1;

        while let Some(local) = self.locals().last() {
            if local.depth.unwrap() <= self.scope_depth() {
                break;
            }

            self.chunk_mut().push_opcode(OpCode::Pop);
            self.locals_mut().pop();
            self.remove_stack_effect(1);
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
        if self.locals().len() == 256 {
            panic!("Too many local variables");
        }

        self.locals_mut().push(Local { name, depth: None });
    }

    fn declare_variable(&mut self) {
        if self.scope_depth() == 0 {
            return;
        }

        let name = self
            .parser
            .previous()
            .lexeme_str(self.parser.lexer.program());

        for local in self.locals().iter().rev() {
            if local.depth.unwrap() < self.scope_depth() {
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
        if self.scope_depth() > 0 {
            return 0;
        }

        self.vm.globals.get_global_idx(
            self.parser
                .previous()
                .lexeme_str(self.parser.lexer.program()),
        )
    }

    fn mark_initialised(&mut self) {
        self.add_stack_effect(1);
        self.locals_mut().last_mut().unwrap().depth = Some(self.scope_depth());
    }

    fn define_variable(&mut self, global_idx: u8) {
        if self.scope_depth() > 0 {
            self.mark_initialised();
            return;
        }

        self.chunk_mut().push_opcode(OpCode::DefineGlobal);
        self.chunk_mut().push_byte(global_idx);
    }

    fn var_decl(&mut self) {
        let global_idx = self.parse_variable("Expect variable name");

        if self.parser.check(TokenKind::Op(OpKind::Equal)) {
            self.expression();
        } else {
            self.chunk_mut().push_opcode(OpCode::Null);
        }

        self.parser.consume(TokenKind::SemiColon);

        self.define_variable(global_idx);
    }

    fn if_statement(&mut self) {
        self.expression();
        self.parser.consume(TokenKind::OpenBrace);
        let jump = self.chunk_mut().push_jump(OpCode::JumpIfFalse);
        
        self.block();
        if self.parser.check(TokenKind::Else) {
            let else_jump = self.chunk_mut().push_jump(OpCode::Jump);
            self.chunk_mut().patch_jump(jump);
            self.parser.consume(TokenKind::OpenBrace);
            self.block();
            self.chunk_mut().patch_jump(else_jump);
        } else {
            self.chunk_mut().patch_jump(jump);
        }
    }

    fn for_loop(&mut self) {
        self.begin_scope();
        self.parser.consume(TokenKind::Atom(AtomKind::Ident));
        self.declare_variable();

        self.parser.consume(TokenKind::In);
        self.parser.consume(TokenKind::Atom(AtomKind::Number));
        self.integer();
        self.mark_initialised();

        let start = self.chunk_mut().jump_target();

        let var_idx = (self.locals().len() - 1) as u8;
        self.chunk_mut().push_opcode(OpCode::GetLocal);
        self.chunk_mut().push_byte(var_idx);

        let op;
        if self.parser.check(TokenKind::Op(OpKind::Greater)) {
            op = OpCode::Less;
        } else if self.parser.check(TokenKind::Op(OpKind::GreaterEqual)) {
            op = OpCode::LessEqual;
        } else {
            panic!("Must use either '>' or '>=' in for loop");
        }

        self.parser.consume(TokenKind::Atom(AtomKind::Number));
        self.integer();
        self.chunk_mut().push_opcode(op);
        let jump = self.chunk_mut().push_jump(OpCode::JumpIfFalse);

        self.parser.consume(TokenKind::OpenBrace);
        self.block();

        self.chunk_mut().push_opcode(OpCode::GetLocal);
        self.chunk_mut().push_byte(var_idx);
        self.chunk_mut().push_constant(Value::float(1.0));
        self.chunk_mut().push_opcode(OpCode::Add);
        self.chunk_mut().push_opcode(OpCode::SetLocal);
        self.chunk_mut().push_byte(var_idx);
        self.chunk_mut().push_opcode(OpCode::Pop);

        self.chunk_mut().push_loop(start);
        self.chunk_mut().patch_jump(jump);
        self.end_scope();
    }

    fn while_loop(&mut self) {
        let start = self.chunk_mut().jump_target();
        self.expression();

        let jump = self.chunk_mut().push_jump(OpCode::JumpIfFalse);

        self.parser.consume(TokenKind::OpenBrace);
        self.begin_scope();
        self.block();
        self.end_scope();

        self.chunk_mut().push_loop(start);

        self.chunk_mut().patch_jump(jump);
    }

    fn statement(&mut self) {
        if self.parser.check(TokenKind::While) {
            self.while_loop();
        } else if self.parser.check(TokenKind::For) {
            self.for_loop();
        } else if self.parser.check(TokenKind::If) {
            self.if_statement();
        } else if self.parser.check(TokenKind::Let) {
            self.var_decl();
        } else if self.parser.check(TokenKind::Return) {
            self.return_statement();
        } else if self.parser.check(TokenKind::OpenBrace) {
            self.begin_scope();
            self.block();
            self.end_scope();
        } else {
            self.expression_statement();
        }
    }

    fn define_native(&mut self, name: &str, native: NativeFn) {
        let native = ObjNative::new(native);
        let native = self.vm.alloc(native);
        let idx = self.vm.globals.get_global_idx(name);
        self.vm.globals.set(idx, Value::obj(native));
    }

    fn define_natives(&mut self) {
        self.define_native("clock", clock_native);
        self.define_native("print", print_native);
    }

    pub fn compile(mut self) -> VM {
        self.define_natives();

        while !self.parser.compare_next(TokenKind::Eof) {
            self.statement();
        }

        self.chunk_mut().push_opcode(OpCode::Null);
        self.chunk_mut().push_opcode(OpCode::Return);

        #[cfg(feature = "decompile")]
        self.chunk_mut().disassemble();

        let function = self.function_stack.pop().unwrap().function;
        let function = self.vm.alloc(function);

        self.vm.push_call_frame(function);

        self.vm
    }
}

fn clock_native(_arg_count: u32, _args: NonNull<Value>) -> Value {
    Value::float(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64(),
    )
}

fn print_native(_arg_count: u32, args: NonNull<Value>) -> Value {
    println!("{}", unsafe { args.read() });
    Value::NULL
}
