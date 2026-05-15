use std::{
    collections::{BTreeMap, HashMap},
    error::Error,
    sync::{Arc, Mutex},
};

use raylib::{
    RaylibHandle, RaylibThread,
    color::Color,
    drawing::RaylibDraw,
    math::Vector2,
    texture::{RenderTexture2D, Texture2D},
};
use serde::{Deserialize, Serialize};

use crate::{
    engine::Col,
    libgui::{Bounds, GUI, Point, ScrollBoxData},
};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DrawObject {
    pub bounds: Bounds,
    pub image_name: Arc<str>,
    pub rotation: f32,
}
#[derive(Clone, Debug)]
pub struct Layer {
    pub values: Arc<Mutex<RenderTexture2D>>,
    pub text_blocks: Vec<(i32, i32, String)>,
    pub widgets: Vec<DrawObject>,
}

#[derive(Clone, Debug)]
pub struct Drawing {
    pub name: String,
    pub layers: [Layer; 16],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectLayer {
    pub values: Box<[Col]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub layers: [ProjectLayer; 16],
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq)]
pub enum DrawingStateMode {
    SelectMode,
    DrawMode,
    EraseMode,
    PlacementMode,
}
pub struct DrawingState {
    pub drawing: Drawing,
    pub done: bool,
    pub current_layer: usize,
    pub mode: DrawingStateMode,
    pub token_table: BTreeMap<Arc<str>, Arc<Texture2D>>,
    pub token_scroll: ScrollBoxData,
    pub token_to_place: Arc<str>,
    pub selected_object: Option<usize>,
    pub texture_to_draw: Arc<str>,
    pub texture_table: BTreeMap<Arc<str>, DrawingTexture>,
    pub texture_scroll: ScrollBoxData,
    pub radius: i32,
}

pub struct DrawingTexture {
    pub name: Arc<str>,
    pub preview: Arc<Texture2D>,
    pub shader: Arc<dyn Fn(i32, i32) -> Color>,
}
impl Layer {
    pub fn new(handle: &mut RaylibHandle, thread: &RaylibThread) -> Self {
        Self {
            values: Arc::new(Mutex::new(
                handle.load_render_texture(thread, 1024, 1024).unwrap(),
            )),
            text_blocks: Vec::new(),
            widgets: Vec::new(),
        }
    }
}

pub fn lerp(p0: f32, p1: f32, amount: f32) -> f32 {
    p0 * (1. - amount) + p1 * amount
}
pub fn noise_1d(x: i32, y: i32, scale: f32, rsyn: &str) -> f32 {
    let scale = scale / 10.0;
    fn pos_vector(x: i32, y: i32, scale: f32, rsyn: &str) -> Vector2 {
        static MAP: Mutex<Option<HashMap<(i32, i32, i32, Arc<str>), f32>>> = Mutex::new(None);
        let mut g = MAP.lock().unwrap();
        if g.is_none() {
            *g = Some(HashMap::new());
        };
        let map = g.as_mut().unwrap();
        let s: Arc<str> = rsyn.into();
        let ist = (x, y, scale as i32, s);
        if let Some(theta) = map.get(&ist) {
            Vector2::new(theta.cos(), theta.sin())
        } else {
            let theta_0 = rand::random::<u32>() % 62_831;
            let theta = theta_0 as f32 / 10_000. ;
            map.insert(ist, theta);
            Vector2::new(theta.cos(), theta.sin())
        }
    }
    let sx = x as f32 * scale;
    let sy = y as f32 * scale;
    let bx = sx.floor() as i32;
    let by = sy.floor() as i32;
    let dx = sx - bx as f32;
    let dy = sy - by as f32;
    let point = Vector2::new(dx, dy);
    let x0_y0 = (point - Vector2::new(0.0, 0.0)).dot(pos_vector(bx, by, scale, rsyn));
    let x1_y0 = (point - Vector2::new(1.0, 0.0)).dot(pos_vector(bx + 1, by, scale, rsyn));
    let x0_y1 = (point - Vector2::new(0.0, 1.0)).dot(pos_vector(bx, by + 1, scale, rsyn));
    let x1_y1 = (point - Vector2::new(1.0, 1.0)).dot(pos_vector(bx + 1, by + 1, scale, rsyn));
    let y_0s = lerp(x0_y0, x1_y0, dx);
    let y_1s = lerp(x0_y1, x1_y1, dx);
    let output = lerp(y_0s, y_1s, dy);
    (output + 1.) / 2.0
}

