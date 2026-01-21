use std::any::TypeId;
use std::marker::PhantomData;
use std::{cell::UnsafeCell, ops::Index};
use std::ops::{Drop, IndexMut};
#[allow(unused)]
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard, atomic::AtomicBool};

struct SharedListInner<T> {
    mutated: AtomicBool,
    locked:Spinlock<bool>,
    list: Vec<SpinRwLock<T>>,
}


pub struct SharedList<T: Clone> {
    list: Arc<SpinRwLock<SharedListInner<T>>>,
    has_lock:AtomicBool,
    debug_reader_count:Mutex<usize>, 
    debug_writer_active:Mutex<bool>,
}
impl<T:Clone> Clone for SharedList<T>{
    fn clone(&self) -> Self {
        Self { list: self.list.clone(), has_lock: AtomicBool::new(false) , debug_reader_count:Mutex::new(0), debug_writer_active:Mutex::new(false)}
    }
}

impl<T: Clone> Default for SharedList<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T:Clone> Drop for SharedList<T>{
    fn drop(&mut self) {
        self.unlock();
    }
}

impl<T: Clone> SharedList<T> {

    pub fn new() -> Self {
        Self {
            list: Arc::new(SpinRwLock::new(SharedListInner {
                mutated: AtomicBool::new(false),
                list: Vec::new(),
                locked:Spinlock::new(false)
            })),
            has_lock:AtomicBool::new(false),
            debug_reader_count:Mutex::new(0), debug_writer_active:Mutex::new(false)
        }
    }
    fn begin_write(&self){
        *self.debug_writer_active.lock().unwrap() = true;
    }
    fn end_write(&self){
        *self.debug_writer_active.lock().unwrap() = false;
    }
    fn begin_read(&self){
        *self.debug_reader_count.lock().unwrap() += 1;
    }
    fn end_read(&self){
        *self.debug_reader_count.lock().unwrap() -= 1;
    }
    /*
     *checks if the list has been mutated, if it has been mutated return true, set its mutation flag to false, otherwise return false
     * */
    fn handle_locks(&self){
        return;
        let lck= self.list.read();
        self.begin_read();
        let gb_lck = lck.get().locked.lock();
        if *gb_lck.get(){
            if self.has_lock.load(std::sync::atomic::Ordering::Acquire){
                
            }else{
                self.end_read();
                drop(gb_lck);
                drop(lck);
                loop{
                    let lck= self.list.read();
                    self.begin_read();
                    let gb_lck = lck.get().locked.lock(); 
                    if !*gb_lck.get(){
                        return;
                    }
                    self.end_read();
                    drop(gb_lck);
                    drop(lck);
                    std::thread::yield_now();
                }  
            }
        }else{

        }
    }
    pub fn consume_mutation(&self) -> bool {
        self.handle_locks();
        let guard = self.list.write();
        self.begin_write();
        if guard.get().mutated.load(std::sync::atomic::Ordering::Acquire) {
            guard.
                get().mutated
                .store(true, std::sync::atomic::Ordering::Release);
            self.end_write();
            true
        } else {
            self.end_write();
            false
        }
    }

    pub fn push(&self, v: T) {
        self.handle_locks();
        self.begin_write();
        let mut guard = self.list.write();
        guard.get_mut().list.push(SpinRwLock::new(v));
        guard.get()
            .mutated
            .store(true, std::sync::atomic::Ordering::Release);
        self.end_write();
        drop(guard);
    }

    pub fn pop(&self) -> Option<T> {
        self.handle_locks();
        let mut guard = self.list.write();
        self.begin_write();
        let out = guard.get_mut().list.pop();
        if out.is_some() {
            guard.get()
                .mutated
                .store(true, std::sync::atomic::Ordering::Release);
        }
        self.end_write();
        drop(guard);
        out.map(|i| i.take())
    }

    pub fn get(&self, index: usize) -> Option<T> {
        self.handle_locks();
        let guard = self.list.read();
        self.begin_read();
        let x = guard.get().list.get(index);
        let out = x.map(|i| {
            let tmp = i.read();
            let get = tmp.get();
            let out = get.clone();
            out
        });
        self.end_read();
        drop(guard);
        out
    }

    pub fn pop_front(&self) -> Option<T> {
        self.handle_locks();
        let mut guard = self.list.write();
        self.begin_write();
        if guard.get().list.is_empty() {
            self.end_write();
            None
        } else {
            let out = guard.get_mut().list.remove(0);
            guard.get()
                .mutated
                .store(true, std::sync::atomic::Ordering::Release);
            self.end_write();
            drop(guard); 
            Some(out.take())
        }
    }

