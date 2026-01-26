use std::{
    cell::{Cell, UnsafeCell},
    fmt::{Debug, Display, Formatter, Write},
    hash::{DefaultHasher, Hash, Hasher},
    mem::MaybeUninit,
    ops::{Deref, DerefMut, Index, IndexMut},
    sync::{
        Arc, Mutex,
        atomic::{
            AtomicBool, AtomicI8, AtomicI16, AtomicI32, AtomicI64, AtomicIsize, AtomicPtr,
            AtomicU8, AtomicU16, AtomicU32, AtomicU64, AtomicUsize,
        },
    },
};

use crate::marathon::BStream;

pub trait Trivial {
    const IS_TRIVIAL: bool = const {
        if std::mem::needs_drop::<Self>() {
            panic!("type is drop");
        } else {
            true
        }
    };
    ///
    /// # Safety
    ///
    /// DO NOT MANUALLY IMPLEMENT THIS FUNCTION PLEASE
    unsafe fn no_drop_impl(&self) {
        println!("{}", Self::IS_TRIVIAL);
    }
}
pub trait TrivialClone: Clone + Trivial {}
impl<T: Trivial + Clone> TrivialClone for T {}

impl<T: Trivial, U: Trivial> Trivial for (T, U) {}
impl<T: Trivial, U: Trivial, V: Trivial> Trivial for (T, U, V) {}
impl<T: Trivial, U: Trivial, V: Trivial, W: Trivial> Trivial for (T, U, V, W) {}
impl<T: Trivial, U: Trivial, V: Trivial, W: Trivial, X: Trivial> Trivial for (T, U, V, W, X) {}
impl<T: Trivial, U: Trivial, V: Trivial, W: Trivial, X: Trivial, Y: Trivial> Trivial
    for (T, U, V, W, X, Y)
{
}
impl<T: Trivial, U: Trivial, V: Trivial, W: Trivial, X: Trivial, Y: Trivial, Z: Trivial> Trivial
    for (T, U, V, W, X, Y, Z)
{
}
impl Trivial for usize {}
impl Trivial for u8 {}
impl Trivial for u16 {}
impl Trivial for u32 {}
impl Trivial for u64 {}
impl Trivial for u128 {}
impl Trivial for isize {}
impl Trivial for i8 {}
impl Trivial for i16 {}
impl Trivial for i32 {}
impl Trivial for i64 {}
impl Trivial for i128 {}
impl Trivial for f64 {}
impl Trivial for f32 {}
impl Trivial for bool {}

impl Trivial for AtomicBool {}
impl Trivial for AtomicUsize {}
impl Trivial for AtomicU8 {}
impl Trivial for AtomicU16 {}
impl Trivial for AtomicU32 {}
impl Trivial for AtomicU64 {}
impl Trivial for AtomicIsize {}
impl Trivial for AtomicI8 {}
impl Trivial for AtomicI16 {}
impl Trivial for AtomicI32 {}
impl Trivial for AtomicI64 {}

impl<T> Trivial for *const T {}
impl<T> Trivial for *mut T {}
impl<T> Trivial for AtomicPtr<T> {}
impl<T> Trivial for UnsafeCell<T> {}
impl<T> Trivial for Cell<T> {}
impl<const COUNT: usize, T: Trivial> Trivial for [T; COUNT] {}
impl<T: ?Sized> Trivial for &T {}
impl<T> Trivial for &mut T {}

pub struct SpinLock<T> {
    cell: UnsafeCell<T>,
    lock: AtomicBool,
}
impl<T: Trivial> Trivial for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub fn new(value: T) -> Self {
        Self {
            cell: UnsafeCell::new(value),
            lock: AtomicBool::new(false),
        }
    }

    unsafe fn mark_locked(&self) {
        while self
            .lock
            .compare_exchange_weak(
                false,
                true,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            )
            .is_err()
        {
            std::hint::spin_loop();
            std::thread::yield_now();
        }
    }

    unsafe fn mark_unlocked(&self) {
        self.lock.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    unsafe fn try_mark_locked(&self) -> bool {
        self.lock
            .compare_exchange(
                false,
                true,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            )
            .is_ok()
    }

    pub fn lock<'a>(&'a self) -> Lock<'a, T> {
        unsafe {
            self.mark_locked();
            Lock { inner: self }
        }
    }

    pub fn try_lock<'a>(&'a self) -> Option<Lock<'a, T>> {
        unsafe {
            if self.try_mark_locked() {
                Some(Lock { inner: self })
            } else {
                None
            }
        }
    }

    pub fn store(&self, value: T) {
        let mut lock = self.lock();
        *lock = value;
    }

    pub fn try_store(&self, value: T) -> Option<T> {
        if let Some(mut lock) = self.try_lock() {
            *lock = value;
            None
        } else {
            Some(value)
        }
    }
}
impl<T: Default> SpinLock<T> {
    pub fn take(&self) -> T {
        let mut lock = self.lock();
        let mut def = Default::default();
        std::mem::swap(&mut def, &mut *lock);
        def
    }