pub fn noise_3d(x: i32, y: i32, scale: f32) -> Color {
    let r = noise_1d(x, y, scale, "r");
    let g = noise_1d(x, y, scale, "g");
    let b = noise_1d(x, y, scale, "b");
    Color {
        r: (r * 255.0) as u8,
        g: (g * 255.0) as u8,
        b: (b * 255.0) as u8,
        a: 255,
    }
}

pub fn noise_1d_layered(x: i32, y: i32, scale: f32, rsyn: &str, layers: i32) -> f32 {
    let mut out = 0.0;
    let mut div = 1.0;
    let mut total = 0.0;
    for _ in 0..layers {
        out += noise_1d(x, y, scale * div, rsyn) / div;
        total += 1. / div;
        div *= 2.;
    }
    out / total
}

pub fn noise_3d_layered(x: i32, y: i32, scale: f32, layers: i32) -> Color {
    let r = noise_1d_layered(x, y, scale, "r", layers) * 255.;
    let g = noise_1d_layered(x, y, scale, "g", layers) * 255.;
    let b = noise_1d_layered(x, y, scale, "b", layers) * 255.;
    Color {
        r: r as u8,
        g: g as u8,
        b: b as u8,
        a: 255,
    }
}

pub fn blend(color0: Color, color1: Color, amount: f32) -> Color {
    let r = color0.r as f32 * (1. - amount) + color1.r as f32 * amount;
    let g = color0.g as f32 * (1. - amount) + color1.g as f32 * amount;
    let b = color0.b as f32 * (1. - amount) + color1.b as f32 * amount;
    let a = color0.a as f32 * (1. - amount) + color1.a as f32 * amount;
    Color {
        r: r as u8,
        g: g as u8,
        b: b as u8,
        a: a as u8,
    }
}

pub fn blend_hsv(color0: Color, color1: Color, amount: f32) -> Color {
    let c0_hsv = color0.color_to_hsv();
    let c1_hsv = color1.color_to_hsv();
    let l = c0_hsv * (1. - amount) + c1_hsv * amount;
    
    Color::color_from_hsv(l.x, l.y, l.z)
}

pub fn blend_3_way(color0: Color, color1: Color, color2: Color, amount: f32) -> Color {
    if amount < 0.5 {
        let v = amount * 2.;
        blend(color0, color1, v)
    } else {
        let v = amount * 2. - 1.;
        blend(color1, color2, v)
    }
}

pub fn blend_3_hsv(color0: Color, color1: Color, color2: Color, amount: f32) -> Color {
    if amount < 0.5 {
        let v = amount * 2.;
        blend_hsv(color0, color1, v)
    } else {
        let v = amount * 2. - 1.;
        blend_hsv(color1, color2, v)
    }
}

pub fn from_grayscale(v: f32) -> Color {
    Color {
        r: (v * 255.) as u8,
        g: (v * 255.) as u8,
        b: (v * 255.) as u8,
        a: 255,
    }
}
pub fn from_rgb(r: f32, g: f32, b: f32) -> Color {
    Color {
        r: (r * 255.) as u8,
        g: (g * 255.) as u8,
        b: (b * 255.) as u8,
        a: 255,
    }
}

pub fn burn(v: f32, thresh: f32) -> f32 {
    if (v - thresh).abs() < 0.1 {
        0.5
    } else if v < thresh {
        0.0
    } else {
        1.0
    }
}
impl DrawingTexture {
    pub fn new(
        name: &str,
        handle: &mut RaylibHandle,
        thread: &RaylibThread,
        kernel: impl Fn(i32, i32) -> Color + 'static,
    ) -> Self {
        let mut preview = raylib::prelude::Image::gen_image_color(25, 4, Color::WHITE);
        for i in 0..4 {
            for j in 0..25 {
                preview.draw_pixel(j, i, kernel(j, i));
            }
        }
        let tex = handle.load_texture_from_image(thread, &preview).unwrap();
        Self {
            name: name.into(),
            preview: Arc::new(tex),
            shader: Arc::new(kernel),
        }
    }
}
pub fn load_token_table(
    _handle: &mut RaylibHandle,
    _thread: &RaylibThread,
) -> BTreeMap<Arc<str>, Arc<Texture2D>> {
    BTreeMap::new()
}

