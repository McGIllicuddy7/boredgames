use std::{
    collections::{BTreeMap, HashMap},
    thread::yield_now,
    time::Duration,
};

use crate::rtils::rtils_useful::BPipe;

use super::common::*;

#[derive(Clone)]
pub struct TextBoxData {
    pub selected: bool,
    pub text: String,
    pub cursor: usize,
    pub selected_section: Option<(usize, usize)>,
}
impl Default for TextBoxData {
    fn default() -> Self {
        Self::new()
    }
}

impl TextBoxData {
    pub fn new() -> Self {
        Self {
            selected: false,
            text: String::new(),
            cursor: 0,
            selected_section: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub text_color: BColor,
    pub background_color: BColor,
    pub object_color: BColor,
    pub object_pressed_color: BColor,
    pub decoration_color: BColor,
    pub shadows: bool,
    pub outline: bool,
}
pub struct SysHandle {
    handle: BPipe<DrawCall>,
    cx: i32,
    cy: i32,
    w: i32,
    h: i32,
    padding_x: i32,
    padding_y: i32,
    ui_mode: SysUiMode,
    text_ratios: BTreeMap<char, f64>,
    max_ratio: f64,
    div_stack: Vec<Div>,
    theme: Theme,
    user_input: UserInput,
    should_exit: bool,
    queue: Vec<DrawCall>,
    text_box_data: HashMap<String, TextBoxData>,
    scroll_box_data: HashMap<String, f32>,
    bootstrap: bool,
}
impl SysHandle {
    pub fn new(
        handle: BPipe<DrawCall>,
        w: i32,
        h: i32,
        ratios: BTreeMap<char, f64>,
        max_ratio: f64,
    ) -> Self {
        SysHandle {
            should_exit: false,
            handle,
            cx: 0,
            cy: 0,
            w,
            h,
            padding_x: 4,
            padding_y: 4,
            ui_mode: SysUiMode::Sequential,
            text_ratios: ratios,
            max_ratio,
            div_stack: Vec::new(),
            queue: Vec::new(),
            theme: Theme {
                background_color: BColor::from_rl_color(Color::NAVY),
                object_color: BColor::from_rl_color(Color::DARKMAGENTA),
                object_pressed_color: BColor::from_rl_color(Color::DARKVIOLET),
                text_color: BColor::from_rl_color(Color::WHITE),
                decoration_color: BColor::from_rl_color(Color::ORCHID),
                shadows: false,
                outline: true,
            },
            user_input: UserInput::new(),
            text_box_data: HashMap::new(),
            scroll_box_data: HashMap::new(),
            bootstrap: true,
        }
    }
    pub fn get_thumbnail_size(&self) -> i32 {
        self.get_div().thumbnail_size
    }

    pub fn set_thumbail_size(&mut self, size: i32) {
        if let Some(mut div) = self.div_stack.pop() {
            div.thumbnail_size = size;
            self.div_stack.push(div);
        }
    }

    pub fn should_exit(&self) -> bool {
        self.should_exit
    }
    pub fn char_width(&self, c: char, h: i32) -> Option<f64> {
        if let Some(r) = self.text_ratios.get(&c) {
            let out = h as f64 * *r;
            Some(out)
        } else {
            if c == '\n' || c == '\r' {
                None
            } else {
                Some(self.max_ratio * h as f64)
            }
        }
    }
    pub fn measure_text(&self, text: &str, h: i32) -> Option<i32> {
        if text.contains('\r') || text.contains('\n') {
            None
        } else {
            let mut out = 0;
            for i in text.chars() {
                let rat = *self.text_ratios.get(&i)?;
                let w = (h as f64 * rat) as i32;
                out += w;
            }
            Some(out)
        }
    }

    pub fn char_location(
        &self,
        index: usize,
        x: i32,
        y: i32,
        text: &str,
        text_height: i32,
        max_w: i32,
    ) -> Pos2 {
        let mut cx = x as f64;
        let mut cy = y as f64;
        let mut id = 0;
        for i in text.chars() {
            if i == '\r' {
                cx = x as f64;
            } else if i == '\n' {
                cx = x as f64;
                cy += text_height as f64;
            } else {
                let wp = self.char_width(i, text_height).unwrap();
                if cx + wp >= (max_w + x) as f64 {
                    cx = x as f64 + wp;
                    cy += text_height as f64;
                } else {
                    cx += wp;
                    if id == index {
                        return Pos2 {
                            x: cx as i32,
                            y: cy as i32,
                        };
                    }
                }
            }
            id += 1;
        }
        Pos2 {
            x: cx as i32,
            y: cy as i32,
        }
    }
    pub fn nearest_char_to(
        &self,
        pos_x: i32,
        pos_y: i32,
        x: i32,
        y: i32,
        text: &str,
        text_height: i32,
        max_w: i32,
    ) -> usize {
        if text.is_empty() {
            return 0;
        }
        let mut cx = x as f64;
        let mut cy = y as f64;
        let mut id = 0;
        let mut min_id = text.len() - 1;
        let mut min_dist = 10000.0;
        for i in text.chars() {
            if i == '\r' {
                cx = x as f64;
            } else if i == '\n' {
                cx = x as f64;
                cy += text_height as f64;
            } else {
                let wp = self.char_width(i, text_height).unwrap();
                if cx + wp >= (max_w + x) as f64 {
                    cx = x as f64 + wp;
                    cy += text_height as f64;
                } else {
                    cx += wp;
                }
            }
            let dist = (pos_y as f64 - cy) * (pos_y as f64 - cy)
                + (pos_x as f64 - cx) * (pos_x as f64 - cx);
            if dist < min_dist {
                min_id = id;
                min_dist = dist;
            }
            id += 1;
        }
        min_id
    }

    pub fn split_by_required_line(&self, text: &str, h: i32, max_w: i32) -> Vec<String> {
        let mut out = Vec::new();
        let mut current = String::new();
        let mut w = 0.0;
        for i in text.chars() {
            if i == '\r' {
                current.push(i);
                out.push(current);
                current = String::new();
                w = 0.;
            } else if i == '\n' {
                out.push(current);
                current = String::new();
                w = 0.;
            } else {
                let wp = self.char_width(i, h).unwrap();
                if w + wp >= max_w as f64 {
                    w = wp;
                    out.push(current);
                    current = String::new();
                    current.push(i);
                } else {
                    current.push(i);
                    w += wp;
                }
            }
        }
        if !current.is_empty() {
            out.push(current);
        }
        out
    }

    pub fn text_get_height_and_lines(&self, text: &str, h: i32, max_w: i32) -> (i32, Vec<String>) {
        let split = self.split_by_required_line(text, h, max_w);
        let mut count = 0;
        for i in &split {
            if !i.ends_with('\r') {
                count += h;
            }
        }
        (count, split)
    }

    pub fn set_sys_ui_mode(&mut self, mode: SysUiMode) {
        self.ui_mode = mode;
    }

    pub fn get_div(&self) -> Div {
        if !self.div_stack.is_empty() {
            self.div_stack[self.div_stack.len() - 1]
        } else {
            Div {
                x: 0,
                y: 0,
                w: self.w,
                h: self.h,
                vertical: false,
                thumbnail_size: DEFAULT_THUMBNAIL_SIZE,
                mode: SysUiMode::Sequential,
            }
        }
    }

    pub fn get_absolute_pos(&self, pos: Pos2) -> Pos2 {
        match self.ui_mode {
            SysUiMode::Absolute => pos,
            SysUiMode::Relative => {
                let div = self.get_div();
                Pos2 {
                    x: div.x + pos.x,
                    y: div.y + pos.y,
                }
            }
            SysUiMode::Sequential => Pos2 {
                x: self.cx + pos.x,
                y: self.cy + pos.y,
            },
        }
    }

    pub fn update_cursor(&mut self, prev_bounds: Rect) {
        match self.ui_mode {
            SysUiMode::Sequential => {}
            SysUiMode::Absolute | SysUiMode::Relative => {
                return;
            }
        }
        let div = self.get_div();
        if div.vertical {
            self.cx = div.x;
            self.cy = prev_bounds.y + prev_bounds.h + 5;
        } else {
            self.cy = div.y;
            self.cx = prev_bounds.x + prev_bounds.w + 5;
        }
    }

    pub fn draw_text_exp(&mut self, x: i32, y: i32, h: i32, max_w: i32, text: &str) {
        let (height, texts) = self.text_get_height_and_lines(text, h, max_w);
        let base = self.get_absolute_pos(Pos2 { x, y });
        let mut current = base;
        for i in texts {
            self.queue.push(DrawCall::DrawText {
                x: current.x,
                y: current.y,
                size: h,
                contents: i,
                color: self.theme.text_color,
            });
            current.y += h;
        }
        self.update_cursor(Rect {
            x: base.x,
            y: base.y,
            w: max_w,
            h: height,
        });
    }

    pub fn draw_text(&mut self, h: i32, max_w: i32, text: &str) {
        self.draw_text_exp(self.padding_x, self.padding_y, h, max_w, text);
    }

    pub fn draw_box_exp(&mut self, x: i32, y: i32, w: i32, h: i32) {
        let base = self.get_absolute_pos(Pos2 { x, y });
        self.queue.push(DrawCall::Rectangle {
            x,
            y,
            w,
            h,
            color: self.theme.object_color,
            drop_shadow: self.theme.shadows,
            outline: self.theme.outline,
        });
        self.update_cursor(Rect {
            x: base.x,
            y: base.y,
            w,
            h,
        });
    }

    pub fn draw_box(&mut self, w: i32, h: i32) {
        self.draw_box_exp(self.padding_x, self.padding_y, w, h)
    }

    pub fn draw_circle_exp(&mut self, x: i32, y: i32, rad: i32) {
        let base = self.get_absolute_pos(Pos2 { x, y });
        self.queue.push(DrawCall::Circle {
            x,
            y,
            rad,
            color: self.theme.object_color,
            drop_shadow: self.theme.shadows,
            outline: self.theme.outline,
        });
        self.update_cursor(Rect {
            x: base.x,
            y: base.y,
            w: rad * 2,
            h: rad * 2,
        });
    }

    pub fn draw_circle(&mut self, rad: i32) {
        self.draw_circle_exp(self.padding_x, self.padding_y, rad)
    }

    pub fn draw_image_exp(&mut self, x: i32, y: i32, w: i32, h: i32, image: &str) {
        let base = self.get_absolute_pos(Pos2 { x, y });
        self.queue.push(DrawCall::DrawImage {
            x,
            y,
            h,
            w,
            contents_ident: image.into(),
        });
        self.update_cursor(Rect {
            x: base.x,
            y: base.y,
            w,
            h,
        });
    }

    pub fn draw_image(&mut self, w: i32, h: i32, image: &str) {
        self.draw_image_exp(self.padding_x, self.padding_y, w, h, image)
    }

    pub fn draw_pixels(&mut self, points: Vec<(Pos2, BColor)>, width: f32) {
        if points.is_empty() {
            return;
        }
        let mut mpoints = points;
        mpoints.iter_mut().for_each(|(x, _)| {
            *x = self.get_absolute_pos(*x);
        });
        let mut min = mpoints[0].0;
        let mut max = mpoints[0].0;
        mpoints.iter().for_each(|(x, _)| {
            if x.x < min.x {
                min.x = x.x;
            }
            if x.y < min.y {
                min.y = x.y;
            }
            if x.x > max.x {
                max.x = x.x;
            }
            if x.y > max.y {
                max.y = x.y;
            }
        });
        self.queue.push(DrawCall::DrawVectors {
            points: mpoints,
            width,
        });
        let w = max.x - min.x;
        let h = max.y - min.y;
        self.update_cursor(Rect {
            x: min.x,
            y: min.y,
            w,
            h,
        });
    }

    pub fn draw_button_exp(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        text_height: i32,
        text: &str,
    ) -> bool {
        let (mut h, texts) = self.text_get_height_and_lines(text, text_height, w - 5);
        h += 10;
        let pos = self.get_absolute_pos(Pos2 { x, y });
        let did_hit = self.user_input.mouse_x() >= pos.x
            && self.user_input.mouse_y() >= pos.y
            && self.user_input.mouse_x() < pos.x + w
            && self.user_input.mouse_y() < pos.y + h;
        let col = if did_hit && self.user_input.left_mouse_down() {
            self.theme.object_pressed_color
        } else {
            self.theme.object_color
        };
        self.queue.push(DrawCall::Rectangle {
            x: pos.x,
            y: pos.y,
            w,
            h,
            color: col,
            drop_shadow: self.theme.shadows,
            outline: self.theme.outline,
        });
        let mut current = pos;
        current.x += 5;
        current.y += 5;
        for i in texts {
            self.queue.push(DrawCall::DrawText {
                x: current.x,
                y: current.y,
                size: text_height,
                contents: i,
                color: self.theme.text_color,
            });
            current.y += text_height;
        }
        self.update_cursor(Rect {
            x: pos.x,
            y: pos.y,
            w,
            h,
        });

        self.user_input.mouse_x() >= pos.x
            && self.user_input.mouse_y() >= pos.y
            && self.user_input.mouse_x() < pos.x + w
            && self.user_input.mouse_y() < pos.y + h
            && self.user_input.left_mouse_released()
    }

    pub fn draw_button(&mut self, w: i32, text_height: i32, text: &str) -> bool {
        self.draw_button_exp(self.padding_x, self.padding_y, w, text_height, text)
    }

    pub fn begin_div_exp(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        vertical: bool,
        mode: SysUiMode,
    ) {
        let pos = self.get_absolute_pos(Pos2 { x, y });
        self.div_stack.push(Div {
            x: pos.x,
            y: pos.y,
            w,
            h,
            vertical,
            mode,
            thumbnail_size: DEFAULT_THUMBNAIL_SIZE,
        });
        self.ui_mode = mode;
        match self.ui_mode {
            SysUiMode::Absolute | SysUiMode::Relative => {}
            SysUiMode::Sequential => {
                self.cx = pos.x;
                self.cy = pos.y;
            }
        }
    }

    pub fn begin_div(&mut self, w: i32, h: i32) {
        self.begin_div_exp(
            self.padding_x,
            self.padding_y,
            w,
            h,
            true,
            SysUiMode::Sequential,
        );
    }

    pub fn end_div(&mut self) {
        let x = self.div_stack.pop().unwrap();
        self.ui_mode = self.get_div().mode;
        self.update_cursor(Rect {
            x: x.x,
            y: x.y,
            w: x.w,
            h: x.h,
        });
    }

    pub fn begin_drawing(&mut self) {
        self.user_input.reset();
        let mut hit = false;
        while !hit {
            if self.bootstrap {
                self.bootstrap = false;
                break;
            }
            yield_now();
            while let Ok(Some(x)) = self.handle.recieve() {
                match x {
                    DrawCall::Update { input } => {
                        hit = true;
                        self.user_input = input;
                    }
                    DrawCall::Exiting => {
                        hit = true;
                        self.should_exit = true;
                        break;
                    }
                    _ => {
                        continue;
                    }
                }
            }
        }

        self.div_stack.clear();
        self.queue.clear();
        self.queue.push(DrawCall::BeginDrawing);
        self.queue.push(DrawCall::ClearBackground {
            color: self.theme.background_color,
        });
        self.cx = 0;
        self.cy = 0;
        self.ui_mode = SysUiMode::Sequential;
    }

    pub fn end_drawing(&mut self) {
        self.queue.push(DrawCall::EndDrawing);
        self.handle.send_multiple(&mut self.queue).unwrap();
        self.div_stack.clear();
        self.queue.clear();
        self.cx = 0;
        self.cy = 0;
        self.ui_mode = SysUiMode::Sequential;
        std::thread::sleep(Duration::from_millis(12));
    }

    pub fn set_cursor(&mut self, x: i32, y: i32) {
        self.cx = x;
        self.cy = y;
    }

    pub fn get_cursor(&self) -> Pos2 {
        Pos2 {
            x: self.cx,
            y: self.cy,
        }
    }

    pub fn draw_text_scroll_box_exp<'a, T>(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        text_height: i32,
        amount: f32,
        upside_down: bool,
        objects: &[T],
        as_string: impl Fn(&T) -> String,
    ) -> f32 {
        let base = self.get_absolute_pos(Pos2 { x, y });
        let mut height = 0;
        let strings: Vec<String> = objects
            .iter()
            .flat_map(|i| {
                let (dh, cons) = self.text_get_height_and_lines(&as_string(i), text_height, w - 10);
                height += dh + 10;
                cons
            })
            .collect();
        self.queue.push(DrawCall::Rectangle {
            x: base.x,
            y: base.y,
            w,
            h,
            color: self.theme.object_color,
            drop_shadow: self.theme.shadows,
            outline: self.theme.outline,
        });
        self.queue.push(DrawCall::Scissor {
            x: base.x,
            y: base.y,
            h,
            w,
        });
        let delta = height as f32 * amount;
        let base_y = if !upside_down {
            base.y as f32 - delta
        } else {
            base.y as f32 - delta + h as f32 - text_height as f32
        };
        let mut y = base_y;
        for i in strings {
            let pos = Pos2 {
                x: base.x,
                y: y as i32,
            };
            let er = i.ends_with('\r');
            if !(y as i32 + text_height < base.y || y as i32 > base.y + h) {
                self.queue.push(DrawCall::DrawText {
                    x: pos.x + 2,
                    y: pos.y,
                    size: text_height,
                    contents: i,
                    color: self.theme.text_color,
                });
            }
            if !er {
                if !upside_down {
                    y += text_height as f32;
                } else {
                    y -= text_height as f32;
                }
            }
        }
        let bx = Rect {
            x: base.x + w - 8,
            y: base.y + ((h as f32 - 16.0) * amount) as i32,
            w: 12,
            h: 16,
        };
        self.queue.push(DrawCall::Rectangle {
            x: bx.x,
            y: bx.y,
            w: bx.w,
            h: bx.h,
            color: self.theme.decoration_color,
            drop_shadow: false,
            outline: true,
        });
        self.queue.push(DrawCall::DrawVectors {
            points: vec![
                (
                    Pos2 { x: bx.x, y: base.y },
                    BColor::from_rl_color(Color::DARKGRAY),
                ),
                (
                    Pos2 {
                        x: bx.x,
                        y: base.y + h,
                    },
                    BColor::from_rl_color(Color::DARKGRAY),
                ),
            ],
            width: 1.0,
        });

        self.queue.push(DrawCall::EndScissor);
        let hovered = self.user_input.mouse_x() >= base.x
            && self.user_input.mouse_y() >= base.y
            && self.user_input.mouse_x() < base.x + w
            && self.user_input.mouse_y() < base.y + h;
        let mut out = amount;
        let bx = Rect {
            x: bx.x - 5,
            y: bx.y - 5,
            w: bx.w + 10,
            h: bx.h + 10,
        };
        if self.user_input.left_mouse_down()
            && bx.check_collision(Pos2 {
                x: self.user_input.mouse_x(),
                y: self.user_input.mouse_y(),
            })
        {
            out += self.user_input.mouse_dy() / h as f32 * 1.5;
        } else if hovered {
            out -= self.user_input.scroll_amount() / h as f32;
        }

        self.update_cursor(Rect {
            x: base.x,
            y: base.y,
            w,
            h,
        });
        out.clamp(0.0, 1.)
    }

    pub fn draw_button_scroll_box_exp<'a, T>(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        text_height: i32,
        amount: f32,
        upside_down: bool,
        objects: &[T],
        as_string: impl Fn(&T) -> String,
    ) -> (f32, Option<usize>) {
        let base = self.get_absolute_pos(Pos2 { x, y });
        let mut height = 0;
        let strings: Vec<(i32, Vec<String>)> = objects
            .iter()
            .map(|i| {
                let (dh, cons) = self.text_get_height_and_lines(&as_string(i), text_height, w - 10);
                height += dh + 10;
                (dh + 5, cons)
            })
            .collect();
        self.queue.push(DrawCall::Rectangle {
            x: base.x,
            y: base.y,
            w,
            h,
            color: self.theme.object_color,
            drop_shadow: self.theme.shadows,
            outline: self.theme.outline,
        });
        self.queue.push(DrawCall::Scissor {
            x: base.x,
            y: base.y,
            h,
            w,
        });
        let delta = height as f32 * amount;
        let base_y = if !upside_down {
            base.y as f32 - delta
        } else {
            base.y as f32 - delta + h as f32 - text_height as f32
        };
        let mut y = base_y;
        let mut idx = 0;
        let mut hit = None;
        for i in strings {
            let pos = Pos2 {
                x: base.x,
                y: y as i32,
            };
            if !(y as i32 + i.0 < base.y || (y as i32) > base.y + h) {
                let did_hit = self.user_input.mouse_x() >= pos.x
                    && self.user_input.mouse_y() >= pos.y
                    && self.user_input.mouse_x() < pos.x + w
                    && self.user_input.mouse_y() < pos.y + i.0;
                let col = if did_hit && self.user_input.left_mouse_down() {
                    self.theme.object_pressed_color
                } else {
                    self.theme.object_color
                };
                self.queue.push(DrawCall::Rectangle {
                    x: pos.x,
                    y: pos.y,
                    w: w - 11,
                    h: i.0,
                    color: col,
                    drop_shadow: self.theme.shadows,
                    outline: self.theme.outline,
                });
                let mut current = pos;
                current.x += 2;
                current.y += 2;
                for i in i.1 {
                    self.queue.push(DrawCall::DrawText {
                        x: current.x,
                        y: current.y,
                        size: text_height,
                        contents: i,
                        color: self.theme.text_color,
                    });
                    current.y += text_height;
                }

                if did_hit && self.user_input.left_mouse_released() {
                    hit = Some(idx);
                }
            }
            if !upside_down {
                y += i.0 as f32 + 5.0;
            } else {
                y -= i.0 as f32 + 5.0;
            }
            idx += 1;
        }
        let bx = Rect {
            x: base.x + w - 8,
            y: base.y + ((h as f32 - 16.0) * amount) as i32,
            w: 12,
            h: 16,
        };
        self.queue.push(DrawCall::Rectangle {
            x: bx.x,
            y: bx.y,
            w: bx.w,
            h: bx.h,
            color: self.theme.decoration_color,
            drop_shadow: false,
            outline: true,
        });
        self.queue.push(DrawCall::DrawVectors {
            points: vec![
                (
                    Pos2 { x: bx.x, y: base.y },
                    BColor::from_rl_color(Color::DARKGRAY),
                ),
                (
                    Pos2 {
                        x: bx.x,
                        y: base.y + h,
                    },
                    BColor::from_rl_color(Color::DARKGRAY),
                ),
            ],
            width: 1.0,
        });
        self.queue.push(DrawCall::EndScissor);
        let hovered = self.user_input.mouse_x() >= base.x
            && self.user_input.mouse_y() >= base.y
            && self.user_input.mouse_x() < base.x + w
            && self.user_input.mouse_y() < base.y + h;
        let mut out = amount;
        let bx = Rect {
            x: bx.x - 5,
            y: bx.y - 5,
            w: bx.w + 10,
            h: bx.h + 10,
        };
        if self.user_input.left_mouse_down()
            && bx.check_collision(Pos2 {
                x: self.user_input.mouse_x(),
                y: self.user_input.mouse_y(),
            })
        {
            out += self.user_input.mouse_dy() / h as f32 * 1.5;
        } else if hovered {
            out -= self.user_input.scroll_amount() / h as f32;
        }
        self.update_cursor(Rect {
            x: base.x,
            y: base.y,
            w,
            h,
        });
        (out.clamp(0.0, 1.0), hit)
    }