    pub fn insert(&self, v: T, index: usize) -> Result<(), T> {
        self.handle_locks();
        let mut guard = self.list.write();
        self.begin_write();
        if guard.get().list.is_empty() {
            self.end_write();
            Err(v)
        } else {
            guard.get()
                .mutated
                .store(true, std::sync::atomic::Ordering::Release);
            guard.get_mut().list.insert(index, SpinRwLock::new(v));
            self.end_write();
            Ok(())
        }
    }

    pub fn swap(&self, v: T, index: usize) -> Result<T, T> {
        self.handle_locks();
        let guard = self.list.read();
        self.begin_read();
        let x = guard.get().list.get(index);
        let mut vp = v;
        if let Some(x) = x {
            let mut xlck = x.write();
            std::mem::swap(xlck.get_mut(), &mut vp);
            guard.get()
                .mutated
                .store(true, std::sync::atomic::Ordering::Release);
            self.end_read();
            Ok(vp)
        } else {
            self.end_read();
            Err(vp)
        }
    }

    pub fn replace(&self, v: T, index: usize) -> Result<(), T> {
        self.handle_locks();
        let guard = self.list.read();;
        self.begin_read();
        if index >= guard.get().list.len() {
            return Err(v);
        }
        let x = guard.get()
            .list
            .get(index)
            .expect("list better be inside the thing");
        guard.get()
            .mutated
            .store(true, std::sync::atomic::Ordering::Release);
        let mut xlock = x.write();
        *xlock.get_mut() = v;
        self.end_read();
        Ok(())
    }

    pub fn iter(&self) -> SharedListIter<T> {
        SharedListIter {
            list: self.list.clone(),
            index: 0,
        }
    }

    pub fn read_at(&self, index: usize) -> Option<SharedListReader<'_, T>> {
        self.handle_locks();
        self.begin_read();
        let list = self.list.read();
        if index < list.get().list.len() {
            unsafe {
                list.get().list[index].mark_reader();
            }
            let out = SharedListReader { list, index , value:self};
            Some(out)
        } else {
            self.end_read();
            None
        }
    }

    pub fn write_at(&self, index: usize) -> Option<SharedListWriter<'_, T>> {
        self.handle_locks();
        let list = self.list.read();
        self.begin_read();
        list.get().mutated
            .store(true, std::sync::atomic::Ordering::Release);
        if index < list.get().list.len() {
            unsafe {
                list.get().list[index].mark_writer();
            }
            let out = SharedListWriter { list, index, _data:Default::default(),value:self};
            Some(out)
        } else {
            self.end_read();
            None
        }
    }

    pub fn len(&self) -> usize {
        self.handle_locks();
        self.begin_read();
        let guard = self.list.read();
        self.end_read();
        guard.get().list.len()
    }

    pub fn lock(&self){
        loop{
            let rd = self.list.read();
            self.begin_read();
            let mut x = rd.get().locked.lock();
            if !*x.get(){
                *x.get_mut() = true;
                self.has_lock.store(true, std::sync::atomic::Ordering::Release);
                drop(x);
                drop(rd);
                self.end_read();
                break;
            }else{
                self.end_read();
                drop(x);
                drop(rd);
                std::thread::yield_now();
            }
        }
    }

    pub fn unlock(&self){
        if self.has_lock.load(std::sync::atomic::Ordering::Acquire){
            let rd = self.list.read();
            self.begin_read();
            let mut x = rd.get().locked.lock();
            *x.get_mut() = false;
            self.end_read();
            self.has_lock.store(false, std::sync::atomic::Ordering::Release);
        }
    }
    pub fn debug_state(&self, msg:&str){
        let reads = *self.debug_reader_count.lock().unwrap();
        let write = *self.debug_writer_active.lock().unwrap();
        println!("{}:reads:{}, being_written:{}", msg,reads,write);
    }
}


pub struct SharedListIter<T: Clone> {
    list: Arc<SpinRwLock<SharedListInner<T>>>,
    index: usize,
}

impl<T: Clone> Iterator for SharedListIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        let lock = self.list.read();
        if self.index < lock.get().list.len() {
            let index = self.index;
            self.index += 1;
            Some(lock.get().list[index].read().get().clone())
        } else {
            None
        }
    }
}

