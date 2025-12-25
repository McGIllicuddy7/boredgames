use std::{collections::HashMap, fs::FileType, net::TcpStream, process::exit};

use eframe::egui::{self, Image, ImageData, ImageSource, Pos2, Rect, Sense, Ui, Vec2};
use local_ip_address::local_ip;

use crate::{
    communication::*,
    utils::{self, try_read_object, write_object},
};
pub struct Client {
    pub state: State,
    pub typed_message: String,
    pub ip_address: String,
    pub username: String,
    pub connection: Option<TcpStream>,
    pub loaded_images: HashMap<String, ImageData>,
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
            loaded_images: HashMap::new(),
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
    pub fn draw_images(&mut self, ui: &mut Ui) {
        let f = std::fs::read_dir(".").unwrap();
        ui.vertical(move |ui| {
            for i in f {
                if let Ok(e) = i {
                    if e.file_type().unwrap().is_file() {
                        let name = e.file_name().into_string().unwrap();
                        if name.ends_with(".png")
                            || name.ends_with(".jpg")
                            || name.ends_with(".jpeg")
                        {
                            let img = Image::new(ImageSource::Uri(
                                ("file://".to_string() + "./" + &name).into(),
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
                                    x: (p.x as i32 / 20 * 20) as f32,
                                    y: (p.y as i32 / 20 * 20) as f32,
                                };
                                let count = self.state.tokens.len();
                                let tname = format!("{:#?}_{:#?}", self.username, count);
                                let fname = format!("file://./{}", name);
                                println!("{:#?}, {:#?}", p2, fname);
                                let ev = Event {
                                    source: self.username.clone(),
                                    data: EventData::TokenCreated {
                                        name: tname,
                                        token: Token {
                                            location: p2,
                                            image: fname,
                                        },
                                    },
                                };
                                if let Some(obj) = self.connection.as_mut() {
                                    let _ = write_object(obj, &ev);
                                }
                            }
                        }
                    }
                }
            }
        });
    }
    pub fn draw_map(&mut self, ui: &mut Ui) {
        let img0 = Image::new(ImageSource::Uri("file://./board.png".into()));
        let maxd = 800.0;
        ui.place(
            Rect {
                min: Pos2::new(50.0, 50.0),
                max: Pos2::new(maxd, maxd),
            },
            img0,
        );
        ui.allocate_rect(
            Rect {
                min: Pos2::new(50.0, 50.0),
                max: Pos2::new(maxd, maxd),
            },
            Sense::empty(),
        );
        for (_name, token) in &mut self.state.tokens {
            //  println!("drew:{_name}");
            let img = Image::new(ImageSource::Uri(token.image.clone().into()));
            let ar = egui::Area::new(_name.clone().into())
                .current_pos(Pos2::new(token.location.x as f32, token.location.y as f32))
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
                    x: r.x as f32,
                    y: r.y as f32,
                };
            }
            if ar.response.drag_stopped() {
                token.location = Pos2 {
                    x: ((token.location.x as i32) / 20 * 20) as f32,
                    y: ((token.location.y as i32) / 20 * 20) as f32,
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
    }
    pub fn update_actual(&mut self, ui: &mut Ui) {
        let should_log = false;
        if let Some(t) = self.connection.as_mut() {
            while let Ok(ev) = try_read_object::<Event>(t, &mut Vec::new()) {
                if let Some(ev) = ev {
                    match ev.data {
                        EventData::SendState { state } => {
                            self.state = state;
                        }
                        EventData::ImageUpload { name, image } => {
                            todo!()
                        }
                        _ => {
                            todo!()
                        }
                    }
                } else {
                    break;
                }
            }
        }
        let mut should_send = false;
        let mut should_connect = false;
        let mut should_host = false;
        let mut username_set = false;
        ui.vertical_centered(|ui| {
            ui.horizontal(|ui| {
                ui.label("enter ip address:");
                ui.text_edit_singleline(&mut self.ip_address);
                if ui.button("connect").clicked() {
                    should_connect = true;
                }
                if ui.button("host own server").clicked() {
                    should_host = true;
                }
                if let Some(s) = self.connection.as_ref() {
                    if s.take_error().is_ok() {
                        ui.label("connected");
                    } else {
                        self.connection = None;
                        ui.label("not connected");
                    }
                } else {
                    ui.label("not connected");
                }
            });
            ui.horizontal(|ui| {
                ui.label("username:");
                ui.text_edit_singleline(&mut self.username);
                if ui.button("enter").clicked() {
                    username_set = true;
                }
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
                self.draw_images(ui);
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
        if should_connect {
            if should_log {
                println!("should connect to:{:#?}", self.ip_address);
            }
            if let Ok(mut con) = TcpStream::connect(&self.ip_address) {
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
            spawn_host(should_log);
            if let Ok(mut con) = TcpStream::connect(local_ip().unwrap().to_string() + ":8080") {
                let _ = write_object(
                    &mut con,
                    &Event {
                        source: self.username.clone(),
                        data: EventData::Connection {
                            username: self.username.clone(),
                        },
                    },
                );
                self.connection = Some(con);
            }
        }
    }
}
pub fn spawn_host(should_log: bool) {
    let _ = std::thread::spawn(move || {
        crate::server::Server::serve(should_log);
    });
}