pub fn load_textures(
    handle: &mut RaylibHandle,
    thread: &RaylibThread,
    list: Vec<(&str, Box<dyn Fn(i32, i32) -> Color>)>,
) -> BTreeMap<Arc<str>, DrawingTexture> {
    let mut out = BTreeMap::new();
    for i in list {
        out.insert(i.0.into(), DrawingTexture::new(i.0, handle, thread, i.1));
    }
    out
}
pub fn load_texture_table(
    handle: &mut RaylibHandle,
    thread: &RaylibThread,
) -> BTreeMap<Arc<str>, DrawingTexture> {
    load_textures(
        handle,
        thread,
        vec![
            (
                "noise_1d",
                Box::new(|x, y| from_grayscale(noise_1d_layered(x, y, 0.1, "noise_1d", 5))),
            ),
            ("noise_3d", Box::new(|x, y| noise_3d_layered(x, y, 0.1, 5))),
            (
                "grass",
                Box::new(|x, y| {
                    blend(
                        Color {
                            r: 32,
                            g: 64,
                            b: 25,
                            a: 255,
                        },
                        Color {
                            r: 25,
                            g: 156 / 2,
                            b: 10,
                            a: 255,
                        },
                        burn(
                            noise_1d_layered(x, y, 0.2, "grass", 5) * 0.9
                                + noise_1d_layered(x, y, 0.8, "grass", 5) * 0.1,
                            0.4,
                        ),
                    )
                }),
            ),
            (
                "dirt",
                Box::new(|x, y| {
                    blend(
                        Color {
                            r: 64,
                            g: 50,
                            b: 20,
                            a: 255,
                        },
                        Color {
                            r: 50,
                            g: 50,
                            b: 20,
                            a: 255,
                        },
                        burn(
                            noise_1d_layered(x, y, 0.2, "grass", 5) * 0.9
                                + noise_1d_layered(x, y, 0.8, "grass", 5) * 0.1,
                            0.4,
                        ),
                    )
                }),
            ),
            (
                "water",
                Box::new(|x, y| {
                    blend_3_way(
                        Color {
                            r: 0,
                            g: 50,
                            b: 100,
                            a: 255,
                        },
                        Color {
                            r: 0,
                            g: 50,
                            b: 75,
                            a: 255,
                        },
                        Color {
                            r: 0,
                            g: 30,
                            b: 50,
                            a: 255,
                        },
                        noise_1d_layered(x, y, 1.0, "water", 5) * 0.9
                            + noise_1d_layered(x, y, 1.5, "water", 5) * 0.1,
                    )
                }),
            ),
            (
                "sand",
                Box::new(|x, y| {
                    blend_3_way(
                        Color {
                            r: 220,
                            g: 220,
                            b: 180,
                            a: 255,
                        },
                        Color {
                            r: 200,
                            g: 200,
                            b: 150,
                            a: 255,
                        },
                        Color {
                            r: 150,
                            g: 150,
                            b: 100,
                            a: 255,
                        },
                        noise_1d_layered(x, y, 0.1, "water", 5) * 0.9
                            + noise_1d_layered(x, y, 1.5, "water", 5) * 0.1,
                    )
                }),
            ),
        ],
    )
}
impl DrawingState {
    pub fn new(name: Option<String>, handle: &mut RaylibHandle, thread: &RaylibThread) -> Self {
        Self {
            token_table: load_token_table(handle, thread),
            current_layer: 0,
            done: false,
            mode: DrawingStateMode::DrawMode,
            drawing: Drawing {
                name: if let Some(n) = name {
                    n
                } else {
                    String::from("new drawing")
                },
                layers: [
                    Layer::new(handle, thread),
                    Layer::new(handle, thread),
                    Layer::new(handle, thread),
                    Layer::new(handle, thread),
                    Layer::new(handle, thread),
                    Layer::new(handle, thread),
                    Layer::new(handle, thread),
                    Layer::new(handle, thread),
                    Layer::new(handle, thread),
                    Layer::new(handle, thread),
                    Layer::new(handle, thread),
                    Layer::new(handle, thread),
                    Layer::new(handle, thread),
                    Layer::new(handle, thread),
                    Layer::new(handle, thread),
                    Layer::new(handle, thread),
                ],
            },
            token_scroll: ScrollBoxData::new(),
            token_to_place: "".into(),
            selected_object: None,
            texture_to_draw: "".into(),
            texture_scroll: ScrollBoxData::new(),
            texture_table: load_texture_table(handle, thread),
            radius: 20,
        }
    }