impl<T: Clone + 'static> IntoIterator for SharedList<T> {
    type Item = T;
    type IntoIter = SharedListIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T: Clone + 'static> IntoIterator for &SharedList<T> {
    type Item = T;
    type IntoIter = SharedListIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct SharedListReader<'a, T: Clone> {
    list: SpinReadGuard<'a, SharedListInner<T>>,
    value:&'a SharedList<T>,
    index: usize,
}
impl<'a, T: Clone> Drop for SharedListReader<'a, T> {
    fn drop(&mut self) {
        unsafe {
            self.value.end_read();
            self.list.get().list[self.index].mark_reader_done();
        }
    }
}
impl<'a, T: Clone> SharedListReader<'a, T> {
    pub fn get(&self) -> &T {
        unsafe { self.list.get().list[self.index].get() }
    }
}
pub struct SharedListWriter<'a, T: Clone> {
    list: SpinReadGuard<'a, SharedListInner<T>>,
    index: usize,
    value:&'a SharedList<T>,
    _data:PhantomData<&'a mut T>,
}
impl<'a, T: Clone> Drop for SharedListWriter<'a, T> {
    fn drop(&mut self) {
        unsafe {
            self.value.end_read();
            self.list.get().list[self.index].mark_writer_done();
        }
    }
}
impl<'a, T: Clone> SharedListWriter<'a, T> {
    pub fn get(&self) -> &T {
        unsafe { self.list.get().list[self.index].get() }
    }
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { self.list.get().list[self.index].get_mut() }
    }
}
pub struct Spinlock<T> {
    v: UnsafeCell<T>,
    lock: AtomicBool,
}

pub struct SpinLockGuard<'a, T> {
    inner: &'a Spinlock<T>,
}
impl<'a, T> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        unsafe { assert!(self.inner.mark_unlock()) }
    }
}

impl<'a, T> SpinLockGuard<'a, T> {
    pub fn get(&self) -> &T {
        unsafe { self.inner.get() }
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { self.inner.get_mut() }
    }
}

impl<T> Spinlock<T> {
    pub fn new(value: T) -> Self {
        Self {
            v: UnsafeCell::new(value),
            lock: AtomicBool::new(false),
        }
    }

    pub unsafe fn get(&self) -> &T {
        unsafe { &*self.v.get() }
    }

    pub unsafe fn get_mut(&self) -> &mut T {
        unsafe { self.v.get().as_mut().unwrap() }
    }

    pub unsafe fn try_mark_locked(&self) -> bool {
        let rs = self.lock.compare_exchange_weak(
            false,
            true,
            std::sync::atomic::Ordering::AcqRel,
            std::sync::atomic::Ordering::Acquire,
        );
        rs.is_ok()
    }

    pub unsafe fn mark_locked(&self) {
        unsafe {
            let mut idx = 0;
            while !self.try_mark_locked() {
                idx += 1;
                if idx > 10 {
                    idx = 0;
                    std::thread::yield_now();
                }
            }
        }
    }

    pub unsafe fn mark_unlock(&self) -> bool {
        self.lock.store(false, std::sync::atomic::Ordering::Release);
        true
    }

    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        unsafe {
            self.mark_locked();
            SpinLockGuard { inner: self }
        }
    }

    pub fn try_lock(&self) -> Option<SpinLockGuard<'_, T>> {
        unsafe {
            if self.try_mark_locked() {
                Some(SpinLockGuard { inner: self })
            } else {
                None
            }
        }
    }
    pub fn take(self) -> T {
        unsafe {
            self.mark_locked();
            self.v.into_inner()
        }
    }
}

unsafe impl<T: Send> Send for Spinlock<T> {}

unsafe impl<T: Sync> Sync for Spinlock<T> {}

#[test]
pub fn spin_lock_test() {
    let lck = Spinlock::new(10);
    let mut x = lck.lock();
    assert!(lck.try_lock().is_none());
    *x.get_mut() = 12;
    drop(x);
    let y = lck.lock();
    assert!(*y.get() == 12);
}

#[test]
pub fn spin_lock_thread_test() {
    let lck = Arc::new(Spinlock::new(0));
    let mut handles = Vec::new();
    let tcount = 100;
    let icount = 10000;
    for _ in 0..tcount {
        let lck2 = lck.clone();
        let h = std::thread::spawn(move || {
            for _ in 0..icount {
                let mut lc = lck2.lock();
                *lc.get_mut() += 1;
            }
        });
        handles.push(h);
    }
    for i in handles {
        i.join().unwrap();
    }
    let lck2 = lck.lock();
    assert!(*lck2.get() == tcount * icount);
}

