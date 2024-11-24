#[cfg(feature = "local_map_scopes")]
use std::collections::HashMap;
use std::ptr::{self, NonNull};

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
    frame_top: *mut CallFrame,
    gc: GC,
    stack: Stack,
    pub globals: Globals,
}

impl VM {
    pub fn new() -> Self {
        VM {
            frames: Vec::new(),
            frame_top: ptr::null_mut(),
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
        self.frame_top = self.frames.last_mut().unwrap() as *mut CallFrame;
    }

    pub fn pop_call_frame(&mut self) -> CallFrame {
        let function = self.frames.pop().unwrap();
        self.stack
            .free_slots(unsafe { (*function.function.function).stack_effect });
        self.frame_top = self.frames.last_mut().unwrap() as *mut CallFrame;
        function
    }

    pub fn frame(&mut self) -> &mut CallFrame {
        unsafe { self.frame_top.as_mut().unwrap_unchecked() }
    }

    #[allow(unused_unsafe)]
    pub fn run(&mut self) {
        let mut ip = self.frame().ip;
        let mut sp = self.stack.top;

        macro_rules! next_byte {
            () => {
                unsafe {
                    let byte = ip.read();
                    ip = ip.add(1);
                    byte
                }
            };
        }

        macro_rules! next_constant {
            () => {
                unsafe {
                    let byte = next_byte!();
                    (*self.frame().function.function).chunk.constants[byte as usize]
                }
            };
        }

        macro_rules! jump {
            ($offset:expr) => {
                unsafe {
                    ip = ip.offset($offset as i8 as isize);
                }
            };
        }

        #[cfg(feature = "trace_execution")]
        macro_rules! current_offset {
            () => {
                ip.wrapping_sub((*self.frame().function.function).chunk.code_ptr() as usize)
                    as usize
            };
        }

        macro_rules! stack_push {
            ($val:expr) => {
                unsafe {
                    sp.write($val);
                    sp = sp.add(1);
                }
            };
        }

        macro_rules! stack_pop {
            () => {
                unsafe {
                    sp = sp.sub(1);
                    sp.read()
                }
            };
        }

        macro_rules! stack_peek {
            ($pos:expr) => {
                unsafe {
                    sp.sub($pos + 1).read()
                }
            }
        }

        macro_rules! binary_op {
            ($op:tt) => {
                {
                    let b = stack_pop!();
                    let a = stack_pop!();

                    if !a.is_float() || !b.is_float() {
                        panic!("Can only do arithmetic operations on floats");
                    }

                    stack_push!(Value::float(a.as_float() $op b.as_float()));
                }
            };
        }

        macro_rules! equality_op {
            ($op:tt) => {
                {
                    let b = stack_pop!();
                    let a = stack_pop!();

                    stack_push!(Value::bool(a $op b));
                }
            }
        }

        macro_rules! comparison_op {
            ($op:tt) => {
                {
                    let b = stack_pop!();
                    let a = stack_pop!();

                    if !a.is_float() || !b.is_float() {
                        panic!("Can only compare floats");
                    }

                    stack_push!(Value::bool(a.as_float() $op b.as_float()));
                }
            };
        }

        self.gc.program_started();

        '_next: loop {
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
                        .disassemble_instruction(current_offset!());
                }
            }