    pub fn draw_button_scroll_box_saved_exp<T>(
        &mut self,
        name: &str,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        text_height: i32,
        upside_down: bool,
        objects: &[T],
        as_string: impl Fn(&T) -> String,
    ) -> Option<usize> {
        let amnt = if let Some(k) = self.scroll_box_data.get(name) {
            *k
        } else {
            self.scroll_box_data.insert(name.to_string(), 0.0);
            0.0
        };
        let (a, x) = self.draw_button_scroll_box_exp(
            x,
            y,
            w,
            h,
            text_height,
            amnt,
            upside_down,
            objects,
            as_string,
        );
        *self.scroll_box_data.get_mut(name).unwrap() = a;
        x
    }

    pub fn draw_text_scroll_box_saved_exp<T>(
        &mut self,
        name: &str,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        text_height: i32,
        upside_down: bool,
        objects: &[T],
        as_string: impl Fn(&T) -> String,
    ) {
        let amnt = if let Some(k) = self.scroll_box_data.get(name) {
            *k
        } else {
            self.scroll_box_data.insert(name.to_string(), 0.0);
            0.0
        };
        let a = self.draw_text_scroll_box_exp(
            x,
            y,
            w,
            h,
            text_height,
            amnt,
            upside_down,
            objects,
            as_string,
        );
        *self.scroll_box_data.get_mut(name).unwrap() = a;
    }

