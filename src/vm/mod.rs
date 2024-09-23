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

                    self.stack.push(Value::float(a.as_float() $op b.as_float()));
                }
            };
        }

        loop {
            use chunk::OpCode as Op;
            match unsafe { self.chunk.next_opcode() } {
                Op::LoadConstant => self.stack.push(self.chunk.next_constant()),
                Op::Nil => self.stack.push(Value::NIL),
                Op::Pop => {
                    self.stack.pop();
                }
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
                        top_ptr.write(Value::float(-top_ptr.read().as_float()))
                    }
                }
                Op::DefineGlobal => {
                    let idx = self.chunk.next_byte();
                    self.chunk.set_global(idx, self.stack.pop());
                }
                Op::GetGlobal => {
                    let idx = self.chunk.next_byte();
                    let value = self.chunk.get_global(idx);

                    if value.is_undef() {
                        panic!("Attempted to get value of undefined variable");
                    }

                    self.stack.push(value);
                }
                Op::SetGlobal => {
                    let idx = self.chunk.next_byte();
                    let prev_value = self.chunk.get_global(idx);

                    if prev_value.is_undef() {
                        panic!("Attemped to set value of undefined variable");
                    }

                    self.chunk.set_global(idx, self.stack.pop());
                }
                Op::Print => println!("{}", self.stack.pop()),
                Op::Return => return,
            }
        }
    }
}