    pub fn try_take(&self) -> Option<T> {
        let mut lock = self.try_lock()?;
        let mut def = Default::default();
        std::mem::swap(&mut def, &mut *lock);
        Some(def)
    }
}

impl<T: Clone> SpinLock<T> {
    pub fn get(&self) -> T {
        let lock = self.lock();
        lock.clone()
    }

    pub fn try_get(&self) -> Option<T> {
        let lock = self.try_lock()?;
        Some(lock.clone())
    }
}

impl<T: Clone> Clone for SpinLock<T> {
    fn clone(&self) -> Self {
        let lock = self.lock();
        Self {
            cell: UnsafeCell::new(lock.clone()),
            lock: AtomicBool::new(false),
        }
    }
}

unsafe impl<T: Send> Send for SpinLock<T> {}
unsafe impl<T: Sync> Sync for SpinLock<T> {}
//impl<T: Trivial> Trivial for SpinLock<T> {}
pub struct Lock<'a, T> {
    inner: &'a SpinLock<T>,
}
impl<'a, T> Drop for Lock<'a, T> {
    fn drop(&mut self) {
        unsafe {
            self.inner.mark_unlocked();
        }
    }
}
impl<'a, T> Deref for Lock<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.inner.cell.get().as_ref().unwrap() }
    }
}

impl<'a, T> DerefMut for Lock<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.inner.cell.get().as_mut().unwrap() }
    }
}

