use std::{
    collections::HashMap,
    error::Error,
    process::{Output, id},
    sync::{Arc, Mutex},
};

use raylib::{
    RaylibHandle, RaylibThread,
    color::Color,
    prelude::{RaylibDraw, RaylibTextureModeExt},
    texture::{Image, RenderTexture2D},
};
use serde::{Deserialize, Serialize};

use crate::{
    gui::{Bounds, GUI, Point, ScrollBoxData, TextBoxData},
    utils::{ObjectId, Stream, generate_id},
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
}

pub struct ClientState {
    pub con: Option<Stream<Event>>,
    pub gui_state: ClientGuiState,
    pub name_input: TextBoxData,
}

#[derive(Clone, Debug)]
pub struct ClientGuiState {
    pub id: UserId,
    pub state: BoardState,
    pub should_continue: bool,
    pub messages: Vec<Message>,
    pub image_scroll: ScrollBoxData,
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

#[derive(Clone, Debug)]
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
        let mut should_send_state = false;
        let mut new_images = Vec::new();
        if let Some(cn) = self.con.as_ref() {
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
                        self.gui_state.messages.push(Message {
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
        }
        let mut ns = self.gui_state.clone();
        let mut gui = GUI::new(&mut ns, handle, thread);
        gui.centered_horizontal(|gui| {
            let dim = 25;
            let sz = 1000 / dim;
            gui.container(300, |gui| {
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
            gui.container(300, |gui| {
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
        });
        gui.render(&mut ns);
        self.gui_state = ns;
        if let Some(x) = self.name_input.output() {
            self.gui_state.next_object_name = x;
        }
        Ok(())
    }

    pub async fn run(&mut self, handle: &mut RaylibHandle, thread: &RaylibThread) {
        while let Ok(_) = self.step(handle, thread).await {
            if !self.gui_state.should_continue {
                break;
            }
            if handle.window_should_close() {
                break;
            }
        }
    }
    pub async fn create_and_run(
        con: Option<Stream<Event>>,
        handle: &mut RaylibHandle,
        thread: &RaylibThread,
    ) {
        let mut slf = Self {
            con,
            gui_state: ClientGuiState {
                should_enumerate: true,
                id: UserId::new(),
                state: BoardState {
                    objects: HashMap::new(),
                    background_image: Arc::from("nyancat.png"),
                    people: HashMap::new(),
                    images: HashMap::new(),
                },
                should_continue: true,
                messages: Vec::new(),
                image_scroll: ScrollBoxData::new(),
                client_mode: ClientMode::SelectMode {
                    selected_object: ObjectId::new_invalid(),
                },
                next_object_name: String::new(),
            },
            name_input: TextBoxData::new(),
        };
        slf.load_image(handle, thread, "orc.png");
        slf.load_image(handle, thread, "nyancat.png");
        slf.run(handle, thread).await
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