    pub fn text_user_input_exp(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        text_height: i32,
        inp: TextBoxData,
    ) -> (Option<String>, TextBoxData) {
        let pos = self.get_absolute_pos(Pos2 { x, y });
        let bx = Rect {
            x: pos.x,
            y: pos.y,
            w,
            h,
        };
        let mut out_selected = inp.selected;
        let mut output = inp.text;
        let mut out_cursor = inp.cursor;
        let selected_section = None;
        let mut returned = None;
        if self.user_input.left_mouse_released() {
            out_selected = bx.check_collision(Pos2 {
                x: self.user_input.mouse_x(),
                y: self.user_input.mouse_y(),
            });
        }
        if out_selected {
            for i in self.user_input.pressed_keys.clone() {
                if i == '\n' {
                    returned = Some(output);
                    output = String::new();
                } else if i as i32 == 127 {
                    if out_cursor < output.len() && out_cursor > 0 {
                        output.remove(out_cursor - 1);
                        out_cursor = out_cursor.saturating_sub(1);
                    } else if out_cursor != 0 {
                        out_cursor -= 1;
                        output.pop();
                    }
                } else {
                    if out_cursor >= output.len() {
                        output.push(i);
                    } else {
                        output.insert(out_cursor, i);
                    }
                    out_cursor += 1;
                }
            }
            if self.user_input.left_arrow_pressed() {
                out_cursor = out_cursor.saturating_sub(1);
            }
            if self.user_input.right_arrow_pressed() && out_cursor <= output.len() {
                out_cursor += 1;
            }
            out_cursor = out_cursor.clamp(0, output.len() + 1);
        }
        self.queue.push(DrawCall::Rectangle {
            x: pos.x,
            y: pos.y,
            w,
            h,
            color: if out_selected {
                self.theme.object_pressed_color
            } else {
                self.theme.object_color
            },
            drop_shadow: self.theme.shadows,
            outline: self.theme.outline,
        });
        self.queue.push(DrawCall::Scissor {
            x: pos.x,
            y: pos.y,
            h,
            w,
        });
        let (th, txt) = self.text_get_height_and_lines(&output, text_height, w - 2);
        let mut cursor = pos;
        if th > h {
            cursor.y -= th - h;
        }
        cursor.x += 2;
        if self.user_input.left_mouse_pressed() {
            let x = self.user_input.mouse_x();
            let y = self.user_input.mouse_y();
            let o2 = output.clone() + " ";
            let id = self.nearest_char_to(x, y, cursor.x, cursor.y, &o2, text_height, w - 2);
            out_cursor = id;
        }
        let lok = self.char_location(out_cursor, cursor.x, cursor.y, &output, text_height, w - 2);
        self.queue.push(DrawCall::Rectangle {
            x: lok.x - 2,
            y: lok.y,
            w: 2,
            h: text_height,
            color: self.theme.decoration_color,
            drop_shadow: false,
            outline: false,
        });
        for i in txt {
            self.queue.push(DrawCall::DrawText {
                x: cursor.x,
                y: cursor.y,
                size: text_height,
                contents: i,
                color: self.theme.text_color,
            });
            cursor.y += text_height;
        }
        self.queue.push(DrawCall::EndScissor);
        self.update_cursor(Rect {
            x: pos.x,
            y: pos.y,
            w,
            h,
        });
        (
            returned,
            TextBoxData {
                selected: out_selected,
                text: output,
                cursor: out_cursor,
                selected_section,
            },
        )
    }

