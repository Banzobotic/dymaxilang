use std::ptr::NonNull;

use super::{chunk::OpCode, object::Obj, value::Value};

pub struct CallFrame {
    pub function: Obj,
    ip: *const u8,
    pub fp_offset: usize,
}

impl CallFrame {
    pub fn new(function: Obj, stack_top: NonNull<Value>, stack_base: *const Value) -> Self {
        let ip = unsafe { (*function.function).chunk.code_ptr() };
        Self {
            function,
            ip,
            fp_offset: unsafe {
                stack_top
                    .as_ptr()
                    .sub((*function.function).arity as usize)
                    .offset_from(stack_base) as usize
            },
        }
    }

    pub fn next_constant(&mut self) -> Value {
        let byte = self.next_byte();
        unsafe { (*self.function.function).chunk.constants[byte as usize] }
    }

    /// # SAFETY
    ///
    /// Undefinied behaviour if the next byte isn't an OpCode
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

    #[cfg(any(feature = "decompile", feature = "trace_execution"))]
    pub fn current_offset(&self) -> usize {
        unsafe {
            self.ip
                .wrapping_sub((*self.function.function).chunk.code_ptr() as usize)
                as usize
        }
    }
}
