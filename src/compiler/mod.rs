use lexer::{AtomKind, Lexer, OpKind, Token, TokenKind};

use crate::vm::{
    chunk::{Chunk, OpCode},
    object::{NativeFn, ObjFunction, ObjNative, ObjString},
    value::Value,
    VM,
};

mod lexer;
mod natives;

struct Parser {
    lexer: lexer::Lexer,
    previous: Option<Token>,
    current: Token,
    had_error: bool,
    handling_error: bool,
}

impl Parser {
    pub fn new(program: String) -> Self {
        let mut lexer = Lexer::new(program);
        let current = lexer.next_token().unwrap();
        Parser {
            lexer,
            previous: None,
            current,
            had_error: false,
            handling_error: false,
        }
    }

    pub fn previous(&self) -> Token {
        self.previous
            .expect("Can't access previous before advancing parser")
    }

    pub fn current(&self) -> Token {
        self.current
    }

    pub fn error_at(&mut self, start: usize, end: usize, line: u32, message: &str) {
        if self.handling_error {
            return;
        }
        self.handling_error = true;

        let line_start = self.lexer.lines[line as usize - 1];
        eprintln!(
            "\x1b[91merror\x1b[0m at [{}:{}]: {message}",
            line,
            start - line_start + 1
        );

        let line_end = if self.lexer.lines.len() > line as usize {
            self.lexer.lines[line as usize]
        } else {
            'outer: {
                for (i, c) in self.lexer.program().char_indices().skip(line_start) {
                    if c == '\n' {
                        break 'outer i + 1;
                    }
                }
                self.lexer.program().len()
            }
        };

        eprintln!("    | ");
        eprint!(
            "{:^4}| {}",
            line,
            &self.lexer.program()[line_start..line_end]
        );
        eprint!("    | ");
        for _ in line_start..start {
            eprint!(" ");
        }
        for _ in start..end {
            eprint!("^");
        }
        eprintln!();
        eprintln!("    | \n");

