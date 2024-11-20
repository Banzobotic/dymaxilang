use std::collections::HashMap;

use super::value::Value;

pub struct Globals {
    pub globals: Vec<Value>,
    global_names: HashMap<String, u8>,
}

impl Globals {
    pub fn new() -> Self {
        Self {
            globals: Vec::new(),
            global_names: HashMap::new(),
        }
    }

    pub fn get(&self, idx: u8) -> Value {
        self.globals[idx as usize]
    }

    pub fn set(&mut self, idx: u8, value: Value) {
        self.globals[idx as usize] = value;
    }

    pub fn get_global_idx(&mut self, name: &str) -> u8 {
        match self.global_names.get(name) {
            Some(idx) => *idx,
            None => {
                let len = self.globals.len() as u8;
                self.global_names.insert(name.to_owned(), len);
                self.globals.push(Value::UNDEF);
                len
            }
        }
    }
}