    pub fn text_user_input_saved_exp(
        &mut self,
        name: &str,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        text_height: i32,
    ) -> Option<String> {
        let data = if let Some(k) = self.text_box_data.get(name) {
            k.clone()
        } else {
            let tmp = TextBoxData::new();
            self.text_box_data.insert(name.to_string(), tmp.clone());
            tmp
        };
        let (out, d2) = self.text_user_input_exp(x, y, w, h, text_height, data);
        *self.text_box_data.get_mut(name).unwrap() = d2;
        out
    }

    pub fn is_char_pressed(&self, ch: char) -> bool {
        self.user_input.pressed_keys.contains(&ch)
    }

    pub fn left_mouse_down(&self) -> bool {
        self.user_input.left_mouse_down()
    }

    pub fn left_mouse_pressed(&self) -> bool {
        self.user_input.left_mouse_pressed()
    }

    pub fn left_mouse_released(&self) -> bool {
        self.user_input.left_mouse_released()
    }

    pub fn right_mouse_down(&self) -> bool {
        self.user_input.right_mouse_down()
    }

    pub fn right_mouse_pressed(&self) -> bool {
        self.user_input.right_arrow_pressed()
    }

    pub fn right_mouse_released(&self) -> bool {
        self.user_input.right_mouse_released()
    }

