use std::{cell::UnsafeCell, collections::HashMap, error::Error, fmt::{Debug, Display}, mem::{MaybeUninit, needs_drop}, ops::{Deref, DerefMut}, ptr::slice_from_raw_parts_mut, sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard, Weak}};


struct ArenaInternal{
    bytes:Arc<[UnsafeCell<u8>]>,
    next:Mutex<usize>,
    destructors:Mutex<HashMap<*const u8, Box<dyn FnOnce()>>>,
    next_arena:Mutex<Option<Box<ArenaInternal>>>,

}
impl Drop for ArenaInternal{
    fn drop(&mut self) {
        let mut dest = self.destructors.lock().unwrap();
        for i in dest.drain(){
            (i.1)();
        }
    }
}
unsafe impl Send for ArenaInternal{

}
unsafe impl Sync for ArenaInternal{
    
}
impl ArenaInternal{
    pub fn new()->Self{
        let mut out = Vec::new();
        out.reserve_exact(4096*16);
        for _ in 0..4096*16{
            out.push(UnsafeCell::new(0));
        }
        Self { bytes: out.into(), next: Mutex::new(0), destructors:Mutex::new(HashMap::new()) ,next_arena:Mutex::new(None)}
    }
    pub fn new_with_capacity(count:usize)->Self{
        let mut out = Vec::new();
        out.reserve_exact(count);
        for _ in 0..count{
            out.push(UnsafeCell::new(0));
        }
        Self { bytes: out.into(), next: Mutex::new(0), destructors:Mutex::new(HashMap::new()) ,next_arena:Mutex::new(None)}
    }
    #[allow(clippy::mut_from_ref)]
    pub fn alloc_bytes(&self, count:usize, align:usize)->&mut [u8]{
        unsafe{
            let mut n = self.next.lock().unwrap();
            let mut l = *n;
            if l%align != 0{
                l+= align-l%align;
            }
            if l+count>= self.bytes.len(){
                let mut n = self.next_arena.lock().unwrap();
                if let Some(nxt)  = n.as_ref(){
                   let nt = nxt.alloc_bytes(count, align) as *mut [u8];
                   (nt).as_mut().unwrap()
                } else{
                    let mut nc = self.bytes.len();
                    while nc< count{
                        nc *= 2;
                    }
                    let next_ar = ArenaInternal::new_with_capacity(nc);
                    *n = Some(Box::new(next_ar));
                    let p = n.as_ref().unwrap();
                    let ptr = p.alloc_bytes(count, align);
                    let ptr2 = ptr as *mut [u8];
                    (ptr2).as_mut().unwrap()
                }
                
            }else{
                let ptr = (self.bytes.as_ptr() as *const _ as *mut u8).add(l);
                *n = l+count;
                slice_from_raw_parts_mut(ptr, count).as_mut().unwrap()
            }

        }
    }
    pub fn mark_to_destroy<T>(&self, ptr:*mut T){
        if !needs_drop::<T>(){
            return;
        }
        let mut map = self.destructors.lock().unwrap();
        if let Some(p) = map.remove(&(ptr as *const u8)){
            p();
        }
        let p = ptr as *mut u8;
        let func = Box::new(move ||{
            unsafe{
                let p2 = p as *mut T;
                std::ptr::drop_in_place(p2);
            }
        });
        map.insert(ptr as *const u8, func);
    }
    #[allow(clippy::mut_from_ref)]
    pub fn alloc<T>(&self, v:T)->&mut T{
        unsafe{
            let out = self.alloc_bytes(size_of::<T>(), align_of::<T>());
            let ptr = out.as_mut_ptr() as *mut T;
            std::ptr::write(ptr, v);
            self.mark_to_destroy(ptr);
            ptr.as_mut().unwrap()
        }
    }
    #[allow(clippy::mut_from_ref)]
    pub fn alloc_space<T>(&self, count:usize)->&mut [MaybeUninit<T>]{
        unsafe{
            let out = self.alloc_bytes(count *size_of::<T>(),  align_of::<T>()) as *mut [u8] as *mut u8;
            let pt =out as *mut MaybeUninit<T>;
            slice_from_raw_parts_mut(pt,count).as_mut().unwrap()
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn realloc<T:Clone>(&self, list:&[T], new_count:usize)->&mut [MaybeUninit<T>]{
        let out = self.alloc_space::<T>(new_count);
        let mut l =list.len();
        if l>new_count{
            l = new_count;
        }
        for i in 0..l{
            let ptr = &mut out[i] as *mut _ as *mut T;
            self.mark_to_destroy(ptr);
            out[i].write(list[i].clone());
        }
        out
    }
}
pub trait FromArena<T>{
    fn new_arena(arena:&Arena, v:T)->Self;
}
pub struct Arena{
    ptr:Arc<ArenaInternal>
}
impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

impl Arena{
     pub fn new()->Self{
        Self {ptr: Arc::new(ArenaInternal::new()) }
    }
    pub fn new_with_capacity(count:usize)->Self{
        Self { ptr: Arc::new(ArenaInternal::new_with_capacity(count)) }
    }
    pub fn alloc_bytes(&self, count:usize, align:usize)->&mut [u8]{
        self.ptr.alloc_bytes(count, align)
    }
    pub fn mark_to_destroy<T>(&self, ptr:*mut T){
        self.ptr.mark_to_destroy(ptr);
    }
    pub fn alloc<T>(&self, v:T)->&mut T{
        self.ptr.alloc(v)
    }
    pub fn alloc_space<T>(&self, count:usize)->&mut [MaybeUninit<T>]{
        self.ptr.alloc_space(count)
    }
    pub fn realloc<T:Clone>(&self, list:&[T], new_count:usize)->&mut [MaybeUninit<T>]{
        self.ptr.realloc(list, new_count)
    }
    pub fn new_obj<T>(&self, v:T)->Object<T>{
        Object::new(self, v)
    }
    pub fn clear(&mut self){

    }

}
#[derive(Clone)]
pub struct Object<T:?Sized>{
    value:*const RwLock<T>, 
    arena:Weak<ArenaInternal>
}
impl<T> PartialEq for Object<T>{
    fn eq(&self, other: &Self) -> bool {
        self.value.eq(&other.value) 
    }
}
unsafe impl<T> Send for Object<T>{

}
unsafe impl<T> Sync for Object<T>{

}
impl<T:Debug+?Sized> Debug for Object<T>{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(v) = self.getc(){
            write!(f, "{:#?}",v.as_ref())
        }else{
            write!(f, "null")
        }

    }
}
pub struct ObjectRef<'a,T:?Sized>{
    v:RwLockReadGuard<'a,T>, 
    _arena_ref:Arc<ArenaInternal>,
}
impl<'a, T> Deref for ObjectRef<'a,T>{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.v
    }
}

