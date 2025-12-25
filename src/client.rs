use std::{net::TcpStream, process::exit};

use eframe::egui::{self, Image, ImageSource, Pos2, Rect, Ui, Vec2};
use local_ip_address::local_ip;

use crate::{communication::*, utils::{self, try_read_object, write_object}};
pub struct Client{
    pub state:State,
    pub typed_message:String,
    pub ip_address:String,
    pub username:String,
    pub connection:Option<TcpStream>,
}
impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client{
    pub fn new()->Self{
        let addr = local_ip().unwrap();
        Self { state: State::new(), typed_message: String::new(), 
            ip_address:addr.to_string()+":8080", connection: None, username:"root".into()}
    }
    pub fn update(&mut self, ui:&mut Ui){
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                self.update_actual(ui);
            });
    }
    pub fn update_actual(&mut self, ui:&mut Ui){
        let should_log = false;
        if let Some(t) = self.connection.as_mut(){
            while let Ok(state) = try_read_object::<State>(t, &mut Vec::new()){
                if let Some(state) = state{
                    self.state = state;
                }else{
                    break;
                }
            }
        }
        let mut should_send = false;
        let mut should_connect = false;
        let mut should_host = false;
        let mut username_set = false;
        ui.vertical_centered(|ui|{
            ui.horizontal(|ui| {
                ui.label("enter ip address:");
                ui.text_edit_singleline(&mut self.ip_address );
                if ui.button("connect").clicked(){
                    should_connect = true;
                }
                if ui.button("host own server").clicked(){
                    should_host = true;
                }
                if let Some(s) = self.connection.as_ref(){
                    if s.take_error().is_ok(){
                        ui.label("connected");
                    }else{
                        self.connection = None;
                        ui.label("not connected");
                    }
                   
                }else{
                    ui.label("not connected");
                }
            });
            ui.horizontal(|ui|{
                ui.label("username:");
                ui.text_edit_singleline(&mut self.username);
                if ui.button("enter").clicked(){
                    username_set = true;
                }
            });
            ui.horizontal(|ui|{
                /*let (p, paint) = ui.allocate_painter(Vec2::new(600.0, 600.0), Sense::all());
                paint.rect(Rect { min: Pos2::new(0.0, 0.0), max: Pos2::new(500.0, 500.0) }, 0.0, Color32::WHITE, Stroke::NONE, egui::StrokeKind::Inside);
                p.
                for (name, token) in &self.state.tokens{
                    paint.circle(Pos2 { x: token.location.x as f32*10.0, y:token.location.y as f32*10.0 }, 5.0, Color32::DARK_GREEN, Stroke::NONE);
                    paint.text(Pos2 { x: token.location.x as f32*10.0, y:token.location.y as f32*10.0 }, Align2::LEFT_TOP, name, FontId::monospace(16.0), Color32::BLACK);
                }*/
                let img = Image::new(ImageSource::Uri("file://./SOLARIS.jpg".into()));
                ui.put(Rect { min: Pos2::new(0.0, 0.0), max: Pos2::new(500.0,500.0) },img);
                 //ui.put(Rect { min: Pos2::new(400.0, 400.0), max: Pos2::new(450.0,450.0) },img2);
                ui.allocate_ui(Vec2::new(500.0, 500.0), |ui| {
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min),|ui|{
                            ui.group(|ui|{
                                ui.set_min_height(400.0);
                                ui.set_min_width(400.0);
                                ui.set_max_height(430.0);
                                ui.set_clip_rect(ui.min_rect());
                                ui.set_min_height(430.0);
                                for i in self.state.messages.iter().rev(){
                                    ui.code(format!("{}:{}",i.0, i.1));
                                }
                            });

                    });
                });
            });

            ui.horizontal(|ui|{
                ui.label("enter message:");
                let foc = ui.text_edit_singleline(&mut self.typed_message);
                if ui.button("send").clicked() || (ui.input(|i| i.key_pressed(egui::Key::Enter)) &&foc.lost_focus()) {
                    should_send = true;
                    foc.request_focus();
                }
              
            });
        });
        if should_connect{
            if should_log{
                println!("should connect to:{:#?}", self.ip_address);
            }
            if let Ok(mut con) = TcpStream::connect(&self.ip_address){
                let _ = write_object(&mut con, &Event{source:self.username.clone(), data:EventData::Connection { username: self.username.clone() }});
                if let Err(_) = write_object(&mut con, &Event{source:self.username.clone(),data:EventData::HeartBeat}){
                    self.connection = None;
                }else{
                    self.connection = Some(con);
                }
              
            }
        }
        if should_send{
            //self.state.messages.push((self.username.clone(),self.typed_message.clone()));
            if self.typed_message.starts_with("\\"){
                let msg = self.typed_message.strip_prefix("\\").unwrap();
                match msg{
                    "exit"=>{
                        exit(0);
                    }
                    _=>{
                        self.typed_message = "\\invalid command".into();
                        should_send = false;
                    }
                }
            }
            if should_send{
                if should_log{
                    println!("should send:{:#}", self.typed_message);
                }    
                if let Some(con) = self.connection.as_mut(){
                    if let Err(a) = utils::write_object(con, &Event{source:self.username.clone(), data:EventData::Message { from: self.username.clone(), contents: self.typed_message.clone(), time_stamp: 0 }}){
                        if should_log{
                            println!("Error:{:#?}",a);
                        }                 
                        self.connection = None;
                    } else if should_log{
                        println!("sent");
                    }
                }
                self.typed_message.clear();
            }
        }
        if should_host{
            spawn_host(should_log);
            if let Ok(mut con) = TcpStream::connect("127.0.0.1:8080"){
                let _ = write_object(&mut con, &Event{source:self.username.clone(), data:EventData::Connection { username: self.username.clone() }});
                self.connection = Some(con);
            }
        }

    }
}
pub fn spawn_host(should_log:bool){
    let _ = std::thread::spawn(move ||{
        crate::server::Server::serve(should_log);
    });
}