    pub fn get_pressed_chars(&self) -> &[char] {
        &self.user_input.pressed_keys
    }

    pub fn get_mouse_x(&self) -> i32 {
        self.user_input.mouse_x()
    }

    pub fn get_mouse_y(&self) -> i32 {
        self.user_input.mouse_y()
    }

    pub fn get_mouse_pos(&self) -> Pos2 {
        Pos2 {
            x: self.user_input.mouse_x(),
            y: self.user_input.mouse_y(),
        }
    }

    pub fn scroll_amount(&self) -> f32 {
        self.user_input.scroll_amount()
    }

    pub fn is_key_down(&self, key: KeyboardKey) -> bool {
        self.user_input.is_key_down(key)
    }

    pub fn is_key_pressed(&self, key: KeyboardKey) -> bool {
        self.user_input.is_key_pressed(key)
    }

    pub fn is_key_released(&self, key: KeyboardKey) -> bool {
        self.user_input.is_key_released(key)
    }

    pub fn is_mouse_button_down(&self, key: MouseButton) -> bool {
        self.user_input.is_mouse_button_down(key)
    }

    pub fn is_mouse_button_pressed(&self, key: MouseButton) -> bool {
        self.user_input.is_mouse_button_pressed(key)
    }

    pub fn is_mouse_button_released(&self, key: MouseButton) -> bool {
        self.user_input.is_mouse_button_released(key)
    }