        self.had_error = true;
    }

    pub fn error_bad_token(&mut self, message: &str) {
        self.error_at(
            self.previous().end,
            self.previous().end + 1,
            self.previous().line,
            message,
        );
    }

    pub fn error(&mut self, message: &str) {
        self.error_at(
            self.previous().start,
            self.previous().end,
            self.previous().line,
            message,
        );
    }

    fn sync(&mut self) {
        self.handling_error = false;
        let mut scope_count = 0;

        while self.current().kind != TokenKind::Eof {
            match self.previous().kind {
                TokenKind::SemiColon if scope_count == 0 => return,
                TokenKind::OpenBrace => scope_count += 1,
                TokenKind::CloseBrace => {
                    scope_count -= 1;
                    if scope_count <= 0 {
                        self.check(TokenKind::SemiColon);
                        return;
                    }
                }
                _ => (),
            }

            match self.current().kind {
                TokenKind::While
                | TokenKind::For
                | TokenKind::If
                | TokenKind::Return
                | TokenKind::Let => return,
                _ => (),
            }

            self.advance();
        }
    }

    pub fn advance(&mut self) {
        self.previous = Some(self.current);

        self.current = loop {
            let token = self.lexer.next_token();

            match token {
                Ok(token) => break token,
                Err(message) => self.error_bad_token(&message),
            }
        };
    }

    pub fn consume(&mut self, kind: TokenKind, error_message: &str) {
        if self.current.kind != kind {
            self.error_bad_token(error_message);
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

#[derive(Debug)]
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
    #[cfg(feature = "local_map_scopes")]
    map_set: Vec<(usize, bool)>,
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
            #[cfg(feature = "local_map_scopes")]
            map_set: Vec::new(),
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

    pub fn push_constant(&mut self, constant: Value) {
        let idx = self.chunk_mut().add_constant(constant);
        if idx <= u8::MAX as usize {
            self.push_opcode(OpCode::LoadConstant);
            self.push_byte(idx as u8);
        } else {
            self.push_opcode(OpCode::LoadConstantExt);
            self.push_byte(((idx >> 16) & 0xFF) as u8);
            self.push_byte(((idx >> 8) & 0xFF) as u8);
            self.push_byte((idx & 0xFF) as u8);
        }
    }

    #[cfg(feature = "local_map_scopes")]
    pub fn push_map(&mut self, target: usize) {
        let line = self.parser.previous().line;
        self.chunk_mut().push_map(target, line);
    }

    pub fn push_jump(&mut self, opcode: OpCode) -> usize {
        self.push_opcode(opcode);
        self.push_byte(0xFF);
        self.push_byte(0xFF);
        self.chunk().jump_target() - 2
    }

    pub fn push_loop(&mut self, target: usize) {
        let offset = self.chunk().jump_target() - target + 3;

        if offset > u16::MAX as usize {
            self.parser.error("loop too long");
        }
        
        self.push_opcode(OpCode::JumpUp);
        self.push_byte((offset >> 8) as u8);
        self.push_byte((offset & 0xFF) as u8)
    }

    fn push_opcode(&mut self, op: OpCode) {
        self.push_byte(op as u8);
    }

    fn push_byte(&mut self, byte: u8) {
        let line = self.parser.previous().line;
        self.chunk_mut().push_byte(byte, line);
    }

    fn push_fn(&mut self) {
        self.function_stack.push(CompilingFunction::new(true));
    }

    fn pop_fn(&mut self) {
        self.push_opcode(OpCode::Null);
        self.push_opcode(OpCode::Return);
        let stack_effect = self.function_stack.last().unwrap().peak_stack_effect;
        let mut func = self.function_stack.pop().unwrap().function;
        func.stack_effect = stack_effect;
        let func = self.vm.alloc(func);
        self.push_constant(Value::obj(func));
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

    fn chunk(&self) -> &Chunk {
        &self.function_stack.last().unwrap().function.chunk
    }

    fn chunk_mut(&mut self) -> &mut Chunk {
        &mut self.current().chunk
    }

    fn integer(&mut self) {
        let token = self.parser.previous();
        let value: f64 = self.parser.lexer.get_token_string(&token).parse().unwrap();
        if value != value.round() {
            self.parser.error("number must be an integer");
        }
        self.push_constant(Value::float(value))
    }

    fn number(&mut self) {
        let token = self.parser.previous();
        let value = self.parser.lexer.get_token_string(&token).parse().unwrap();
        self.push_constant(Value::float(value));
    }

    fn string(&mut self) {
        let token = self.parser.previous();
        let value = self.parser.lexer.get_token_string(&token);
        let Ok(value) = escape_bytes::unescape(value.as_bytes()).map(|v| String::from_utf8(v).unwrap()) else {
            self.parser.error("invalid escape in string");
            return;
        };
        let obj = ObjString::new(&value[1..value.len() - 1]);
        let obj = self.vm.alloc(obj);
        self.push_constant(Value::obj(obj));
    }

    fn resolve_local(&mut self, name: &str) -> Option<u8> {
        for (i, local) in self.locals().iter().enumerate().rev() {
            if name == local.name {
                if local.depth.is_none() {
                    self.parser
                        .error("can't reference local in its own initialiser");
                }

                return Some(i as u8);
            }
        }

        None
    }

    fn identifier(&mut self) {
        let (get_op, set_op);
        let name = self
            .parser
            .previous()
            .lexeme_str(self.parser.lexer.program())
            .to_owned();
        let mut arg = self.resolve_local(&name);

        match arg {
            Some(_) => {
                get_op = OpCode::GetLocal;
                set_op = OpCode::SetLocal;
            }
            None => {
                arg = Some(self.vm.globals.get_global_idx(&name));
                get_op = OpCode::GetGlobal;
                set_op = OpCode::SetGlobal;
            }
        }

        if self.parser.check(TokenKind::Op(OpKind::Equal)) {
            self.expression();
            self.push_opcode(set_op);
            self.push_byte(arg.unwrap());
        } else {
            self.push_opcode(get_op);
            self.push_byte(arg.unwrap());
        }
    }

    fn map_access(&mut self) {
        self.expression();
        self.parser.consume(
            TokenKind::Op(OpKind::CloseSquare),
            "expected ']' after map access",
        );

        if self.parser.check(TokenKind::Op(OpKind::Equal)) {
            #[cfg(feature = "local_map_scopes")]
            if let Some(set) = self.function_stack.last_mut().unwrap().map_set.last_mut() {
                *set = (set.0, true);
            }

            self.expression();
            self.push_opcode(OpCode::SetMap);
        } else {
            self.push_opcode(OpCode::GetMap);
        }
    }

    fn function(&mut self) {
        self.push_fn();
        self.begin_scope();

        self.parser.consume(
            TokenKind::Op(OpKind::OpenParen),
            "expected '(' to enclose arguments in function definition",
        );
        if !self.parser.compare_next(TokenKind::Op(OpKind::CloseParen)) {
            loop {
                self.current().arity += 1;
                if self.current().arity > 255 {
                    self.parser.error("can't have more than 255 parameters");
                }
                self.parse_variable("expected parameter");
                self.mark_initialised();

                if !self.parser.check(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.parser.consume(
            TokenKind::Op(OpKind::CloseParen),
            "expected ')' after function arguments",
        );
        self.parser
            .consume(TokenKind::OpenBrace, "expected '{' after arguments");
        self.block();

        #[cfg(feature = "local_map_scopes")]
        self.finish_map_scope();
        self.pop_fn();
    }

    fn call(&mut self) {
        let mut arg_count = 0;
        if !self.parser.compare_next(TokenKind::Op(OpKind::CloseParen)) {
            loop {
                if arg_count == u8::MAX {
                    self.parser.error("can't have more than 255 arguments");
                }
                arg_count += 1;

                self.expression();

                if !self.parser.check(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.parser.consume(
            TokenKind::Op(OpKind::CloseParen),
            "expected ')' after arguments to function call",
        );

        self.push_opcode(OpCode::Call);
        self.push_byte(arg_count);
    }

    fn expression_bp(&mut self, min_bp: u8) {
        fn prefix_bp(op: OpKind) -> Option<((), u8)> {
            Some(match op {
                OpKind::Bang => ((), 15),
                OpKind::Minus => ((), 15),
                _ => return None,
            })
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
                OpKind::OpenParen | OpKind::OpenSquare => (17, 18),
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
                AtomKind::True => self.push_constant(Value::TRUE),
                AtomKind::False => self.push_constant(Value::FALSE),
                AtomKind::Null => self.push_opcode(OpCode::Null),
                AtomKind::Fn => self.function(),
            },
            TokenKind::Op(OpKind::OpenParen) => {
                self.expression_bp(0);
                assert!(self.parser.check(TokenKind::Op(OpKind::CloseParen)));
            }
            TokenKind::Op(op) => {
                if let Some(((), r_bp)) = prefix_bp(op) {
                    self.expression_bp(r_bp);

                    match op {
                        OpKind::Bang => self.push_opcode(OpCode::Not),
                        OpKind::Minus => self.push_opcode(OpCode::Negate),
                        _ => unreachable!("Non prefix operator returned from prefix_bp"),
                    }
                } else {
                    self.parser.error(&format!(
                        "'{}' is not a prefix operator",
                        self.parser
                            .previous()
                            .lexeme_str(self.parser.lexer.program())
                    ))
                }
            }
            _ => self.parser.error(&format!(
                "'{}' can't be used in an expression",
                self.parser
                    .previous()
                    .lexeme_str(self.parser.lexer.program())
            )),
        }

        while let TokenKind::Op(op) = self.parser.current().kind {
            if let Some((l_bp, r_bp)) = infix_bp(op) {
                if l_bp < min_bp {
                    break;
                }
                self.parser.advance();

                if op == OpKind::And {
                    let jump = self.push_jump(OpCode::JumpIfFalseNoPop);
                    self.push_opcode(OpCode::Pop);
                    self.expression_bp(r_bp);
                    self.chunk_mut().patch_jump(jump);
                    continue;
                } else if op == OpKind::Or {
                    let jump = self.push_jump(OpCode::JumpIfTrueNoPop);
                    self.push_opcode(OpCode::Pop);
                    self.expression_bp(r_bp);
                    self.chunk_mut().patch_jump(jump);
                    continue;
                } else if op == OpKind::OpenParen {
                    self.call();
                    continue;
                } else if op == OpKind::OpenSquare {
                    self.map_access();
                    continue;
                }

                self.expression_bp(r_bp);

                match op {
                    OpKind::Plus => self.push_opcode(OpCode::Add),
                    OpKind::Minus => self.push_opcode(OpCode::Sub),
                    OpKind::Mul => self.push_opcode(OpCode::Mul),
                    OpKind::Div => self.push_opcode(OpCode::Div),
                    OpKind::DoubleEqual => self.push_opcode(OpCode::Equal),
                    OpKind::BangEqual => self.push_opcode(OpCode::NotEqual),
                    OpKind::Greater => self.push_opcode(OpCode::Greater),
                    OpKind::GreaterEqual => self.push_opcode(OpCode::GreaterEqual),
                    OpKind::Less => self.push_opcode(OpCode::Less),
                    OpKind::LessEqual => self.push_opcode(OpCode::LessEqual),
                    _ => unreachable!("{:?} not handled", op),
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
        self.parser
            .consume(TokenKind::SemiColon, "expected ';' after expression");
        self.push_opcode(OpCode::Pop);
    }

    fn return_statement(&mut self) {
        if !self.function_stack.last().unwrap().is_function {
            self.parser.error("can't use return when not in a function");
        }

        if self.parser.compare_next(TokenKind::SemiColon) {
            self.push_opcode(OpCode::Null);
        } else {
            self.expression();
        }
        self.push_opcode(OpCode::Return);
        self.parser
            .consume(TokenKind::SemiColon, "expected ';' after return statement");
    }

    #[cfg(feature = "local_map_scopes")]
    fn open_map_scope(&mut self) {
        let target = self.chunk_mut().jump_target();
        self.function_stack
            .last_mut()
            .unwrap()
            .map_set
            .push((target, false));
    }

    #[cfg(feature = "local_map_scopes")]
    fn finish_map_scope(&mut self) {
        let (target, map_set) = self
            .function_stack
            .last_mut()
            .unwrap()
            .map_set
            .pop()
            .unwrap();
        if map_set {
            self.push_map(target);
        }
    }

    fn begin_scope(&mut self) {
        self.function_stack.last_mut().unwrap().scope_depth += 1;

        #[cfg(feature = "local_map_scopes")]
        self.open_map_scope();
    }

    fn end_scope(&mut self) {
        self.function_stack.last_mut().unwrap().scope_depth -= 1;

        while let Some(local) = self.locals().last() {
            if local.depth.unwrap() <= self.scope_depth() {
                break;
            }

            self.push_opcode(OpCode::Pop);
            self.locals_mut().pop();
            self.remove_stack_effect(1);
        }

        #[cfg(feature = "local_map_scopes")]
        self.finish_map_scope();
    }

    fn block(&mut self) {
        while !self.parser.compare_next(TokenKind::CloseBrace)
            && !self.parser.compare_next(TokenKind::Eof)
        {
            self.statement();
        }

        self.parser
            .consume(TokenKind::CloseBrace, "expected '}' after block");
    }

    fn add_local(&mut self, name: String) {
        if self.locals().len() == 256 {
            self.parser
                .error("can't have more than 256 local variables per function");
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
            .lexeme_str(self.parser.lexer.program())
            .to_owned();

        let mut had_error = false;
        for local in self.locals().iter().rev() {
            if local.depth.unwrap() < self.scope_depth() {
                break;
            }

            if name == local.name {
                had_error = true;
            }
        }
        if had_error {
            self.parser.error(&format!(
                "there is already a variable with name '{}' in this scope",
                self.parser
                    .previous()
                    .lexeme_str(self.parser.lexer.program())
            ));
        }

        self.add_local(name.to_owned());
    }

    fn parse_variable(&mut self, error_message: &str) -> u8 {
        self.parser
            .consume(TokenKind::Atom(AtomKind::Ident), error_message);

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

        self.push_opcode(OpCode::DefineGlobal);
        self.push_byte(global_idx);
    }

    fn var_decl(&mut self) {
        let global_idx = self.parse_variable("expected variable name");

        if self.parser.check(TokenKind::Op(OpKind::Equal)) {
            self.expression();
        } else {
            self.push_opcode(OpCode::Null);
        }

        self.parser.consume(
            TokenKind::SemiColon,
            "expected ';' after variable declaration",
        );

        self.define_variable(global_idx);
    }

    fn if_statement(&mut self) {
        self.expression();
        self.parser
            .consume(TokenKind::OpenBrace, "expected '{' after condition");
        let jump = self.push_jump(OpCode::JumpIfFalse);

        self.begin_scope();
        self.block();
        self.end_scope();

        if self.parser.check(TokenKind::Else) {
            let else_jump = self.push_jump(OpCode::Jump);
            self.chunk_mut().patch_jump(jump);
            self.parser
                .consume(TokenKind::OpenBrace, "expected '{' after else");
            self.begin_scope();
            self.block();
            self.end_scope();
            self.chunk_mut().patch_jump(else_jump);
        } else {
            self.chunk_mut().patch_jump(jump);
        }
    }

    fn for_loop(&mut self) {
        self.begin_scope();
        self.parser.consume(
            TokenKind::Atom(AtomKind::Ident),
            "expected loop variable name",
        );
        self.declare_variable();

        self.parser
            .consume(TokenKind::In, "expected 'in' after loop variable");
        if self.parser.check(TokenKind::Atom(AtomKind::Number)) {
            self.integer();
        } else if self.parser.check(TokenKind::Atom(AtomKind::Ident)) {
            self.identifier();
        } else {
            self.parser.error("expected either integer or identifer for start of range");
        }

        let start = self.chunk_mut().jump_target();


        let var_idx = (self.locals().len() - 1) as u8;
        self.push_opcode(OpCode::GetLocal);
        self.push_byte(var_idx);

        let op = if self.parser.check(TokenKind::Op(OpKind::Greater)) {
            OpCode::Less
        } else if self.parser.check(TokenKind::Op(OpKind::GreaterEqual)) {
            OpCode::LessEqual
        } else {
            self.parser
                .error("must use either '>' or '>=' in for loop range");
            return;
        };

        if self.parser.check(TokenKind::Atom(AtomKind::Number)) {
            self.integer();
        } else if self.parser.check(TokenKind::Atom(AtomKind::Ident)) {
            self.identifier();
        } else {
            self.parser.error("expected either integer or identifer for end of range");
        }
        self.push_opcode(op);
        self.mark_initialised();
        let jump = self.push_jump(OpCode::JumpIfFalse);

        self.begin_scope();
        self.parser
            .consume(TokenKind::OpenBrace, "expected '{' after range");
        self.block();

        self.push_opcode(OpCode::GetLocal);
        self.push_byte(var_idx);
        self.push_constant(Value::float(1.0));
        self.push_opcode(OpCode::Add);
        self.push_opcode(OpCode::SetLocal);
        self.push_byte(var_idx);
        self.push_opcode(OpCode::Pop);
        self.end_scope();

        self.push_loop(start);
        self.chunk_mut().patch_jump(jump);
        self.end_scope();
    }

    fn while_loop(&mut self) {
        let start = self.chunk_mut().jump_target();
        self.expression();

        let jump = self.push_jump(OpCode::JumpIfFalse);

        self.parser
            .consume(TokenKind::OpenBrace, "expected '{' after condition");
        self.begin_scope();
        self.block();
        self.end_scope();

        self.push_loop(start);

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

        if self.parser.handling_error {
            self.parser.sync();
        }
    }

    fn define_native(&mut self, name: &str, native: NativeFn) {
        let native = ObjNative::new(native);
        let native = self.vm.alloc(native);
        let idx = self.vm.globals.get_global_idx(name);
        self.vm.globals.set(idx, Value::obj(native));
    }

    fn define_natives(&mut self) {
        use natives::*;

        self.define_native("time", native_time);
        self.define_native("print", native_print);
        self.define_native("read", native_read);
        self.define_native("num", native_num);
        self.define_native("abs", native_abs);
        self.define_native("split", native_split);
        self.define_native("split_into", native_split_into);
        self.define_native("chars", native_chars);
        self.define_native("chars_into", native_chars_into);
        self.define_native("sort", native_sort);
    }

    pub fn compile(mut self) -> VM {
        self.define_natives();

        while !self.parser.compare_next(TokenKind::Eof) {
            self.statement();
        }

        self.push_opcode(OpCode::Null);
        self.push_opcode(OpCode::Return);

        #[cfg(feature = "decompile")]
        self.chunk_mut().disassemble();

        if self.parser.had_error {
            std::process::exit(101);
        }

        let function = self.function_stack.pop().unwrap().function;
        let function = self.vm.alloc(function);

        self.vm.push_call_frame(function);

        self.vm
    }
}
