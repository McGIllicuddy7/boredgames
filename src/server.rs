use std::{
    collections::HashMap,
    error::Error,
    io::ErrorKind,
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex, atomic::AtomicBool},
    thread::JoinHandle,
};

use crate::communication::*;
use crate::utils::{read_object, try_read_object, write_object};
pub struct UserConnection {
    pub username: String,
    pub stream: TcpStream,
}
pub struct Server {
    pub clients: HashMap<String, UserConnection>,
    pub new_connections: Arc<Mutex<Vec<TcpStream>>>,
    pub owner: String,
}
impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            messages: Vec::new(),
            tokens: HashMap::new(),
            map: HashMap::new(),
            gm: HashMap::new(),
        }
    }
}
pub static SHOULD_DIE: AtomicBool = AtomicBool::new(false);
pub static EXISTS: AtomicBool = AtomicBool::new(false);
impl Server {
    pub fn handle_client(should_log: bool, _name: &String, con: &mut UserConnection) -> Vec<Event> {
        let mut events = Vec::new();
        let mut buf = Vec::new();
        while let Some(t) = try_read_object::<Event>(&mut con.stream, &mut buf)
            .or_else(|_| Ok::<Option<Event>, Box<dyn Error>>(None))
            .unwrap()
        {
            if should_log {
                println!("log:{:#?}", t.source);
            }
            events.push(t);
        }
        events
    }