    pub fn draw_button_image_exp(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        text_height: i32,
        text: &str,
        image: &str,
    ) -> bool {
        let (mut h, texts) =
            self.text_get_height_and_lines(text, text_height, w - (self.get_thumbnail_size() + 5));
        if h < self.get_thumbnail_size() {
            h = self.get_thumbnail_size();
        }
        let pos = self.get_absolute_pos(Pos2 { x, y });
        let did_hit = self.user_input.mouse_x() >= pos.x
            && self.user_input.mouse_y() >= pos.y
            && self.user_input.mouse_x() < pos.x + w
            && self.user_input.mouse_y() < pos.y + h;
        let col = if did_hit && self.user_input.left_mouse_down() {
            self.theme.object_pressed_color
        } else {
            self.theme.object_color
        };
        self.queue.push(DrawCall::Rectangle {
            x: pos.x,
            y: pos.y,
            w,
            h,
            color: col,
            drop_shadow: self.theme.shadows,
            outline: self.theme.outline,
        });
        let mut current = pos;

        self.queue.push(DrawCall::DrawImage {
            x: current.x,
            y: current.y + (h - self.get_thumbnail_size()) / 2,
            h: self.get_thumbnail_size(),
            w: self.get_thumbnail_size(),
            contents_ident: image.into(),
        });
        current.x += self.get_thumbnail_size() + 5;
        current.y += 5;
        for i in texts {
            self.queue.push(DrawCall::DrawText {
                x: current.x,
                y: current.y,
                size: text_height,
                contents: i,
                color: self.theme.text_color,
            });
            current.y += text_height;
        }
        self.update_cursor(Rect {
            x: pos.x,
            y: pos.y,
            w,
            h,
        });

        self.user_input.mouse_x() >= pos.x
            && self.user_input.mouse_y() >= pos.y
            && self.user_input.mouse_x() < pos.x + w
            && self.user_input.mouse_y() < pos.y + h
            && self.user_input.left_mouse_released()
    }

