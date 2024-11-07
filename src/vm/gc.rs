use super::{
    object::{Obj, ObjKind},
    value::Value,
};

pub struct GC {
    objects: Vec<Option<Obj>>,
    free_slots: Vec<usize>,
    greys: Vec<Obj>,
    bytes_allocated: usize,
    next_gc: usize,
}

impl GC {
    const HEAP_GROW_FACTOR: usize = 2;
    
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            free_slots: Vec::new(),
            greys: Vec::new(),
            bytes_allocated: 0,
            next_gc: 1024 * 1024,
        }
    }

    pub fn alloc<T>(&mut self, obj: impl GCAlloc<T>) -> Obj {
        obj.alloc(self)
    }

    pub fn mark(&mut self, obj: impl GCMark) {
        obj.mark(self)
    }

    pub fn trace(&mut self) {
        while let Some(obj) = self.greys.pop() {
            #[cfg(feature = "debug_gc")]
            println!("Blacken: {:?} {obj}", obj.kind());
            
            match unsafe { obj.common.read().kind } {
                ObjKind::String => (),
            }
        }
    }

    pub fn sweep(&mut self) {
        for i in 0..self.objects.len() {
            if let Some(obj) = self.objects[i].as_mut() {
                if unsafe { obj.common.read().mark } {
                    unsafe {
                        (*obj.common).mark = false
                    }
                } else {
                    self.bytes_allocated -= obj.size();
                    self.objects[i].take().unwrap().free()
                }
            }
        }
    }

    pub fn collect(&mut self) {
        self.trace();
        self.sweep();

        self.next_gc = self.bytes_allocated * Self::HEAP_GROW_FACTOR;
    }

    pub fn should_gc(&self) -> bool {
        self.bytes_allocated > self.next_gc || cfg!(feature = "clobber_gc")   
    }

    pub fn free_everything(&mut self) {
        for i in 0..self.objects.len() {
            if let Some(obj) = self.objects[i].take() {
                obj.free()
            }
        }
    }
}

pub trait GCAlloc<T> {
    fn alloc(self, gc: &mut GC) -> Obj;
}

impl<T> GCAlloc<T> for T
where
    Obj: From<*mut T>,
{
    fn alloc(self, gc: &mut GC) -> Obj {
        let obj_ptr = Box::into_raw(Box::new(self));
        let obj: Obj = obj_ptr.into();

        gc.bytes_allocated += obj.size();

        if let Some(i) = gc.free_slots.pop() {
            gc.objects[i] = Some(obj)
        } else {
            gc.objects.push(Some(obj));
        }

        obj_ptr.into()
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

        #[cfg(feature = "debug_gc")]
        println!("Mark: {:?} {obj}", obj.kind());
        
        if unsafe { !obj.common.read().mark } {
            unsafe { (*obj.common).mark = true };
            gc.greys.push(obj);
        }
    }
}
