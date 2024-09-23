use chunk::Chunk;
use stack::Stack;
use value::Value;

pub mod chunk;
pub mod stack;
pub mod value;

pub struct VM {
    chunk: Chunk,
    stack: Stack,
}

impl VM {
    pub fn new(chunk: Chunk) -> Self {
        let stack = Stack::new();

        VM { chunk, stack }
    }

    pub fn run(&mut self) {
        macro_rules! binary_op {
            ($op:tt) => {
                {
                    let b = self.stack.pop();
                    let a = self.stack.pop();

                    unsafe { self.stack.push(Value::new_float(a.as_float() $op b.as_float())) };
                }
            };
        }
        
        loop {
            use chunk::OpCode as Op;
            match unsafe { self.chunk.next_opcode() } {
                Op::LoadConstant => self.stack.push(self.chunk.next_constant()),
                Op::Add => binary_op!(+),
                Op::Sub => binary_op!(-),
                Op::Mul => binary_op!(*),
                Op::Div => binary_op!(/),
                Op::Negate => {
                    if !self.stack.peek().is_float() {
                        panic!("Can only negate numbers");
                    }
                    unsafe {
                        let top_ptr = self.stack.top().sub(1);
                        top_ptr.write(Value::new_float(-top_ptr.read().as_float()))
                    }
                }
                Op::Return => {
                    println!("{}", self.stack.pop());
                    return;
                }
            }
        }
    }
}
