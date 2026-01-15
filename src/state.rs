pub use crate::rtils::rtils_useful::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    sync::{Arc, Mutex},
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct Id {
    id: u64,
}
impl Id {
    pub const fn invalid() -> Self {
        Self { id: 0 }
    }
    pub const fn get(&self) -> u64 {
        self.id
    }
}

pub const ID_COUNT: u64 = 64000;
pub struct IdAllocatorObj {
    start: u64,
    used: BTreeSet<u64>,
}
impl IdAllocatorObj {
    pub const fn new(start: u64) -> Self {
        Self {
            start,
            used: BTreeSet::new(),
        }
    }

    pub fn alloc(&mut self) -> Id {
        let mut min = 0;
        for i in self.start..self.start + ID_COUNT {
            if !self.used.contains(&i) {
                self.used.insert(i);
                min = i;
                break;
            }
        }
        assert!(min != 0);
        Id { id: min }
    }

    pub fn dealloc(&mut self, value: Id) {
        assert!(self.used.remove(&value.get()));
    }

    pub fn bloc_ptr(&self) -> u64 {
        self.start
    }
}

pub struct IdAllocator {
    inner: Arc<Mutex<IdAllocatorObj>>,
}
impl IdAllocator {
    pub fn new(start: u64) -> Self {
        Self {
            inner: Arc::new(Mutex::new(IdAllocatorObj::new(start))),
        }
    }

    pub fn alloc(&self) -> Id {
        let mut lsck = self.inner.lock().unwrap();
        lsck.alloc()
    }

    pub fn dealloc(&self, to_free: Id) {
        let mut lsck = self.inner.lock().unwrap();
        lsck.dealloc(to_free);
    }

    pub fn bloc_ptr(&self) -> u64 {
        let lsck = self.inner.lock().unwrap();
        lsck.bloc_ptr()
    }
}

pub struct GlobalIdManager {
    used: Arc<Mutex<BTreeSet<u64>>>,
}
impl GlobalIdManager {
    pub fn new() -> Self {
        let mut x = BTreeSet::new();
        x.insert(0);
        Self {
            used: Arc::new(Mutex::new(x)),
        }
    }

    pub fn alloc_bloc(&self) -> u64 {
        let mut out = 0;
        let mut lck = self.used.lock().unwrap();
        for i in 1..u64::MAX / ID_COUNT {
            if !lck.contains(&i) {
                lck.insert(i);
                out = i;
            }
        }
        assert!(out != 0);
        out * ID_COUNT
    }

    pub fn free_bloc(&self, block: u64) {
        let mut lck = self.used.lock().unwrap();
        assert!(lck.remove(&(block / ID_COUNT)));
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectType {
    Token { image: String, scale: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
    pub pos: Pos,
    pub display_name: String,
    pub object_type: ObjectType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageData {
    HandShake {
        state: State,
        allocator_start: u64,
        user_id: Id,
    },
    Connect {
        username: String,
    },
    Disconnect {
        username: String,
        allocator_start: u64,
    },
    MoveObject {
        id: Id,
        to: Pos,
    },
    DestroyObject {
        id: Id,
    },
    CreateObject {
        id: Id,
        obj: Object,
    },
    UpdateObject {
        id: Id,
        obj: Object,
    },
    FullStateUpdate{
        state:State,
    },
    RequestFullStateUpdate,
    }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetaData {
    pub sender: Id,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub meta: MessageMetaData,
    pub data: MessageData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub objects: HashMap<Id, Object>,
    pub self_id: Id,
}
impl State {
    pub fn handle_messsage(&mut self, message: Message) -> Throws<()> {
        if message.meta.sender == self.self_id {
            return Ok(());
        }
        match message.data {
            MessageData::MoveObject { id, to } => {
                let x = self.objects.get_mut(&id).throw()?;
                x.pos = to;
            }
            MessageData::CreateObject { id, obj } => {
                if self.objects.contains_key(&id) {
                    todo!();
                } else {
                    self.objects.insert(id, obj);
                }
            }
            MessageData::DestroyObject { id } => {
                if !self.objects.contains_key(&id) {
                    todo!();
                } else {
                    self.objects.remove(&id);
                }
            }
            MessageData::UpdateObject { id, obj } => {
                *self.objects.get_mut(&id).throw()? = obj;
            }
            MessageData::FullStateUpdate { state }=>{
                *self = state;
            }
            _ => {
                todo!()
            }
        }
        Ok(())
    }
    pub fn new(id:Id)->Self{
        Self { objects:HashMap::new() , self_id:id }
    }
}
