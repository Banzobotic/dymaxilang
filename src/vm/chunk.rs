use std::{collections::HashMap, ptr::NonNull};

use super::value::Value;

#[repr(u8)]
pub enum OpCode {
    LoadConstant,
    Nil,
    Pop,
    Add,
    Sub,
    Mul,
    Div,
    Negate,
    DefineGlobal,
    GetGlobal,
    SetGlobal,
    Print,
    Return,
}

#[derive(Clone, Debug)]
pub struct Chunk {
    stack: Vec<u8>,
    ip: NonNull<u8>,
    constants: Vec<Value>,
    globals: Vec<Value>,
    global_names: HashMap<String, u8>,
}

impl Chunk {
    pub fn new() -> Self {
        let mut stack = Vec::with_capacity(8);
        // SAFETY: stack is already initialised so pointer to it is non-null
        let ip = unsafe { NonNull::new_unchecked(stack.as_mut_ptr()) };

        Self {
            stack,
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

    pub fn push_opcode(&mut self, opcode: OpCode) {
        self.push_byte(opcode as u8);
    }

    pub fn push_byte(&mut self, byte: u8) {
        let at_capacity = self.stack.len() == self.stack.capacity();

        self.stack.push(byte);

        // if vector has been grown then it may be at a different location in memory
        if at_capacity {
            // SAFETY: stack is initialised so pointer to it is non-null
            self.ip = unsafe { NonNull::new_unchecked(self.stack.as_mut_ptr()) }
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
}
