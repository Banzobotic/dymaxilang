use chunk::Chunk;
use gc::{GCAlloc, GC};
use object::{Obj, ObjString};
use stack::Stack;
use value::Value;

pub mod chunk;
pub mod gc;
pub mod object;
pub mod stack;
pub mod value;

pub struct VM {
    pub chunk: Chunk,
    gc: GC,
    stack: Stack,
}

impl VM {
    pub fn new() -> Self {
        VM {
            chunk: Chunk::new(),
            gc: GC::new(),
            stack: Stack::new(),
        }
    }

    pub fn alloc<T>(&mut self, obj: impl GCAlloc<T>) -> Obj {
        self.run_gc();
        self.gc.alloc(obj)
    }

    fn run_gc(&mut self) {
        if self.gc.should_gc() {
            #[cfg(feature = "debug_gc")]
            println!("--- GC START ---");

            self.mark_roots();
            self.gc.collect();

            #[cfg(feature = "debug_gc")]
            println!("--- GC END ---");
        }
    }

    fn mark_roots(&mut self) {
        let mut stack_ptr = self.stack.base();
        while stack_ptr != self.stack.top().as_ptr() {
            unsafe {
                self.gc.mark(*stack_ptr);
                stack_ptr = stack_ptr.add(1);
            }
        }

        for value in self.chunk.constants.iter_mut() {
            self.gc.mark(*value);
        }

        for value in self.chunk.globals.iter_mut() {
            self.gc.mark(*value);
        }
    }

    pub fn run(&mut self) {
        macro_rules! binary_op {
            ($op:tt) => {
                {
                    let b = self.stack.pop();
                    let a = self.stack.pop();

                    if !a.is_float() || !b.is_float() {
                        panic!("Can only do arithmetic operations on floats");
                    }

                    self.stack.push(Value::float(a.as_float() $op b.as_float()));
                }
            };
        }

        macro_rules! equality_op {
            ($op:tt) => {
                {
                    let b = self.stack.pop();
                    let a = self.stack.pop();

                    self.stack.push(Value::bool(a $op b));
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
            #[cfg(feature = "trace_execution")]
            {
                let mut stack_ptr = self.stack.base();
                while stack_ptr != self.stack.top().as_ptr() {
                    print!("[ {} ]", unsafe { *stack_ptr });
                    stack_ptr = unsafe { stack_ptr.add(1) };
                }
                println!();
                self.chunk
                    .disassemble_instruction(self.chunk.current_offset());
            }

            use chunk::OpCode as Op;
            match unsafe { self.chunk.next_opcode() } {
                Op::LoadConstant => self.stack.push(self.chunk.next_constant()),
                Op::Nil => self.stack.push(Value::NULL),
                Op::Pop => {
                    self.stack.pop();
                }
                Op::Add => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();

                    if a.is_float() && b.is_float() {
                        self.stack.push(Value::float(a.as_float() + b.as_float()))
                    } else if a.is_string() && b.is_string() {
                        let new_str = unsafe {
                            format!(
                                "{}{}",
                                (*a.as_obj().string).value,
                                (*b.as_obj().string).value
                            )
                        };
                        let obj = ObjString::new(&new_str);
                        let obj = self.alloc(obj);
                        self.stack.push(Value::obj(obj))
                    } else {
                        panic!("Can only add strings and floats");
                    }
                }
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
                Op::Jump => {
                    let offset = self.chunk.next_byte();

                    self.chunk.jump(offset);
                }
                Op::JumpIfFalse => {
                    let offset = self.chunk.next_byte();

                    if !self.stack.pop().as_bool() {
                        self.chunk.jump(offset);
                    }
                }
                Op::JumpIfFalseNoPop => {
                    let offset = self.chunk.next_byte();

                    if !self.stack.peek(0).as_bool() {
                        self.chunk.jump(offset);
                    }
                }
                Op::JumpIfTrueNoPop => {
                    let offset = self.chunk.next_byte();

                    if self.stack.peek(0).as_bool() {
                        self.chunk.jump(offset);
                    }
                }
                Op::Return => {
                    self.gc.free_everything();
                    return;
                }
            }
        }
    }
}