pub struct Arena {
    buffer: Arc<UnsafeCell<[u8]>>,
    next_ptr: Cell<usize>,
    next: UnsafeCell<Option<Arc<Arena>>>,
    lock: Mutex<()>,
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
        let buf1: Arc<[u8]> = v.into();
        let buf2 = unsafe { Arc::from_raw(Arc::into_raw(buf1) as *const UnsafeCell<[u8]>) };
        Self {
            buffer: buf2,
            next_ptr: Cell::new(0),
            lock: Mutex::new(()),
            next: UnsafeCell::new(None),
        }
    }
    pub fn new_sized(size: usize) -> Self {
        let count = if 4096 * 4096 > size {
            4096 * 4096
        } else {
            let mut tmp = 4096 * 4096;
            while tmp < size {
                tmp += 4096 * 4096;
            }
            tmp
        };
        let mut v = Vec::new();
        v.reserve_exact(count);
        v.extend(std::iter::repeat_n(0, count));
        let buf1: Arc<[u8]> = v.into();
        let buf2 = unsafe { Arc::from_raw(Arc::into_raw(buf1) as *const UnsafeCell<[u8]>) };
        Self {
            buffer: buf2,
            next_ptr: Cell::new(0),
            lock: Mutex::new(()),
            next: UnsafeCell::new(None),
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn alloc_bytes(&self, count: usize, align: usize) -> &mut [u8] {
        let _lock = self.lock.lock().unwrap();
        let len = count;
        let mut nxt = self.next_ptr.get();
        if !nxt.is_multiple_of(align) {
            nxt = nxt + align - nxt % align;
        }
        //safety, aligned pointer, guarrantees unique access to a location.
        unsafe {
            if nxt + len >= self.buffer.get().as_ref().unwrap().len() {
                if let Some(next) = self.next.get().as_ref().unwrap() {
                    next.alloc_bytes(len, align)
                } else {
                    let out = Arena::new_sized(count);
                    *self.next.get().as_mut().unwrap() = Some(Arc::new(out));
                    self.next
                        .get()
                        .as_ref()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .alloc_bytes(len, align)
                }
            } else {
                let out = &mut self.buffer.get().as_mut().unwrap()[nxt..nxt + len];
                self.next_ptr.set(nxt + len);
                out
            }
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn alloc<T: Trivial>(&self, value: T) -> &mut T {
        assert!(T::IS_TRIVIAL);
        assert!(!std::mem::needs_drop::<T>());
        unsafe {
            let bytes = self.alloc_bytes(size_of_val(&value), align_of_val(&value));
            let obj = bytes.as_mut_ptr() as *mut T;
            obj.write(value);
            obj.as_mut().unwrap()
        }
    }

    pub fn debug_mem_usage(&self) -> usize {
        let _lock = self.lock.lock().unwrap();
        let base_count = self.next_ptr.get();
        unsafe {
            if let Some(nxt) = self.next.get().as_ref().unwrap() {
                base_count + nxt.debug_mem_usage()
            } else {
                base_count
            }
        }
    }
}

unsafe impl Send for Arena {}
unsafe impl Sync for Arena {}
#[derive(Clone)]
pub enum List<'a, T: TrivialClone> {
    Empty(&'a Arena),
    Node(&'a ListNode<'a, T>),
}
impl<'a, T: TrivialClone> Trivial for List<'a, T> {}

#[derive(Clone)]
pub struct ListNode<'a, T: TrivialClone> {
    value: &'a T,
    next: List<'a, T>,
    arena: &'a Arena,
}

impl<'a, T: TrivialClone> Trivial for ListNode<'a, T> {}
impl<'a, T: TrivialClone> List<'a, T> {
    pub fn new(arena: &'a Arena, value: T) -> &'a Self {
        let tmp = arena.alloc(ListNode {
            value: arena.alloc(value),
            next: List::Empty(arena),
            arena,
        });
        arena.alloc(Self::Node(tmp))
    }

    pub fn get_arena(&self) -> &'a Arena {
        match self {
            List::Empty(arena) => arena,
            List::Node(list_node) => list_node.arena,
        }
    }

    pub fn cons(&self, value: T) -> &'a Self {
        let ar = self.get_arena();
        let node = ar.alloc(ListNode {
            value: ar.alloc(value),
            next: self.clone(),
            arena: ar,
        });
        ar.alloc(List::Node(node))
    }

    pub fn car(&self) -> &'a T {
        match self {
            List::Empty(_) => todo!(),
            List::Node(list_node) => list_node.value,
        }
    }

    pub fn cdr(&self) -> Self {
        match self {
            List::Empty(ar) => List::Empty(ar),
            List::Node(list_node) => list_node.next.clone(),
        }
    }

    pub fn get(&self, index: usize) -> Option<&'a T> {
        let mut i = 0;
        let mut current = self.clone();
        while let Self::Node(n) = current {
            if i == index {
                return Some(n.value);
            }
            i += 1;
            current = n.next.clone()
        }
        None
    }

    pub fn reverse(&self) -> &'a Self {
        let mut base: &'a List<'_, _> = self.get_arena().alloc(List::Empty(self.get_arena()));
        for i in self.clone() {
            base = base.cons(i);
        }
        base
    }

    pub const fn len(&self) -> usize {
        let mut out = 0;
        let mut next = self;
        while let Self::Node(n) = next {
            out += 1;
            next = &n.next;
        }
        out
    }

    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<'a, T: Debug + TrivialClone> Debug for List<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_list();
        let slf = self.clone();
        for i in slf {
            dbg.entry(&i);
        }
        dbg.finish()
    }
}

impl<'a, T: TrivialClone> Iterator for List<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            List::Empty(_) => None,
            List::Node(list_node) => {
                let out = Some(list_node.value.clone());
                *self = list_node.next.clone();
                out
            }
        }
    }
}

impl<'a, T: TrivialClone> Index<usize> for List<'a, T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).unwrap()
    }
}

pub enum ListMut<'a, T: TrivialClone> {
    Empty(&'a Arena),
    Node(&'a mut ListNodeMut<'a, T>),
}

impl<'a, T: TrivialClone> Trivial for ListMut<'a, T> {}
impl<'a, T: TrivialClone> Clone for ListMut<'a, T> {
    fn clone(&self) -> Self {
        match self {
            Self::Empty(ar) => Self::Empty(ar),
            Self::Node(x) => {
                let tmp = &**x;
                let tmp = tmp.arena.alloc(tmp.clone());
                Self::Node(tmp)
            }
        }
    }
}

pub struct ListNodeMut<'a, T: TrivialClone> {
    value: &'a mut T,
    next: ListMut<'a, T>,
    arena: &'a Arena,
}
impl<'a, T: TrivialClone> Clone for ListNodeMut<'a, T> {
    fn clone(&self) -> Self {
        Self {
            value: self.arena.alloc(self.value.clone()),
            next: self.next.clone(),
            arena: self.arena,
        }
    }
}
impl<'a, T: TrivialClone> Trivial for ListNodeMut<'a, T> {}

