use std::{collections::{HashMap, HashSet}, error::Error, ops::{Add, Sub}, sync::{Arc, atomic::{AtomicBool, AtomicU64}}, time::{self, Duration}};


use serde::{Deserialize, Serialize};
use serde_derive::{Deserialize, Serialize};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, spawn, sync::{Mutex, RwLock}, task::spawn_blocking};

use crate::imaglib::{self, draw::{Color, Image, Vec2}};
#[derive(Serialize, Deserialize,Debug,Clone)]
pub struct Token{
    pub image:String,
    pub pos:Vec2,
    pub time_stamp:u64,
}
#[derive(Serialize, Deserialize,Debug,Clone)]
pub struct BoardState{
    pub tokens:HashMap<String, Token>, 
    pub background_image:String,
}
impl BoardState{
    pub fn new()->Self{
        Self { tokens: HashMap::new(), background_image: String::new()}
    }
}
#[derive(Serialize, Deserialize,Debug)]
pub enum Event{
    UploadImage{name:String, image:Image},
    MoveToken{name:String, location:Vec2}, 
    CreateToken{name:String, image:String,location:Vec2}, 
    DestroyToken{name:String},
    Connect{name:String}, 
    Disconnect{name:String},
    BoardUpdate{state:BoardState},
    Message{username:String, text:String},
    HeartBeat,
}
#[derive(Clone)]
pub struct ImageUpload{
    pub name:String,
    pub image:Image,
}
pub struct ThreadState{
    pub to_write:Mutex<Vec<String>>,
    pub messages:Mutex<Vec<(String,String)>>,
}
impl PartialEq for ThreadState{
    fn eq(&self, other: &Self) -> bool {
        self as *const ThreadState == other as *const ThreadState
    }
}
impl std::hash::Hash for ThreadState{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self as *const ThreadState as u64);
    }
}
impl Eq for ThreadState{

}
pub struct Server{
    pub state:Mutex<BoardState>,
    pub events:Mutex<Vec<Event>>, 
    pub done:AtomicBool,
    pub updated:AtomicBool,
    pub threads:Mutex<HashSet<Arc<ThreadState>>>,
    pub images:Arc<RwLock<HashMap<String, Image>>>,
    pub messages:Mutex<Vec<(String,String)>>,
}
impl Server{
    pub async fn create(){
        let s = Self{state:Mutex::new(BoardState::new()), events:Mutex::new(Vec::new()),done:AtomicBool::new(false), updated:AtomicBool::new(false), images:Arc::new(RwLock::new(HashMap::new())), threads:Mutex::new(HashSet::new()), messages:Mutex::new(Vec::new())};
        let arc = Arc::new(s);
        Server::run(arc).await;
    }
    pub async fn run(this:Arc<Self>){
        let rt = this.clone();
        spawn(
            Server::handle_requests(rt)
        );
        println!("begin loop");
        loop{
            let mut lock = this.events.lock().await;
            let mut state= this.state.lock().await;
            for i in lock.as_slice(){
                match i{
                    Event::UploadImage { name, image } => {
                        let mut l = this.images.write().await;
                        l.insert(name.clone(), image.clone());
                        let locks = this.threads.lock().await;
                        for j in locks.iter(){
                            let mut t = j.to_write.lock().await;
                            t.push(name.clone());
                        }
                        println!("recieved:{name}");
                    }
                    Event::MoveToken { name, location } =>{
                        if !state.tokens.contains_key(name){
                            continue;
                        }
                        state.tokens.get_mut(name).unwrap().pos = *location;
                    }
                    Event::CreateToken { name, image, location } =>{
                        let t = Token { image: image.clone(), pos:*location , time_stamp:now()};
                        state.tokens.insert(name.clone(), t);
                    }
                    Event::DestroyToken { name } => {
                        state.tokens.remove(name);
                    }
                    Event::Connect { name:_ } => {

                    }
                    Event::Disconnect { name :_} => {

                    }
                    Event::BoardUpdate { state:_ } => {

                    }
                    Event::Message { username, text } =>{
                        println!("{username}: {text}");
                        let threads = this.threads.lock().await;
                        for j in threads.iter(){
                            let mut messages =j.messages.lock().await;
                            messages.push((username.clone(), text.clone()));
                        }
                    }
                    Event::HeartBeat => {

                    }
                }
                //println!("{:#?}",i);
            }
            lock.clear();
            this.updated.store(true, std::sync::atomic::Ordering::Release);
            if this.done.load(std::sync::atomic::Ordering::Acquire){
                break;
            }
            drop(lock);
            drop(state);
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
    pub async fn handle_requests(this:Arc<Self>){
        println!("handling requests");
        let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
        println!("listening on address:{:#?}", listener.local_addr().unwrap().ip());
        loop{
            if this.done.load(std::sync::atomic::Ordering::Acquire){
                break;
            }
            let Ok((socket,_)) =listener.accept().await else{
                continue;
            };
            let t = this.clone();
            let state = Arc::new(ThreadState{to_write:Mutex::new(Vec::new()),messages:Mutex::new(Vec::new())});
            let mut states = this.threads.lock().await;
            states.insert(state.clone());
            spawn(
                Self::handle_client(t,socket, state));
        }
    }
    pub async fn handle_client(this:Arc<Self>,mut stream:TcpStream, state:Arc<ThreadState>){
        let mut buffer = Vec::new();
        let images = this.images.read().await;
        for i in images.iter(){
            let e = Event::UploadImage { name: i.0.clone(), image: i.1.clone() };
            let _= write_object(&e, &mut stream).await;
        }
        drop(images);
        loop{
            if this.done.load(std::sync::atomic::Ordering::Acquire){
                break;
            }
            let l = this.state.lock().await;
            let e = Event::BoardUpdate { state: l.clone() };
            let _ = write_object(&e, &mut stream).await;
            drop(l);
            let Ok(message) =read_object(&mut stream, &mut buffer).await else {
                continue;
            };
            match &message{
                Event::Disconnect { name:_ } => {
                    break;
                }
                _=>{

                }
            }
            let mut lock = this.events.lock().await;
            lock.push(message);
            drop(lock);
            let mut lock = state.to_write.lock().await;
            let images = this.images.read().await;
            for i in lock.as_slice(){
                if !images.contains_key(i){
                    continue;
                }
                let e = Event::UploadImage { name: i.clone(), image: images[i].clone() };
                let _ = write_object(&e, &mut stream).await;
            }
            lock.clear();
            drop(images);
            drop(lock);
            let mut lock = state.messages.lock().await;
            for i in lock.as_slice(){
                let e = Event::Message { username: i.0.to_string(), text: i.1.to_string() };
                let _= write_object(&e, &mut stream).await;
            }
            lock.clear();;
            drop(lock);
        }

    }
}
pub async fn read_object<'de,T:Deserialize<'de>>(stream:&mut TcpStream, buf:&'de mut Vec<u8>)->Result<T, Box<dyn Error>>{
    let count = stream.read_u64().await? as usize;
    buf.clear();
    buf.reserve_exact(count);
    buf.resize(count, 0);
    stream.read_exact(buf).await?;
    let out = Ok(serde_json::from_slice(buf)?);
    out
}
pub async fn write_object<T:Serialize>(value:&T,stream:&mut TcpStream)->Result<(),Box<dyn Error>>{
    let s = serde_json::to_string(value)?;
    stream.write_u64(s.len() as u64).await?;
    stream.write_all(s.as_bytes()).await?;
    Ok(())
}
pub async fn try_read_object<'de,T:Deserialize<'de>>(stream:&mut TcpStream, buf:&'de mut Vec<u8>)->Result<Option<T>, Box<dyn Error>>{
    let mut buff = [0u8; 8];
    let in_count = tokio::time::timeout(Duration::from_micros(10),stream.peek(&mut buff)).await??;
    if in_count<8{
        return Ok(None);
    }
    let count = stream.read_u64().await?;
    buf.clear();
    for _ in 0..count{
        buf.push(0)
    }
    stream.read_exact(buf).await?;
    let out = Ok(Some(serde_json::from_slice(buf)?));
    out
}
impl BoardState{
    pub fn update_state(&mut self,other:BoardState){
        let mut to_remove_list:Vec<String> = Vec::new();
        let time = now();
        for i in &mut self.tokens{
            if i.1.time_stamp+1>time{
                continue;
            }
            if let Some(t) = other.tokens.get(i.0){
                *i.1 = t.clone();
            }else{
                to_remove_list.push(i.0.to_string());
            }
        }
        for i in other.tokens{
            if i.1.time_stamp+1>time{
                continue;
            }
            if !self.tokens.contains_key(&i.0){
                self.tokens.insert(i.0, i.1);
            }
        }
        for i in to_remove_list{
            self.tokens.remove(&i);
        }
    }
}
pub fn now()->u64{
   std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
}