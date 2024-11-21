use std::ptr::NonNull;

use call_frame::CallFrame;
use gc::{GCAlloc, GC};
use globals::Globals;
use object::{Obj, ObjKind, ObjString};
use stack::Stack;
use value::Value;

pub mod call_frame;
pub mod chunk;
pub mod gc;
pub mod globals;
pub mod object;
pub mod stack;
pub mod value;

pub struct VM {
    frames: Vec<CallFrame>,
    gc: GC,
    stack: Stack,
    pub globals: Globals,
}

impl VM {
    pub fn new() -> Self {
        VM {
            frames: Vec::new(),
            gc: GC::new(),
            stack: Stack::new(),
            globals: Globals::new(),
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
        while stack_ptr != self.stack.top.as_ptr() {
            unsafe {
                self.gc.mark(*stack_ptr);
                stack_ptr = stack_ptr.add(1);
            }
        }

        for frame in self.frames.iter_mut() {
            self.gc.mark(frame.function)
        }

        for value in self.globals.globals.iter_mut() {
            self.gc.mark(*value);
        }
    }

    pub fn call(&mut self, function: Obj, arg_count: u8) {
        let arity = unsafe { (*function.function).arity };
        if arg_count as u32 != arity {
            panic!("Expected {arity} arguments but got {arg_count}");
        }

        self.push_call_frame(function);
    }

    pub fn call_value(&mut self, function: Value, arg_count: u8) {
        if function.is_obj() {
            match function.as_obj().kind() {
                ObjKind::Function => self.call(function.as_obj(), arg_count),
                ObjKind::Native => {
                    let native = unsafe { (*function.as_obj().native).function };
                    let result = native(arg_count as u32, unsafe {
                        self.stack.top.sub(arg_count as usize)
                    });
                    self.stack.top = unsafe { self.stack.top.sub(arg_count as usize + 1) };
                    self.stack.push(result);
                }
                _ => panic!("Can only call functions"),
            }
            return;
        }
        panic!("Can only call functions");
    }

    pub fn push_call_frame(&mut self, function: Obj) {
        self.stack
            .allocate_slots(unsafe { (*function.function).stack_effect });
        self.frames
            .push(CallFrame::new(function, self.stack.top, self.stack.base()));
    }

    pub fn pop_call_frame(&mut self) -> CallFrame {
        let function = self.frames.pop().unwrap();
        self.stack
            .free_slots(unsafe { (*function.function.function).stack_effect });
        function
    }

    pub fn frame(&mut self) -> &mut CallFrame {
        self.frames.last_mut().unwrap()
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

        self.gc.program_started();

        loop {
            #[cfg(feature = "trace_execution")]
            {
                let mut stack_ptr = self.stack.base();
                while stack_ptr != self.stack.top.as_ptr() {
                    print!("[ {} ]", unsafe { *stack_ptr });
                    stack_ptr = unsafe { stack_ptr.add(1) };
                }
                println!();
                unsafe {
                    (*self.frame().function.function)
                        .chunk
                        .disassemble_instruction(self.frame().current_offset());
                }
            }

            use chunk::OpCode as Op;
            match unsafe { self.frame().next_opcode() } {
                Op::LoadConstant => {
                    let value = self.frame().next_constant();
                    self.stack.push(value);
                }
                Op::Null => self.stack.push(Value::NULL),
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
                        let top_ptr = self.stack.top.sub(1);
                        top_ptr.write(Value::float(-top_ptr.read().as_float()));
                    }
                }
                Op::Not => {
                    if !self.stack.peek(0).is_bool() {
                        panic!("Can only not boolean values");
                    }
                    unsafe {
                        let top_ptr = self.stack.top.sub(1);
                        top_ptr.write(Value::bool(!top_ptr.read().as_bool()));
                    }
                }
                Op::DefineGlobal => {
                    let idx = self.frame().next_byte();
                    self.globals.set(idx, self.stack.pop());
                }
                Op::GetGlobal => {
                    let idx = self.frame().next_byte();
                    let value = self.globals.get(idx);

                    if value.is_undef() {
                        panic!("Attempted to get value of undefined variable");
                    }

                    self.stack.push(value);
                }
                Op::SetGlobal => {
                    let idx = self.frame().next_byte();
                    let prev_value = self.globals.get(idx);

                    if prev_value.is_undef() {
                        panic!("Attemped to set value of undefined variable");
                    }

                    self.globals.set(idx, self.stack.peek(0));
                }
                Op::GetLocal => unsafe {
                    let offset = self.frame().next_byte() as usize;
                    let fp_offset = self.frame().fp_offset;
                    self.stack
                        .push(self.stack.base().add(offset + fp_offset).read());
                },
                Op::SetLocal => unsafe {
                    self.stack
                        .base_mut()
                        .add(self.frame().next_byte() as usize)
                        .write(self.stack.peek(0));
                },
                Op::Jump => {
                    let offset = self.frame().next_byte();

                    self.frame().jump(offset);
                }
                Op::JumpIfFalse => {
                    let offset = self.frame().next_byte();

                    if !self.stack.pop().as_bool() {
                        self.frame().jump(offset);
                    }
                }
                Op::JumpIfFalseNoPop => {
                    let offset = self.frame().next_byte();

                    if !self.stack.peek(0).as_bool() {
                        self.frame().jump(offset);
                    }
                }
                Op::JumpIfTrueNoPop => {
                    let offset = self.frame().next_byte();

                    if self.stack.peek(0).as_bool() {
                        self.frame().jump(offset);
                    }
                }
                Op::Call => {
                    let arg_count = self.frame().next_byte();
                    let function = self.stack.peek(arg_count as usize);
                    self.call_value(function, arg_count);
                }
                Op::Return => {
                    let result = self.stack.pop();
                    let old_frame = self.pop_call_frame();

                    if self.frames.is_empty() {
                        self.gc.free_everything();
                        return;
                    }

                    self.stack.top = unsafe {
                        NonNull::new_unchecked(self.stack.base_mut().add(old_frame.fp_offset - 1))
                    };
                    self.stack.push(result);
                }
            }
        }
    }
}