pub struct ObjectMut<'a,T:?Sized>{
    v:RwLockWriteGuard<'a,T>, 
    _arena_ref:Arc<ArenaInternal>,
}
impl<'a, T> Deref for ObjectMut<'a,T>{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.v
    }
}
impl<'a, T> DerefMut for ObjectMut<'a,T>{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.v
    }
}
impl <'a, T:?Sized> AsRef<T> for ObjectMut<'a, T>{
    fn as_ref(&self) -> &T {
        &self.v
    }
}
impl <'a, T:?Sized> AsMut<T> for ObjectMut<'a, T>{
    fn as_mut(&mut self) -> &mut T {
        &mut self.v
    }
}
impl <'a, T:?Sized> AsRef<T> for ObjectRef<'a, T>{
    fn as_ref(&self) -> &T {
        &self.v
    }
}
#[macro_export]
macro_rules! new {
    ($arena:ident,$T:ty, $($args:expr),+) => {
        <$T>::new(&$arena,  $($args),+)
    };
}
impl <T> Object<T>{
    pub fn new(arena:&Arena,v:T)->Self{
        let value = arena.alloc(RwLock::new(v)) as *const RwLock<T>;
        let arena =Arc::downgrade(&arena.ptr);
        Self { value, arena}
    }
    pub fn null()->Self{
        Self { value: std::ptr::null::<u8>() as *const RwLock<T>, arena: Weak::new() }
    }
} 
impl <T:?Sized> Object<T>{
    pub fn is_null(&self)->bool{
        if self.value.is_null(){
            true
        } else if self.arena.upgrade().is_some(){
            false
        }else{
            true
        }
    }
    pub fn is_valid(&self)->bool{
        !self.is_null()
    }
    pub fn get(&self)->ObjectRef<T>{
        if self.value.is_null(){
            todo!()
        }
        let Some(_arena_ref) = self.arena.upgrade()else {
            todo!()
        };
        unsafe{
            ObjectRef{v:(*self.value).read().unwrap(),_arena_ref}
        }
    }
    pub fn get_mut(&self)->ObjectMut<T>{
       if self.value.is_null(){
            todo!()
        }
        let Some(_arena_ref) = self.arena.upgrade()else {
            todo!()
        };
        unsafe{
            ObjectMut{v:(*self.value).write().unwrap(), _arena_ref}
        }
    }
    pub fn getc(&self)->Result<ObjectRef<T>, MemoryException>{
            if self.value.is_null(){
                return Err(MemoryException::NullPtr);
            }
            let Some(_arena_ref) = self.arena.upgrade() else {
                return Err(MemoryException::DanglingPtr { address: self.value as *const usize as usize })
            };
            unsafe{
                Ok(ObjectRef{v:(*self.value).read().unwrap(),_arena_ref})
            }
    }
    pub fn get_mutc(&self)->Result<ObjectMut<T>, MemoryException>{
        if self.value.is_null(){
            return Err(MemoryException::NullPtr);
        }
        let Some(_arena_ref) = self.arena.upgrade() else
        {
            return Err(MemoryException::DanglingPtr { address: self.value as *const usize as usize })
        };
        unsafe{
            Ok(ObjectMut{v:(*self.value).write().unwrap(),_arena_ref})
        }
    }
}
impl <T, const size:usize> Into<Object<[T]>> for Object<[T; size]>{
    fn into(self) -> Object<[T]> {
        unsafe{
            Object::<[T]>{ value: self.value.as_ref().unwrap(), arena: self.arena }
        }

    }
}

