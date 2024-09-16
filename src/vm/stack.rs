use std::ptr::NonNull;

use super::value::Value;

pub struct Stack {
    // TODO: make stack use MaybeUninit data
    stack: [Value; 256],
    top: NonNull<Value>,
}

impl Stack {
    pub fn new() -> Self {
        let mut stack = [0; 256];
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
}
