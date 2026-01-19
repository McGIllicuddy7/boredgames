pub use crate::rtils::rtils_useful::*;
use crate::server::MessagePipe;
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
        self.used.remove(&value.get());
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
impl Default for GlobalIdManager {
    fn default() -> Self {
        Self::new()
    }
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
        for i in 1..(u64::MAX / ID_COUNT) {
            if !lck.contains(&i) {
                lck.insert(i);
                out = i;
                break;
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Layer {
    Map,
    Tokens,
    Gm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectType {
    Token { image: String, scale: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
    pub pos: Pos,
    pub layer: Layer,
    pub display_name: String,
    pub object_type: ObjectType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageData {
    HandShake {
        state: State,
        allocator_start: u64,
        images: HashMap<String, Arc<[u8]>>,
        users: HashMap<Id, UserClient>,
        user_id: Id,
    },
    Connect {
        username: UserClient,
        connected_id: Id,
    },
    Disconnect {
        username: UserClient,
        disconnected_id: Id,
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
    ChangeObjectLayer {
        id: Id,
        layer: Layer,
    },
    FullStateUpdate {
        state: State,
        connections: HashMap<Id, UserClient>,
    },
    EntireStateUpdate {
        state: State,
        connections: HashMap<Id, UserClient>,
        images: HashMap<String, Arc<[u8]>>,
    },
    RequestFullStateUpdate,
    RequestEntireUpdate,
    ImageUpload {
        name: String,
        data: Arc<[u8]>,
    },
    ImageDelete {
        to_delete: String,
    },
    Msg {
        from: String,
        contents: String,
    },
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
    pub fn handle_message(&mut self, message: Message) -> Throws<()> {
        if message.meta.sender == self.self_id {
            return Ok(());
        }
        match message.data {
            MessageData::MoveObject { id, to } => {
                let x = self.objects.get_mut(&id).throw()?;
                x.pos = to;
            }
            MessageData::ChangeObjectLayer { id, layer } => {
                let x = self.objects.get_mut(&id).throw()?;
                x.layer = layer;
            }
            MessageData::CreateObject { id, obj } => {
                if let std::collections::hash_map::Entry::Vacant(e) = self.objects.entry(id) {
                    e.insert(obj);
                } else {
                    todo!();
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
            _ => {
                todo!()
            }
        }
        Ok(())
    }
    pub fn new(id: Id) -> Self {
        Self {
            objects: HashMap::new(),
            self_id: id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserClient {
    pub username: String,
}

pub struct ClientData {
    pub images: HashMap<String, Arc<[u8]>>,
    pub state: State,
    pub connection: MessagePipe,
    pub others: HashMap<Id, UserClient>,
    pub allocator: IdAllocator,
    pub messages: Vec<(String, String)>,
    pub created_images: Vec<String>,
    pub deleted_images: Vec<String>,
    pub notify_new_image: bool,
    pub notify_deleted_image: bool,
    pub notify_new_message: bool,
}
impl ClientData {
    pub fn new(mut connection: MessagePipe, username: String) -> Throws<Self> {
        let msg = connection.read_message_blocking()?;
        if let MessageData::HandShake {
                mut state,
                allocator_start,
                images,
                users,
                user_id,
            } = msg.data {
            state.self_id = user_id;
            connection.write_message(Message {
                meta: MessageMetaData { sender: user_id },
                data: MessageData::Connect {
                    username: UserClient { username },
                    connected_id: user_id,
                },
            })?;
            return Ok(Self {
                images,
                state,
                connection,
                others: users,
                allocator: IdAllocator::new(allocator_start),
                notify_new_image: false,
                notify_deleted_image: false,
                notify_new_message: false,
                messages: Vec::new(),
                created_images: Vec::new(),
                deleted_images: Vec::new(),
            });
        }
        todo!()
    }

    pub fn update(&mut self) -> Throws<()> {
        for i in &mut self.connection {
            let x = i?;
            match &x.data {
                MessageData::ImageUpload { name, data } => {
                    self.created_images.push(name.clone());

                    self.notify_new_image = true;
                    self.images.insert(name.clone(), data.clone());
                }
                MessageData::HandShake {
                    state,
                    allocator_start,
                    images,
                    users,
                    user_id,
                } => {
                    self.images = images.clone();
                    self.others = users.clone();
                    self.allocator = IdAllocator::new(*allocator_start);
                    self.state = state.clone();
                    self.state.self_id = *user_id;
                }
                MessageData::FullStateUpdate { state, connections } => {
                    self.state = state.clone();
                    self.others = connections.clone();
                }
                MessageData::EntireStateUpdate {
                    state,
                    connections,
                    images,
                } => {
                    self.state = state.clone();
                    self.others = connections.clone();
                    self.images = images.clone();
                }
                MessageData::DestroyObject { id } => {
                    let sid = *id;
                    let r = self.state.handle_message(x);
                    self.allocator.dealloc(sid);
                    r?
                }
                MessageData::Connect {
                    username,
                    connected_id,
                } => {
                    self.others.insert(*connected_id, username.clone());
                }
                MessageData::Disconnect {
                    username: _,
                    disconnected_id,
                    allocator_start: _,
                } => {
                    self.others.remove(disconnected_id);
                }
                MessageData::ImageDelete { to_delete } => {
                    self.deleted_images.push(to_delete.clone());
                    self.images.remove(to_delete);
                    self.notify_deleted_image = true;
                }
                MessageData::Msg { from, contents } => {
                    self.messages.push((from.clone(), contents.clone()));
                    self.notify_new_message = true;
                }
                _ => {
                    self.state.handle_message(x)?;
                }
            }
        }
        self.others.remove(&self.state.self_id);
        Ok(())
    }

    pub fn run_cmd(&mut self, msg: Message) -> Throws<()> {
        let mut msg2 = msg.clone();
        msg2.meta.sender = Id::invalid();
        match &msg.data {
            MessageData::ImageUpload { name, data } => {
                self.created_images.push(name.clone());
                self.notify_new_image = true;
                self.images.insert(name.clone(), data.clone());
            }
            MessageData::ImageDelete { to_delete } => {
                self.deleted_images.push(to_delete.clone());
                self.images.remove(to_delete);
                self.notify_deleted_image = true;
            }
            MessageData::CreateObject { id: _, obj: _ } => {
                self.state.handle_message(msg2)?;
            }
            MessageData::DestroyObject { id: _ } => {
                self.state.handle_message(msg2)?;
            }
            MessageData::MoveObject { id: _, to: _ } => {
                self.state.handle_message(msg2)?;
            }
            MessageData::UpdateObject { id: _, obj: _ } => {
                self.state.handle_message(msg2)?;
            }
            MessageData::ChangeObjectLayer { id: _, layer: _ } => {
                self.state.handle_message(msg2)?;
            }
            MessageData::Msg { from, contents } => {
                self.messages.push((from.clone(), contents.clone()));
            }
            _ => {}
        }
        self.connection.write_message(msg)?;
        Ok(())
    }

    pub fn new_frame(&mut self) {
        self.notify_new_image = false;
        self.notify_new_message = false;
        self.notify_deleted_image = false;
        self.created_images.clear();
        self.deleted_images.clear();
        self.messages.clear();
    }

    pub fn take_new_messages(&mut self) -> Vec<(String, String)> {
        
        self.messages.clone()
    }

    pub fn take_new_images(&mut self) -> HashMap<String, Arc<[u8]>> {
        let mut out = HashMap::new();
        for i in &self.created_images {
            out.insert(i.clone(), self.images[i].clone());
        }
        out
    }

    pub fn take_deleted_images(&mut self) -> Vec<String> {
        let mut out = Vec::new();
        for i in &self.deleted_images {
            out.push(i.clone());
        }
        out
    }
}

pub struct ServerControl {
    pub messages: BPipe<crate::server::ServerCtl>,
    pub join_handler: std::thread::JoinHandle<()>,
}
