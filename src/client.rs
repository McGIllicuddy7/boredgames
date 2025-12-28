use eframe::egui::{self, Color32, Image, ImageSource, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use local_ip_address::local_ip;
use std::{collections::HashSet, net::SocketAddr, net::TcpStream, process::exit, thread::sleep};

use crate::{
    communication::*,
    server::{EXISTS, SHOULD_DIE},
    utils::{self, try_read_object, write_object},
};
#[derive(PartialEq)]
pub enum Mode {
    MoveAndPlace,
    Measure,
    Draw,
}
pub struct Client {
    pub state: State,
    pub typed_message: String,
    pub ip_address: String,
    pub username: String,
    pub connection: Option<TcpStream>,
    pub loaded_images: HashSet<String>,
    pub people: Vec<String>,
    pub owns_server: bool,
    pub working_layer: Layer,
    pub mode: Mode,
}
impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Self {
        let addr = get_ip();
        let mut out = Self {
            state: State::new(),
            typed_message: String::new(),
            ip_address: addr.to_string() + ":8080",
            connection: None,
            username: "root".into(),
            loaded_images: HashSet::new(),
            owns_server: false,
            people: Vec::new(),
            working_layer: Layer::Base,
            mode: Mode::MoveAndPlace,
        };
        if let Ok(p) = std::fs::read_dir(path()) {
            let s = p
                .into_iter()
                .filter(|i| {
                    if let Ok(tmp) = i {
                        tmp.file_name().to_string_lossy().ends_with(".bored")
                    } else {
                        false
                    }
                })
                .count();
            out.state.name = format!("map_{:#?}", s);
        }
        out
    }
    pub fn update(&mut self, ui: &mut Ui) {
        ui.with_layout(
            egui::Layout::top_down_justified(egui::Align::Center),
            |ui| {
                self.update_actual(ui);
            },
        );
    }
    pub fn draw_images(&mut self, should_log: bool, ui: &mut Ui) {
        let path = path();
        let Ok(f) = std::fs::read_dir(path) else {
            println!("failed to read: {}", path);
            return;
        };
        ui.vertical(move |ui| {
            for i in f {
                if let Ok(e) = i
                    && e.file_type().unwrap().is_file()
                {
                    let name = e.file_name().into_string().unwrap();
                    if name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".jpeg") {
                        let img = Image::new(ImageSource::Uri(
                            ("file://".to_string() + path + &name).into(),
                        ));
                        let s = match self.mode {
                            Mode::MoveAndPlace => Sense::all(),
                            _ => Sense::empty(),
                        };
                        let r = ui.add(egui::Button::image_and_text(img, name.clone()).sense(s));
                        if r.drag_stopped() {
                            let p = ui.input(|i| i.pointer.latest_pos().unwrap());
                            if p.x < 100.0 || p.y < 100.0 || p.x > 860. || p.y > 860.0 {
                                continue;
                            }
                            let p2 = Pos2 {
                                x: (p.x as i32 / 20 * 20) as f32,
                                y: (p.y as i32 / 20 * 20) as f32,
                            };
                            let count = self.state.tokens.len()
                                + self.state.map.len()
                                + self.state.gm.len();
                            let tname = format!("{:#?}_{:#?}", self.username, count);
                            let fname = format!("file://{}{}", path, name);
                            if should_log {
                                println!("{:#?}, {:#?}", p2, fname);
                            }
                            let ev = Event {
                                source: self.username.clone(),
                                data: EventData::TokenCreated {
                                    name: tname.clone(),
                                    token: Token {
                                        location: p2,
                                        image: name.clone(),
                                        scale: 1,
                                        display_name: String::new(),
                                    },
                                    layer: self.working_layer.clone(),
                                },
                            };
                            let n = path.to_string() + &name;
                            let ev0 = Event {
                                source: self.username.clone(),
                                data: EventData::ImageUpload {
                                    name: name.clone(),
                                    image: std::fs::read(n).unwrap(),
                                },
                            };
                            if let Some(obj) = self.connection.as_mut() {
                                if !self.loaded_images.contains(&name) {
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
    pub fn draw_layer(
        mutable: bool,
        ui: &mut Ui,
        values: &mut std::collections::HashMap<String, Token>,
        maxd: f32,
        connection: &mut Option<TcpStream>,
        username: String,
        layer: Layer,
    ) {
        for (name, token) in values {
            if let Err(e) = std::fs::File::open(path().to_string() + &token.image) {
                println!("{:#?}:{:#?}", token.image, e);
                continue;
            }
            let img = Image::new(ImageSource::Uri(
                ("file://".to_string() + path() + &token.image).into(),
            ));
            let scale = token.scale as f32;
            let changed = false;
            let ar = if mutable {
                let ar = egui::Area::new(name.clone().into())
                    .current_pos(Pos2::new(token.location.x, token.location.y))
                    .show(ui.ctx(), |ui| {
                        let img2 = img.fit_to_exact_size(Vec2::new(20.0 * scale, 20.0 * scale));
                        ui.add(img2);
                        ui.small(&token.display_name);
                    });
                ar
            } else {
                let ar = egui::Area::new(name.clone().into())
                    .current_pos(Pos2::new(token.location.x, token.location.y))
                    .sense(Sense::empty())
                    .show(ui.ctx(), |ui| {
                        let img2 = img.fit_to_exact_size(Vec2::new(20.0 * scale, 20.0 * scale));
                        ui.add(img2);
                        ui.small(&token.display_name);
                    });
                ar
            };
            if ar.response.dragged() {
                let mut r =
                    Pos2::new(token.location.x, token.location.y) + ar.response.drag_delta();
                if r.x < 100.0 {
                    r.x = 100.0;
                }
                if r.x > maxd {
                    r.x = maxd;
                }
                if r.y < 100.0 {
                    r.y = 100.0;
                }
                if r.y > maxd {
                    r.y = maxd;
                }
                token.location = Pos2 { x: r.x, y: r.y };
            }
            if ar.response.drag_stopped() || changed {
                token.location = Pos2 {
                    x: ((token.location.x as i32) / 20 * 20) as f32,
                    y: ((token.location.y as i32) / 20 * 20) as f32,
                };
                if let Some(c) = connection.as_mut() {
                    write_object(
                        c,
                        &Event {
                            source: username.clone(),
                            data: EventData::TokenMoved {
                                name: name.clone(),
                                to: token.clone(),
                                time_stamp: 0,
                                layer: layer.clone(),
                            },
                        },
                    )
                    .unwrap();
                }
            }
        }
    }
    pub fn draw_map(&mut self, ui: &mut Ui) {
        let name = path().to_string() + "board.png";
        if std::fs::File::open(&name).is_err() {
            return;
        }
        let maxd = 860.0;
        let p = ui.painter();
        p.rect_filled(
            Rect {
                min: Pos2::new(100.0, 100.0),
                max: Pos2::new(maxd, maxd),
            },
            0.0,
            Color32::WHITE,
        );
        for i in 1..750 / 20 + 1 {
            p.line(
                vec![
                    Pos2::new((i * 20 + 100) as f32, 100.),
                    Pos2::new((i * 20 + 100) as f32, maxd),
                ],
                Stroke::new(1.0, Color32::BLACK),
            );
        }
        for i in 1..750 / 20 + 1 {
            p.line(
                vec![
                    Pos2::new(100., (i * 20 + 100) as f32),
                    Pos2::new(maxd, (i * 20 + 100) as f32),
                ],
                Stroke::new(1.0, Color32::BLACK),
            );
        }
        ui.allocate_rect(
            Rect {
                min: Pos2::new(100.0, 100.0),
                max: Pos2::new(maxd, maxd),
            },
            Sense::empty(),
        );
        Self::draw_layer(
            self.working_layer == Layer::Map && self.mode == Mode::MoveAndPlace,
            ui,
            &mut self.state.map,
            maxd,
            &mut self.connection,
            self.username.clone(),
            Layer::Map,
        );
        if self.working_layer != Layer::Base {
            ui.scope(|ui| {
                ui.set_opacity(0.9);
                Self::draw_layer(
                    self.working_layer == Layer::Base && self.mode == Mode::MoveAndPlace,
                    ui,
                    &mut self.state.gm,
                    maxd,
                    &mut self.connection,
                    self.username.clone(),
                    Layer::Base,
                );
            });
        } else {
            Self::draw_layer(
                self.working_layer == Layer::Base && self.mode == Mode::MoveAndPlace,
                ui,
                &mut self.state.tokens,
                maxd,
                &mut self.connection,
                self.username.clone(),
                Layer::Base,
            );
        }
        if self.working_layer != Layer::Gm {
            ui.scope(|ui| {
                //ui.set_opacity(0.5);
                Self::draw_layer(
                    self.working_layer == Layer::Gm && self.mode == Mode::MoveAndPlace,
                    ui,
                    &mut self.state.gm,
                    maxd,
                    &mut self.connection,
                    self.username.clone(),
                    Layer::Gm,
                );
            });
        } else {
            Self::draw_layer(
                self.working_layer == Layer::Gm && self.mode == Mode::MoveAndPlace,
                ui,
                &mut self.state.gm,
                maxd,
                &mut self.connection,
                self.username.clone(),
                Layer::Gm,
            );
        }
    }
    pub fn map_controls(&mut self, should_log: bool, ui: &mut Ui) {
        _ = should_log;
        ui.collapsing("map settings", |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("map name:");
                    ui.text_edit_singleline(&mut self.state.name)
                });
                if ui.button("save").clicked() {
                    let s = serde_json::to_string_pretty(&self.state).unwrap();
                    let pth = path().to_string() + &self.state.name + ".bored";
                    let _ = std::fs::write(pth, s);
                }
            });
        });
    }
    pub fn server_info(
        &mut self,
        ui: &mut Ui,
        should_log: bool,
        should_connect: &mut bool,
        should_host: &mut bool,
        username_set: &mut bool,
    ) {
        ui.horizontal(|ui| {
            ui.label("ip address:");
            ui.text_edit_singleline(&mut self.ip_address);
            if self.connection.is_none() {
                if ui.button("connect").clicked() {
                    *should_connect = true;
                }
                if ui.button("host own server").clicked() {
                    *should_host = true;
                }
            } else {
                if ui.button("disconnect").clicked() {
                    if self.owns_server {
                        write_object(
                            &mut self.connection.as_mut().unwrap(),
                            &Event {
                                source: self.username.clone(),
                                data: EventData::Kill {
                                    password: "bridget".into(),
                                },
                            },
                        )
                        .unwrap();
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
                    if should_log {
                        println!("{:#?}", e);
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
                *username_set = true;
            }
            if let Some(_) = self.connection.as_ref() {
                self.username = old;
            }
        });
    }
    pub fn event_loop_iter(&mut self, should_log: bool) {
        if let Some(t) = self.connection.as_mut() {
            loop {
                let tr = try_read_object::<Event>(t, &mut Vec::new());
                if tr.is_err() {
                    if let Err(e) = tr {
                        if let Ok(t) = e.downcast::<std::io::Error>() {
                            match t.kind() {
                                std::io::ErrorKind::WouldBlock => {
                                    break;
                                }
                                std::io::ErrorKind::ConnectionReset => {
                                    break;
                                }
                                std::io::ErrorKind::UnexpectedEof => {
                                    break;
                                }
                                _ => {
                                    println!("disconnected {:#?}", t);
                                    self.connection = None;
                                    break;
                                }
                            }
                        }
                    }
                    break;
                }
                let Ok(ev) = tr else {
                    break;
                };
                if let Some(ev) = ev {
                    match ev.data {
                        EventData::SendState { state } => {
                            self.state = state;
                        }
                        EventData::ImageUpload { name, image } => {
                            if should_log {
                                println!("uploaded:{:#?}", name);
                            }
                            let _ = std::fs::create_dir("assets");
                            let e = std::fs::write(path().to_string() + &name, &image);
                            if let Err(e) = e {
                                println!("{:#?}", e);
                            }
                            //let img = Image::new(ImageSource::Bytes { uri: name.clone().into(), bytes: image.into()});
                            self.loaded_images.insert(name);
                        }
                        EventData::PersonalUpdate { people } => {
                            self.people = people;
                        }
                        _ => {
                            todo!()
                        }
                    }
                } else {
                    break;
                }
            }
        } else {
            self.connection = None;
        }
    }
    pub fn map_switching(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            let files = std::fs::read_dir(path()).unwrap();
            for i in files {
                if let Ok(p) = i {
                    let name = p.file_name().to_str().unwrap().to_string();
                    if let Some(n) = name.strip_suffix(".bored") {
                        if ui.button(n).clicked() {
                            let Ok(s) = std::fs::read_to_string(path().to_string() + &name) else {
                                continue;
                            };
                            let res_state: Result<State, _> = serde_json::from_str(&s);
                            if let Ok(s) = res_state {
                                if let Some(t) = self.connection.as_mut() {
                                    let _ = write_object(
                                        t,
                                        &Event {
                                            source: self.username.clone(),
                                            data: EventData::SendState { state: s },
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
            }
        });
    }
    pub fn user_info(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("connected users:");
            for i in &self.people {
                if ui.button(i).clicked() {}
            }
        });
    }
    pub fn tools(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.allocate_rect(
                Rect {
                    min: Pos2::new(0.0, 0.0),
                    max: Pos2::new(100.0, 100.0),
                },
                Sense::empty(),
            );
            ui.group(|ui| {
                ui.label("change mode:");
                if ui.button("tokens").clicked() {
                    self.working_layer = Layer::Base;
                }
                if ui.button("map").clicked() {
                    self.working_layer = Layer::Map;
                }
                if ui.button("gm").clicked() {
                    self.working_layer = Layer::Gm;
                }
            });
            ui.group(|ui| {
                ui.label("tools");
                if ui.button("select").clicked() {
                    self.mode = Mode::MoveAndPlace;
                }
                if ui.button("measure").clicked() {
                    self.mode = Mode::Measure;
                }
                if ui.button("draw").clicked() {
                    self.mode = Mode::Draw;
                }
            });
        });
    }
    pub fn update_actual(&mut self, ui: &mut Ui) {
        let should_log = false;
        let mut should_send = false;
        let mut should_connect = false;
        let mut should_host = false;
        let mut username_set = false;
        self.event_loop_iter(should_log);
        ui.vertical_centered(|ui| {
            self.user_info(ui);
            ui.horizontal(|ui| {
                self.tools(ui);
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
                self.draw_images(should_log, ui);
                self.map_switching(ui);
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
            ui.collapsing("server info", |ui| {
                self.server_info(
                    ui,
                    should_log,
                    &mut should_connect,
                    &mut should_host,
                    &mut username_set,
                );
            });
            self.map_controls(should_log, ui);
        });
        if should_connect && self.connection.is_none() {
            if should_log {
                println!("should connect to:{:#?}", self.ip_address);
            }
            let addr = SocketAddr::new(
                self.ip_address
                    .strip_suffix(":8080")
                    .unwrap()
                    .parse()
                    .unwrap(),
                8080,
            );
            if let Ok(mut con) =
                TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(3))
            {
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
                    if should_log {
                        println!("diconnected");
                    }
                    self.connection = None;
                } else {
                    self.connection = Some(con);
                }
            }
        }
        if should_send {
            if self.typed_message.starts_with("\\") {
                let msg = self.typed_message.strip_prefix("\\").unwrap();
                match msg {
                    "exit" => {
                        exit(0);
                    }
                    "kill" => {}
                    _ => {
                        if !msg.starts_with("roll ")
                            && !msg.starts_with("kick ")
                            && !msg.starts_with("roll_cheat ")
                        {
                            self.typed_message = "\\invalid command".into();
                            should_send = false;
                        }
                    }
                }
            }
            if should_send {
                if should_log {
                    println!("should send:{:#}", self.typed_message);
                }
                if let Some(con) = self.connection.as_mut() {
                    if self.typed_message == "\\kill" {
                        todo!()
                    } else if self.typed_message.starts_with("\\roll ") {
                        todo!()
                    } else if self.typed_message.starts_with("\\kick ") {
                        todo!()
                    } else if self.typed_message.starts_with("\\ roll_cheat") {
                        todo!()
                    } else if let Err(a) = utils::write_object(
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
            while !EXISTS.load(std::sync::atomic::Ordering::Acquire)
                && !SHOULD_DIE.load(std::sync::atomic::Ordering::Acquire)
            {}
            if !SHOULD_DIE.load(std::sync::atomic::Ordering::Acquire) {
                if let Ok(mut con) = TcpStream::connect(get_ip() + ":8080") {
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
                } else {
                    if should_log {
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