impl<'a, T: TrivialClone> ListMut<'a, T> {
    pub fn new(arena: &'a Arena, value: T) -> &'a Self {
        let tmp = arena.alloc(ListNodeMut {
            value: arena.alloc(value),
            next: ListMut::Empty(arena),
            arena,
        });
        arena.alloc(Self::Node(tmp))
    }

    pub fn get_arena(&self) -> &'a Arena {
        match self {
            ListMut::Empty(arena) => arena,
            ListMut::Node(list_node) => list_node.arena,
        }
    }

    pub fn cons(&self, value: T) -> &'a mut Self {
        let ar = self.get_arena();
        let node = ar.alloc(ListNodeMut {
            value: ar.alloc(value),
            next: self.clone(),
            arena: ar,
        });
        ar.alloc(ListMut::Node(node))
    }

    pub fn car(&'a mut self) -> &'a mut T {
        match self {
            ListMut::Empty(_) => todo!(),
            ListMut::Node(list_node) => list_node.value,
        }
    }

    pub fn cdr(self) -> &'a mut Self {
        match self {
            ListMut::Empty(ar) => ar.alloc(ListMut::Empty(ar)),
            ListMut::Node(list_node) => &mut list_node.next,
        }
    }

    pub fn get(&self, index: usize) -> Option<&T>
where {
        let mut i = 0;
        let mut current = self;
        while let ListMut::Node(c) = current {
            if i == index {
                return Some(&*c.value);
            }
            i += 1;
            current = &c.next;
        }
        None
    }
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        let mut i = 0;
        let mut current = self;
        while let ListMut::Node(c) = current {
            if i == index {
                return Some(&mut *c.value);
            }
            i += 1;
            current = &mut c.next;
        }
        None
    }

    pub fn get_node(&self, index: usize) -> Option<&'a ListNodeMut<'a, T>> {
        let mut i = 0;
        let mut current = self.clone();
        while let Self::Node(n) = current {
            if i == index {
                return Some(n);
            }
            i += 1;
            current = n.next.clone()
        }
        None
    }
    pub fn get_node_mut(&mut self, index: usize) -> Option<&'a mut ListNodeMut<'a, T>> {
        let mut i = 0;
        let mut current = self.clone();
        while let Self::Node(n) = current {
            if i == index {
                return Some(n);
            }
            i += 1;
            current = n.next.clone()
        }
        None
    }

    pub fn reverse(&'a self) -> &'a Self {
        let mut base: &'a ListMut<'_, _> = self.get_arena().alloc(ListMut::Empty(self.get_arena()));
        let mut n = self;
        while let ListMut::Node(node) = n {
            base = base.cons(node.value.clone());
            n = &node.next;
        }
        base
    }

    pub fn as_const(&'a self) -> List<'a, T> {
        match self {
            ListMut::Empty(ar) => List::Empty(ar),
            ListMut::Node(n) => {
                let ar = n.arena;
                let next = &n.next;
                let value: &'a T = n.value;
                let nxt = next.as_const();
                let node: ListNode<'a, T> = ListNode {
                    value,
                    next: nxt,
                    arena: ar,
                };
                let node_ptr = ar.alloc(node);
                List::Node(node_ptr)
            }
        }
    }

    pub const fn len(&self) -> usize {
        let mut out = 0;
        let mut next = self;
        while let Self::Node(n) = next {
            out += 1;
            next = &n.next;
        }
        out
    }

    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
impl<'a, T: Debug + TrivialClone> Debug for ListMut<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_list();
        for i in 0..self.len() {
            dbg.entry(&self[i]);
        }
        dbg.finish()
    }
}

impl<'a, T: TrivialClone> Iterator for ListMut<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Empty(_) => None,
            Self::Node(list_node) => {
                let out = Some(list_node.value.clone());
                *self = list_node.next.clone();
                out
            }
        }
    }
}

impl<'a, T: TrivialClone> Index<usize> for ListMut<'a, T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        let a: Option<&T> = self.get(index);
        a.unwrap()
    }
}
impl<'a, T: TrivialClone> IndexMut<usize> for ListMut<'a, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}

