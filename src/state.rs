use crate::dos::tilemap::{
    BACKGROUND_LAYER, FOREGROUND_LAYER, HIDDEN_LAYER, LAYER_COUNT, TILE_LAYER, TileMap, TileMapData,
};
pub use crate::id::GlobalId;
use crate::id::{ArachneId, IdPageAllocator};
use crate::rtils::marathon::BStream;
use crate::rtils::rtils_useful::BPipe;
use crate::voip::{VoipCmd, spawn_voip, spawn_voip_server};
use crate::{dos::BColor, id::IdAllocator};

use raylib::color::Color;
use raylib::texture::Image;
pub use serde::{Deserialize, Serialize};
pub use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::io::BufRead;
pub use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::{collections::HashMap, io::stdin, net::TcpListener, process::exit};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageContents {
    Image {
        contents: Arc<[BColor]>,
        name: String,
        width: i32,
        height: i32,
    },
    File {
        contents: Arc<[u8]>,
        name: String,
    },
    Text {
        contents: Arc<String>,
    },
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserMessage {
    pub timestamp: u64,
    pub user_name: String,
    pub contents: MessageContents,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerState {
    pub messages_start: u64,
    pub messages: VecDeque<Message>,
    pub users: HashMap<GlobalId, String>,
    pub map: TileMap,
    pub name: String,
    pub id: GlobalId,
    pub alloc: Arc<IdAllocator>,
    pub current_layer: usize,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MapUpdate {
    SetImageTable(Vec<String>),
    Rescale {
        width: i32,
        height: i32,
    },
    SetTile {
        layer: usize,
        x: i32,
        y: i32,
        to: u16,
    },
    CreateSprite {
        id: u64,
        image: String,
        layer: usize,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        name: String,
    },
    DestroySprite {
        id: u64,
    },
    MoveSprite {
        id: u64,
        x: i32,
        y: i32,
    },
    SpriteImage {
        id: u64,
        img: u64,
    },
    ChangeSpriteLayer {
        id: u64,
        layer: usize,
    },
    ScaleSprite {
        id: u64,
        w: i32,
        h: i32,
    },
    SendTable(TileMapData),
    RequestData,
    RenameSprite {
        id: u64,
        name: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    SendMessage(UserMessage),
    Connect {
        username: String,
        id: GlobalId,
    },
    HandShake {
        state: ServerState,
        allocator_base: u64,
        user_id: GlobalId,
    },
    Disconnect {
        username: String,
        id: GlobalId,
    },
    SetUsername {
        id: GlobalId,
        to: String,
    },
    UpdateMap {
        update: MapUpdate,
    },
    YouShouldLeave,
    RequestMessagePage {
        indx: u64,
        before: bool,
    },
    MessagePage {
        start_indx: u64,
        contents: Vec<Message>,
    },
}
impl ServerState {
    pub fn update(&mut self, update: MapUpdate, connection: &BStream<Message>) {
        match update {
            MapUpdate::SetImageTable(items) => {
                self.map.set_draw_table(&items);
            }
            MapUpdate::Rescale { width, height } => {
                let data = self.map.get_data();
                let mut out = data.clone();
                for l in 0..data.layers.len() {
                    let mut v = Vec::new();
                    for i in 0..height {
                        for j in 0..width {
                            if i < data.map_height && j < data.map_width {
                                v.push(AtomicU64::new(
                                    data.layers[l][(i * data.map_height + j) as usize]
                                        .load(std::sync::atomic::Ordering::Acquire),
                                ));
                            } else {
                                v.push(AtomicU64::new(0));
                            }
                        }
                    }
                }
                out.map_height = height;
                out.map_width = width;
                self.map.set_data(out);
            }
            MapUpdate::SetTile { layer, x, y, to } => {
                self.map.set_tile(layer, x, y, to);
            }
            MapUpdate::SpriteImage { id, img } => {
                let mut s = self.map.get_sprite(id);
                s.image_id = img;
                self.map.set_sprite(id, s);
            }
            MapUpdate::CreateSprite {
                id,
                image,
                x,
                y,
                w,
                layer,
                h,
                name: _,
            } => {
                let img = self.map.get_image_id(&image);
                let img = if img.is_some() {
                    img.unwrap()
                } else {
                    self.map.load_image(&image)
                };
                self.map.create_sprite_with_id(id, x, y, w, h, img, layer);
            }
            MapUpdate::DestroySprite { id } => {
                self.alloc.free_id(GlobalId::create(id));
                self.map.delete_sprite(id);
            }
            MapUpdate::MoveSprite { id, x, y } => {
                let mut s = self.map.get_sprite(id);
                s.x_pos = x as i16;
                s.y_pos = y as i16;
                self.map.set_sprite(id, s);
            }
            MapUpdate::ChangeSpriteLayer { id, layer } => {
                let mut s = self.map.get_sprite(id);
                s.layer = layer as u8;
                self.map.set_sprite(id, s);
            }
            MapUpdate::ScaleSprite { id, w, h } => {
                let mut s = self.map.get_sprite(id);
                s.width = w as u16;
                s.height = h as u16;
                self.map.set_sprite(id, s);
            }
            MapUpdate::SendTable(map) => {
                self.map.set_data(map);
            }
            MapUpdate::RequestData => {
                connection
                    .send(Message::UpdateMap {
                        update: MapUpdate::SendTable(self.map.get_data()),
                    })
                    .unwrap();
            }
            MapUpdate::RenameSprite { id, name } => {
                let mut spr = self.map.get_sprite(id);
                spr.display_name = name.into();
                self.map.set_sprite(id, spr);
            }
        }
    }
    pub fn print_message(msg: &Message) {
        match msg {
            Message::SendMessage(msg) => match &msg.contents {
                MessageContents::File { contents, name } => {
                    println!("$file:{}", name);
                    let _ = std::fs::write(name, contents);
                }
                MessageContents::Image {
                    contents,
                    name,
                    width,
                    height,
                } => {
                    println!("{}-$image:{}", msg.user_name, name);
                    let mut img = Image::gen_image_color(*width, *height, Color::WHITE);
                    for i in 0..*height {
                        for j in 0..*width {
                            img.draw_pixel(j, i, contents[(i * *width + j) as usize].as_rl_color());
                        }
                    }
                    img.export_image(name);
                }
                MessageContents::Text { contents } => {
                    println!("{}:{}", msg.user_name, contents);
                }
            },
            Message::Connect { username, id: _ } => {
                println!("{}: connected", username);
            }
            Message::Disconnect { username, id: _ } => {
                println!("{}: disconnected", username);
            }
            Message::HandShake {
                state,
                allocator_base,
                user_id,
            } => {
                println!("id:{:#?}", user_id);
                for i in &state.messages {
                    Self::print_message(i);
                }
                println!("{:#?}", allocator_base);
            }
            Message::YouShouldLeave => {}
            Message::SetUsername { id, to } => {
                println!("{} set name to {}", id.get(), to);
            }
            Message::UpdateMap { update } => match update {
                MapUpdate::SetImageTable(items) => {
                    println!("set image table:{:#?}", items);
                }
                MapUpdate::Rescale { width, height } => {
                    println!("map rescale to:{} by {}", width, height);
                }
                MapUpdate::SetTile { x, y, to, layer } => {
                    println!("on layer:{},set {} {} to {}", layer, x, y, to);
                }
                MapUpdate::SpriteImage { id, img } => {
                    println!("set image of {} to {}", id, img);
                }
                MapUpdate::CreateSprite {
                    id,
                    image,
                    layer,
                    x,
                    y,
                    w,
                    h,
                    name,
                } => {
                    println!(
                        "created sprite:{} at {} {} on layer{} with width:{} and height{}, named:{} with image:{}",
                        id, x, y, layer, w, h, name, image
                    );
                }
                MapUpdate::DestroySprite { id } => {
                    println!("destroyed sprite:{}", id);
                }
                MapUpdate::ChangeSpriteLayer { id, layer } => {
                    println!("changed {} layer to {} ", id, layer);
                }
                MapUpdate::MoveSprite { id, x, y } => {
                    println!("moved {} to {} {}", id, x, y);
                }
                MapUpdate::ScaleSprite { id, w, h } => {
                    println!("scaled {} to {} by {}", id, w, h);
                }
                MapUpdate::SendTable(_) => {
                    println!("table update");
                }
                MapUpdate::RequestData => {
                    println!("table request");
                }
                MapUpdate::RenameSprite { id, name } => {
                    println!("renamed:{} to {}", id, name);
                }
            },
            Message::RequestMessagePage { indx: _, before: _ } => {}
            Message::MessagePage {
                start_indx: _,
                contents: _,
            } => {}
        }
    }

    pub fn text_client(
        &mut self,
        connection: BStream<Message>,
        stdin_p: &mut BPipe<String>,
        con: String,
    ) {
        let mut should_dc = false;
        let mut should_exit = false;
        let mut voip = None;
        connection
            .send(Message::Connect {
                username: self.name.clone(),
                id: GlobalId::invalid(),
            })
            .unwrap();
        loop {
            if should_dc {
                let _ = connection.send(Message::Disconnect {
                    username: self.name.clone(),
                    id: self.id,
                });
                break;
            }
            if should_exit {
                let _ = connection.send(Message::Disconnect {
                    username: self.name.clone(),
                    id: self.id,
                });
                exit(0);
            }
            while let Ok(Some(msg)) = connection.receive() {
                Self::print_message(&msg);
                match msg {
                    Message::Connect { username, id } => {
                        self.users.insert(id, username);
                    }
                    Message::Disconnect { username: _, id } => {
                        self.users.remove(&id);
                    }
                    Message::HandShake {
                        state,
                        allocator_base,
                        user_id,
                    } => {
                        let name = self.name.clone();
                        *self = state;
                        self.name = name;
                        self.alloc = Arc::new(IdAllocator::new(allocator_base));
                        self.id = user_id;
                        voip = Some(spawn_voip(&con, user_id).unwrap());
                    }
                    Message::YouShouldLeave => {
                        should_dc = true;
                    }
                    Message::UpdateMap { update } => {
                        self.update(update, &connection);
                    }
                    Message::SetUsername { id, to } => {
                        *self.users.get_mut(&id).unwrap() = to;
                    }
                    Message::SendMessage(_) => {}
                    Message::RequestMessagePage { indx: _, before: _ } => {}
                    Message::MessagePage {
                        start_indx: _,
                        contents: _,
                    } => {}
                }
            }
            if should_dc {
                connection
                    .send(Message::Disconnect {
                        username: self.name.clone(),
                        id: self.id,
                    })
                    .unwrap();
            }

            let Some(cmd) = stdin_p.recieve().unwrap() else {
                continue;
            };
            let cmd = cmd.strip_suffix("\n").unwrap().to_string();
            if cmd.is_empty() {
                continue;
            }
            self.run_cmd(
                cmd,
                &connection,
                &mut should_dc,
                &mut should_exit,
                &mut voip,
            );
        }
    }
    pub fn run_cmd(
        &mut self,
        cmd: String,
        connection: &BStream<Message>,
        should_dc: &mut bool,
        should_exit: &mut bool,
        voip: &mut Option<BStream<VoipCmd>>,
    ) {
        if let Some(nxt) = cmd.strip_prefix("$") {
            if nxt.starts_with("$") {
                connection
                    .send(Message::SendMessage(UserMessage {
                        timestamp: std::time::UNIX_EPOCH.elapsed().unwrap().as_secs(),
                        user_name: self.name.clone(),
                        contents: MessageContents::Text {
                            contents: nxt.to_string().into(),
                        },
                    }))
                    .unwrap();
            } else {
                let Some((cmd2, contents)) = nxt.split_once(" ") else {
                    match nxt {
                        "disconnect" => {
                            *should_dc = true;
                            return;
                        }
                        "exit" => {
                            *should_dc = true;
                            *should_exit = true;
                            println!("exiting");
                            return;
                        }
                        "!mute" => {
                            if let Some(x) = voip.as_mut() {
                                x.send(VoipCmd::UnMute).unwrap();
                            }
                            return;
                        }
                        "mute" => {
                            if let Some(x) = voip.as_mut() {
                                x.send(VoipCmd::Mute).unwrap();
                            }
                            return;
                        }
                        "deafen" => {
                            if let Some(x) = voip.as_mut() {
                                x.send(VoipCmd::Deafen).unwrap();
                            }
                            return;
                        }
                        "!deafen" => {
                            if let Some(x) = voip.as_mut() {
                                x.send(VoipCmd::UnDeafen).unwrap();
                            }
                            return;
                        }
                        "print" => {
                            let layers = ["background", "tiles", "hidden", "foreground"];
                            let sprites = self.map.get_sprites();
                            for i in 0..LAYER_COUNT {
                                if i == HIDDEN_LAYER && self.current_layer != HIDDEN_LAYER {
                                    return;
                                }
                                println!("layer:{}", layers[i]);
                                for s in sprites.values() {
                                    if s.layer == i as u8 {
                                        println!("{:#?}", s);
                                    }
                                }
                            }
                            return;
                        }
                        _ => {
                            return;
                        }
                    }
                };
                match cmd2 {
                    "name" => {
                        self.name = contents.to_string();
                        let msg = Message::SetUsername {
                            id: self.id,
                            to: contents.into(),
                        };
                        connection.send(msg).unwrap();
                    }
                    "image" => {
                        let Ok(mut img) = raylib::prelude::Image::load_image(contents) else {
                            return;
                        };
                        let mut cols = Vec::new();
                        cols.reserve_exact((img.height() * img.width()) as usize);
                        for i in 0..img.height() {
                            for j in 0..img.width() {
                                cols.push(BColor::from_rl_color(img.get_color(j, i)));
                            }
                        }
                        connection
                            .send(Message::SendMessage(UserMessage {
                                timestamp: std::time::UNIX_EPOCH.elapsed().unwrap().as_secs(),
                                user_name: self.name.clone(),
                                contents: MessageContents::Image {
                                    contents: cols.into(),
                                    name: contents.into(),
                                    width: img.width(),
                                    height: img.height(),
                                },
                            }))
                            .unwrap();
                    }
                    "file" => {
                        let Ok(bytes) = std::fs::read(contents) else {
                            return;
                        };
                        connection
                            .send(Message::SendMessage(UserMessage {
                                timestamp: std::time::UNIX_EPOCH.elapsed().unwrap().as_secs(),
                                user_name: self.name.clone(),
                                contents: MessageContents::File {
                                    contents: bytes.into(),
                                    name: contents.to_string(),
                                },
                            }))
                            .unwrap();
                    }
                    "disconnect" => {
                        *should_dc = true;
                    }
                    "exit" => {
                        *should_dc = true;
                        *should_exit = true;
                        println!("exiting");
                    }
                    "create" => {
                        let mut s = contents.split_ascii_whitespace();
                        let Some(img) = s.next() else {
                            return;
                        };
                        let Some(x) = s.next() else {
                            return;
                        };
                        let Some(y) = s.next() else {
                            return;
                        };

                        let Ok(x) = x.parse::<i32>() else {
                            return;
                        };
                        let Ok(y) = y.parse::<i32>() else {
                            return;
                        };
                        let image = img.to_string();
                        let id = self.alloc.alloc_id();
                        let img_id = self.map.get_image_id(img);
                        let img = if let Some(img_id) = img_id {
                            img_id
                        } else {
                            self.map.load_image(img)
                        };
                        self.map.create_sprite_with_id(
                            id.get(),
                            x,
                            y,
                            1,
                            1,
                            img,
                            self.current_layer,
                        );
                        let msg = Message::UpdateMap {
                            update: MapUpdate::CreateSprite {
                                id: id.get(),
                                image,
                                layer: self.current_layer,
                                x,
                                y,
                                w: 1,
                                h: 1,
                                name: String::new(),
                            },
                        };
                        connection.send(msg).unwrap();
                        println!("created:{}", id.get());
                    }
                    "destroy" => {
                        let mut s = contents.split_ascii_whitespace();
                        let Some(id) = s.next() else {
                            return;
                        };
                        let Ok(id) = id.parse::<u64>() else {
                            return;
                        };
                        if !self.map.check_sprite_exists(id) {
                            return;
                        }
                        self.map.delete_sprite(id);
                        self.alloc.free_id(GlobalId::create(id));
                        let msg = Message::UpdateMap {
                            update: MapUpdate::DestroySprite { id },
                        };
                        connection.send(msg).unwrap();
                    }
                    "move-id" => {
                        let mut s = contents.split_ascii_whitespace();
                        let Some(name) = s.next() else {
                            return;
                        };
                        let Some(x) = s.next() else {
                            return;
                        };
                        let Some(y) = s.next() else {
                            return;
                        };

                        let Ok(x) = x.parse::<i32>() else {
                            return;
                        };
                        let Ok(y) = y.parse::<i32>() else {
                            return;
                        };
                        let Ok(id) = name.parse::<u64>() else {
                            return;
                        };
                        if !self.map.check_sprite_exists(id) {
                            return;
                        }
                        let mut spr = self.map.get_sprite(id);
                        spr.x_pos = x as i16;
                        spr.y_pos = y as i16;
                        self.map.set_sprite(id, spr);
                        let msg = Message::UpdateMap {
                            update: MapUpdate::MoveSprite { id, x, y },
                        };
                        connection.send(msg).unwrap();
                    }
                    "move" => {
                        let mut s = contents.split_ascii_whitespace();
                        let Some(name) = s.next() else {
                            return;
                        };
                        let Some(x) = s.next() else {
                            return;
                        };
                        let Some(y) = s.next() else {
                            return;
                        };

                        let Ok(x) = x.parse::<i32>() else {
                            return;
                        };
                        let Ok(y) = y.parse::<i32>() else {
                            return;
                        };
                        let sprites = self.map.get_sprites();
                        let mut id = 0;
                        for i in &sprites {
                            if i.1.display_name.as_ref() == name {
                                id = *i.0;
                                break;
                            }
                        }
                        if id == 0 {
                            return;
                        }
                        if !self.map.check_sprite_exists(id) {
                            return;
                        }
                        let mut spr = self.map.get_sprite(id);
                        spr.x_pos = x as i16;
                        spr.y_pos = y as i16;
                        self.map.set_sprite(id, spr);
                        let msg = Message::UpdateMap {
                            update: MapUpdate::MoveSprite { id, x, y },
                        };
                        connection.send(msg).unwrap();
                    }
                    "sprite-image-id" => {
                        let mut s = contents.split_ascii_whitespace();
                        let Some(name) = s.next() else {
                            return;
                        };
                        let Some(img) = s.next() else {
                            return;
                        };
                        let Ok(id) = name.parse::<u64>() else {
                            return;
                        };
                        if !self.map.check_sprite_exists(id) {
                            return;
                        }
                        let mut spr = self.map.get_sprite(id);
                        let img = if let Some(id) = self.map.get_image_id(img) {
                            id
                        } else {
                            self.map.load_image(img)
                        };
                        spr.image_id = img;
                        self.map.set_sprite(id, spr);
                        let msg = Message::UpdateMap {
                            update: MapUpdate::SpriteImage { id, img },
                        };
                        connection.send(msg).unwrap();
                    }
                    "sprite-image" => {
                        let mut s = contents.split_ascii_whitespace();
                        let Some(name) = s.next() else {
                            return;
                        };
                        let Some(img) = s.next() else {
                            return;
                        };
                        let mut id = 0;
                        for (i, j) in self.map.get_sprites() {
                            if j.display_name.as_ref() == name {
                                id = i;
                                break;
                            }
                        }
                        if id == 0 {
                            return;
                        }
                        let mut spr = self.map.get_sprite(id);
                        let img = if let Some(id) = self.map.get_image_id(img) {
                            id
                        } else {
                            self.map.load_image(img)
                        };
                        spr.image_id = img;
                        self.map.set_sprite(id, spr);
                        let msg = Message::UpdateMap {
                            update: MapUpdate::SpriteImage { id, img },
                        };
                        connection.send(msg).unwrap();
                    }
                    "sprite-layer" => {
                        let mut s = contents.split_ascii_whitespace();
                        let Some(name) = s.next() else {
                            return;
                        };
                        let Some(ly) = s.next() else {
                            return;
                        };
                        let Ok(id) = name.parse::<u64>() else {
                            return;
                        };
                        if !self.map.check_sprite_exists(id) {
                            return;
                        }
                        let mut spr = self.map.get_sprite(id);
                        let layer = match ly {
                            "background" => BACKGROUND_LAYER,
                            "tile" => TILE_LAYER,
                            "hidden" => HIDDEN_LAYER,
                            "foreground" => FOREGROUND_LAYER,
                            _ => {
                                return;
                            }
                        };
                        spr.layer = layer as u8;
                        self.map.set_sprite(id, spr);
                        let msg = Message::UpdateMap {
                            update: MapUpdate::ChangeSpriteLayer { id, layer },
                        };
                        connection.send(msg).unwrap();
                    }
                    "layer" => {
                        let mut s = contents.split_ascii_whitespace();
                        let Some(to) = s.next() else {
                            return;
                        };
                        match to {
                            "background" => {
                                self.current_layer = BACKGROUND_LAYER;
                            }
                            "tile" => {
                                self.current_layer = TILE_LAYER;
                            }
                            "hidden" => {
                                self.current_layer = HIDDEN_LAYER;
                            }
                            "foreground" => {
                                self.current_layer = FOREGROUND_LAYER;
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        } else {
            let Ok(_) = connection.send(Message::SendMessage(UserMessage {
                timestamp: std::time::UNIX_EPOCH.elapsed().unwrap().as_secs(),
                user_name: self.name.clone(),
                contents: MessageContents::Text {
                    contents: cmd.into(),
                },
            })) else {
                println!("failed to send");
                return;
            };
        }
    }
}

pub fn run_text_mode() {
    let mut stdin_p = stdin_thread();
    let mut name = "bridget".to_string();
    loop {
        let Some(input) = stdin_p.recieve().unwrap() else {
            continue;
        };
        let mut inputs = input.split_ascii_whitespace();
        let Some(cmd) = inputs.next() else {
            continue;
        };
        match cmd {
            "name" => {
                let Some(to) = inputs.next() else {
                    continue;
                };
                name = to.into();
            }
            "host" => {
                let Some(to) = inputs.next() else {
                    continue;
                };
                let (stream, inp) = BStream::create();
                host_server(to, inp);
                let mut state = ServerState {
                    messages_start: 0,
                    messages: VecDeque::new(),
                    users: HashMap::new(),
                    id: GlobalId::invalid(),
                    name: name.clone(),
                    map: TileMap::new(100, 100, "bg.png".to_string(), Vec::new()),
                    alloc: Arc::new(IdAllocator::new(0)),
                    current_layer: TILE_LAYER,
                };
                state.text_client(stream, &mut stdin_p, to.to_string());
            }
            "join" => {
                let Some(to) = inputs.next() else {
                    continue;
                };
                let Ok(stream) = std::net::TcpStream::connect((to, 8008)) else {
                    continue;
                };
                let mut state = ServerState {
                    messages_start: 0,
                    messages: VecDeque::new(),
                    users: HashMap::new(),
                    id: GlobalId::invalid(),
                    name: name.clone(),
                    map: TileMap::new(100, 100, "bg.png".to_string(), Vec::new()),
                    current_layer: TILE_LAYER,
                    alloc: Arc::new(IdAllocator::new(0)),
                };
                let st = BStream::from_stream(stream);
                state.text_client(st, &mut stdin_p, to.to_string());
            }
            "exit" => {
                break;
            }
            _ => {}
        }
    }
}

pub fn host_server(port: &str, stream: BStream<Message>) {
    let port = port.to_string();
    std::thread::spawn(|| run_server(port, stream));
}

pub struct User {
    pub name: String,
    pub connection: BStream<Message>,
    pub page_start: u64,
    pub id: GlobalId,
}
pub const PAGE_SIZE: u64 = 32;
pub fn run_server(port: String, in_stream: BStream<Message>) {
    println!("running server!");
    let serv = spawn_voip_server(port.clone());
    let pages = IdPageAllocator::new();
    let gal = IdAllocator::new(pages.alloc_page());
    let listen = TcpListener::bind((port.as_str(), 8008)).unwrap();
    listen.set_nonblocking(true).unwrap();
    let (send, rec) = BPipe::create();
    std::thread::spawn(move || {
        loop {
            while let Ok((stream, _)) = listen.accept() {
                send.send(stream).unwrap();
            }
        }
    });
    let mut connections: HashMap<GlobalId, User> = HashMap::new();
    let mut state = ServerState {
        messages_start: 0,
        messages: VecDeque::new(),
        users: HashMap::new(),
        name: "host".to_string(),
        id: gal.alloc_id(),
        map: TileMap::new(100, 100, "bg.png".to_string(), Vec::new()),
        current_layer: TILE_LAYER,
        alloc: Arc::new(IdAllocator::new(0)),
    };
    let mut new_connections = Vec::new();
    new_connections.push(in_stream);
    let mut page = Vec::new();
    loop {
        while let Ok(Some(x)) = rec.recieve() {
            new_connections.push(BStream::from_stream(x));
        }
        let mut messages = Vec::new();
        for (i, j) in &mut connections {
            while let Ok(Some(msg)) = j.connection.receive() {
                messages.push((*i, msg));
            }
        }
        for i in new_connections {
            let b = i;
            let Ok(x) = b.receive_wait() else {
                continue;
            };
            if let Message::Connect { username, id: _ } = x {
                let ps = pages.alloc_page();
                let id = gal.alloc_id();
                b.send(Message::HandShake {
                    state: state.clone(),
                    allocator_base: ps,
                    user_id: id,
                })
                .unwrap();
                connections.insert(
                    id,
                    User {
                        name: username.clone(),
                        connection: b,
                        page_start: ps,
                        id,
                    },
                );
                messages.push((id, Message::Connect { username, id }));
            }
        }
        new_connections = Vec::new();
        for (from, msg) in messages {
            state.messages.push_back(msg.clone());
            match &msg {
                Message::Disconnect { username: _, id } => {
                    println!("dced");
                    if let Some(c) = connections.get(id) {
                        pages.free_page(c.page_start);
                    }
                    connections.remove(id);
                }
                Message::UpdateMap { update } => {
                    state.update(
                        update.clone(),
                        &connections.get_mut(&from).unwrap().connection,
                    );
                }
                Message::RequestMessagePage { indx, before } => {
                    let (to_get, idx) = if *before {
                        if *indx == 0 {
                            continue;
                        }
                        let id = (*indx - 1) / PAGE_SIZE;
                        (
                            format!("./history/page_{}.msgpack", (*indx - 1) / PAGE_SIZE),
                            id * PAGE_SIZE,
                        )
                    } else {
                        let id = (*indx + 1) / PAGE_SIZE;
                        (
                            format!("./history/page_{}.msgpack", (*indx + 1) / PAGE_SIZE),
                            id * PAGE_SIZE,
                        )
                    };
                    if let Ok(bytes) = std::fs::read(to_get)
                        && let Ok(ser) = rmp_serde::from_slice(&bytes)
                    {
                        let _ = connections.get_mut(&from).unwrap().connection.send(
                            Message::MessagePage {
                                start_indx: idx,
                                contents: ser,
                            },
                        );
                    }
                }
                _ => {}
            }
            for (id, user) in &mut connections {
                if *id != from {
                    user.connection.send(msg.clone()).unwrap();
                }
            }
        }
        while state.messages.len() > PAGE_SIZE as usize {
            page.push(state.messages.pop_front().unwrap());
            println!("hit");
            if page.len() >= PAGE_SIZE as usize {
                std::fs::write(
                    format!(
                        "./history/page_{}.msgpack",
                        state.messages_start / PAGE_SIZE
                    ),
                    rmp_serde::to_vec(&page).unwrap(),
                )
                .unwrap();
                page.clear();
            }
        }
        if connections.is_empty() {
            break;
        }
    }
    serv.send(()).unwrap();
}

pub fn stdin_thread() -> BPipe<String> {
    let (con, con2) = BPipe::<String>::create();
    std::thread::spawn(move || {
        let mut lock = stdin().lock();
        let mut out = String::new();
        loop {
            let r = con2.recieve();
            match r {
                Ok(e) => {
                    if e.is_some() {
                        break;
                    }
                }
                Err(_) => {
                    break;
                }
            }

            let x = lock.read_line(&mut out);
            if x.is_err() {
                break;
            }
            con2.send(out.clone()).unwrap();
            out.clear();
        }
    });
    con
}
