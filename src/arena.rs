use std::{
    cell::{Cell, UnsafeCell},
    rc::Rc,
    sync::Arc,
};

pub struct Arena {
    buffer: Rc<UnsafeCell<[u8]>>,
    next_ptr: Cell<usize>,
    next: UnsafeCell<Option<Arc<Arena>>>,
}
impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

impl Arena {
    pub fn new() -> Self {
        let count = 4096 * 4096;
        let mut v = Vec::new();
        v.reserve_exact(count);
        v.extend(std::iter::repeat_n(0, count));
        let buf1: Rc<[u8]> = v.into();
        let buf2 = unsafe { Rc::from_raw(Rc::into_raw(buf1) as *const UnsafeCell<[u8]>) };
        Self {
            buffer: buf2,
            next_ptr: Cell::new(0),
            next: UnsafeCell::new(None),
        }
    }
    pub fn new_sized(size: usize) -> Self {
        let mut sz = 4096 * 4096;
        while sz < size {
            sz += 4096 * 4096;
        }
        let count = sz;
        let mut v = Vec::new();
        v.reserve_exact(count);
        v.extend(std::iter::repeat_n(0, count));
        let buf1: Rc<[u8]> = v.into();
        let buf2 = unsafe { Rc::from_raw(Rc::into_raw(buf1) as *const UnsafeCell<[u8]>) };
        Self {
            buffer: buf2,
            next_ptr: Cell::new(0),
            next: UnsafeCell::new(None),
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn alloc_bytes(&self, count: usize) -> &mut [u8] {
        let len = if count.is_multiple_of(16) {
            count
        } else {
            count + 16 - count % 16
        };
        let nxt = self.next_ptr.get();
        //safety, aligned pointer, guarrantees unique access to a location.
        unsafe {
            if nxt + len >= self.buffer.get().as_ref().unwrap().len() {
                if let Some(nxt) = self.next.get().as_ref().unwrap() {
                    let out = nxt.alloc_bytes(len);
                    out
                } else {
                    let tmp = Arena::new_sized(len);
                    *self.next.get().as_mut().unwrap() = Some(Arc::new(tmp));
                    let x = self
                        .next
                        .get()
                        .as_ref()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .alloc_bytes(len);
                    x
                }
            } else {
                let out = &mut self.buffer.get().as_mut().unwrap()[nxt..nxt + len];
                self.next_ptr.set(nxt + len);
                out
            }
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn alloc<T: Copy>(&self, value: T) -> &mut T {
        assert!(!std::mem::needs_drop::<T>());
        unsafe {
            let bytes = self.alloc_bytes(size_of_val(&value));
            let obj = bytes.as_mut_ptr() as *mut T;
            obj.write(value);
            obj.as_mut().unwrap()
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn alloc_array<T: Copy>(&self, value: &[T]) -> &mut [T] {
        assert!(!std::mem::needs_drop::<T>());
        unsafe {
            let bytes = self.alloc_bytes(size_of_val(&value) * value.len());
            let obj = std::ptr::slice_from_raw_parts_mut(bytes.as_mut_ptr() as *mut T, value.len());
            for i in 0..value.len() {
                let tmp = (obj as *mut T).add(i);
                tmp.write(value[i]);
            }
            obj.as_mut().unwrap()
        }
    }
}

#[derive(Clone, Copy)]
pub enum List<'a, T> {
    Empty,
    Node {
        value: &'a T,
        next: &'a List<'a, T>,
        arena: &'a Arena,
    },
}