#[derive(Debug)]
pub enum MemoryException{
    NullPtr,DanglingPtr{address:usize}, IndexOutOfBounds{index:usize, len:usize}
}
impl Display for MemoryException{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            Self::DanglingPtr{address}=>{
                write!(f, "attempt to access dangling pointer:{:#?}", *address as *const u8)
            }
            Self::NullPtr=>{
                write!(f, "attempt to access null pointer")
            }
            Self::IndexOutOfBounds { index, len }=>{
                write!(f, "attempt to access out of bounds: index:{}, length:{}", *index, *len)
            }
        }

    }
}
impl Error for MemoryException{

}
/* 
pub struct ArrayList<'a,T:Clone>{
    list:*mut [MaybeUninit<T>],
    len: usize, 
    arena:&'a Arena
}
impl<'a,T:Clone> ArrayList<'a,T>{
    pub fn new(arena:&'a Arena)->Self{
        Self { list: slice_from_raw_parts_mut(null_mut(), 0), len: 0, arena: arena }
    }
    pub fn push(&mut self,v:T){
        unsafe{
            if self.list.len() == 0{
                self.list = self.arena.alloc_space(8);
            }else if self.list.len()<=self.len+1{
                self.list = self.arena.realloc(slice_from_raw_parts(self.list as *const _ as * const T, self.len).as_ref().unwrap(), self.list.len()*2);
            }
            let ptr = (self.list as *mut _ as *mut T).add(self.len);
            self.arena.mark_to_destroy(ptr);
            self.len+=1;
            ptr.write(v);
        }
    }
    pub fn get(&self, idx:usize)->Option<&T>{
        unsafe{
            if idx>=self.len{
                None
            }else{
                Some((self.list as *const _ as *const T).add(idx).as_ref().unwrap())
            }
        }
    }
    pub fn get_mut(&mut self, idx:usize)->Option<&mut T>{
        unsafe{
            if idx>=self.len{
                None
            }else{
                Some((self.list as *mut  _ as *mut T).add(idx).as_mut().unwrap())
            }
        }
    }
    pub fn as_slice(&self)->&[T]{
        unsafe{
            slice_from_raw_parts(self.list as *const _ as *const T, self.len).as_ref().unwrap()
        }
    }
    pub fn as_mut(&mut self)->&mut [T]{
        unsafe{
            slice_from_raw_parts_mut(self.list as *mut _ as *mut T, self.len).as_mut().unwrap()
        }
    }
    pub fn len(&self)->usize{
        self.len
    }
}

impl<'a, T: Clone> AsRef<[T]> for ArrayList<'a, T>{
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}
impl<'a, T: Clone> AsMut<[T]> for ArrayList<'a, T>{
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut()
    }
}
impl<'a, T:Clone> Index<usize> for ArrayList<'a, T>{
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).unwrap()
    }
}
impl<'a, T:Clone> IndexMut<usize> for ArrayList<'a, T>{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}
impl <'a, T:Clone> Index<Range<usize>> for ArrayList<'a, T>{
    type Output = [T];
    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self.as_slice()[index]
    }
}
impl <'a, T:Clone> IndexMut<Range<usize>> for ArrayList<'a, T>{
    fn index_mut(&mut self, index: Range<usize>) -> &mut Self::Output {
        &mut self.as_mut()[index]
    }
}
impl <'a, T:Clone+Debug> Debug for ArrayList<'a, T>{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_list();
        for i in self.as_ref(){
            s.entry(i);
        }
        s.finish()
    }
}
impl <'a, T:Clone> Clone for ArrayList<'a, T>{
    fn clone(&self) -> Self {
        let ptr = self.arena.realloc(self.as_slice(), self.len());
        Self { list: ptr, len: self.len(), arena: self.arena }
    }
}*/