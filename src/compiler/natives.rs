#![allow(clippy::useless_format)]

use std::{ptr::NonNull, time::SystemTime};

use ordered_float::OrderedFloat;

use crate::vm::{object::ObjString, value::Value, VM};


pub fn native_time(_arg_count: u32, _args: NonNull<Value>, _vm: *mut VM) -> Value {
    Value::float(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64(),
    )
}

pub fn native_print(arg_count: u32, args: NonNull<Value>, _vm: *mut VM) -> Value {
    if arg_count == 0 {
        println!();
    } else {
        for i in 0..arg_count {
            println!("{}", unsafe {args.add(i as usize).read()})
        }
    }
    Value::NULL
}

pub fn native_read(arg_count: u32, args: NonNull<Value>, vm: *mut VM) -> Value {
    unsafe {
        if arg_count != 1 {
            (*vm).runtime_error((*vm).frame().ip, format!("expected 1 argument but got {arg_count}"));
        }
        let value = args.read();
        if !value.is_string() {
            (*vm).runtime_error((*vm).frame().ip, format!("file path must be a string"));
        }
        let Ok(text) = std::fs::read_to_string((*value.as_obj().string).value.as_ref()) else {
            (*vm).runtime_error((*vm).frame().ip, format!("file not found"));
        };
        let obj = ObjString::new(text.trim());
        let obj = (*vm).alloc(obj);
        Value::obj(obj)
    }
}

pub fn native_num(arg_count: u32, args: NonNull<Value>, vm: *mut VM) -> Value {
    unsafe {
        if arg_count != 1 {
            (*vm).runtime_error((*vm).frame().ip, format!("expected 1 argument but got {arg_count}"));
        }
        let value = args.read();
        if !value.is_string() {
            (*vm).runtime_error((*vm).frame().ip, format!("can only convert strings to numbers"));
        }
        let Ok(num) = (*value.as_obj().string).value.trim().parse() else {
            (*vm).runtime_error((*vm).frame().ip, format!("string must represent a valid number"));
        };
        Value::float(num)
    }
}

pub fn native_abs(arg_count: u32, args: NonNull<Value>, vm: *mut VM) -> Value {
    unsafe {
        if arg_count != 1 {
            (*vm).runtime_error((*vm).frame().ip, format!("expected 1 argument but {arg_count}"));
        }
        let value = args.read();
        if !value.is_float() {
            (*vm).runtime_error((*vm).frame().ip, format!("can only get the absolute value of numbers"));
        }

        Value::float(value.as_float().abs())
    }
}

pub fn native_split(arg_count: u32, args: NonNull<Value>, vm: *mut VM) -> Value {
    unsafe {
        if !(1..=2).contains(&arg_count) {
            (*vm).runtime_error((*vm).frame().ip, format!("expected 1 or 2 arguments but got {arg_count}"));
        }
        let key = {
            let obj = ObjString::new("split");
            let obj = (*vm).alloc(obj);
            Value::obj(obj)
        };
        split_impl(args, vm, key, arg_count == 1)
    }
}

pub fn native_split_into(arg_count: u32, args: NonNull<Value>, vm: *mut VM) -> Value {
    unsafe {
        if !(2..=3).contains(&arg_count) {
            (*vm).runtime_error((*vm).frame().ip, format!("expected 2 or 3 arguments but got {arg_count}"));
        }
        let key = args.add(arg_count as usize - 1).read();
        split_impl(args, vm, key, arg_count == 2)
    }
}

#[inline]
pub fn split_impl(args: NonNull<Value>, vm: *mut VM, key: Value, whitespace: bool) -> Value {
    unsafe {
        let value = args.read();
        if !value.is_string() {
            (*vm).runtime_error((*vm).frame().ip, format!("can only split strings"));
        }
        let str = (*value.as_obj().string).value.as_ref();

        let mut count = 0.0;
        if whitespace {
            for (i, x) in str.split_whitespace().enumerate() {
                count += 1.0;
                let obj = ObjString::new(x);
                let obj = (*vm).alloc(obj);
                (*vm).globals.global_map.entry(key).or_default().insert(Value::float(i as f64), Value::obj(obj));
            }
        } else {
            let pat = args.add(1).read();
            if !pat.is_string() {
                (*vm).runtime_error((*vm).frame().ip, format!("split pattern must be a string"));
            }
            let pat = (*pat.as_obj().string).value.as_ref();

            for (i, x) in str.split(pat).enumerate() {
                count += 1.0;
                let obj = ObjString::new(x);
                let obj = (*vm).alloc(obj);
                (*vm).globals.global_map.entry(key).or_default().insert(Value::float(i as f64), Value::obj(obj));
            }
        }
        
        Value::float(count)
    }
}

pub fn native_sort(arg_count: u32, args: NonNull<Value>, vm: *mut VM) -> Value {
    unsafe {
        if arg_count != 3 {
            (*vm).runtime_error((*vm).frame().ip, format!("expected 3 arguments but got {arg_count}"));
        }
        
        let key = args.read();
        let Some(map) = (*vm).globals.global_map.get_mut(&key) else {
            (*vm).runtime_error((*vm).frame().ip, format!("'{key}' has no values associated with it"))
        };
        
        let start = args.add(1).read();
        let end = args.add(2).read();
        if !start.is_float() || !end.is_float() {
            (*vm).runtime_error((*vm).frame().ip, format!("can only sort data indexed by numbers"));
        }
        let start = start.as_float();
        let end = end.as_float();
        if start != start.round() || end != end.round() {
            (*vm).runtime_error((*vm).frame().ip, format!("can only sort data indexed by integers"));
        }
        let start = start as usize;
        let end = end as usize;

        let mut buf = Vec::with_capacity(end - start);

        for i in start..end {
            let Some(value) = map.get(&Value::float(i as f64)) else {
                (*vm).runtime_error((*vm).frame().ip, format!("no value at index {i}"));
            };
            if !value.is_float() {
                (*vm).runtime_error((*vm).frame().ip, format!("can only sort numbers"));
            }
            
            buf.push(std::mem::transmute::<f64, OrderedFloat<f64>>(value.as_float()));
        }

        buf.sort_unstable();

        for i in start..end {
            map.insert(Value::float(i as f64), Value::float(std::mem::transmute::<OrderedFloat<f64>, f64>(buf[i - start])));
        }
        
        Value::NULL
    }
}