            use chunk::OpCode as Op;
            match unsafe { std::mem::transmute::<u8, Op>(next_byte!()) } {
                Op::LoadConstant => {
                    let value = next_constant!();
                    stack_push!(value);
                }
                Op::Null => stack_push!(Value::NULL),
                Op::Pop => {
                    stack_pop!();
                }
                Op::Add => {
                    let b = stack_pop!();
                    let a = stack_pop!();

                    if a.is_float() && b.is_float() {
                        stack_push!(Value::float(a.as_float() + b.as_float()))
                    } else if a.is_string() && b.is_string() {
                        let new_str = unsafe {
                            format!(
                                "{}{}",
                                (*a.as_obj().string).value,
                                (*b.as_obj().string).value
                            )
                        };
                        let obj = ObjString::new(&new_str);
                        self.stack.top = sp;
                        let obj = self.alloc(obj);
                        stack_push!(Value::obj(obj))
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
                    if !stack_peek!(0).is_float() {
                        panic!("Can only negate numbers");
                    }
                    unsafe {
                        let top_ptr = sp.sub(1);
                        top_ptr.write(Value::float(-top_ptr.read().as_float()));
                    }
                }
                Op::Not => {
                    if !stack_peek!(0).is_bool() {
                        panic!("Can only not boolean values");
                    }
                    unsafe {
                        let top_ptr = sp.sub(1);
                        top_ptr.write(Value::bool(!top_ptr.read().as_bool()));
                    }
                }
                Op::DefineGlobal => {
                    let idx = next_byte!();
                    self.globals.set(idx, stack_pop!());
                }
                Op::GetGlobal => {
                    let idx = next_byte!();
                    let value = self.globals.get(idx);

                    if value.is_undef() {
                        panic!("Attempted to get value of undefined variable");
                    }

                    stack_push!(value);
                }
                Op::SetGlobal => {
                    let idx = next_byte!();
                    let prev_value = self.globals.get(idx);

                    if prev_value.is_undef() {
                        panic!("Attemped to set value of undefined variable");
                    }

                    self.globals.set(idx, stack_peek!(0));
                }
                Op::GetLocal => {
                    let offset = next_byte!() as usize;
                    let fp_offset = self.frame().fp_offset;
                    stack_push!(self.stack.base().add(offset + fp_offset).read());
                },
                Op::SetLocal => unsafe {
                    self.stack
                        .base_mut()
                        .add(next_byte!() as usize)
                        .write(stack_peek!(0));
                },
                Op::GetMap => {
                    let key = stack_pop!();
                    let map_key = stack_pop!();

                    #[cfg(feature = "local_map_scopes")]
                    for map in unsafe { (*self.frame_top).local_maps.iter().rev() } {
                        if let Some(value_map) = map.get(&map_key) {
                            if let Some(value) = value_map.get(&key) {
                                stack_push!(*value);
                                continue '_next;
                            }
                        }
                    }

                    if let Some(value_map) = self.globals.global_map.get(&map_key) {
                        if let Some(value) = value_map.get(&key) {
                            stack_push!(*value);
                            continue;
                        }
                    }

                    panic!("Key not found");
                }
                Op::SetMap => {
                    let value = stack_pop!();
                    let key = stack_pop!();
                    let map_key = stack_pop!();

                    #[cfg(feature = "local_map_scopes")]
                    if let Some(map) = self.frame().local_maps.last_mut() {
                        map.entry(map_key).or_default().insert(key, value);
                    } else {
                        self.globals
                            .global_map
                            .entry(map_key)
                            .or_default()
                            .insert(key, value);
                    }
                    #[cfg(not(feature = "local_map_scopes"))]
                    self.globals
                        .global_map
                        .entry(map_key)
                        .or_default()
                        .insert(key, value);

                    stack_push!(value);
                }
                #[cfg(feature = "local_map_scopes")]
                Op::PushMap => {
                    self.frame().local_maps.push(HashMap::new());
                }
                #[cfg(feature = "local_map_scopes")]
                Op::PopMap => {
                    self.frame().local_maps.pop();
                }
                Op::Jump => {
                    let offset = next_byte!();

                    jump!(offset);
                }
                Op::JumpIfFalse => {
                    let offset = next_byte!();

                    if !stack_pop!().as_bool() {
                        jump!(offset);
                    }
                }
                Op::JumpIfFalseNoPop => {
                    let offset = next_byte!();

                    if !stack_peek!(0).as_bool() {
                        jump!(offset);
                    }
                }
                Op::JumpIfTrueNoPop => {
                    let offset = next_byte!();

                    if stack_peek!(0).as_bool() {
                        jump!(offset);
                    }
                }
                Op::Call => {
                    let arg_count = next_byte!();
                    let function = stack_peek!(arg_count as usize);
                    self.frame().ip = ip;
                    self.stack.top = sp;
                    self.call_value(function, arg_count);
                    ip = self.frame().ip;
                    sp = self.stack.top;
                }
                Op::Return => {
                    if self.frames.len() == 1 {
                        self.gc.free_everything();
                        return;
                    }

                    let result = stack_pop!();
                    let old_frame = self.pop_call_frame();
                    ip = self.frame().ip;

                    sp = unsafe {
                        NonNull::new_unchecked(self.stack.base_mut().add(old_frame.fp_offset - 1))
                    };
                    stack_push!(result);
                }
            }
        }
    }
}
