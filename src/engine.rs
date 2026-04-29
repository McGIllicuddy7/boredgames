use std::{
    collections::HashMap,
    error::Error,
    net::IpAddr,
    process::{Output, id},
    sync::{Arc, Mutex, atomic::AtomicBool},
};

use raylib::{
    RaylibHandle, RaylibThread,
    color::Color,
    prelude::{RaylibDraw, RaylibTextureModeExt},
    texture::{Image, RenderTexture2D},
};
use serde::{Deserialize, Serialize};
use tokio::{
    net::{TcpListener, TcpStream},
    task::yield_now,
};

use crate::{
    gui::{Bounds, GUI, Point, ScrollBoxData, TextBoxData},
    utils::{BPipe, BStream, ObjectId, SharedList, Stream, generate_id},
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TImage {
    pub width: i32,
    pub height: i32,
    pub values: Box<[Col]>,
    #[serde(skip)]
    pub texture: Option<Arc<std::sync::Mutex<RenderTexture2D>>>,
}

impl PartialEq for TImage {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width && self.height == other.height && self.values == other.values
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BoardState {
    pub objects: HashMap<ObjectId, Object>,
    pub background_image: Arc<str>,
    pub people: HashMap<UserId, Arc<str>>,
    pub images: HashMap<Arc<str>, TImage>,
    pub messages: Vec<Message>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Object {
    pub name: String,
    pub owner: UserId,
    pub id: ObjectId,
    pub bounds: Bounds,
    pub data: ObjectData,
}

#[derive(Serialize, Deserialize, Clone, Debug, Copy, PartialEq)]
pub struct Col {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ObjectData {
    Token {
        token_image_name: Arc<str>,
    },
    DrawingRectangle {
        tint: Col,
        rotation: f32,
        width: i32,
        height: i32,
    },
    DrawingCircle {
        tint: Col,
    },
    DrawingSpline {
        tint: Col,
        points: Vec<Point>,
        rotation: f32,
    },
    Text {
        text: String,
        height: i32,
    },
}

#[repr(transparent)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserId {
    id: ObjectId,
}

impl UserId {
    pub const fn is_valid(&self) -> bool {
        self.id.is_valid()
    }

    pub const fn is_invalid(&self) -> bool {
        self.id.is_invalid()
    }

    pub const fn new_invalid() -> Self {
        Self {
            id: ObjectId::new_invalid(),
        }
    }

    pub fn new() -> Self {
        Self {
            id: ObjectId::new(),
        }
    }
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Event {
    pub source: UserId,
    pub data: EventData,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum EventData {
    Message { contents: Arc<str> },
    RequestEntireState,
    EntireState { state: BoardState },
    UserConnected { name: Arc<str> },
    UserDisconnected,
    ObjectCreated { id: ObjectId, value: Object },
    ObjectDestroyed { id: ObjectId },
    ObjectUpdated { id: ObjectId, value: Object },
    KickRequest { to_kick: UserId },
    UploadImage { name: Arc<str>, data: TImage },
    SetBackgroundImage { to: Arc<str> },
}

pub struct ClientState {
    pub con: Option<BStream<Event>>,
    pub gui_state: ClientGuiState,
    pub name_input: TextBoxData,
}

#[derive(Clone, Debug)]
pub struct ClientGuiState {
    pub id: UserId,
    pub state: BoardState,
    pub user_name: Arc<str>,
    pub should_continue: bool,
    pub image_scroll: ScrollBoxData,
    pub message_scroll: ScrollBoxData,
    pub message_input: TextBoxData,
    pub user_scroll: ScrollBoxData,
    pub uname_input: TextBoxData,
    pub client_mode: ClientMode,
    pub next_object_name: String,
    pub should_enumerate: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    Background,
    Token,
    Foreground,
    Gm,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClientMode {
    SelectMode { selected_object: ObjectId },
    PlacingTokens { selected_image: Arc<str> },
    DrawingModeCircle { start: Point, object: ObjectId },
    DrawingModeRectangle { start: Point, object: ObjectId },
    ClientModeDrawingLine { start: Point, object: ObjectId },
}

impl ClientGuiState {
    pub fn next_name(&self) -> String {
        let base = self.next_object_name.clone();
        let mut act = base.clone();
        if self.should_enumerate && act.is_empty() {
            act = format!("1");
        }
        let mut count = 1;
        if self.should_enumerate {
            'outer: loop {
                for (_, i) in &self.state.objects {
                    if i.name == act {
                        act = format!("{}{}", base, count);
                        count += 1;
                        continue 'outer;
                    }
                }
                break 'outer;
            }
        }
        act
    }
}
impl ClientMode {
    pub fn name(&self) -> &'static str {
        match self {
            ClientMode::SelectMode { selected_object: _ } => "select mode",
            ClientMode::PlacingTokens { selected_image: _ } => "placement mode",
            ClientMode::DrawingModeCircle {
                start: _,
                object: _,
            } => "drawing circle mode",
            ClientMode::DrawingModeRectangle {
                start: _,
                object: _,
            } => "drawing rectangle mode",
            ClientMode::ClientModeDrawingLine {
                start: _,
                object: _,
            } => "drawing line mode",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub from: UserId,
    pub from_name: Arc<str>,
    pub contents: Arc<str>,
}
impl ClientState {
    pub async fn step(
        &mut self,
        handle: &mut RaylibHandle,
        thread: &RaylibThread,
    ) -> Result<(), Box<dyn Error>> {
        let mut ns = self.gui_state.clone();
        let mut gui = GUI::new(&mut ns, handle, thread);
        let update_stack: SharedList<ObjectId> = SharedList::new();
        let created_stack: SharedList<ObjectId> = SharedList::new();
        let created_stack_act = created_stack.clone();
        let update_stack_act = update_stack.clone();
        gui.centered_horizontal(|gui| {
            let dim = 25;
            let sz = 1000 / dim;
            gui.container(300, |gui| {
                gui.p1(format!("username:{}", ns.user_name));
                gui.p2("change user name");
                gui.text_input(&ns.uname_input, 16, 32);
                let ident = match &ns.client_mode {
                    ClientMode::SelectMode { selected_object: _ }
                    | ClientMode::ClientModeDrawingLine {
                        start: _,
                        object: _,
                    }
                    | ClientMode::DrawingModeCircle {
                        start: _,
                        object: _,
                    }
                    | ClientMode::PlacingTokens { selected_image: _ }
                    | ClientMode::DrawingModeRectangle {
                        start: _,
                        object: _,
                    } => "object name",
                };
                gui.p1(ident.to_string() + " entry");
                gui.text_input(&self.name_input, 16, 32);
                gui.p1(format!("object name:{}", ns.next_object_name));
                if ns.should_enumerate {
                    gui.button("stop enumeration", 16, |ns| {
                        ns.should_enumerate = false;
                    });
                } else {
                    gui.button("enumerate", 16, |ns| ns.should_enumerate = true);
                }
                gui.p2("Connected Users");
                gui.scroll_box(600, &ns.user_scroll, |gui| {
                    for i in &ns.state.people {
                        gui.p2(i.1);
                    }
                });
            });
            #[allow(unused)]
            gui.canvas(1000, 1000, move |bounds, state, cmds, handle, thread| {
                let mouse_pressed =
                    handle.is_mouse_button_pressed(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT);
                let mouse_released =
                    handle.is_mouse_button_released(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT);
                let mouse_down =
                    handle.is_mouse_button_down(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT);
                let mouse_pos = {
                    let mouse_pos = handle.get_mouse_position();
                    let rat = 1000. / bounds.width as f32;
                    let start_x = bounds.x;
                    let start_y = bounds.y;
                    let dx = mouse_pos.x - start_x as f32;
                    let dy = mouse_pos.y - start_y as f32;
                    let out = Point {
                        x: (dx * rat).round() as i32,
                        y: (dy * rat).round() as i32,
                    };
                    out
                };
                for i in 0..sz {
                    cmds.draw_line(i * dim, 0, i * dim, 1000, 1.0, Color::BLACK);
                }
                for i in 0..sz {
                    cmds.draw_line(0, i * dim, 1000, i * dim, 1.0, Color::BLACK);
                }
                let selected = match &state.client_mode {
                    ClientMode::SelectMode { selected_object } => selected_object.clone(),
                    ClientMode::ClientModeDrawingLine {
                        start: _,
                        object: _,
                    }
                    | ClientMode::DrawingModeCircle {
                        start: _,
                        object: _,
                    }
                    | ClientMode::DrawingModeRectangle {
                        start: _,
                        object: _,
                    }
                    | ClientMode::PlacingTokens { selected_image: _ } => ObjectId::new_invalid(),
                };
                for (id, obj) in &mut state.state.objects {
                    if *id == selected {
                        continue;
                    }
                    match &obj.data {
                        ObjectData::Token { token_image_name } => {
                            if let Some(img) = state.state.images.get(token_image_name) {
                                let img = img.texture.as_ref().unwrap();
                                cmds.draw_render_texture_scaled(
                                    img,
                                    obj.bounds.x * dim + dim / 2,
                                    obj.bounds.y * dim + dim / 2,
                                    obj.bounds.width * dim,
                                    obj.bounds.height * dim,
                                );
                            } else {
                                cmds.draw_rectangle(
                                    obj.bounds.x * dim + dim / 2,
                                    obj.bounds.y * dim + dim / 2,
                                    obj.bounds.width * dim,
                                    obj.bounds.height * dim,
                                    Color::RED,
                                );
                            }
                        }
                        ObjectData::DrawingRectangle {
                            tint,
                            rotation,
                            width,
                            height,
                        } => {
                            cmds.draw_rectangle(
                                obj.bounds.x * dim,
                                obj.bounds.y * dim,
                                obj.bounds.width * dim,
                                obj.bounds.height * dim,
                                Color {
                                    r: tint.r,
                                    g: tint.g,
                                    b: tint.b,
                                    a: tint.a,
                                },
                            );
                        }
                        ObjectData::DrawingCircle { tint } => {
                            cmds.draw_circle(
                                obj.bounds.x * dim + obj.bounds.width * dim / 2,
                                obj.bounds.y * dim + obj.bounds.width * dim / 2,
                                (obj.bounds.width as f32 / 2. * dim as f32),
                                Color {
                                    r: tint.r,
                                    g: tint.g,
                                    b: tint.b,
                                    a: tint.a,
                                },
                            );
                        }
                        ObjectData::DrawingSpline {
                            tint,
                            points,
                            rotation,
                        } => {
                            cmds.draw_lines(
                                points.clone(),
                                1.0,
                                Color {
                                    r: tint.r,
                                    g: tint.g,
                                    b: tint.b,
                                    a: tint.a,
                                },
                            );
                        }
                        ObjectData::Text { text, height } => {
                            cmds.draw_text(
                                (Arc::<str>::from(text.as_str())),
                                obj.bounds.x,
                                obj.bounds.y,
                                *height,
                                Color::BLACK,
                            );
                        }
                    }
                    if !obj.name.is_empty() {
                        cmds.draw_text(
                            obj.name.clone(),
                            obj.bounds.x * dim,
                            obj.bounds.y * dim + obj.bounds.height * dim + 5,
                            16,
                            Color::BLACK,
                        );
                    }
                }
                let mut nm = state.next_name();
                match &mut state.client_mode {
                    ClientMode::SelectMode { selected_object } => {
                        if let Some(g) = state.state.objects.get_mut(&selected_object) {
                            match &g.data {
                                ObjectData::Token { token_image_name } => {
                                    if let Some(img) = state.state.images.get(token_image_name) {
                                        let img = img.texture.as_ref().unwrap();
                                        cmds.draw_render_texture_scaled(
                                            img,
                                            mouse_pos.x - g.bounds.width / 2 * dim,
                                            mouse_pos.y - g.bounds.height / 2 * dim,
                                            g.bounds.width * dim,
                                            g.bounds.height * dim,
                                        );
                                    } else {
                                        cmds.draw_rectangle(
                                            mouse_pos.x - g.bounds.width / 2 * dim,
                                            mouse_pos.y - g.bounds.height / 2 * dim,
                                            g.bounds.width * dim,
                                            g.bounds.height * dim,
                                            Color::RED,
                                        );
                                    }
                                }
                                ObjectData::DrawingRectangle {
                                    tint,
                                    rotation,
                                    width,
                                    height,
                                } => {
                                    cmds.draw_rectangle(
                                        mouse_pos.x - g.bounds.width / 2 * dim,
                                        mouse_pos.y - g.bounds.height / 2 * dim,
                                        g.bounds.width * dim,
                                        g.bounds.height * dim,
                                        Color {
                                            r: tint.r,
                                            g: tint.g,
                                            b: tint.b,
                                            a: tint.a,
                                        },
                                    );
                                }
                                ObjectData::DrawingCircle { tint } => {
                                    cmds.draw_circle(
                                        mouse_pos.x,
                                        mouse_pos.y,
                                        (g.bounds.width as f32 / 2.) * dim as f32,
                                        Color {
                                            r: tint.r,
                                            g: tint.g,
                                            b: tint.b,
                                            a: tint.a,
                                        },
                                    );
                                }
                                ObjectData::DrawingSpline {
                                    tint,
                                    points,
                                    rotation,
                                } => {
                                    let offset_x =
                                        mouse_pos.x - g.bounds.x + (g.bounds.width / 2 * dim);
                                    let offset_y =
                                        mouse_pos.y - g.bounds.y + (g.bounds.height / 2 * dim);
                                    let mut p2 = points.clone();
                                    for i in &mut p2 {
                                        i.x += offset_x;
                                        i.y += offset_y;
                                    }
                                    cmds.draw_lines(
                                        p2,
                                        1.0,
                                        Color {
                                            r: tint.r,
                                            g: tint.g,
                                            b: tint.b,
                                            a: tint.a,
                                        },
                                    );
                                }
                                ObjectData::Text { text, height } => {
                                    cmds.draw_text(
                                        (Arc::<str>::from(text.as_str())),
                                        mouse_pos.x,
                                        mouse_pos.y,
                                        *height,
                                        Color::BLACK,
                                    );
                                }
                            }
                            if !g.name.is_empty() {
                                cmds.draw_text(
                                    g.name.clone(),
                                    mouse_pos.x,
                                    mouse_pos.y + g.bounds.height * dim,
                                    16,
                                    Color::BLACK,
                                );
                            }
                            if mouse_released {
                                let p0_x = mouse_pos.x / dim - g.bounds.width / 2;
                                let p0_y = mouse_pos.y / dim - g.bounds.width / 2;
                                g.bounds.x = p0_x;
                                g.bounds.y = p0_y;
                                update_stack.push_back(selected_object.clone());
                                *selected_object = ObjectId::new_invalid();
                            }
                        } else {
                            if mouse_pressed {
                                let mut new_selected = ObjectId::new_invalid();
                                for (id, i) in &state.state.objects {
                                    let bounds_act = Bounds {
                                        x: i.bounds.x * dim,
                                        y: i.bounds.y * dim,
                                        width: i.bounds.width * dim,
                                        height: i.bounds.height * dim,
                                    };
                                    if bounds_act.contains_point(mouse_pos) {
                                        new_selected = id.clone();
                                        break;
                                    }
                                }
                                *selected_object = new_selected;
                            }
                        }
                    }
                    ClientMode::PlacingTokens { selected_image } => {
                        if handle
                            .is_mouse_button_released(raylib::ffi::MouseButton::MOUSE_BUTTON_RIGHT)
                        {
                            state.client_mode = ClientMode::SelectMode {
                                selected_object: ObjectId::new_invalid(),
                            };
                        } else {
                            if mouse_released
                                && (Bounds {
                                    x: 0,
                                    y: 0,
                                    width: 1000,
                                    height: 1000,
                                })
                                .contains_point(mouse_pos)
                            {
                                let p0_x = mouse_pos.x / dim;
                                let p0_y = mouse_pos.y / dim;
                                let id = ObjectId::new();
                                let obj = Object {
                                    name: nm,
                                    owner: state.id.clone(),
                                    id: id.clone(),
                                    bounds: Bounds {
                                        x: p0_x,
                                        y: p0_y,
                                        width: 1,
                                        height: 1,
                                    },
                                    data: ObjectData::Token {
                                        token_image_name: selected_image.clone(),
                                    },
                                };
                                created_stack.push_back(id.clone());
                                state.state.objects.insert(id, obj);
                            }
                        }
                    }
                    ClientMode::DrawingModeCircle { start, object }
                    | ClientMode::DrawingModeRectangle { start, object } => {
                        let mut obj = state.state.objects.get_mut(&*object).unwrap();
                        let mut dx = mouse_pos.x / dim - obj.bounds.x;
                        let mut dy = mouse_pos.y / dim - obj.bounds.y;
                        if dx < 0 {
                            obj.bounds.x += dx;
                            obj.bounds.width = -dx;
                        }
                        if dy < 0 {
                            obj.bounds.y += dy;
                            obj.bounds.height = dy;
                        }
                        if dx >= 0 {
                            obj.bounds.width = dx;
                        }
                        if dy >= 0 {
                            obj.bounds.height = dy;
                        }
                        if obj.bounds.width < 1 {
                            obj.bounds.width = 1;
                        }
                        if obj.bounds.height < 1 {
                            obj.bounds.height = 1;
                        }
                        if mouse_released {
                            created_stack.push_back(object.clone());
                            state.client_mode = ClientMode::SelectMode {
                                selected_object: ObjectId::new_invalid(),
                            };
                        }
                    }
                    ClientMode::ClientModeDrawingLine { start, object } => {
                        let mut obj = state.state.objects.get_mut(&*object).unwrap();
                        match &mut obj.data {
                            ObjectData::DrawingSpline {
                                tint,
                                points,
                                rotation,
                            } => {
                                points.push(mouse_pos);
                            }
                            _ => {
                                unreachable!()
                            }
                        }
                        if mouse_released {
                            created_stack.push_back(object.clone());
                            state.client_mode = ClientMode::SelectMode {
                                selected_object: ObjectId::new_invalid(),
                            };
                        }
                    }
                }
                if mouse_pos.x >= 0
                    && mouse_pos.y >= 0
                    && mouse_pos.x <= 1000
                    && mouse_pos.y <= 1000
                {
                    if handle.is_key_down(raylib::ffi::KeyboardKey::KEY_C) && mouse_pressed {
                        let id = generate_id();
                        let nm = state.next_name();
                        let mut object = Object {
                            name: nm,
                            owner: state.id.clone(),
                            id: id.clone(),
                            bounds: Bounds {
                                x: mouse_pos.x / dim,
                                y: mouse_pos.y / dim,
                                width: 1,
                                height: 1,
                            },
                            data: ObjectData::DrawingCircle {
                                tint: Col {
                                    r: 255,
                                    g: 32,
                                    b: 32,
                                    a: 255,
                                },
                            },
                        };
                        state.state.objects.insert(id.clone(), object);
                        state.client_mode = ClientMode::DrawingModeCircle {
                            start: mouse_pos,
                            object: id,
                        };
                    }
                    if handle.is_key_down(raylib::ffi::KeyboardKey::KEY_R) && mouse_pressed {
                        let id = generate_id();
                        let nm = state.next_name();
                        let mut object = Object {
                            name: nm,
                            owner: state.id.clone(),
                            id: id.clone(),
                            bounds: Bounds {
                                x: mouse_pos.x / dim,
                                y: mouse_pos.y / dim,
                                width: 1,
                                height: 1,
                            },
                            data: ObjectData::DrawingRectangle {
                                rotation: 0.,
                                width: 1,
                                height: 1,
                                tint: Col {
                                    r: 255,
                                    g: 32,
                                    b: 32,
                                    a: 255,
                                },
                            },
                        };
                        state.state.objects.insert(id.clone(), object);
                        state.client_mode = ClientMode::DrawingModeRectangle {
                            start: mouse_pos,
                            object: id,
                        };
                    }
                    if handle.is_key_down(raylib::ffi::KeyboardKey::KEY_S) && mouse_pressed {
                        let id = generate_id();
                        let nm = state.next_name();
                        let mut object = Object {
                            name: nm,
                            owner: state.id.clone(),
                            id: id.clone(),
                            bounds: Bounds {
                                x: mouse_pos.x / dim,
                                y: mouse_pos.y / dim,
                                width: 1,
                                height: 1,
                            },
                            data: ObjectData::DrawingSpline {
                                tint: Col {
                                    r: 255,
                                    g: 32,
                                    b: 32,
                                    a: 255,
                                },
                                points: vec![mouse_pos],
                                rotation: 0.0,
                            },
                        };
                        state.state.objects.insert(id.clone(), object);
                        state.client_mode = ClientMode::ClientModeDrawingLine {
                            start: mouse_pos,
                            object: id,
                        }
                    }
                }
            });
            gui.container(250, |gui| {
                gui.p1(format!("{:#?}", ns.client_mode.name()));
                gui.button("deselect_image", 16, |state| {
                    state.client_mode = ClientMode::SelectMode {
                        selected_object: ObjectId::new_invalid(),
                    };
                });
                gui.scroll_box(900, &self.gui_state.image_scroll, |gui| {
                    for (name, _img) in &self.gui_state.state.images {
                        let n2 = name.clone();
                        gui.button_1(&name, move |state| {
                            state.client_mode = ClientMode::PlacingTokens {
                                selected_image: n2.clone(),
                            };
                        });
                    }
                });
            });
            gui.container(250, |gui| {
                gui.button_1("exit", |state| {
                    state.should_continue = false;
                });
                gui.scroll_box_rev(600, &ns.message_scroll, |gui| {
                    for i in &ns.state.messages {
                        gui.p2(format!("{}:{}", i.from_name, i.contents));
                    }
                });
                gui.text_input(&ns.message_input, 16, 40);
            });
        });
        gui.render(&mut ns);
        self.gui_state = ns;
        let mut events = Vec::new();
        if let Some(x) = self.name_input.output() {
            self.gui_state.next_object_name = x;
        }
        while let Some(x) = created_stack_act.pop_front() {
            let Some(v) = self.gui_state.state.objects.get(&x) else {
                events.push(Event {
                    source: self.gui_state.id.clone(),
                    data: EventData::ObjectDestroyed { id: x },
                });
                continue;
            };
            events.push(Event {
                source: self.gui_state.id.clone(),
                data: EventData::ObjectCreated {
                    id: x,
                    value: v.clone(),
                },
            });
            continue;
        }
        while let Some(x) = update_stack_act.pop_front() {
            let Some(v) = self.gui_state.state.objects.get(&x) else {
                events.push(Event {
                    source: self.gui_state.id.clone(),
                    data: EventData::ObjectDestroyed { id: x },
                });
                continue;
            };
            events.push(Event {
                source: self.gui_state.id.clone(),
                data: EventData::ObjectUpdated {
                    id: x,
                    value: v.clone(),
                },
            });
            continue;
        }
        if let Some(x) = self.gui_state.message_input.output() {
            let msg = Message {
                from: self.gui_state.id.clone(),
                from_name: self.gui_state.user_name.clone(),
                contents: x.into(),
            };
            self.gui_state.state.messages.push(msg.clone());
            events.push(Event {
                source: self.gui_state.id.clone(),
                data: EventData::Message {
                    contents: msg.contents.clone(),
                },
            });
        }
        if let Some(x) = self.gui_state.uname_input.output() {
            self.gui_state.user_name = x.into();
            self.gui_state
                .state
                .people
                .insert(self.gui_state.id.clone(), self.gui_state.user_name.clone());
            events.push(Event {
                source: self.gui_state.id.clone(),
                data: EventData::UserConnected {
                    name: self.gui_state.user_name.clone(),
                },
            });
        }
        self.handle_network(events, handle, thread).await?;
        yield_now().await;
        Ok(())
    }

    pub async fn handle_network(
        &mut self,
        events: Vec<Event>,
        handle: &mut RaylibHandle,
        thread: &RaylibThread,
    ) -> Result<(), Box<dyn Error>> {
        let Some(cn) = self.con.as_mut() else {
            return Ok(());
        };
        for i in events {
            cn.send(&i).await?;
        }
        let mut should_send_state = false;
        let mut new_images = Vec::new();
        while let Some(i) = cn.try_receive().await? {
            let from_name = match self.gui_state.state.people.get(&i.source) {
                Some(x) => x.clone(),
                None => "INVALID USER".into(),
            };
            match i.data {
                EventData::EntireState { state } => {
                    self.gui_state.state = state;
                    for (i, _) in &self.gui_state.state.images {
                        new_images.push(i.clone());
                    }
                }
                EventData::Message { contents } => {
                    self.gui_state.state.messages.push(Message {
                        from: i.source.clone(),
                        from_name,
                        contents,
                    });
                }
                EventData::RequestEntireState => {
                    should_send_state = true;
                }
                EventData::UserConnected { name } => {
                    self.gui_state.state.people.insert(i.source.clone(), name);
                }
                EventData::UserDisconnected => {
                    self.gui_state.state.people.remove(&i.source);
                }
                EventData::ObjectCreated { id, value } => {
                    self.gui_state.state.objects.insert(id, value);
                }
                EventData::ObjectDestroyed { id } => {
                    self.gui_state.state.objects.remove(&id);
                }
                EventData::ObjectUpdated { id, value } => {
                    self.gui_state.state.objects.insert(id, value);
                }
                EventData::KickRequest { to_kick } => {
                    if to_kick == self.gui_state.id {
                        self.gui_state.should_continue = false;
                    }
                }
                EventData::UploadImage { name, data } => {
                    new_images.push(name.clone());
                    self.gui_state.state.images.insert(name, data);
                }
                EventData::SetBackgroundImage { to } => {
                    self.gui_state.state.background_image = to;
                }
            }
        }
        for i in new_images {
            let g = self.gui_state.state.images.get_mut(&i).unwrap();
            let mut img = handle
                .load_render_texture(&thread, g.width as u32, g.height as u32)
                .unwrap();
            let mut draw = handle.begin_texture_mode(&thread, &mut img);
            for y in 0..g.height {
                for x in 0..g.width {
                    let tmp = g.values[(y * g.width + x) as usize];
                    draw.draw_pixel(
                        x,
                        y,
                        Color {
                            r: tmp.r,
                            g: tmp.g,
                            b: tmp.b,
                            a: tmp.a,
                        },
                    );
                }
            }
            drop(draw);
            g.texture = Some(Arc::new(Mutex::new(img)));
        }
        if should_send_state {
            cn.send(&Event {
                source: self.gui_state.id.clone(),
                data: EventData::EntireState {
                    state: self.gui_state.state.clone(),
                },
            })
            .await?;
        }
        Ok(())
    }

    pub async fn run(
        &mut self,
        handle: &mut RaylibHandle,
        thread: &RaylibThread,
    ) -> Result<(), Box<dyn Error>> {
        loop {
            self.step(handle, thread).await?;
            if !self.gui_state.should_continue {
                println!("should not continue");
                break;
            }
            if handle.window_should_close() {
                println!("window_should_close");
                break;
            }
        }
        println!("done");
        Ok(())
    }

    pub async fn create_and_run(
        con: Option<BStream<Event>>,
        username: Arc<str>,
        handle: &mut RaylibHandle,
        thread: &RaylibThread,
    ) -> Result<(), Box<dyn Error>> {
        let id = UserId::new();
        let mut istate = BoardState {
            objects: HashMap::new(),
            background_image: Arc::from("nyancat.png"),
            people: HashMap::new(),
            messages: Vec::new(),
            images: HashMap::new(),
        };
        if let Some(con) = con.as_ref() {
            con.send(&Event {
                source: id.clone(),
                data: EventData::UserConnected {
                    name: username.clone(),
                },
            })
            .await?;
            let data = con.receive().await?;
            match data.data {
                EventData::EntireState { state } => {
                    istate = state;
                }
                _ => {
                    return Err("failed".into());
                }
            }
        };
        let mut slf = Self {
            con,
            gui_state: ClientGuiState {
                user_name: username.clone(),
                should_enumerate: true,
                id,
                state: istate,
                should_continue: true,
                image_scroll: ScrollBoxData::new(),
                message_scroll: ScrollBoxData::new(),
                message_input: TextBoxData::new(),
                uname_input: TextBoxData::new(),
                client_mode: ClientMode::SelectMode {
                    selected_object: ObjectId::new_invalid(),
                },
                next_object_name: String::new(),
                user_scroll: ScrollBoxData::new(),
            },
            name_input: TextBoxData::new(),
        };

        slf.load_image(handle, thread, "orc.png");
        slf.load_image(handle, thread, "nyancat.png");
        slf.run(handle, thread).await?;
        Ok(())
    }

    pub fn load_image(&mut self, handle: &mut RaylibHandle, thread: &RaylibThread, name: &str) {
        let mut img = Image::load_image(name).unwrap();
        let mut tmp = vec![
            Col {
                r: 0,
                g: 0,
                b: 0,
                a: 255
            };
            (img.height() * img.width()) as usize
        ]
        .into_boxed_slice();
        for y in 0..img.height() {
            for x in 0..img.width() {
                let t2 = img.get_color(x, y);
                tmp[(y * img.width() + x) as usize] = Col {
                    r: t2.r,
                    g: t2.g,
                    b: t2.b,
                    a: t2.a,
                };
            }
        }
        let mut g = TImage {
            width: img.width(),
            height: img.height(),
            values: tmp,
            texture: None,
        };
        let mut img = handle
            .load_render_texture(&thread, g.width as u32, g.height as u32)
            .unwrap();
        let mut draw = handle.begin_texture_mode(&thread, &mut img);
        for y in 0..g.height {
            for x in 0..g.width {
                let tmp = g.values[(y * g.width + x) as usize];
                draw.draw_pixel(
                    x,
                    y,
                    Color {
                        r: tmp.r,
                        g: tmp.g,
                        b: tmp.b,
                        a: tmp.a,
                    },
                );
            }
        }
        drop(draw);
        g.texture = Some(Arc::new(Mutex::new(img)));
        self.gui_state.state.images.insert(name.into(), g);
    }
}

pub struct WelcomeState {
    pub last_error_message: String,
    pub should_exit: bool,
    pub should_join: bool,
    pub should_host: bool,
    pub should_host_local: bool,
    pub user_name: String,
    pub user_name_data: TextBoxData,
    pub to_join: String,
    pub to_join_data: TextBoxData,
}

pub async fn game_loop(handle: &mut RaylibHandle, thread: &RaylibThread) {
    let mut state = WelcomeState {
        last_error_message: String::new(),
        should_exit: false,
        should_host: false,
        should_join: false,
        should_host_local: false,
        user_name: String::new(),
        user_name_data: TextBoxData::new(),
        to_join: String::new(),
        to_join_data: TextBoxData::new(),
    };
    while !state.should_exit {
        let mut gui = GUI::new(&state, handle, thread);
        gui.centered_horizontal(|gui| {
            state.should_exit = false;
            state.should_host = false;
            state.should_join = false;
            state.should_host_local = false;
            gui.container(300, |gui| {
                if !state.last_error_message.is_empty() {
                    gui.p1(format!("Error:{}", state.last_error_message));
                    gui.button("dismiss", 12, |state| {
                        state.last_error_message.clear();
                    });
                }
                gui.p1(format!("User Name:{}", state.user_name));
                gui.p1("Edit User Name");
                gui.text_input(&state.user_name_data, 16, 32);
            });
            gui.container(900, |gui| {
                gui.h1("WELCOME TO BORED GAMES!");
                gui.h4("Press any key to continue");
                gui.h1(" ");
                gui.button_1("host", |state| {
                    state.should_host = true;
                });
                gui.button_1("connect", |state| {
                    state.should_join = true;
                });
                gui.button_1("run locally", |state| {
                    state.should_host_local = true;
                });
                gui.button_1("exit", |state| {
                    state.should_exit = true;
                });
                gui.h1(" ");
            });
            gui.container(400, |gui| {
                let adr = address().0;
                gui.p1(format!("local address:{}", adr));
                gui.p1(format!("current address to connect to:{}", state.to_join));
                gui.p1("Edit Target Address");
                gui.text_input(&state.to_join_data, 16, 32);
                gui.button("set target to local", 16, move |state| {
                    state.to_join = adr.clone();
                });
            });
        });
        gui.render(&mut state);
        if handle.window_should_close() {
            break;
        }
        if let Some(to_join) = state.to_join_data.output() {
            state.to_join = to_join;
        }
        if let Some(uname) = state.user_name_data.output() {
            state.user_name = uname;
        }
        if state.should_host {
            if let Err(e) = game_host(handle, &thread, &state).await {
                state.last_error_message = e.to_string();
                println!("{:#?}", e);
            }
        } else if state.should_join {
            if state.user_name.is_empty() {
                state.last_error_message = "Error Must have a non empty username".into();
            } else {
                if let Err(e) = game_join(handle, thread, &state).await {
                    state.last_error_message = e.to_string();
                    println!("{:#?}", e);
                }
            }
        } else if state.should_host_local {
            if let Err(e) =
                ClientState::create_and_run(None, state.user_name.clone().into(), handle, thread)
                    .await
            {
                state.last_error_message = e.to_string();
            }
        }
        if handle.window_should_close() {
            break;
        }
    }
}

pub const PORT: u16 = 4242;
pub static SERVER_SETUP: AtomicBool = AtomicBool::new(false);
pub static SERVER_ERRORED: AtomicBool = AtomicBool::new(false);
pub static SERVER_SHOULD_CLOSE: AtomicBool = AtomicBool::new(false);
pub async fn game_host(
    handle: &mut RaylibHandle,
    thread: &RaylibThread,
    state: &WelcomeState,
) -> Result<(), Box<dyn Error>> {
    SERVER_ERRORED.store(false, std::sync::atomic::Ordering::SeqCst);
    let (st1, st2) = BPipe::create();
    let bst1 = BStream::from_pipe(st1);
    let bst2 = BStream::from_pipe(st2);
    tokio::task::spawn(run_server(bst1));
    while !SERVER_SETUP.load(std::sync::atomic::Ordering::SeqCst) {
        if SERVER_ERRORED.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("server errored".into());
        }
        tokio::task::yield_now().await;
    }
    ClientState::create_and_run(Some(bst2), state.user_name.clone().into(), handle, thread).await?;
    SERVER_SHOULD_CLOSE.store(true, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

pub async fn game_join(
    handle: &mut RaylibHandle,
    thread: &RaylibThread,
    state: &WelcomeState,
) -> Result<(), Box<dyn Error>> {
    println!("connecting to:{}", state.to_join);
    let strm = TcpStream::connect((state.to_join.clone(), PORT)).await?;
    ClientState::create_and_run(
        Some(BStream::from_stream(Stream::new(strm))),
        state.user_name.clone().into(),
        handle,
        thread,
    )
    .await?;
    Ok(())
}

pub async fn run_server(bst: BStream<Event>) {
    if let Err(e) = server_loop(Some(bst)).await {
        println!("errored:{:#?}", e.to_string());
        SERVER_ERRORED.store(true, std::sync::atomic::Ordering::SeqCst);
    };
}

pub fn address() -> (String, u16) {
    (local_ip::get().unwrap().to_string(), PORT)
    //("127.0.0.1".to_string(), PORT)
}

pub async fn server_loop(host: Option<BStream<Event>>) -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(address()).await?;
    let (connections, con) = BPipe::create();
    let (done, done0) = BPipe::create();
    let handle = tokio::task::spawn(socket_loop(listener, con, done0));
    let mut game_state = BoardState {
        messages: Vec::new(),
        objects: HashMap::new(),
        background_image: Arc::from("nyancat.png"),
        people: HashMap::new(),
        images: HashMap::new(),
    };
    let mut thost = host;
    let mut people: HashMap<UserId, BStream<Event>> = HashMap::new();
    while !SERVER_SHOULD_CLOSE.load(std::sync::atomic::Ordering::SeqCst) {
        let mut events: Vec<Event> = Vec::new();
        let mut new_connections = Vec::new();
        if let Some(bst) = thost.take() {
            let Ok(tmp) = bst.receive().await else {
                continue;
            };
            let id = tmp.source.clone();
            events.push(tmp);
            new_connections.push(id.clone());
            people.insert(id, bst);
        }
        while let Some(i) = connections.try_receive() {
            let bst: Stream<Event> = Stream::new(i);
            let Ok(tmp) = bst.receive().await else {
                continue;
            };
            let id = tmp.source.clone();
            events.push(tmp);
            new_connections.push(id.clone());
            people.insert(id, BStream::from_stream(bst));
        }
        for (_, j) in &people {
            while let Some(n) = j.try_receive().await? {
                events.push(n);
            }
        }
        for i in events {
            match &i.data {
                EventData::Message { contents } => {
                    let nm = if let Some(name) = game_state.people.get(&i.source) {
                        name.clone().into()
                    } else {
                        "invalid name".into()
                    };
                    game_state.messages.push(Message {
                        from: i.source.clone(),
                        from_name: nm,
                        contents: contents.clone(),
                    });
                }
                EventData::RequestEntireState => {
                    if let Some(x) = people.get(&i.source) {
                        let _ = x
                            .send(&Event {
                                source: UserId::new_invalid(),
                                data: EventData::EntireState {
                                    state: game_state.clone(),
                                },
                            })
                            .await;
                    }
                    continue;
                }
                EventData::EntireState { state } => {
                    game_state = state.clone();
                }
                EventData::UserConnected { name } => {
                    game_state.people.insert(i.source.clone(), name.clone());
                }
                EventData::UserDisconnected => {
                    game_state.people.remove(&i.source);
                    people.remove(&i.source);
                }
                EventData::ObjectCreated { id, value } => {
                    game_state.objects.insert(id.clone(), value.clone());
                }
                EventData::ObjectDestroyed { id } => {
                    game_state.objects.remove(&id);
                }
                EventData::ObjectUpdated { id, value } => {
                    game_state.objects.insert(id.clone(), value.clone());
                }
                EventData::KickRequest { to_kick } => {
                    _ = to_kick;
                }
                EventData::UploadImage { name, data } => {
                    game_state.images.insert(name.clone(), data.clone());
                }
                EventData::SetBackgroundImage { to } => {
                    game_state.background_image = to.clone();
                }
            }
            for (id, st) in &people {
                if i.source != *id {
                    let _ = st.send(&i).await;
                }
            }
        }
        for i in new_connections {
            let Some(bst) = people.get(&i) else {
                continue;
            };
            let _ = bst
                .send(&Event {
                    source: UserId::new_invalid(),
                    data: EventData::EntireState {
                        state: game_state.clone(),
                    },
                })
                .await;
        }
    }
    println!("done");
    done.send(());
    handle.await?;
    Ok(())
}

pub async fn socket_loop(listener: TcpListener, stream: BPipe<TcpStream>, done: BPipe<()>) {
    while !SERVER_SHOULD_CLOSE.load(std::sync::atomic::Ordering::SeqCst) {
        let acc = listener.accept();
        SERVER_SETUP.store(true, std::sync::atomic::Ordering::SeqCst);
        let dne = done.receive();
        let output;
        tokio::select! {
            val = acc => {
                output = val;
            }
            _ = dne => {
                break;
            }
        };
        if let Ok(x) = output {
            stream.send(x.0);
        } else {
            println!("failed somehow");
        }
    }
}
