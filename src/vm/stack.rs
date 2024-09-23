use std::ptr::NonNull;

use super::value::Value;

pub struct Stack {
    #[allow(unused)]
    stack: Vec<Value>,
    top: NonNull<Value>,
}

impl Stack {
    pub fn new() -> Self {
        let mut stack = Vec::with_capacity(256);
        let top = unsafe { NonNull::new_unchecked(stack.as_mut_ptr()) };

        Self { stack, top }
    }

    pub fn push(&mut self, val: Value) {
        unsafe { 
            self.top.write(val);
            self.top = self.top.add(1);
        }
    }

    pub fn pop(&mut self) -> Value {
        unsafe {
            self.top = self.top.sub(1);
            self.top.read()
        }
    }

    pub fn peek(&mut self) -> Value {
        unsafe {
            self.top.read()
        }
    }

    pub fn top(&self) -> NonNull<Value> {
        self.top
    }
}
