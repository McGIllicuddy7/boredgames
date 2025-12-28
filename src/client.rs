use std::{collections::HashSet, net::TcpStream, net::SocketAddr, process::exit, thread::sleep};

use eframe::egui::{self, Color32, Image, ImageSource, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use local_ip_address::local_ip;

use crate::{
    communication::*, server::{EXISTS, SHOULD_DIE}, utils::{self, try_read_object, write_object}
};
pub enum Layer{
    Base, Map, Gm,
}
pub struct Client {
    pub state: State,
    pub typed_message: String,
    pub ip_address: String,
    pub username: String,
    pub connection: Option<TcpStream>,
    pub loaded_images: HashSet<String>,
    pub owns_server:bool,
    pub working_layer:Layer
}
impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Self {
        let addr = local_ip().unwrap();
        Self {
            state: State::new(),
            typed_message: String::new(),
            ip_address: addr.to_string() + ":8080",
            connection: None,
            username: "root".into(),
            loaded_images: HashSet::new(),
            owns_server:false,
            working_layer:Layer::Base
        }
    }
    pub fn update(&mut self, ui: &mut Ui) {
        ui.with_layout(
            egui::Layout::top_down_justified(egui::Align::Center),
            |ui| {
                self.update_actual(ui);
            },
        );
    }
    pub fn draw_images(&mut self,should_log: bool, ui: &mut Ui) {
        let path = path();
        let Ok(f) = std::fs::read_dir(path)else{
            println!("failed to read: {}", path);
            return;
        };
        ui.vertical(move |ui| {
            for i in f {
                if let Ok(e) = i
                    && e.file_type().unwrap().is_file() {
                        let name = e.file_name().into_string().unwrap();
                        if name.ends_with(".png")
                            || name.ends_with(".jpg")
                            || name.ends_with(".jpeg")
                        {
                            let img = Image::new(ImageSource::Uri(
                                ("file://".to_string() + path+ &name).into(),
                            ));
                            let r = ui.add(
                                egui::Button::image_and_text(img, name.clone()).sense(Sense::all()),
                            );
                            if r.drag_stopped() {
                                let p = ui.input(|i| i.pointer.latest_pos().unwrap());
                                if p.x < 50.0 || p.y < 50.0 || p.x > 800. || p.y > 800.0 {
                                    continue;
                                }
                                let p2 = Pos2 {
                                    x: (p.x as i32 / 20 * 20+10) as f32,
                                    y: (p.y as i32 / 20 * 20+10) as f32,
                                };
                                let count = self.state.tokens.len();
                                let tname = format!("{:#?}_{:#?}", self.username, count);
                                let fname = format!("file://{}{}", path,name);
                                if should_log{
                                    println!("{:#?}, {:#?}", p2, fname);
                                }                     
                                let ev = Event {
                                    source: self.username.clone(),
                                    data: EventData::TokenCreated {
                                        name: tname.clone(),
                                        token: Token {
                                            location: p2,
                                            image: name.clone(),
                                        },
                                    },
                                };
                                let n = path.to_string()+&name;
                                let ev0 = Event {
                                    source: self.username.clone(),
                                    data: EventData::ImageUpload { name:name.clone(), image:std::fs::read(n).unwrap()} 
                                };
                                if let Some(obj) = self.connection.as_mut() {
                                    if !self.loaded_images.contains(&name){
                                        self.loaded_images.insert(name);
                                        let _ = write_object(obj, &ev0);
                                    }
                                    let _ = write_object(obj, &ev);
                                }

                            }
                        }
                    }
            }
        });
    }
    pub fn draw_map(&mut self, ui: &mut Ui) {
        let name = path().to_string()+"board.png";
        if std::fs::File::open(&name).is_err(){
                return;
        } 
        let img0 = Image::new(ImageSource::Uri(("file://".to_string()+&name).into()));
        let maxd = 810.0;
        ui.place(
            Rect {
                min: Pos2::new(50.0, 50.0),
                max: Pos2::new(maxd, maxd),
            },
            img0,
        );
        let p = ui.painter();
        for i in 1..750/20+1{
            p.line(vec![Pos2::new((i*20+50) as f32, 50.),Pos2::new((i*20+50) as f32,maxd)], Stroke::new(1.0, Color32::BLACK));
        }
        for i in 1..750/20+1{
            p.line(vec![Pos2::new(50., (i*20+50) as f32),Pos2::new(maxd,(i*20+50) as f32)], Stroke::new(1.0, Color32::BLACK));
        }
        ui.allocate_rect(
            Rect {
                min: Pos2::new(50.0, 50.0),
                max: Pos2::new(maxd, maxd),
            },
            Sense::empty(),
        );
        for (_name, token) in &mut self.state.tokens {
            //  println!("drew:{_name}");
            if let Err(e) = std::fs::File::open(path().to_string()+&token.image){
                println!("{:#?}:{:#?}", token.image, e);
                continue;
            }
            let img = Image::new(ImageSource::Uri(("file://".to_string()+ path()+&token.image).into()));
            let ar = egui::Area::new(_name.clone().into())
                .current_pos(Pos2::new(token.location.x, token.location.y))
                .show(ui.ctx(), move |ui| {
                    let img2 = img.fit_to_exact_size(Vec2::new(20.0, 20.0));
                    ui.add(img2);
                });
            if ar.response.dragged() {
                let mut r =
                    Pos2::new(token.location.x, token.location.y) + ar.response.drag_delta();
                if r.x < 50.0 {
                    r.x = 50.0;
                }
                if r.x > maxd {
                    r.x = maxd;
                }
                if r.y < 50.0 {
                    r.y = 50.0;
                }
                if r.y > maxd {
                    r.y = maxd;
                }
                token.location = Pos2 {
                    x: r.x,
                    y: r.y,
                };
            }
            if ar.response.drag_stopped() {
                token.location = Pos2 {
                    x: ((token.location.x as i32) / 20 * 20+10) as f32,
                    y: ((token.location.y as i32) / 20 * 20+10) as f32,
                };
                if let Some(c) = self.connection.as_mut() {
                    write_object(
                        c,
                        &Event {
                            source: self.username.clone(),
                            data: EventData::TokenMoved {
                                name: _name.clone(),
                                to: token.location,
                                time_stamp: 0,
                            },
                        },
                    )
                    .unwrap();
                }
            }
        }
        self.map_controls(false, ui);
    }
    pub fn map_controls(&mut self, should_log: bool,ui:&mut Ui){
        _ = should_log;
        ui.collapsing("map settings", |ui|{
            ui.vertical(|ui|{
                ui.horizontal(|ui|{
                        ui.label("map name:");
                        ui.text_edit_singleline(&mut self.state.name)
                });
                if ui.button("save").clicked(){
                    let s = serde_json::to_string_pretty(&self.state).unwrap();
                    let pth = path().to_string()+&self.state.name;
                    if !std::fs::exists(&pth).unwrap(){
                        let _ = std::fs::write(pth, s);
                    }
                }
            });

   
        });
    }
    pub fn update_actual(&mut self, ui: &mut Ui) {
        let should_log =false;
        if let Some(t) = self.connection.as_mut() {
            loop{
                let tr = try_read_object::<Event>(t, &mut Vec::new());
                if tr.is_err(){
                    if let Err(e) = tr{
                        if let Ok(t) = e.downcast::<std::io::Error>(){
                            match t.kind(){
                                std::io::ErrorKind::WouldBlock=>{
                                    break;
                                }
                                std::io::ErrorKind::ConnectionReset=>{
                                    break;
                                }
                                std::io::ErrorKind::UnexpectedEof=>{
                                    break;
                                }
                                _=>{
                                    println!("disconnected {:#?}", t);
                                    self.connection = None;
                                    break;
                                }
                            }
                        }
                    }
                    break;
                }
                let Ok(ev) = tr else{
                    break;
                };
                if let Some(ev) = ev {
                    match ev.data {
                        EventData::SendState { state } => {
                            self.state = state;
                        }
                        EventData::ImageUpload { name, image } => {
                            if should_log{
                                println!("uploaded:{:#?}", name);
                            }                    
                            let _ = std::fs::create_dir("assets");
                            let e = std::fs::write(path().to_string()+&name, &image);
                            if let Err(e) = e{
                                println!("{:#?}",e);
                            }
                            //let img = Image::new(ImageSource::Bytes { uri: name.clone().into(), bytes: image.into()});
                            self.loaded_images.insert(name);
                        }
                        _ => {
                            todo!()
                        }
                    }
                } else {
                    break;
                }
            }
        }else{
            self.connection = None;
        }
        let mut should_send = false;
        let mut should_connect = false;
        let mut should_host = false;
        let mut username_set = false;
        ui.vertical_centered(|ui| {
            ui.horizontal(|ui| {
                ui.label("ip address:");
                ui.text_edit_singleline(&mut self.ip_address);
                if self.connection.is_none(){
                    if ui.button("connect").clicked() {
                        should_connect = true;
                    }
                    if ui.button("host own server").clicked() {
                        should_host = true;
                    }
                }else{
                    if ui.button("disconnect").clicked(){
                        if self.owns_server{
                            write_object(&mut self.connection.as_mut().unwrap(), &Event{source:self.username.clone(), data:EventData::Kill{
                                password:"bridget".into()
                            }}).unwrap();
                        }
                        self.owns_server = false;
                        self.connection = None;
                    }
                }
                if let Some(s) = self.connection.as_ref() {
                    let e = s.take_error();
                    if e.is_ok() {
                        ui.label("connected");
                    } else {
                        if should_log{
                            println!("{:#?}",e);
                        }               
                        self.connection = None;
                        ui.label("not connected");
                    }
                } else {
                    ui.label("not connected");
                }
            });
            ui.horizontal(|ui| {
                ui.label("username:");
                let old = self.username.clone();
                ui.text_edit_singleline(&mut self.username);
                if ui.button("enter").clicked() {
                    username_set = true;
                }
                if let Some(_) = self.connection.as_ref(){
                    self.username = old;
                }
                //ui.label(std::fs::canonicalize(".").unwrap().to_str().unwrap().to_string());
                //let args:Vec<String> = std::env::args().collect();
                //ui.label(args[0].clone());
            });
            ui.horizontal(|ui| {
                self.draw_map(ui);
                ui.allocate_ui(Vec2::new(200.0, 500.0), |ui| {
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                        ui.group(|ui| {
                            ui.set_min_height(400.0);
                            ui.set_min_width(200.0);
                            ui.set_max_height(430.0);
                            ui.set_clip_rect(ui.min_rect());
                            ui.set_min_height(430.0);
                            for i in self.state.messages.iter().rev() {
                                ui.code(format!("{}:{}", i.0, i.1));
                            }
                        });
                    });
                });
                self.draw_images(should_log,ui);
            });
            ui.horizontal(|ui| {
                ui.label("enter message:");
                let foc = ui.text_edit_singleline(&mut self.typed_message);
                if ui.button("send").clicked()
                    || (ui.input(|i| i.key_pressed(egui::Key::Enter)) && foc.lost_focus())
                {
                    should_send = true;
                    foc.request_focus();
                }
            });
        });
        if should_connect && self.connection.is_none(){
            if should_log {
                println!("should connect to:{:#?}", self.ip_address);
            }
            let addr =SocketAddr::new(self.ip_address.strip_suffix(":8080").unwrap().parse().unwrap(), 8080);
            if let Ok(mut con) = TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(3)) {
                let _ = write_object(
                    &mut con,
                    &Event {
                        source: self.username.clone(),
                        data: EventData::Connection {
                            username: self.username.clone(),
                        },
                    },
                );
                if let Err(_) = write_object(
                    &mut con,
                    &Event {
                        source: self.username.clone(),
                        data: EventData::HeartBeat,
                    },
                ) {
                    self.ip_address = local_ip().unwrap().to_string() + ":8080";
                    if should_log{
                        println!("diconnected");
                    }
                    self.connection = None;
                } else {
                    self.connection = Some(con);
                }
            }
        }
        if should_send {
            //self.state.messages.push((self.username.clone(),self.typed_message.clone()));
            if self.typed_message.starts_with("\\") {
                let msg = self.typed_message.strip_prefix("\\").unwrap();
                match msg {
                    "exit" => {
                        exit(0);
                    }
                    _ => {
                        self.typed_message = "\\invalid command".into();
                        should_send = false;
                    }
                }
            }
            if should_send {
                if should_log {
                    println!("should send:{:#}", self.typed_message);
                }
                if let Some(con) = self.connection.as_mut() {
                    if self.typed_message == "\\kill"{

                    }
                    if let Err(a) = utils::write_object(
                        con,
                        &Event {
                            source: self.username.clone(),
                            data: EventData::Message {
                                from: self.username.clone(),
                                contents: self.typed_message.clone(),
                                time_stamp: 0,
                            },
                        },
                    ) {
                        if should_log {
                            println!("Error:{:#?}", a);
                        }
                        self.connection = None;
                    } else if should_log {
                        println!("sent");
                    }
                }
                self.typed_message.clear();
            }
        }
        if should_host {
            EXISTS.store(false, std::sync::atomic::Ordering::Release);
            spawn_host(should_log);
            while !EXISTS.load(std::sync::atomic::Ordering::Acquire) && !SHOULD_DIE.load(std::sync::atomic::Ordering::Acquire){
            }
            if !SHOULD_DIE.load(std::sync::atomic::Ordering::Acquire){
                if let Ok(mut con) = TcpStream::connect(local_ip().unwrap().to_string() + ":8080") {
                    sleep(std::time::Duration::from_millis(15));
                    let _ = write_object(
                        &mut con,
                        &Event {
                            source: self.username.clone(),
                            data: EventData::Connection {
                                username: self.username.clone(),
                            },
                        },
                    );
                    self.owns_server = true;
                    self.connection = Some(con);
                }else{
                    if should_log{
                        println!("failed"); 
                    }
                }
            }

        }
    }
}
pub fn spawn_host(should_log: bool) {
    let _ = std::thread::spawn(move || {
        crate::server::Server::serve(should_log);
    });
}