    pub fn draw_button_image(&mut self, w: i32, text_height: i32, text: &str, image: &str) -> bool {
        self.draw_button_image_exp(self.padding_x, self.padding_y, w, text_height, text, image)
    }

    pub fn draw_button_image_scroll_box_exp<'a, T>(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        text_height: i32,
        amount: f32,
        upside_down: bool,
        objects: &[T],
        as_string: impl Fn(&T) -> String,
        as_image: impl Fn(&T) -> String,
    ) -> (f32, Option<usize>) {
        let base = self.get_absolute_pos(Pos2 { x, y });
        let mut height = 0;
        let strings: Vec<(i32, Vec<String>, String)> = objects
            .iter()
            .map(|i| {
                let (mut dh, cons) = self.text_get_height_and_lines(
                    &as_string(i),
                    text_height,
                    w - (self.get_thumbnail_size() + 5),
                );
                if dh < self.get_thumbnail_size() {
                    dh = self.get_thumbnail_size();
                }
                height += dh + 10;
                (dh + 5, cons, as_image(i))
            })
            .collect();
        self.queue.push(DrawCall::Rectangle {
            x: base.x,
            y: base.y,
            w,
            h,
            color: self.theme.object_color,
            drop_shadow: self.theme.shadows,
            outline: self.theme.outline,
        });
        self.queue.push(DrawCall::Scissor {
            x: base.x,
            y: base.y,
            h,
            w,
        });
        let delta = height as f32 * amount;
        let base_y = if !upside_down {
            base.y as f32 - delta
        } else {
            base.y as f32 - delta + h as f32 - text_height as f32
        };
        let mut y = base_y;
        let mut idx = 0;
        let mut hit = None;
        for i in strings {
            let pos = Pos2 {
                x: base.x,
                y: y as i32,
            };
            if !(y as i32 + i.0 < base.y || (y as i32) > base.y + h) {
                let did_hit = self.user_input.mouse_x() >= pos.x
                    && self.user_input.mouse_y() >= pos.y
                    && self.user_input.mouse_x() < pos.x + w
                    && self.user_input.mouse_y() < pos.y + i.0;
                let col = if did_hit && self.user_input.left_mouse_down() {
                    self.theme.object_pressed_color
                } else {
                    self.theme.object_color
                };
                self.queue.push(DrawCall::Rectangle {
                    x: pos.x,
                    y: pos.y,
                    w: w - 11,
                    h: i.0,
                    color: col,
                    drop_shadow: self.theme.shadows,
                    outline: self.theme.outline,
                });
                self.queue.push(DrawCall::DrawImage {
                    x: pos.x,
                    y: pos.y + (i.0 - self.get_thumbnail_size()) / 2,
                    w: self.get_thumbnail_size(),
                    h: self.get_thumbnail_size(),
                    contents_ident: i.2,
                });
                let mut current = pos;
                current.x += self.get_thumbnail_size() + 5;
                current.y += 2;
                for i in i.1 {
                    self.queue.push(DrawCall::DrawText {
                        x: current.x,
                        y: current.y,
                        size: text_height,
                        contents: i,
                        color: self.theme.text_color,
                    });
                    current.y += text_height;
                }

                if did_hit && self.user_input.left_mouse_released() {
                    hit = Some(idx);
                }
            }
            if !upside_down {
                y += i.0 as f32 + 5.0;
            } else {
                y -= i.0 as f32 + 5.0;
            }
            idx += 1;
        }
        let bx = Rect {
            x: base.x + w - 8,
            y: base.y + ((h as f32 - 16.0) * amount) as i32,
            w: 12,
            h: 16,
        };
        self.queue.push(DrawCall::Rectangle {
            x: bx.x,
            y: bx.y,
            w: bx.w,
            h: bx.h,
            color: self.theme.decoration_color,
            drop_shadow: false,
            outline: true,
        });
        self.queue.push(DrawCall::DrawVectors {
            points: vec![
                (
                    Pos2 { x: bx.x, y: base.y },
                    BColor::from_rl_color(Color::DARKGRAY),
                ),
                (
                    Pos2 {
                        x: bx.x,
                        y: base.y + h,
                    },
                    BColor::from_rl_color(Color::DARKGRAY),
                ),
            ],
            width: 1.0,
        });
        self.queue.push(DrawCall::EndScissor);
        let hovered = self.user_input.mouse_x() >= base.x
            && self.user_input.mouse_y() >= base.y
            && self.user_input.mouse_x() < base.x + w
            && self.user_input.mouse_y() < base.y + h;
        let mut out = amount;
        let bx = Rect {
            x: bx.x - 5,
            y: bx.y - 5,
            w: bx.w + 10,
            h: bx.h + 10,
        };
        if self.user_input.left_mouse_down()
            && bx.check_collision(Pos2 {
                x: self.user_input.mouse_x(),
                y: self.user_input.mouse_y(),
            })
        {
            out += self.user_input.mouse_dy() / h as f32 * 1.5;
        } else if hovered {
            out -= self.user_input.scroll_amount() / h as f32;
        }
        self.update_cursor(Rect {
            x: base.x,
            y: base.y,
            w,
            h,
        });
        (out.clamp(0.0, 1.0), hit)
    }

    pub fn draw_button_image_scroll_box_saved_exp<T>(
        &mut self,
        name: &str,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        text_height: i32,
        upside_down: bool,
        objects: &[T],
        as_string: impl Fn(&T) -> String,
        as_image: impl Fn(&T) -> String,
    ) -> Option<usize> {
        let amnt = if let Some(k) = self.scroll_box_data.get(name) {
            *k
        } else {
            self.scroll_box_data.insert(name.to_string(), 0.0);
            0.0
        };
        let (a, hit) = self.draw_button_image_scroll_box_exp(
            x,
            y,
            w,
            h,
            text_height,
            amnt,
            upside_down,
            objects,
            as_string,
            as_image,
        );
        *self.scroll_box_data.get_mut(name).unwrap() = a;
        hit
    }

    pub fn draw_button_scroll_box<T>(
        &mut self,
        name: &str,
        w: i32,
        h: i32,
        text_height: i32,
        upside_down: bool,
        objects: &[T],
        as_string: impl Fn(&T) -> String,
    ) -> Option<usize> {
        self.draw_button_scroll_box_saved_exp(
            name,
            self.padding_x,
            self.padding_y,
            w,
            h,
            text_height,
            upside_down,
            objects,
            as_string,
        )
    }

    pub fn draw_text_scroll_box<T>(
        &mut self,
        name: &str,
        w: i32,
        h: i32,
        text_height: i32,
        upside_down: bool,
        objects: &[T],
        as_string: impl Fn(&T) -> String,
    ) {
        self.draw_text_scroll_box_saved_exp(
            name,
            self.padding_x,
            self.padding_y,
            w,
            h,
            text_height,
            upside_down,
            objects,
            as_string,
        );
    }

    pub fn draw_button_image_scroll_box<T>(
        &mut self,
        name: &str,
        w: i32,
        h: i32,
        text_height: i32,
        upside_down: bool,
        objects: &[T],
        as_string: impl Fn(&T) -> String,
        as_image: impl Fn(&T) -> String,
    ) -> Option<usize> {
        self.draw_button_image_scroll_box_saved_exp(
            name,
            self.padding_x,
            self.padding_y,
            w,
            h,
            text_height,
            upside_down,
            objects,
            as_string,
            as_image,
        )
    }

    pub fn get_theme(&self) -> Theme {
        self.theme
    }

    pub fn get_theme_ref(&self) -> &Theme {
        &self.theme
    }

    pub fn get_theme_mut(&mut self) -> &mut Theme {
        &mut self.theme
    }
    pub fn send_draw_calls(&mut self, calls: Vec<DrawCall>, bounds: Rect) {
        for i in calls {
            self.queue.push(i);
        }
        self.update_cursor(bounds);
    }
}
