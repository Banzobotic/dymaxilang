#[cfg(feature = "local_map_scopes")]
use std::collections::HashMap;
use std::ptr::NonNull;

use super::{object::Obj, value::Value};

pub struct CallFrame {
    pub function: Obj,
    pub ip: *const u8,
    pub fp_offset: usize,
    #[cfg(feature = "local_map_scopes")]
    pub local_maps: Vec<HashMap<Value, HashMap<Value, Value>>>,
}

impl CallFrame {
    pub fn new(function: Obj, stack_top: NonNull<Value>, stack_base: *const Value) -> Self {
        let ip = unsafe { (*function.function).chunk.code_ptr() };
        Self {
            function,
            ip,
            fp_offset: unsafe {
                stack_top
                    .as_ptr()
                    .sub((*function.function).arity as usize)
                    .offset_from(stack_base) as usize
            },
            #[cfg(feature = "local_map_scopes")]
            local_maps: Vec::new(),
        }
    }
}