    pub fn update(
        &mut self,
        handle: &mut RaylibHandle,
        thread: &RaylibThread,
    ) -> Result<(), Box<dyn Error>> {
        let mut gui = GUI::new(self, handle, thread);
        gui.centered_horizontal(|ns| {
            ns.container(300, |ns| {
                ns.p2(format!("current radius:{}", self.radius));
                ns.button_3("increase radius", |i| {
                    if i.radius == 2 {
                        i.radius = 5;
                    } else {
                        i.radius += 5;
                        if i.radius > 50 {
                            i.radius = 50;
                        }
                        if i.radius < 2 {
                            i.radius = 2;
                        }
                    }
                });
                ns.button_3("decrease radius", |i| {
                    if i.radius == 5 {
                        i.radius = 2;
                    } else {
                        i.radius -= 5;
                        if i.radius > 50 {
                            i.radius = 50;
                        }
                        if i.radius < 2 {
                            i.radius = 2;
                        }
                    }
                });
                ns.p2(match self.mode {
                    DrawingStateMode::DrawMode => "mode:drawing",
                    DrawingStateMode::EraseMode => "mode:erase",
                    DrawingStateMode::PlacementMode => "mode:placement",
                    DrawingStateMode::SelectMode => "mode:selection",
                });
                ns.button_2("selection mode", |state| {
                    state.mode = DrawingStateMode::SelectMode;
                });
                ns.button_2("erase mode", |state| {
                    state.mode = DrawingStateMode::EraseMode;
                });
                ns.button_2("placement mode", |state| {
                    state.mode = DrawingStateMode::PlacementMode;
                });
                ns.button_2("draw mode", |state| {
                    state.mode = DrawingStateMode::DrawMode;
                });
                ns.p2(format!("current layer:{}", self.current_layer + 1));
                for i in 0..16 {
                    ns.button_3(format!("edit layer {}", i + 1), move |tmp| {
                        tmp.current_layer = i;
                    });
                }
            });
            ns.canvas(1024, 1024, |bounds, state, cmds, handle, _thread| {
                let _mouse_pressed =
                    handle.is_mouse_button_pressed(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT);
                let mouse_released =
                    handle.is_mouse_button_released(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT);
                let mouse_down =
                    handle.is_mouse_button_down(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT);
                let mouse_pos = {
                    let mouse_pos = handle.get_mouse_position();
                    let rat = bounds.width as f32 / 1024.;
                    let start_x = bounds.x;
                    let start_y = bounds.y;
                    let dx = mouse_pos.x - start_x as f32;
                    let dy = mouse_pos.y - start_y as f32;
                    
                    Point {
                        x: (dx * rat).round() as i32,
                        y: (dy * rat).round() as i32,
                    }
                };
                cmds.draw_rectangle(0, 0, 1024, 1024, Color::BLACK);
                for i in 0..16 {
                    cmds.draw_render_texture_scaled(
                        &state.drawing.layers[i].values,
                        0,
                        0,
                        1024 * 2,
                        1024 * 2,
                    );
                    for j in state.drawing.layers[i].widgets.iter() {
                        if let Some(text) = state.token_table.get(&j.image_name) {
                            cmds.draw_texture_scaled(text, j.bounds.x, j.bounds.y, 1024, 1024);
                        } else {
                            cmds.draw_rectangle(
                                j.bounds.x - j.bounds.width / 2,
                                j.bounds.y - j.bounds.height / 2,
                                j.bounds.width,
                                j.bounds.height,
                                Color::RED,
                            );
                        }
                    }
                }
                if (Bounds {
                    x: 0,
                    y: 0,
                    width: 1024,
                    height: 1024,
                })
                .contains_point(mouse_pos)
                {
                    match state.mode {
                        DrawingStateMode::SelectMode => {
                            if handle.is_mouse_button_pressed(
                                raylib::ffi::MouseButton::MOUSE_BUTTON_RIGHT,
                            ) {
                                state.mode = DrawingStateMode::PlacementMode;
                            }
                            if let Some(x) = state.selected_object {
                                let old_y = state.drawing.layers[state.current_layer].widgets[x]
                                    .bounds
                                    .y;
                                let old_x = state.drawing.layers[state.current_layer].widgets[x]
                                    .bounds
                                    .x;
                                let old_width = state.drawing.layers[state.current_layer].widgets
                                    [x]
                                    .bounds
                                    .width;
                                let old_height = state.drawing.layers[state.current_layer].widgets
                                    [x]
                                    .bounds
                                    .height;
                                if handle.is_key_down(raylib::ffi::KeyboardKey::KEY_W) {
                                    state.drawing.layers[state.current_layer].widgets[x]
                                        .bounds
                                        .y -= 1;
                                }
                                if handle.is_key_down(raylib::ffi::KeyboardKey::KEY_S) {
                                    state.drawing.layers[state.current_layer].widgets[x]
                                        .bounds
                                        .y += 1;
                                }
                                if handle.is_key_down(raylib::ffi::KeyboardKey::KEY_A) {
                                    state.drawing.layers[state.current_layer].widgets[x]
                                        .bounds
                                        .x -= 1;
                                }
                                if handle.is_key_down(raylib::ffi::KeyboardKey::KEY_D) {
                                    state.drawing.layers[state.current_layer].widgets[x]
                                        .bounds
                                        .x += 1;
                                }
                                if handle.is_key_down(raylib::ffi::KeyboardKey::KEY_Q) {
                                    state.drawing.layers[state.current_layer].widgets[x]
                                        .rotation += 6.28 / 100.0;
                                }
                                if handle.is_key_down(raylib::ffi::KeyboardKey::KEY_E) {
                                    state.drawing.layers[state.current_layer].widgets[x]
                                        .rotation -= 6.28 / 100.0;
                                }
                                if handle.is_key_down(raylib::ffi::KeyboardKey::KEY_E) {
                                    state.drawing.layers[state.current_layer].widgets[x]
                                        .rotation -= 6.28 / 100.0;
                                }
                                if handle.is_key_down(raylib::ffi::KeyboardKey::KEY_TAB) {
                                    state.drawing.layers[state.current_layer].widgets[x]
                                        .bounds
                                        .width += 5;
                                }
                                if handle.is_key_down(raylib::ffi::KeyboardKey::KEY_LEFT_SHIFT) {
                                    state.drawing.layers[state.current_layer].widgets[x]
                                        .bounds
                                        .width -= 5;
                                }
                                if handle.is_key_down(raylib::ffi::KeyboardKey::KEY_R) {
                                    state.drawing.layers[state.current_layer].widgets[x]
                                        .bounds
                                        .height += 5;
                                }
                                if handle.is_key_down(raylib::ffi::KeyboardKey::KEY_F) {
                                    state.drawing.layers[state.current_layer].widgets[x]
                                        .bounds
                                        .height -= 5;
                                }
                                if !state.drawing.layers[state.current_layer].widgets[x]
                                    .bounds
                                    .intersects(
                                        &Bounds {
                                            x: 0,
                                            y: 0,
                                            width: 1024,
                                            height: 1024,
                                        } ,
                                    )
                                    || state.drawing.layers[state.current_layer].widgets[x]
                                        .bounds
                                        .width
                                        < 1
                                    || state.drawing.layers[state.current_layer].widgets[x]
                                        .bounds
                                        .height
                                        < 1
                                {
                                    state.drawing.layers[state.current_layer].widgets[x]
                                        .bounds
                                        .x = old_x;
                                    state.drawing.layers[state.current_layer].widgets[x]
                                        .bounds
                                        .y = old_y;
                                    state.drawing.layers[state.current_layer].widgets[x]
                                        .bounds
                                        .width = old_width;
                                    state.drawing.layers[state.current_layer].widgets[x]
                                        .bounds
                                        .height = old_height;
                                }
                            }

                            if mouse_released {
                                let mut hit = false;
                                for (i, j) in state.drawing.layers[state.current_layer]
                                    .widgets
                                    .iter()
                                    .enumerate()
                                {
                                    if j.bounds.contains_point(mouse_pos) {
                                        state.selected_object = Some(i);
                                        hit = true;
                                        break;
                                    }
                                }
                                if !hit {
                                    state.selected_object = None;
                                }
                            }
                        }
                        DrawingStateMode::DrawMode => {
                            cmds.render_texture_mode(
                                &state.drawing.layers[state.current_layer].values,
                                |cmds| {
                                    let mouse_pos = {
                                        let mouse_pos = handle.get_mouse_position();
                                        //  let rat = bounds.width as f32 / (1024.);
                                        let rat = (961.) / (handle.get_screen_width() as f32);
                                        let start_x = bounds.x;
                                        let start_y = bounds.y;
                                        let dx = mouse_pos.x - start_x as f32;
                                        let dy = mouse_pos.y - start_y as f32;
                                        let mut p = Point {
                                            x: (dx * rat).round() as i32,
                                            y: (dy * rat).round() as i32,
                                        };
                                        p.x += 512;
                                        p.y += 512;
                                        p
                                    };
                                    if let Some(y) = state.texture_table.get(&state.texture_to_draw)
                                    {
                                        if mouse_down {
                                            for dy in -state.radius + mouse_pos.y
                                                ..=state.radius + mouse_pos.y
                                            {
                                                for dx in -state.radius + mouse_pos.x
                                                    ..=state.radius + mouse_pos.x
                                                {
                                                    if dx >= 0 && dy >= 0 && dx < 1024 && dy < 1024
                                                    {
                                                        let rad = (dx - mouse_pos.x)
                                                            * (dx - mouse_pos.x)
                                                            + (dy - mouse_pos.y)
                                                                * (dy - mouse_pos.y);
                                                        let r2 = ((state.radius * state.radius)
                                                            as f32
                                                            * (noise_1d_layered(
                                                                dx,
                                                                dy,
                                                                1.0,
                                                                "layer noise",
                                                                2,
                                                            ) * 1.5)
                                                                .clamp(0.5, 1.5))
                                                            as i32;
                                                        if rad < r2 {
                                                            cmds.draw_pixel(
                                                                dx,
                                                                dy,
                                                                (y.shader)(dx, dy),
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        if mouse_down {
                                            for dy in -state.radius + mouse_pos.y
                                                ..=state.radius + mouse_pos.y
                                            {
                                                for dx in -state.radius + mouse_pos.x
                                                    ..=state.radius + mouse_pos.x
                                                {
                                                    if dx >= 0 && dy >= 0 && dx < 1024 && dy < 1024
                                                    {
                                                        let rad = (dx - mouse_pos.x)
                                                            * (dx - mouse_pos.x)
                                                            + (dy - mouse_pos.y)
                                                                * (dy - mouse_pos.y);
                                                        let r2 = ((state.radius * state.radius)
                                                            as f32
                                                            * (noise_1d_layered(
                                                                dx,
                                                                dy,
                                                                0.1,
                                                                "layer noise",
                                                                2,
                                                            ) * 1.5)
                                                                .clamp(0.8, 1.5))
                                                            as i32;
                                                        if rad < r2 {
                                                            cmds.draw_pixel(dx, dy, Color::RED);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                },
                            );
                        }
                        DrawingStateMode::EraseMode => {
                            if mouse_down {
                                for dy in -state.radius + mouse_pos.y..=state.radius + mouse_pos.x {
                                    for dx in
                                        -state.radius + mouse_pos.x..=state.radius + mouse_pos.x
                                    {
                                        if dx >= 0 && dy >= 0 && dx < 1024 && dy < 1024 {}
                                    }
                                }
                            }
                        }
                        DrawingStateMode::PlacementMode => {
                            if handle.is_mouse_button_pressed(
                                raylib::ffi::MouseButton::MOUSE_BUTTON_RIGHT,
                            ) {
                                state.mode = DrawingStateMode::SelectMode;
                            } else {
                                if let Some(_x) = state.token_table.get(&state.token_to_place) {
                                } 
                            }
                        }
                    }
                }
            });
            ns.container(600, |ns| {
                ns.h3("textures");
                ns.scroll_box(400, &self.texture_scroll, |ns| {
                    for i in self.texture_table.iter() {
                        let t2 = i.0.clone();
                        ns.button_image(i.1.preview.clone(), move |state| {
                            state.mode = DrawingStateMode::DrawMode;
                            state.texture_to_draw = t2.clone();
                        });
                    }
                });
                ns.h3("objects");
                ns.scroll_box(400, &self.token_scroll, |ns| {
                    for i in self.token_table.iter() {
                        let t2 = i.0.clone();
                        ns.button_image(i.1.clone(), move |state| {
                            state.mode = DrawingStateMode::PlacementMode;
                            state.token_to_place = t2.clone();
                        });
                    }
                });
            });
        });
        gui.render_fps(self);
        Ok(())
    }
}

pub async fn game_loop(handle: &mut RaylibHandle, thread: &RaylibThread, name: Option<String>) {
    let mut state = DrawingState::new(name, handle, thread);
    loop {
        if let Err(x) = state.update(handle, thread) {
            println!("{:#?}", x);
            break;
        }
        if state.done {
            break;
        }
        if handle.window_should_close() {
            break;
        }
    }
}
