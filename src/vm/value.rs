use std::mem;
use std::fmt;

#[derive(Clone, Copy)]
pub struct Value {
    value: u64,
}

impl Value {
    pub fn new_float(value: f64) -> Self {
        unsafe { mem::transmute(value) }
    }

    pub fn is_float(&self) -> bool {
        true
    }

    pub unsafe fn as_float(&self) -> f64 {
        mem::transmute(self.value)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // SAFETY: self is always a float
        write!(f, "{}", unsafe { self.as_float() })
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // SAFETY: self is always a float
        write!(f, "{}", unsafe { self.as_float() })
    }
}

