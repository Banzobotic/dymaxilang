use std::ptr::NonNull;

use super::{object::Obj, value::Value};

pub struct GC {
    objects: Vec<Obj>,
    greys: Vec<Obj>,
    current_mark: bool,
}

impl GC {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            greys: Vec::new(),
            current_mark: true,
        }
    }
    
    pub fn alloc<T>(&mut self, obj: impl GCAlloc<T>) -> T {
        obj.alloc(self)
    }

    pub fn mark(&mut self, obj: impl GCMark) {
        obj.mark(self)
    }
}

pub trait GCAlloc<T> {
    fn alloc(self, gc: &mut GC) -> T;
}

impl<T> GCAlloc<*mut T> for T
where
    Obj: From<*mut T>,
{
    fn alloc(self, gc: &mut GC) -> *mut T {
        let obj_ptr = Box::into_raw(Box::new(self));
        let obj = obj_ptr.into();

        gc.objects.push(obj);
        obj_ptr
    }
}

pub trait GCMark {
    fn mark(self, gc: &mut GC);
}

impl GCMark for Value {
    fn mark(self, gc: &mut GC) {
        if self.is_obj() {
            self.as_obj().mark(gc);
        }
    }
}

impl<T: Into<Obj>> GCMark for T {
    fn mark(self, gc: &mut GC) {
        let obj = self.into();
        if !unsafe { obj.common.read().mark == gc.current_mark } {
            unsafe { (*obj.common).mark = gc.current_mark };
            gc.greys.push(obj);
        }
    }
}
