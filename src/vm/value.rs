use std::fmt;
use std::mem;

use ordered_float::OrderedFloat;

use super::object::Obj;
use super::object::ObjCommon;
use super::object::ObjKind;

const SIGN_BIT: u64 = 0x8000000000000000;
const QNAN: u64 = 0x7ffc000000000000;

const TAG_UNDEF: u64 = 4;
const TAG_NULL: u64 = 1;
const TAG_FALSE: u64 = 2;
const TAG_TRUE: u64 = 3;

#[derive(Clone, Copy)]
pub struct Value {
    value: u64,
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

    pub fn obj(obj: Obj) -> Self {
        unsafe {
            Self {
                value: SIGN_BIT | QNAN | obj.common as u64,
            }
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

    pub fn is_obj(&self) -> bool {
        self.value & (SIGN_BIT | QNAN) == SIGN_BIT | QNAN
    }

    pub fn is_string(&self) -> bool {
        self.is_obj() && matches!(self.as_obj().kind(), ObjKind::String)
    }

    pub fn as_float(&self) -> f64 {
        f64::from_bits(self.value)
    }

    pub fn as_bool(&self) -> bool {
        self.value == Self::TRUE.value
    }

    pub fn as_obj(&self) -> Obj {
        ((self.value & !(SIGN_BIT | QNAN)) as *mut ObjCommon).into()
    }
}

impl std::cmp::PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        if self.is_float() && other.is_float() {
            unsafe {
                std::mem::transmute::<f64, OrderedFloat<f64>>(self.as_float())
                    == std::mem::transmute::<f64, OrderedFloat<f64>>(other.as_float())
            }
        } else if self.is_obj() && other.is_obj() {
            self.as_obj() == other.as_obj()
        } else {
            self.value == other.value
        }
    }
}
impl std::cmp::Eq for Value {}

impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if self.is_string() {
            unsafe { (*self.as_obj().string).value.hash(state) };
        } else if self.is_float() {
            unsafe { std::mem::transmute::<f64, OrderedFloat<f64>>(self.as_float()).hash(state) }
        } else {
            self.value.hash(state);
        }
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
        } else if self.is_obj() {
            self.as_obj().to_string()
        } else {
            String::from("undefined")
        };

        write!(f, "{str}")
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let str = if self.is_float() {
            self.as_float().to_string()
        } else if self.is_bool() {
            self.as_bool().to_string()
        } else if self.is_null() {
            String::from("null")
        } else if self.is_obj() {
            format!("{:?}", self.as_obj())
        } else {
            String::from("undefined")
        };

        write!(f, "{str}")
    }
}