#[derive(Debug)]
pub struct Map<'a, T: TrivialClone + Hash + Eq, U: TrivialClone> {
    table: &'a mut ListMut<'a, ListMut<'a, (T, U)>>,
}
impl<'a, T: TrivialClone + Hash + Eq, U: TrivialClone> Trivial for Map<'a, T, U> {}
impl<'a, T: TrivialClone + Hash + Eq + Debug, U: TrivialClone + Debug> Map<'a, T, U> {
    pub fn new(arena: &'a Arena) -> Self {
        assert!(Self::IS_TRIVIAL);
        let mut list = arena.alloc(ListMut::Empty(arena));
        for _ in 0..64 {
            list = list.cons(ListMut::Empty(arena));
        }
        Self { table: list }
    }
    pub fn with_capacity(arena: &'a Arena, capacity: usize) -> Self {
        assert!(Self::IS_TRIVIAL);
        let mut list = arena.alloc(ListMut::Empty(arena));
        for _ in 0..capacity {
            list = list.cons(ListMut::Empty(arena));
        }
        Self { table: list }
    }

    pub fn insert(&mut self, key: T, value: U) -> Option<U> {
        if self.occupancy() > 0.8 {
            self.resize(self.table.len() * 8);
        }
        let mut hs = DefaultHasher::new();
        key.hash(&mut hs);
        let idx = hs.finish() as usize;
        let len = self.table.len();
        let ls = &mut self.table[idx % len];
        let ls_len = ls.len();
        for i in 0..ls_len {
            let (k, v) = &mut ls[i];
            if *k == key {
                let mut vp = value;
                std::mem::swap(v, &mut vp);
                return Some(vp);
            }
        }
        let ar = ls.get_arena();
        let mut nxt = ListMut::Empty(ar);
        std::mem::swap(&mut nxt, ls);
        let node = ar.alloc(ListNodeMut {
            value: ar.alloc((key, value)),
            next: nxt,
            arena: ar,
        });
        let tmp = ListMut::Node(node);
        self.table[idx % len] = tmp;
        None
    }

    pub fn get(&self, key: &T) -> Option<&U> {
        let mut hs = DefaultHasher::new();
        key.hash(&mut hs);
        let idx = hs.finish() as usize;
        let len = self.table.len();
        let ls = &self.table[idx % len];
        let ls_len = ls.len();
        for i in 0..ls_len {
            let (k, v) = &ls[i];
            if *k == *key {
                return Some(v);
            }
        }
        None
    }

    pub fn get_mut(&mut self, key: &T) -> Option<&mut U> {
        let mut hs = DefaultHasher::new();
        key.hash(&mut hs);
        let idx = hs.finish() as usize;
        let len = self.table.len();
        let ls = &mut self.table[idx % len];
        let ls_len = ls.len();
        for i in 0..ls_len {
            let (k, _) = &ls[i];
            if *k == *key {
                let (_, v) = &mut ls[i];
                return Some(v);
            }
        }
        None
    }

    pub fn remove(&mut self, key: &T) -> Option<(T, U)> {
        let mut hs = DefaultHasher::new();
        key.hash(&mut hs);
        let idx = hs.finish() as usize;
        let len = self.table.len();
        let ls = &mut self.table[idx % len];
        let ls_len = ls.len();
        let arena = ls.get_arena();
        for i in 0..ls_len {
            let (k, _) = ls.get(i).unwrap();
            if *k != *key {
                continue;
            }
            if i == 0 {
                let nxt = ls.get_node_mut(0).unwrap();
                let value = nxt.value.clone();
                let mut nxt_ptr = ListMut::Empty(arena);
                std::mem::swap(&mut nxt.next, &mut nxt_ptr);
                self.table[idx % len] = nxt_ptr;
                return Some(value);
            } else {
                let nxt = ls.get_node_mut(i).unwrap();
                let value = nxt.value.clone();
                let mut nxt_ptr = ListMut::Empty(arena);
                std::mem::swap(&mut nxt.next, &mut nxt_ptr);
                ls.get_node_mut(i - 1).unwrap().next = nxt_ptr;
                return Some(value);
            }
        }
        None
    }

    pub fn resize(&mut self, new_size: usize) {
        let mut out = Self::with_capacity(self.table.get_arena(), new_size);
        for i in 0..self.table.len() {
            for j in 0..self.table[i].len() {
                let (k, v) = self.table[i][j].clone();
                out.insert(k, v);
            }
        }
        *self = out;
    }

