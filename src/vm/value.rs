use std::fmt;
use std::mem;

const SIGN_BIT: u64 = 0x8000000000000000;
const QNAN: u64 = 0x7ffc000000000000;

const TAG_UNDEF: u64 = 4;
const TAG_NULL: u64 = 1;
const TAG_FALSE: u64 = 2;
const TAG_TRUE: u64 = 3;

#[derive(Clone, Copy)]
pub struct Value {
    pub value: u64,
}

impl Value {
    pub const UNDEF: Self = Self {
        value: QNAN | TAG_UNDEF,
    };
    pub const NULL: Self = Self {
        value: QNAN | TAG_NULL,
    };
    pub const TRUE: Self = Self {
        value: QNAN | TAG_TRUE,
    };
    pub const FALSE: Self = Self {
        value: QNAN | TAG_FALSE,
    };

    pub fn float(value: f64) -> Self {
        unsafe { mem::transmute(value) }
    }

    pub fn bool(value: bool) -> Self {
        if value {
            Value::TRUE
        } else {
            Value::FALSE
        }
    }

    pub fn is_float(&self) -> bool {
        self.value & QNAN != QNAN
    }

    pub fn is_bool(&self) -> bool {
        self.value | 1 == Self::TRUE.value
    }

    pub fn is_null(&self) -> bool {
        self.value == Self::NULL.value
    }

    pub fn is_undef(&self) -> bool {
        self.value == Self::UNDEF.value
    }

    pub fn as_float(&self) -> f64 {
        f64::from_bits(self.value)
    }

    pub fn as_bool(&self) -> bool {
        self.value == Self::TRUE.value
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let str = if self.is_float() {
            self.as_float().to_string()
        } else if self.is_bool() {
            self.as_bool().to_string()
        } else if self.is_null() {
            String::from("null")
        } else {
            String::from("undefined")
        };

        write!(f, "{str}")
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self}")
    }
}
