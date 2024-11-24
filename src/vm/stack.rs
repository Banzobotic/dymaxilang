use std::ptr::NonNull;

use super::value::Value;

pub struct Stack {
    #[allow(unused)]
    stack: Vec<Value>,
    pub top: NonNull<Value>,
    max_use: usize,
}

impl Stack {
    pub fn new() -> Self {
        let mut stack = Vec::with_capacity(256);
        let top = unsafe { NonNull::new_unchecked(stack.as_mut_ptr()) };

        Self {
            stack,
            top,
            max_use: 0,
        }
    }

    pub fn push(&mut self, val: Value) {
        unsafe {
            self.top.write(val);
            self.top = self.top.add(1);
        }
    }

    pub fn base(&self) -> *const Value {
        self.stack.as_ptr()
    }

    pub fn base_mut(&mut self) -> *mut Value {
        self.stack.as_mut_ptr()
    }

    pub fn allocate_slots(&mut self, slots: u32) {
        self.max_use += slots as usize;

        if self.max_use > self.stack.capacity() {
            let offset = unsafe { self.top.as_ptr().offset_from(self.base()) as usize };
            self.stack.reserve(self.stack.capacity() * 2);
            self.top = unsafe { NonNull::new_unchecked(self.base_mut().add(offset)) };
        }
    }

    pub fn free_slots(&mut self, slots: u32) {
        self.max_use -= slots as usize;
    }
}
