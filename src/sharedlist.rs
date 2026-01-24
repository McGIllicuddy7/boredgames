use std::{
    sync::{Arc, RwLock, atomic::AtomicBool},
    thread::yield_now,
};

struct SharedListInner<T> {
    mutated: bool,
    locked: bool,
    list: Vec<T>,
}

pub struct SharedList<T: Clone> {
    list: Arc<RwLock<SharedListInner<T>>>,
    has_lock: AtomicBool,
}
impl<T: Clone> Clone for SharedList<T> {
    fn clone(&self) -> Self {
        let has_lock = AtomicBool::new(self.has_lock.load(std::sync::atomic::Ordering::SeqCst));
        Self {
            list: self.list.clone(),
            has_lock,
        }
    }
}
impl<T: Clone> Default for SharedList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> SharedList<T> {
    pub fn new() -> Self {
        Self {
            list: Arc::new(RwLock::new(SharedListInner {
                locked: false,
                mutated: false,
                list: Vec::new(),
            })),
            has_lock: AtomicBool::new(false),
        }
    }
    pub fn handle_locks(&self) {
        let has_lock = self.has_lock.load(std::sync::atomic::Ordering::SeqCst);
        if has_lock {
            return;
        }
        loop {
            let lck = self.list.read().unwrap();
            if !lck.locked {
                break;
            }
            drop(lck);
            yield_now();
        }
    }
    pub fn push(&self, v: T) {
        self.handle_locks();
        let mut list = self.list.write().unwrap();
        list.list.push(v);
        list.mutated = true;
    }

    pub fn pop(&self) -> Option<T> {
        self.handle_locks();
        let mut list = self.list.write().unwrap();
        list.mutated = true;
        list.list.pop()
    }

    pub fn len(&self) -> usize {
        let list = self.list.read().unwrap();
        list.list.len()
    }

    pub fn consume_mutation(&self) -> bool {
        self.handle_locks();
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
        list.list.get(index).cloned()
    }

    pub fn insert(&self, index: usize, value: T) -> Option<T> {
        self.handle_locks();
        let mut list = self.list.write().unwrap();
        if list.list.len() <= index {
            Some(value)
        } else {
            list.mutated = true;
            list.list.insert(index, value);
            None
        }
    }

    pub fn replace(&self, index: usize, value: T) -> Result<T, T> {
        self.handle_locks();
        let mut list = self.list.write().unwrap();
        let mut v = value;
        if list.list.len() <= index {
            Err(v)
        } else {
            std::mem::swap(&mut v, &mut list.list[index]);
            list.mutated = true;
            Ok(v)
        }
    }

    pub fn set(&self, index: usize, value: T) -> Option<T> {
        self.replace(index, value).err()
    }

    pub fn remove(&self, index: usize) -> Option<T> {
        self.handle_locks();
        let mut list = self.list.write().unwrap();
        if list.list.len() <= index {
            None
        } else {
            list.mutated = true;
            Some(list.list.remove(index))
        }
    }

    pub fn lock(&self) {
        loop {
            let mut list = self.list.write().unwrap();
            if list.locked {
                drop(list);
                yield_now();
            } else {
                self.has_lock
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                list.locked = false;
                break;
            }
        }
    }

    pub fn has_lock(&self) -> bool {
        self.has_lock.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn unlock(&self) {
        if !self.has_lock() {
            return;
        }
        let mut load = self.list.write().unwrap();
        load.locked = false;
        self.has_lock
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }
}
