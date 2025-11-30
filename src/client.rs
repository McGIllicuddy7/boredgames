use minifb::{Window, WindowOptions};
use tokio::{net::TcpStream, spawn, sync::{Mutex, RwLock}, task::spawn_blocking};

use crate::{client, imaglib::{self, draw::{Image, Vec2, colors::{BLACK, GREEN, WHITE}}}, state::{BoardState, Event, Server, ThreadState, read_object, try_read_object, write_object}};
use std::{collections::{HashMap, HashSet}, error::Error, io::{Read, Write, stdin, stdout}, os::unix::fs::PermissionsExt, sync::{Arc, atomic::AtomicBool}};
pub struct Client{
    pub server:Option<Arc<Server>>,
    pub state:BoardState,
    pub images:Arc<RwLock<HashMap<String, Image>>>, 
    pub name:String,
}
pub enum ClientServerInterface{
    Server{s:Arc<Server>}, Stream{s:TcpStream},
}
impl ClientServerInterface{
    pub async fn write_event(&mut self, e:Event){
        match self{
            ClientServerInterface::Server { s } => {
                let mut lock = s.events.lock().await;
                lock.push(e);
                drop(lock);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            },
            ClientServerInterface::Stream { s } => {
                write_object(&e, s).await.unwrap();
            },
        }
    }
}
impl Client{
    pub async fn update(&mut self, stream:&mut ClientServerInterface)->Result<(), Box<dyn Error>>{
        match stream{
            ClientServerInterface::Server { s } => {
                if s.updated.load(std::sync::atomic::Ordering::Acquire){
                    s.updated.store(false, std::sync::atomic::Ordering::Relaxed);
                    self.state = s.state.lock().await.clone();
                }
            },
            ClientServerInterface::Stream { s } => {
                let mut buf = Vec::new();
                loop{
                    buf.clear();
                    let ev = try_read_object::<Event>(s, &mut buf).await;
                    if ev.is_err(){
                        return  Ok(());
                    }
                    let Ok(ev1) = ev else {
                        return Ok(());
                    };
                    if let Some(e) =ev1{
                        match e{
                            Event::BoardUpdate { state } => {
                                self.state.update_state(state);
                            },
                            Event::UploadImage { name, image }=>{
                                let mut l = self.images.write().await;
                                l.insert(name, image);
                            }
                            Event::Message { username, text }=>{
                                if username !=self.name{
                                    println!("{username}:{text}");
                                }
    
                            }
                            _=>{

                            }
                        }
                    }
                }

            },
        }
        Ok(())
    }
    pub async fn run_client_terminal(&mut self, mut stream:ClientServerInterface){
        let mut input = String::new();
        let mut username = String::new();
        print!("please enter your username:");
        stdout().flush().unwrap();
        stdin().read_line(&mut username).unwrap();
        username = username.trim().to_string();
        self.name = username.clone();
        stream.write_event(Event::Connect { name: username.clone() }).await;
        loop{
            input.clear();
             let _ = self.update(&mut stream).await;
            print!("please enter a command: ");
            stdout().flush().unwrap();
            let e = stdin().read_line(&mut input);
            if e.is_err(){
                continue;
            }
            let inp:Vec<&str> = input.split_whitespace().collect();
            if inp.is_empty(){
                continue;
            }
            if inp[0] == "exit"{
                let e = Event::Disconnect { name: username.clone() };
                stream.write_event(e).await;
                break;
            }else if inp[0] == "move"{
                if inp.len()<4{
                    continue;
                }
                let name = inp[1];
                let Ok(pos_x) = inp[2].parse::<i32>()else {
                    continue;
                };
                let Ok(pos_y) = inp[3].parse::<i32>()else {
                    continue;
                };
                let e = Event::MoveToken { name:name.to_string(), location: Vec2::new(pos_x, pos_y) };
                stream.write_event(e).await;
            } else if inp[0] == "message"{
                let msg = input.strip_prefix("message ").unwrap().to_string();
                let event = Event::Message { username: username.clone(), text:msg };
                stream.write_event(event).await;
            }else if inp[0] == "create"{
                if inp.len()<5{
                    continue;
                }
                let name = inp[1];
                let Ok(pos_x) = inp[2].parse::<i32>()else {
                    continue;
                };
                let Ok(pos_y) = inp[3].parse::<i32>()else {
                    continue;
                };
                let token_name = inp[4];
                let e = Event::CreateToken { name: name.to_string(), image: token_name.to_string(), location:Vec2::new(pos_x, pos_y) }; 
                stream.write_event(e).await;
                println!("created token\n");
            }else if inp[0] == "destroy"{
                if inp.len()<2{
                    continue;
                }
                let e = Event::DestroyToken { name: inp[1].to_string() };
                stream.write_event(e).await;

            }else if inp[0] == "upload"{
                if inp.len()<2{
                    continue;
                }
                let name = inp[1];
                let Ok(image )= imaglib::draw::Image::load(name) else{
                    continue;
                };
                let mut l = self.images.write().await;
                l.insert(name.to_string(), image.clone());
                let e = Event::UploadImage { name: name.to_string(), image };
                drop(l);
                stream.write_event(e).await;
            }else if inp[0] == "print"{
                let e = Event::HeartBeat;
                stream.write_event(e).await;
                println!("{:#?}",self.state);
            } else if inp[0] == "update"{
                let e = Event::HeartBeat;
                stream.write_event(e).await;
            }else if inp[0] == "images"{
                let images = self.images.read().await;
                println!("list of images:");
                for i in images.iter(){
                    println!("{}", i.0);
                }
                let e = Event::HeartBeat;
                drop(images);
                stream.write_event(e).await; 
            }else{
                println!("error: unknown command {:#?}", inp[0]);
            }
        }
    }
    pub async fn run(){
        let mut response = String::new();
        let mut client = Client{server:None, state:BoardState::new(), images:Arc::new(RwLock::new(HashMap::new())), name:String::new()};
        loop{
                print!("do you wish to host?(y,n):");
                stdout().flush().unwrap();
                stdin().read_line(&mut response).unwrap();
                let r = response.trim();
                if r == "y"{
                    let s = Server{state:Mutex::new(BoardState::new()), events:Mutex::new(Vec::new()),done:AtomicBool::new(false), threads:Mutex::new(HashSet::new()), updated:AtomicBool::new(false),images:client.images.clone(), messages:Mutex::new(Vec::new())};
                    let arc = Arc::new(s);
                    let sr = arc.clone();
                    client.server = Some(arc.clone());
                    spawn(Server::run(arc));
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    println!("use gui?(y,n)");
                    response.clear();
                    stdin().read_line(&mut response).unwrap();
                    if response.trim() == "y"{
                        GuiClient::run(client, ClientServerInterface::Server { s: sr }).await;
                    }else{
                        client.run_client_terminal(ClientServerInterface::Server { s: sr}).await;
                    }
                    break;
                    
                }else if r == "n"{
                    response.clear();
                    println!("please enter ip address to connect to:");
                    stdin().read_line(&mut response).unwrap(); 
                    let con =  TcpStream::connect(response.trim()).await;
                    if let Ok(stream) =con{
                        println!("connected");
                        println!("use gui?(y,n)");
                        response.clear();
                        stdin().read_line(&mut response).unwrap();
                        if response.trim() == "y"{
                            GuiClient::run(client, ClientServerInterface::Stream { s: stream }).await;
                        } else{
                            client.run_client_terminal(ClientServerInterface::Stream { s: stream }).await;
                        }
                        break;
                    }else if let Err(e) = con{
                        println!("failed to connect to:{}", e);
                    }
                }else{
                    response.clear();
                    println!("please enter one of y(for yes) or n(for no)");
                }
        }

    }
}
pub struct GuiState{
    window:Window,
}
#[derive(Clone)]
pub struct ObjectRef{
    pub is_image:bool, 
    pub name:String,
}
pub struct GuiClient{
    pub client:Client, 
    pub messages:Vec<(String, String)>,
    pub image:Image,
    pub selected_item:Option<ObjectRef>,
    pub gui_state:GuiState,
    pub stream:ClientServerInterface,
    pub name:String,
    pub obj_count:u64,
}
impl GuiClient{
    pub fn init(cl:Client, stream:ClientServerInterface)->Self{
        let mut username =String::new();
        println!("please enter your username");
        stdin().read_line(&mut username).unwrap();
        let window = minifb::Window::new("bored-games",1200, 480*2, WindowOptions{borderless:false, title:true, resize:false, scale:minifb::Scale::X1, scale_mode:minifb::ScaleMode::Center,topmost:true, transparency:false, none:false}).unwrap();
        let img = Image::new(1200,480*2);
        Self { client:cl, messages: Vec::new(), image: img, selected_item: None, gui_state: GuiState { window },  stream, name:username.trim().to_string(), obj_count:0}
    }
    pub async fn handle_events(&mut self){
        let win = &self.gui_state.window;
        let mouse_down = win.get_mouse_down(minifb::MouseButton::Left);
        let mouse_pos = win.get_mouse_pos(minifb::MouseMode::Clamp).unwrap();
        let mouse_x = mouse_pos.0 as i32;
        let mouse_y = mouse_pos.1 as i32;
        let mut event = Event::HeartBeat;
        self.client.update(&mut self.stream).await.unwrap();
        if let Some(p) = self.selected_item.clone(){ 
            if !mouse_down{
                self.selected_item = None;
                if p.is_image{
                    
                }else{
                    let px = (mouse_x-100)/50;
                    let py = (mouse_y -100)/50;
                    if let Some(tk) = self.client.state.tokens.get_mut(&p.name){
                        if px >= 0 && py>= 0 && px<17 && py<17{
                            event= Event::MoveToken { name: p.name, location: Vec2::new(px, py) };
                            tk.pos.x = px;
                            tk.pos.y = py;
                        }
                    }

                }
            }
        }else{
            self.selected_item = None;
            if mouse_down{
                for i in &self.client.state.tokens{
                    let p = i.1.pos*50+ Vec2::new(100, 100);
                    let min_x  = p.x-25;
                    let min_y = p.y-25;
                    let max_x = p.x+25;
                    let max_y = p.y+25;
                    if mouse_x>min_x && mouse_x<max_x && mouse_y>min_y && mouse_y <max_y{
                        self.selected_item = Some(ObjectRef { is_image: false, name: i.0.clone() })
                    }
            }

            }
        }
        self.stream.write_event(event).await;
    }
    pub async fn draw_gui(&mut self){
        self.image.clear(BLACK);
        self.image.draw_rect(50, 50, 900, 900, WHITE);
        let client = &self.client;
        let images =  self.client.images.read().await;
        if self.client.state.background_image != "
        "{
            if let Some(i) = images.get(&client.state.background_image){
                self.image.draw_rect_image(0,0,1200, 480*2, i);
            }
        }
        for i in &self.client.state.tokens{
            if let Some(j) = images.get(&i.1.image){
                self.image.draw_rect_image(i.1.pos.x*50-25+100, i.1.pos.y*50-25+100,50, 50,j);
            }else{
                self.image.draw_circ(i.1.pos*10, 25, GREEN);
            }
        }
    }
    pub async fn update(&mut self){
        self.handle_events().await;
        self.draw_gui().await;
        self.image.draw(&mut self.gui_state.window);

    }
    pub async fn run(cl:Client, stream:ClientServerInterface){
        let mut state = Self::init(cl, stream);
        while state.gui_state.window.is_open() {
            state.update().await;
        }
    }
}