use std::{fmt::{Debug, Display}, ptr};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ObjKind {
    String,
}

pub union Obj {
    pub common: *mut ObjCommon,
    pub string: *mut ObjString,
}

impl Obj {
    pub fn kind(&self) -> ObjKind {
        unsafe { self.common.read().kind }
    }

    pub fn size(&self) -> usize {
        unsafe {
            match self.kind() {
                ObjKind::String => (*self.string).value.len() + size_of::<ObjString>(),
            }
        }
    }

    pub fn free(self) {
        #[cfg(feature = "debug_gc")]
        println!("Free: {:?} {}", self.kind(), self);
        
        unsafe {
            match self.kind() {
                ObjKind::String => drop(Box::from_raw(self.string)),
            }
        }
    }
}

impl Debug for Obj {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl Display for Obj {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind() {
            ObjKind::String => write!(f, "{}", unsafe { &(*self.string).value }),
        }
    }
}

impl PartialEq for Obj {
    fn eq(&self, other: &Self) -> bool {
        if self.kind() == ObjKind::String && other.kind() == ObjKind::String {
            unsafe {
                (*self.string).value == (*other.string).value
            }
        } else {
            unsafe {
                ptr::eq(self.common, other.common)
            }
        }
    }
}

macro_rules! from_obj_impl {
    ($($name:ident $kind:ty),+) => {
        $(
            impl From<*mut $kind> for Obj {
                fn from($name: *mut $kind) -> Self {
                    Obj { $name }
                }
            }
        )*
    }
}

from_obj_impl! {
    common ObjCommon,
    string ObjString
}

#[repr(C)]
pub struct ObjCommon {
    pub kind: ObjKind,
    pub mark: bool,
}

impl ObjCommon {
    pub fn new(kind: ObjKind) -> Self {
        ObjCommon { kind, mark: false }
    }
}

#[repr(C)]
pub struct ObjString {
    pub common: ObjCommon,
    pub value: Box<str>,
}

impl ObjString {
    pub fn new(value: &str) -> Self {
        let common = ObjCommon::new(ObjKind::String);
        ObjString {
            common,
            value: value.into(),
        }
    }
}