    pub fn occupancy(&self) -> f64 {
        let len = self.table.len();
        let bins = len as f64;
        let mut hits = 0.0;
        for i in 0..len {
            hits += self.table[i].len() as f64;
        }
        return hits / bins;
    }
}

pub struct BString<'a> {
    buf: &'a mut [u8],
    len: usize,
    arena: &'a Arena,
}
impl<'a> Trivial for BString<'a> {}

impl<'a> BString<'a> {
    pub fn new(arena: &'a Arena) -> Self {
        Self {
            buf: arena.alloc_bytes(16, 1),
            len: 0,
            arena: arena,
        }
    }

    pub fn push(&mut self, ch: char) {
        let sz = ch.len_utf8();
        if self.len + sz < self.buf.len() {
            ch.encode_utf8(&mut self.buf[self.len..self.len + sz]);
        } else {
            let buf2 = self.arena.alloc_bytes(self.buf.len() * 2, 1);
            for i in 0..self.len {
                buf2[i] = self.buf[i];
            }
            self.buf = buf2;
            ch.encode_utf8(&mut self.buf[self.len..self.len + sz]);
        }
        self.len += sz;
    }

    pub fn get_str(&self) -> &str {
        let bytes = &self.buf[0..self.len];
        std::str::from_utf8(bytes).unwrap()
    }

    pub fn concat(&mut self, v: &str) {
        for i in v.chars() {
            self.push(i);
        }
    }

    pub fn concat_writeable<T: Display>(&mut self, v: &T) {
        write!(self, "{}", v).unwrap();
    }

    pub fn concat_debug<T: Debug>(&mut self, v: &T) {
        write!(self, "{:#?}", v).unwrap();
    }

    pub fn take(self) -> &'a str {
        std::str::from_utf8(&self.buf[0..self.buf.len()]).unwrap()
    }
}

impl<'a> AsRef<str> for BString<'a> {
    fn as_ref(&self) -> &str {
        self.get_str()
    }
}

impl<'a> Display for BString<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_str())
    }
}
impl<'a> Debug for BString<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_str())
    }
}

impl<'a> std::fmt::Write for BString<'a> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        for i in s.chars() {
            self.push(i);
        }
        std::fmt::Result::Ok(())
    }
}

impl<'a> Hash for BString<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let s = self.as_ref();
        s.hash(state);
    }
}

pub fn dyn_sprintf<'a>(arena: &'a Arena, format: &str, args: &[&dyn Display]) -> BString<'a> {
    let mut out = BString::new(arena);
    let mut it = format.chars();
    let mut index = 0;
    loop {
        let Some(c) = it.next() else {
            break;
        };
        if c == '%' {
            let Some(c1) = it.next() else {
                break;
            };
            if c1 == '%' {
                out.push('%');
            } else if c1 == 'd' {
                out.concat_writeable(&args[index]);
                index += 1;
            } else if c1 == 'f' {
                out.concat_writeable(&args[index]);
                index += 1;
            } else if c1 == 's' {
                out.concat_writeable(&args[index]);
                index += 1;
            } else if c1 == 'u' {
                out.concat_writeable(&args[index]);
                index += 1;
            } else if c1 == '*' {
                out.concat_writeable(&args[index]);
                index += 1;
            } else {
                todo!()
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[macro_export]
macro_rules! sprintf {
    ($arena:expr, $fmt:literal) => {
        dyn_sprintf($arena, $fmt, &[])
    };
    ($arena:expr,$fmt:literal, $($args:expr),+) => {
        dyn_sprintf($arena, $fmt,(&[$(&$args), +]))
    };
}

pub struct Shared<'a, T: TrivialClone> {
    ptr: &'a SpinLock<T>,
}
impl<'a, T: TrivialClone> Clone for Shared<'a, T> {
    fn clone(&self) -> Self {
        Self { ptr: self.ptr }
    }
}
impl<'a, T: TrivialClone> Copy for Shared<'a, T> {}
impl<'a, T: TrivialClone> Shared<'a, T> {
    pub fn create(arena: &'a Arena, value: T) -> Self {
        Self {
            ptr: arena.alloc(SpinLock::new(value)),
        }
    }

    pub fn load(&self) -> T {
        self.ptr.get()
    }

    pub fn store(&self, value: T) {
        self.ptr.store(value);
    }

    pub fn lock(&self) -> Lock<'a, T> {
        self.ptr.lock()
    }
}

impl<'a, T: TrivialClone> Trivial for Shared<'a, T> {}
