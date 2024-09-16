use std::ptr::NonNull;

#[repr(u8)]
pub enum OpCode {
    LoadConstant,
    Add,
    Sub,
    Return,
}

pub struct Chunk {
    stack: Vec<u8>,
    ip: NonNull<u8>,
}

impl Chunk {
    pub fn new() -> Self {
        let mut stack = Vec::with_capacity(8);
        // SAFETY: stack is already initialised so pointer to it is non-null
        let ip = unsafe { NonNull::new_unchecked(stack.as_mut_ptr()) };

        Self { stack, ip }
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
}
