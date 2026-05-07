use std::{
    collections::BTreeMap,
    error::Error,
    sync::{Arc, Mutex},
};

use raylib::{
    RaylibHandle, RaylibThread,
    color::Color,
    drawing::{RaylibDraw, RaylibTextureModeExt},
    texture::{RenderTexture2D, Texture2D},
    window,
};
use serde::{Deserialize, Serialize};

use crate::{
    engine::Col,
    libgui::{Bounds, GUI, Point, ScrollBoxData},
    utils::Heap,
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
                handle.load_render_texture(&thread, 1024, 1024).unwrap(),
            )),
            text_blocks: Vec::new(),
            widgets: Vec::new(),
        }
    }
}

impl DrawingTexture {
    pub fn new(
        name: &str,
        handle: &mut RaylibHandle,
        thread: &RaylibThread,
        kernel: impl Fn(i32, i32) -> Color + 'static,
    ) -> Self {
        let mut preview = raylib::prelude::Image::gen_image_color(25, 25, Color::WHITE);
        for i in 0..25 {
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
    handle: &mut RaylibHandle,
    thread: &RaylibThread,
) -> BTreeMap<Arc<str>, Arc<Texture2D>> {
    BTreeMap::new()
}
pub fn load_texture_table(
    handle: &mut RaylibHandle,
    thread: &RaylibThread,
) -> BTreeMap<Arc<str>, DrawingTexture> {
    BTreeMap::new()
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
            radius: 5,
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
                        if i.radius > 100 {
                            i.radius = 100;
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
                        if i.radius > 100 {
                            i.radius = 100;
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
            ns.canvas(1024, 1024, |bounds, state, cmds, handle, thread| {
                let mouse_pressed =
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
                    let p = Point {
                        x: (dx * rat).round() as i32,
                        y: (dy * rat).round() as i32,
                    };
                    p
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
                                        &(&Bounds {
                                            x: 0,
                                            y: 0,
                                            width: 1024,
                                            height: 1024,
                                        }),
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
                                                    if dx >= 0
                                                        && dy >= 0
                                                        && dx < 1024 * 2
                                                        && dy < 1024 * 2
                                                    {
                                                        let rad = (dx - mouse_pos.x)
                                                            * (dx - mouse_pos.x)
                                                            + (dy - mouse_pos.y)
                                                                * (dy - mouse_pos.y);
                                                        let r2 = state.radius * state.radius;
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
                                                    if dx >= 0
                                                        && dy >= 0
                                                        && dx < 1024 * 2
                                                        && dy < 1024 * 2
                                                    {
                                                        let rad = (dx - mouse_pos.x)
                                                            * (dx - mouse_pos.x)
                                                            + (dy - mouse_pos.y)
                                                                * (dy - mouse_pos.y);
                                                        let r2 = state.radius * state.radius;
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
                                if let Some(x) = state.token_table.get(&state.token_to_place) {
                                } else {
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
