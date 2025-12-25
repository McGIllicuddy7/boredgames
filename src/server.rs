use std::{
    collections::HashMap,
    error::Error,
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use eframe::egui::{ImageData, Pos2};
use local_ip_address::local_ip;

use crate::communication::*;
use crate::utils::{read_object, try_read_object, write_object};
pub struct UserConnection {
    pub username: String,
    pub stream: TcpStream,
}
pub struct Server {
    pub clients: HashMap<String, UserConnection>,
    pub new_connections: Arc<Mutex<Vec<TcpStream>>>,
}
impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            tokens: HashMap::new(),
        }
    }
}
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
            messages: Vec::new(),
            tokens: HashMap::new(),
        };
        app_state.tokens.insert(
            "test token".into(),
            Token {
                location: Pos2 { x: 80., y: 80.0 },
                image: "file://./orc.png".into(),
            },
        );
        let mut state_changed;
        let mut loaded_images:HashMap<String, ImageData> = HashMap::new();
        let mut uploads = Vec::new();
        'outer: loop {
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
                        if i.source == "root" {
                            break 'outer;
                        }
                    }
                    EventData::HeartBeat => {
                        continue;
                    }
                    EventData::ImageUpload {name, image }=>{
                        state_changed = true;
                        uploads.push(name.clone());
                        loaded_images.insert(name, image);
                    }
                    EventData::TokenMoved { name, to, time_stamp:_ }=>{
                        state_changed = true;
                        if let Some(t) = app_state.tokens.get_mut(&name){
                            t.location = to;
                        }
                    }
                    EventData::SendState { state }=>{
                        state_changed = true;
                        app_state = state;
                    }
                }
            }
            let mut lck = match this.new_connections.lock() {
                Ok(t) => t,
                Err(t) => t.into_inner(),
            };
            let mut read_buf = Vec::new();
            let l = lck.len();
            for mut i in lck.drain(0..l) {
                state_changed = true;
                let Ok(message) = read_object::<Event>(&mut i, &mut read_buf) else {
                    if should_log {
                        println!("failed to read");
                    }
                    continue;
                };
                match message.data {
                    EventData::Message {
                        from: _,
                        contents: _,
                        time_stamp: _,
                    } => {
                        continue;
                    }
                    EventData::Connection { username } => {
                        if !this.clients.contains_key(&username) {
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
                    EventData::ImageUpload { name:_, image:_ }=>{
                        continue;
                    }
                    EventData::TokenMoved { name:_, to:_, time_stamp:_ }=>{
                        continue;
                    }
                    EventData::SendState { state }=>{
                        continue;
                    }
                }
            }
            lck.clear();
            drop(lck);
            if state_changed {
                for i in &mut this.clients {
                    let _ = write_object(&mut i.1.stream, &Event{source:"_server".into(), data:EventData::SendState { state: app_state.clone() }});
                }
                for j in &uploads{
                    for i in &mut this.clients{
                        let _ = write_object(&mut i.1.stream, &Event{
                            source:"_server".into(), data:EventData::ImageUpload { name:j.clone() , image: loaded_images[j].clone() }
                        });
                    }
                }
            }
        }
        drop(this);
        let _ = handle.join();
    }
    pub fn accept_clients(should_log: bool, list: Arc<Mutex<Vec<TcpStream>>>) {
        let Ok(stream) = TcpListener::bind(local_ip().unwrap().to_string() + ":8080") else {
            println!("failed to create");
            return;
        };
        for i in stream.incoming().flatten() {
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
    pub fn serve(should_log: bool) {
        let server = Server {
            clients: HashMap::new(),
            new_connections: Arc::new(Mutex::new(Vec::new())),
        };
        let connects = server.new_connections.clone();
        let handle = std::thread::spawn(move || Self::accept_clients(should_log, connects));
        Self::handle_clients(should_log, server, handle);
    }
}
