pub const PAGE_SIZE: u64 = 2 << 28;
pub const MAX_PAGE_COUNT: u64 = u64::MAX / PAGE_SIZE;
pub use crate::rtils::marathon::ArachneId;
pub use serde::{Deserialize, Serialize};
pub use std::collections::{BTreeMap, BTreeSet};
pub use std::sync::{Arc, Mutex};
pub struct GlobalIdAllocatorInner {
    pub resident_page_set: BTreeSet<u64>,
    pub allocated_page_set: BTreeSet<u64>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdAllocatorInner {
    pub start: u64,
    pub used: BTreeSet<u64>,
}

impl GlobalIdAllocatorInner {
    pub fn alloc_page(&mut self) -> u64 {
        for i in 1..MAX_PAGE_COUNT {
            let idx = i * PAGE_SIZE;
            if !self.allocated_page_set.contains(&idx) && !self.resident_page_set.contains(&idx) {
                self.allocated_page_set.insert(idx);
                self.resident_page_set.insert(idx);
                return idx;
            }
        }
        panic!("no pages remaining");
    }

    pub fn free_page(&mut self, start: u64) {
        self.allocated_page_set.remove(&start);
    }

    pub fn collect_garbage<U: ArachneId, T>(&mut self, map: &BTreeMap<U, T>) {
        for key in map.keys() {
            let page = (key.get() / PAGE_SIZE) * PAGE_SIZE;
            if !self.allocated_page_set.contains(&page) && self.resident_page_set.contains(&page) {
                self.resident_page_set.remove(&page);
            }
        }
    }
}

impl IdAllocatorInner {
    pub fn alloc_id(&mut self) -> u64 {
        for i in self.start..self.start + PAGE_SIZE {
            if !self.used.contains(&i) {
                self.used.insert(i);
                return i;
            }
        }
        panic!("no ids remaining");
    }
    pub fn free_id(&mut self, id: u64) {
        self.used.remove(&id);
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Hash, Ord, Serialize, Debug, Deserialize)]
#[repr(transparent)]
pub struct GlobalId {
    inner: u64,
}

impl ArachneId for GlobalId {
    fn create(x: u64) -> Self {
        Self { inner: x }
    }

    fn get(&self) -> u64 {
        self.inner
    }
}

pub struct IdPageAllocator {
    inner: Mutex<GlobalIdAllocatorInner>,
}

impl Default for IdPageAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl IdPageAllocator {
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(GlobalIdAllocatorInner {
                resident_page_set: BTreeSet::new(),
                allocated_page_set: BTreeSet::new(),
            }),
        }
    }

    pub fn alloc_page(&self) -> u64 {
        self.inner.lock().unwrap().alloc_page()
    }

    pub fn free_page(&self, start: u64) {
        self.inner.lock().unwrap().free_page(start);
    }

    pub fn collect_garbage<U: ArachneId, T>(&self, map: &BTreeMap<U, T>) {
        self.inner.lock().unwrap().collect_garbage(map);
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct IdAllocator {
    inner: Mutex<IdAllocatorInner>,
}

impl IdAllocator {
    pub const fn new(start: u64) -> Self {
        Self {
            inner: Mutex::new(IdAllocatorInner {
                start,
                used: BTreeSet::new(),
            }),
        }
    }

    pub fn alloc_id(&self) -> GlobalId {
        GlobalId::create(self.inner.lock().unwrap().alloc_id())
    }

    pub fn free_id(&self, id: GlobalId) {
        self.inner.lock().unwrap().free_id(id.get());
    }
}
