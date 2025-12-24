use std::{collections::{HashMap, VecDeque}, net::{TcpListener, TcpStream}, sync::{Arc, Mutex}, thread::JoinHandle};



use crate::{try_catch, utils::{read_object, try_read_object}};
use crate::communication::*;
pub struct UserConnection{
    pub username:String,
    pub stream:TcpStream,
}
pub struct Server{
    pub clients:HashMap<String, UserConnection>, 
    pub new_connections:Arc<Mutex<VecDeque<TcpStream>>>, 
}
impl State{
    pub fn new()->Self{
        Self { messages: Vec::new() }
    }
}
impl Server{
    pub fn handle_client(name:&String, con:&mut UserConnection)->Vec<Event>{
        let mut events = Vec::new();
        let mut buf = Vec::new();
        try_catch!(
            {
                while let Some(t) = try_read_object::<Event>(&mut con.stream,&mut buf)?{
                    events.push(t);
                }
            }
            catch |_e| {
                return vec![Event{source:name.to_owned(), data:EventData::Disconnection { username: name.to_owned() }}];
            }
        );
        events
    }
    pub fn handle_clients(mut this:Self, handle:JoinHandle<()>){
        let mut state = State{messages:Vec::new()};
        'outer:loop{
            let mut events = Vec::new();
            for (name, con) in &mut this.clients{
                events.append(&mut Self::handle_client(name, con));
            }
            for i in events{
                match i.data{
                    EventData::Message { from, contents, time_stamp:_ } =>{
                        state.messages.push((from, contents));
                    }
                    EventData::Connection { username } => {
                        println!("{:#?} connected", username)
                    }
                    EventData::Disconnection { username } =>{
                        println!("{:#?} disconnected", username);
                        this.clients.remove(&username);
                    }
                    EventData::Kill { password:_ }=>{
                        if i.source == "bridget"{
                            break 'outer;
                        }
                    }
                }
            }
            let mut lck = match this.new_connections.lock(){
                Ok(t) => {t}
                Err(t) => {t.into_inner()}
            };
            let mut read_buf = Vec::new();
            let l = lck.len();
            for mut i in lck.drain(0..l){
                let Ok(message) = read_object::<Event>(&mut i, &mut read_buf) else {
                    continue;
                };
                match message.data{
                    EventData::Message { from:_, contents:_, time_stamp:_ } => {
                        continue;
                    }
                    EventData::Connection { username } => {
                        this.clients.insert(username.clone(), UserConnection { username, stream: i });
                    }
                    EventData::Disconnection { username:_ } => {
                        continue;
                    }
                    EventData::Kill { password:_ } => {
                        continue;
                    }
                }
            }
            lck.clear();
        }
        drop(this);
        let _ = handle.join();
    }
    pub fn accept_clients(list:Arc<Mutex<VecDeque<TcpStream>>>){
        let stream = TcpListener::bind("127.0.0.1:8080").unwrap();
        for i in stream.incoming(){
            if let Ok(i) = i{
               let lsck = list.lock();
               let mut lock = match lsck{
                    Ok(l ) => l,
                    Err(l) => l.into_inner()
                };
                lock.push_back(i);
                drop(lock);
            }
        }
    }
    pub fn serve(){
        let server = Server{clients:HashMap::new(), new_connections:Arc::new(Mutex::new(VecDeque::new()))};
        let connects = server.new_connections.clone();
        let handle = std::thread::spawn(move ||{Self::accept_clients(connects)});
        Self::handle_clients(server, handle);
    }
}