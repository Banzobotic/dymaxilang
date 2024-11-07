use chunk::Chunk;
use stack::Stack;
use value::Value;

pub mod chunk;
pub mod gc;
pub mod object;
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

        macro_rules! equality_op {
            ($op:tt) => {
                {
                    let b = self.stack.pop();
                    let a = self.stack.pop();

                    // TODO: fix when objects added
                    self.stack.push(Value::bool(a.value $op b.value));
                }
            }
        }

        macro_rules! comparison_op {
            ($op:tt) => {
                {
                    let b = self.stack.pop();
                    let a = self.stack.pop();

                    if !a.is_float() || !b.is_float() {
                        panic!("Can only compare floats");
                    }

                    self.stack.push(Value::bool(a.as_float() $op b.as_float()));
                }
            };
        }

        loop {
            use chunk::OpCode as Op;
            match unsafe { self.chunk.next_opcode() } {
                Op::LoadConstant => self.stack.push(self.chunk.next_constant()),
                Op::Nil => self.stack.push(Value::NULL),
                Op::Pop => {
                    self.stack.pop();
                }
                Op::Add => binary_op!(+),
                Op::Sub => binary_op!(-),
                Op::Mul => binary_op!(*),
                Op::Div => binary_op!(/),
                Op::Equal => equality_op!(==),
                Op::NotEqual => equality_op!(!=),
                Op::Greater => comparison_op!(>),
                Op::GreaterEqual => comparison_op!(>=),
                Op::Less => comparison_op!(<),
                Op::LessEqual => comparison_op!(<=),
                Op::Negate => {
                    if !self.stack.peek(0).is_float() {
                        panic!("Can only negate numbers");
                    }
                    unsafe {
                        let top_ptr = self.stack.top().sub(1);
                        top_ptr.write(Value::float(-top_ptr.read().as_float()));
                    }
                }
                Op::Not => {
                    if !self.stack.peek(0).is_bool() {
                        panic!("Can only not boolean values");
                    }
                    unsafe {
                        let top_ptr = self.stack.top().sub(1);
                        top_ptr.write(Value::bool(!top_ptr.read().as_bool()));
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

                    self.chunk.set_global(idx, self.stack.peek(0));
                }
                Op::GetLocal => unsafe {
                    self.stack.push(
                        self.stack
                            .base()
                            .add(self.chunk.next_byte() as usize)
                            .read(),
                    );
                },
                Op::SetLocal => unsafe {
                    self.stack
                        .base_mut()
                        .add(self.chunk.next_byte() as usize)
                        .write(self.stack.peek(0));
                },
                Op::Print => println!("{}", self.stack.pop()),
                Op::Return => return,
            }
        }
    }
}