    pub fn handle_clients(should_log: bool, mut this: Self, handle: JoinHandle<()>) {
        let mut app_state = State {
            name: String::new(),
            messages: Vec::new(),
            tokens: HashMap::new(),
            map: HashMap::new(),
            gm: HashMap::new(),
        };
        let mut state_changed;
        let mut loaded_images: HashMap<String, Vec<u8>> = HashMap::new();
        let mut uploads = Vec::new();
        'outer: loop {
            if SHOULD_DIE.load(std::sync::atomic::Ordering::Acquire) {
                break;
            }
            uploads.clear();
            let mut events = Vec::new();
            for (name, con) in &mut this.clients {
                let ev = Self::handle_client(should_log, name, con);
                for i in ev {
                    if should_log {
                        println!("{:#?}", name);
                    }
                    events.push(i);
                }
            }
            state_changed = false;
            for i in events {
                match i.data {
                    EventData::Message {
                        from,
                        contents,
                        time_stamp: _,
                    } => {
                        state_changed = true;
                        app_state.messages.push((from, contents));
                    }
                    EventData::Connection { username } => {
                        state_changed = true;
                        if should_log {
                            println!("{:#?} connected", username);
                        }
                    }
                    EventData::Disconnection { username } => {
                        state_changed = true;
                        if should_log {
                            println!("{:#?} disconnected", username);
                        }
                        this.clients.remove(&username);
                    }
                    EventData::Kill { password: _ } => {
                        state_changed = true;
                        if should_log {
                            println!("killed");
                        }
                        if i.source == this.owner {
                            break 'outer;
                        }
                    }
                    EventData::HeartBeat => {
                        continue;
                    }
                    EventData::ImageUpload { name, image } => {
                        state_changed = true;
                        uploads.push(name.clone());
                        let _ = std::fs::write(path().to_string() + &name, &image);
                        loaded_images.insert(name, image);
                    }
                    EventData::TokenMoved {
                        name,
                        to,
                        time_stamp: _,
                        layer,
                    } => {
                        state_changed = true;
                        if let Some(t) = app_state.tokens.get_mut(&name) {
                            *t = to.clone();
                        }
                        if let Some(t) = app_state.map.get_mut(&name) {
                            *t = to.clone();
                        }
                        if let Some(t) = app_state.gm.get_mut(&name) {
                            *t = to.clone();
                        }
                    }
                    EventData::SendState { state } => {
                        state_changed = true;
                        app_state = state;
                    }
                    EventData::TokenDestroyed { name, layer } => {
                        state_changed = true;
                        app_state.map.remove(&name);
                        app_state.tokens.remove(&name);
                        app_state.gm.remove(&name);
                    }
                    EventData::TokenCreated { name, token, layer } => {
                        state_changed = true;
                        if !app_state.map.contains_key(&name)
                            && !app_state.gm.contains_key(&name)
                            && !app_state.tokens.contains_key(&name)
                        {
                            match layer {
                                Layer::Base => {
                                    app_state.tokens.insert(name, token);
                                }
                                Layer::Map => {
                                    app_state.map.insert(name, token);
                                }
                                Layer::Gm => {
                                    app_state.gm.insert(name, token);
                                }
                            }
                        }
                    }
                    EventData::PersonalUpdate { people: _ } => {
                        continue;
                    }
                }
            }
            let mut lck = match this.new_connections.lock() {
                Ok(t) => t,
                Err(t) => t.into_inner(),
            };
            let mut read_buf = Vec::new();
            let l = lck.len();
            let mut to_recheck = Vec::new();
            for mut i in lck.drain(0..l) {
                let message;
                let e = read_object::<Event>(&mut i, &mut read_buf);
                match e {
                    Ok(ev) => {
                        state_changed = true;
                        message = ev;
                    }
                    Err(e) => {
                        if let Ok(e) = e.downcast::<std::io::Error>() {
                            match e.kind() {
                                ErrorKind::WouldBlock => {
                                    to_recheck.push(i);
                                    continue;
                                }
                                _ => {
                                    continue;
                                }
                            }
                        } else {
                            continue;
                        }
                    }
                }
                match message.data {
                    EventData::Message {
                        from: _,
                        contents: _,
                        time_stamp: _,
                    } => {
                        continue;
                    }
                    EventData::Connection { username } => {
                        state_changed = true;
                        if !this.clients.contains_key(&username) {
                            for j in &loaded_images {
                                let e = Event {
                                    source: "_server".into(),
                                    data: EventData::ImageUpload {
                                        name: j.0.clone(),
                                        image: j.1.clone(),
                                    },
                                };
                                let _ = write_object(&mut i, &e);
                            }
                            if this.owner == "" {
                                this.owner = username.clone()
                            }
                            this.clients.insert(
                                username.clone(),
                                UserConnection {
                                    username,
                                    stream: i,
                                },
                            );
                        }
                    }
                    EventData::Disconnection { username: _ } => {
                        continue;
                    }
                    EventData::Kill { password: _ } => {
                        continue;
                    }
                    EventData::HeartBeat => {
                        continue;
                    }
                    EventData::ImageUpload { name: _, image: _ } => {
                        continue;
                    }
                    EventData::TokenMoved {
                        name: _,
                        to: _,
                        time_stamp: _,
                        layer: _,
                    } => {
                        continue;
                    }
                    EventData::SendState { state: _ } => {
                        continue;
                    }
                    EventData::TokenDestroyed { name: _, layer: _ } => {
                        continue;
                    }
                    EventData::TokenCreated {
                        name: _,
                        token: _,
                        layer: _,
                    } => {
                        continue;
                    }
                    EventData::PersonalUpdate { people: _ } => {
                        continue;
                    }
                }
            }
            *lck = to_recheck;
            drop(lck);
            if state_changed {
                let mut people: Vec<String> =
                    this.clients.iter().map(|(i, _)| i.to_owned()).collect();
                people.sort_unstable();
                for i in &mut this.clients {
                    let _ = write_object(
                        &mut i.1.stream,
                        &Event {
                            source: "_server".into(),
                            data: EventData::SendState {
                                state: app_state.clone(),
                            },
                        },
                    );

                    let _ = write_object(
                        &mut i.1.stream,
                        &Event {
                            source: "_server".into(),
                            data: EventData::PersonalUpdate {
                                people: people.clone(),
                            },
                        },
                    );
                }
                for j in &uploads {
                    let e = Event {
                        source: "_server".into(),
                        data: EventData::ImageUpload {
                            name: j.clone(),
                            image: loaded_images[j].clone(),
                        },
                    };

                    for i in &mut this.clients {
                        let _ = write_object(&mut i.1.stream, &e);
                    }
                }
            }
        }
        println!("died");
        SHOULD_DIE.store(true, std::sync::atomic::Ordering::Release);
        drop(this);
        let _ = handle.join();
    }
    pub fn accept_clients(should_log: bool, list: Arc<Mutex<Vec<TcpStream>>>) {
        let ip = get_ip();
        let Ok(stream) = TcpListener::bind(ip + ":8080") else {
            EXISTS.store(true, std::sync::atomic::Ordering::Release);
            SHOULD_DIE.store(true, std::sync::atomic::Ordering::Release);
            println!("failed to create");
            return;
        };
        stream.set_nonblocking(true).unwrap();
        EXISTS.store(true, std::sync::atomic::Ordering::Release);
        loop {
            if SHOULD_DIE.load(std::sync::atomic::Ordering::Acquire) {
                println!("should die");
                break;
            }
            if let Ok((i, _)) = stream.accept() {
                if should_log {
                    println!("accepted");
                }
                let lsck = list.lock();
                let mut lock = match lsck {
                    Ok(l) => l,
                    Err(l) => l.into_inner(),
                };
                lock.push(i);
                drop(lock);
            }
        }
        println!("died");
        EXISTS.store(false, std::sync::atomic::Ordering::Release);
    }
    pub fn serve(should_log: bool) {
        SHOULD_DIE.store(false, std::sync::atomic::Ordering::Release);
        let server = Server {
            clients: HashMap::new(),
            owner: String::new(),
            new_connections: Arc::new(Mutex::new(Vec::new())),
        };
        let connects = server.new_connections.clone();
        let handle = std::thread::spawn(move || Self::accept_clients(should_log, connects));
        Self::handle_clients(should_log, server, handle);
    }
}
