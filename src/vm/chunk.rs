use std::{collections::HashMap, ptr::NonNull};

use super::value::Value;

#[repr(u8)]
#[derive(Debug)]
pub enum OpCode {
    LoadConstant,
    Nil,
    Pop,
    Add,
    Sub,
    Mul,
    Div,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Not,
    Negate,
    DefineGlobal,
    GetGlobal,
    SetGlobal,
    GetLocal,
    SetLocal,
    Print,
    Return,
    Jump,
    JumpIfFalse,
}

#[derive(Clone, Debug)]
pub struct Chunk {
    code: Vec<u8>,
    ip: NonNull<u8>,
    pub constants: Vec<Value>,
    pub globals: Vec<Value>,
    pub global_names: HashMap<String, u8>,
}

impl Chunk {
    pub fn new() -> Self {
        let mut code = Vec::with_capacity(8);
        // SAFETY: stack is already initialised so pointer to it is non-null
        let ip = unsafe { NonNull::new_unchecked(code.as_mut_ptr()) };

        Self {
            code,
            ip,
            constants: Vec::new(),
            globals: Vec::new(),
            global_names: HashMap::new(),
        }
    }

    fn add_constant(&mut self, constant: Value) -> u8 {
        self.constants.push(constant);
        self.constants.len() as u8 - 1
    }

    pub fn push_constant(&mut self, constant: Value) {
        let idx = self.add_constant(constant);
        self.push_opcode(OpCode::LoadConstant);
        self.push_byte(idx);
    }

    pub fn push_jump(&mut self, opcode: OpCode) -> usize {
        self.push_opcode(opcode);
        self.push_byte(0xFF);
        self.jump_target() - 1
    }

    pub fn push_loop(&mut self, target: usize) {
        let offset = (target as isize - self.code.len() as isize - 2) as u8;
        self.push_opcode(OpCode::Jump);
        self.push_byte(offset);
    }

    pub fn patch_jump(&mut self, jump_idx: usize) {
        let offset = (self.code.len() as isize - jump_idx as isize - 1) as u8;
        self.code[jump_idx] = offset;
    }

    pub fn jump_target(&self) -> usize {
        self.code.len()
    }

    pub fn push_opcode(&mut self, opcode: OpCode) {
        self.push_byte(opcode as u8);
    }

    pub fn push_byte(&mut self, byte: u8) {
        let at_capacity = self.code.len() == self.code.capacity();

        self.code.push(byte);

        // if vector has been grown then it may be at a different location in memory
        if at_capacity {
            // SAFETY: stack is initialised so pointer to it is non-null
            self.ip = unsafe { NonNull::new_unchecked(self.code.as_mut_ptr()) }
        }
    }

    pub fn next_constant(&mut self) -> Value {
        let byte = self.next_byte();
        self.constants[byte as usize]
    }

    /// # SAFETY
    ///
    /// Undefinied behaviour if the next byte wasn't an OpCode
    pub unsafe fn next_opcode(&mut self) -> OpCode {
        std::mem::transmute(self.next_byte())
    }

    pub fn next_byte(&mut self) -> u8 {
        // SAFETY: if the compiler and vm are implemented correctly, pc will never point to uninitialised memory
        let byte = unsafe { self.ip.read() };
        self.ip = unsafe { self.ip.add(1) };
        byte
    }

    pub fn jump(&mut self, offset: u8) {
        self.ip = unsafe { self.ip.offset(offset as i8 as isize) }
    }

    pub fn get_global_idx(&mut self, name: &str) -> u8 {
        match self.global_names.get(name) {
            Some(idx) => *idx,
            None => {
                let len = self.globals.len() as u8;
                self.global_names.insert(name.to_owned(), len);
                self.globals.push(Value::UNDEF);
                len
            }
        }
    }

    pub fn get_global(&mut self, idx: u8) -> Value {
        self.globals[idx as usize]
    }

    pub fn set_global(&mut self, idx: u8, value: Value) {
        self.globals[idx as usize] = value;
    }

    #[cfg(any(feature = "decompile", feature = "trace_execution"))]
    pub fn current_offset(&self) -> usize {
        unsafe { self.ip.sub(self.code.as_ptr() as usize).as_ptr() as usize }
    }

    #[cfg(any(feature = "decompile", feature = "trace_execution"))]
    pub fn disassemble_instruction(&self, offset: usize) -> usize {
        print!("{:04X} ", offset);

        use OpCode as Op;
        match unsafe { std::mem::transmute::<u8, OpCode>(self.code[offset]) } {
            op @ (Op::Nil
            | Op::Pop
            | Op::Add
            | Op::Sub
            | Op::Mul
            | Op::Div
            | Op::Equal
            | Op::NotEqual
            | Op::Greater
            | Op::GreaterEqual
            | Op::Less
            | Op::LessEqual
            | Op::Not
            | Op::Negate
            | Op::Print
            | Op::Return) => {
                println!("{:?}", op);
                offset + 1
            }
            op @ (Op::LoadConstant
            | Op::DefineGlobal
            | Op::GetGlobal
            | Op::SetGlobal
            | Op::GetLocal
            | Op::SetLocal
            | Op::Jump
            | Op::JumpIfFalse) => {
                let constant = self.code[offset + 1];
                println!("{:16} {:04X}", format!("{:?}", op), constant);
                offset + 2
            }
        }
    }

    #[cfg(feature = "decompile")]
    pub fn disassemble(&self) {
        let mut offset = 0;
        while offset < self.code.len() {
            offset = self.disassemble_instruction(offset);
        }
    }
}
