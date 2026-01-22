use std::any::TypeId;
use std::marker::PhantomData;
use std::ops::{Drop, IndexMut};
#[allow(unused)]
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard, atomic::AtomicBool};
use std::{cell::UnsafeCell, ops::Index};

struct SharedListInner<T> {
    mutated: bool,
    list: Vec<T>,
}

#[derive(Clone)]
pub struct SharedList<T: Clone> {
    list: Arc<RwLock<SharedListInner<T>>>,
}
impl<T: Clone> SharedList<T> {
    pub fn new() -> Self {
        Self {
            list: Arc::new(RwLock::new(SharedListInner {
                mutated: false,
                list: Vec::new(),
            })),
        }
    }

    pub fn push(&self, v: T) {
        let mut list = self.list.write().unwrap();
        list.list.push(v);
        list.mutated = true;
    }

    pub fn pop(&self) -> Option<T> {
        let mut list = self.list.write().unwrap();
        list.mutated = true;
        list.list.pop()
    }

    pub fn len(&self) -> usize {
        let list = self.list.read().unwrap();
        list.list.len()
    }

    pub fn consume_mutation(&self) -> bool {
        let mut list = self.list.write().unwrap();
        if list.mutated {
            list.mutated = false;
            true
        } else {
            false
        }
    }

    pub fn get(&self, index: usize) -> Option<T> {
        let list = self.list.read().unwrap();
        list.list.get(index).map(|i| i.clone())
    }
}