struct SpinRwLockData {
    writer: bool,
    readers: usize,
}
pub struct SpinRwLock<T> {
    lock: Mutex<SpinRwLockData>,
    value: UnsafeCell<T>,
}
impl<T> SpinRwLock<T> {
    pub fn new(value: T) -> Self {
        Self {
            lock: Mutex::new(SpinRwLockData {
                writer: false,
                readers: 0,
            }),
            value: UnsafeCell::new(value),
        }
    }

    pub unsafe fn try_mark_writer(&self) -> bool {
        let Ok(mut lock) = self.lock.try_lock() else {
            return false;
        };
        if !lock.writer && lock.readers == 0 {
            lock.writer = true;
            true
        } else {
            false
        }
    }

    pub unsafe fn try_mark_reader(&self) -> bool {
        let Ok(mut lock) = self.lock.try_lock() else {
            return false;
        };
        if !lock.writer {
            lock.readers += 1;
            true
        } else {
            false
        }
    }

    pub unsafe fn mark_reader(&self) {
        unsafe {
            let mut count = 0;
            while !self.try_mark_reader() {
                count += 1;
                if count > 100 {
                    std::thread::yield_now();
                    count =0;
                }
            }
        }
    }

    pub unsafe fn mark_writer(&self) {
        unsafe {
            let mut count = 0;
            while !self.try_mark_writer() {
                count += 1;
                if count > 100 {
                    std::thread::yield_now();
                    count =0;
                }
            }
        }
    }

    pub unsafe fn mark_reader_done(&self) {
        let mut lock = self.lock.lock().unwrap();
        assert!(lock.readers > 0);
        lock.readers -= 1;
    }

    pub unsafe fn mark_writer_done(&self) {
        let mut lock = self.lock.lock().unwrap();
        assert!(lock.writer);
        lock.writer = false;
    }

    pub unsafe fn get(&self) -> &T {
        unsafe { self.value.get().as_ref().unwrap() }
    }

    pub unsafe fn get_mut(&self) -> &mut T {
        unsafe { self.value.get().as_mut().unwrap() }
    }

    pub fn try_read(&self) -> Option<SpinReadGuard<'_, T>> {
        unsafe {
            let can_get = self.try_mark_reader();
            if can_get {
                Some(SpinReadGuard { inner: self })
            } else {
                None
            }
        }
    }

    pub fn try_write(&self) -> Option<SpinWriteGuard<'_, T>> {
        unsafe {
            let can_get = self.try_mark_writer();
            if can_get {
                Some(SpinWriteGuard { inner: self })
            } else {
                None
            }
        }
    }

    pub fn read(&self) -> SpinReadGuard<'_, T> {
        unsafe {
            self.mark_reader();
            SpinReadGuard { inner: self }
        }
    }

    pub fn write(&self) -> SpinWriteGuard<'_, T> {
        unsafe {
            self.mark_writer();
            SpinWriteGuard { inner: self }
        }
    }

    pub fn take(self) -> T {
        unsafe {
            let mut count = 0;
            while !self.try_mark_writer() {
                count += 1;
                if count > 10 {
                    std::thread::yield_now();
                    count =0;
                }
 
            }
            self.value.into_inner()
        }
    }
}
unsafe impl<T> Send for SpinRwLock<T>{

}
unsafe impl<T> Sync for SpinRwLock<T>{

}
pub struct SpinReadGuard<'a, T> {
    inner: &'a SpinRwLock<T>,
}

pub struct SpinWriteGuard<'a, T> {
    inner: &'a SpinRwLock<T>,
}

impl<'a, T> Drop for SpinReadGuard<'a, T> {
    fn drop(&mut self) {
        unsafe {
            self.inner.mark_reader_done();
        }
    }
}

impl<'a, T> Drop for SpinWriteGuard<'a, T> {
    fn drop(&mut self) {
        unsafe {
            self.inner.mark_writer_done();
        }
    }
}

impl<'a, T> SpinReadGuard<'a, T> {
    pub fn get(&self) -> &T {
        unsafe { self.inner.get() }
    }
}

impl<'a, T> SpinWriteGuard<'a, T> {
    pub fn get(&self) -> &T {
        unsafe { self.inner.get() }
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { self.inner.get_mut() }
    }
}
