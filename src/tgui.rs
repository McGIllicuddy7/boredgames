use std::{cell::Cell, fmt::Debug, rc::Rc};

use raylib::{
    color::Color,
    ffi::MouseButton,
    math::{Rectangle, Vector2},
    prelude::{RaylibDraw, RaylibDrawHandle, RaylibScissorModeExt},
};

pub fn scale_x(handle: &mut RaylibDrawHandle) -> f32 {
    16.0 * (handle.get_screen_width() as f32) / 1000.0
}
pub fn scale_y(handle: &mut RaylibDrawHandle) -> f32 {
    20.0 * (handle.get_screen_height() as f32) / 1000.0
}

#[derive(Clone)]
pub struct TGuiOutput<T> {
    output: Rc<Cell<Option<T>>>,
}

impl<T: Debug> Debug for TGuiOutput<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x = self.output.take();
        let out = f.debug_struct("TGuiOutput").field("output", &x).finish();
        self.output.set(x);
        out
    }
}

impl<T> Default for TGuiOutput<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> TGuiOutput<T> {
    pub fn new() -> Self {
        Self {
            output: Rc::new(Cell::new(None)),
        }
    }
    pub fn take(&self) -> Option<T> {
        self.output.take()
    }
    pub fn send(&self, v: T) {
        self.output.set(Some(v));
    }
}

pub trait GuiObject: Debug {
    fn as_clone(&self) -> Box<dyn GuiObject>;
    fn shift(&mut self, amount: i32, vertical: bool);
    fn update_bounds(&mut self, bounds: Boundary);
    fn draw(&mut self, handle: &mut RaylibDrawHandle);
}
impl Clone for Box<dyn GuiObject> {
    fn clone(&self) -> Self {
        self.as_clone()
    }
}

#[derive(Clone, Debug)]
pub enum TGuiDraw {
    DrawString {
        string: String,
        x: i32,
        y: i32,
        max_width: i32,
        color: Color,
    },
    DrawBox {
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        color: Color,
        final_bounds: TGuiOutput<ComputedBoundary>,
    },
    DrawButton {
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        color: Color,
        pressed: TGuiOutput<bool>,
        text: String,
    },
    Container {
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        children: Vec<TGuiDraw>,
        vertical: bool,
        padding_x: i32,
        padding_y: i32,
        color: Color,
    },
    ScrollBox {
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        children: Vec<TGuiDraw>,
        scroll_amount: TGuiOutput<i32>,
        current_scroll_amount: i32,
        padding_x: i32,
        padding_y: i32,
        color: Color,
        upside_down: bool,
    },
    BoxedGuiObject {
        obj: Box<dyn GuiObject>,
    },
    Image {
        path: String,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct Boundary {
    pub x: i32,
    pub y: i32,
    pub h: i32,
    pub w: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct ComputedBoundary {
    pub gui_coords: Boundary,
    pub pixel_coords: Boundary,
}

pub fn get_string_bounds(s: &str, x: i32, y: i32, width: i32) -> Boundary {
    let mut dx = x;
    let w = x + width;
    let mut dy = y;
    let mut max_x = 0;
    let mut max_y = 0;
    for i in s.chars() {
        if i != '\n' {
            dx += 1;
            if dx >= w {
                dx = x;
                dy += 1;
            }
        } else {
            dx = x;
            dy += 1;
        }
        if dx > max_x {
            max_x = dx;
        }
        if dy > max_y {
            max_y = dy;
        }
    }
    Boundary {
        x,
        y,
        h: (max_y - y),
        w: max_x - x + 2,
    }
}

impl TGuiDraw {
    pub fn get_min_boundary(&self) -> Boundary {
        match self {
            TGuiDraw::DrawString {
                string,
                x,
                y,
                max_width,
                color: _,
            } => get_string_bounds(string, *x, *y, *max_width),
            TGuiDraw::DrawBox {
                x,
                y,
                w,
                h,
                color: _,
                final_bounds: _,
            } => Boundary {
                x: *x,
                y: *y,
                h: *h,
                w: *w,
            },
            TGuiDraw::DrawButton {
                x,
                y,
                w,
                h,
                color: _,
                pressed: _,
                text,
            } => {
                let by = get_string_bounds(text, *x + 1, *y + 1, *w - 1);
                let b_y = if by.h > *h { by.h } else { *h };

                Boundary {
                    x: *x,
                    y: *y,
                    h: b_y,
                    w: *w,
                }
            }
            TGuiDraw::Container {
                x,
                y,
                w,
                h,
                children,
                vertical,
                padding_x,
                padding_y,
                color: _,
            } => {
                let mut dw = *w;
                let mut dh = *h;
                for i in children {
                    let bs = i.get_min_boundary();
                    let tdw = bs.x + bs.w - *x + padding_x;
                    let tdh = bs.y + bs.h - *y + padding_y;
                    if *vertical {
                        if tdw > dw {
                            dw = tdw;
                        }
                        dh += tdh;
                    } else {
                        if tdh > dh {
                            dh = tdh;
                        }
                        dw += tdw;
                    }
                }
                dw += padding_x;
                dh += padding_y;
                /*if dw < w - padding_x * 2 {
                    dw = w - padding_x * 2;
                }
                if dh < w - padding_x * 2 {
                    dh = h- padding_y* 2;
                }*/
                Boundary {
                    x: *x,
                    y: *y,
                    h: dh,
                    w: dw,
                }
            }
            TGuiDraw::ScrollBox {
                x,
                y,
                w,
                h,
                children,
                scroll_amount: _,
                color: _,
                padding_x,
                padding_y: _,
                current_scroll_amount: _,
                upside_down: _,
            } => {
                let mut min_w = *w;
                for i in children {
                    let bounds = i.get_min_boundary();
                    if bounds.w + 2 * padding_x + 1 > min_w {
                        min_w = bounds.w + 2 * padding_x + 1;
                    }
                }
                Boundary {
                    x: *x,
                    y: *y,
                    h: *h,
                    w: min_w,
                }
            }
            TGuiDraw::BoxedGuiObject { obj: _ } => {
                todo!()
            }
            TGuiDraw::Image {
                x,
                y,
                h,
                w,
                path: _,
            } => Boundary {
                x: *x,
                y: *y,
                h: *h,
                w: *w,
            },
        }
    }

    pub fn update_bounds(&mut self, b: Boundary) {
        match self {
            TGuiDraw::DrawString {
                string: _,
                x,
                y,
                max_width,
                color: _,
            } => {
                *x = b.x;
                *y = b.y;
                *max_width = b.w;
            }
            TGuiDraw::DrawBox {
                x,
                y,
                w,
                h,
                color: _,
                final_bounds: _,
            } => {
                *x = b.x;
                *y = b.y;
                *w = b.w;
                *h = b.h;
            }
            TGuiDraw::DrawButton {
                x,
                y,
                w,
                h: _,
                color: _,
                pressed: _,
                text: _,
            } => {
                *x = b.x;
                *y = b.y;
                *w = b.w;
            }
            TGuiDraw::Container {
                x,
                y,
                w,
                h,
                children,
                vertical,
                padding_x,
                padding_y,
                color: _,
            } => {
                *x = b.x;
                *y = b.y;
                *w = b.w;
                *h = b.h;
                let mut bb = b;
                bb.x += *padding_x;
                bb.y += *padding_y;
                bb.h -= *padding_y * 2;
                bb.w -= *padding_x * 2;
                set_bounds(children, bb, *vertical);
            }
            TGuiDraw::ScrollBox {
                x,
                y,
                w,
                h: _,
                children,
                scroll_amount: _,
                color: _,
                padding_x: _,
                padding_y: _,
                current_scroll_amount: _,
                upside_down: _,
            } => {
                let dx = b.x - *x;
                for i in children {
                    i.shift(dx, false);
                }
                *x = b.x;
                *y = b.y;
                *w = b.w;
            }
            TGuiDraw::BoxedGuiObject { obj } => {
                obj.update_bounds(b);
            }
            TGuiDraw::Image {
                path: _,
                x,
                y,
                w,
                h,
            } => {
                *x = b.x;
                *y = b.y;
                *w = b.w;
                *h = b.h;
            }
        }
    }
    pub fn draw(&mut self, draw_handle: &mut RaylibDrawHandle) {
        match self {
            TGuiDraw::DrawString {
                string,
                x,
                y,
                max_width,
                color,
            } => {
                draw_string(draw_handle, string, *x, *y, *max_width, *color);
            }
            TGuiDraw::DrawBox {
                x,
                y,
                w,
                h,
                color,
                final_bounds,
            } => {
                draw_rectangle(draw_handle, *x, *y, *w, *h, *color);
                let bbase = Boundary {
                    x: *x,
                    y: *y,
                    h: *h,
                    w: *w,
                };
                let sx = scale_x(draw_handle);
                let sy = scale_y(draw_handle);
                let b_comp = Boundary {
                    x: (*x as f32 * sx) as i32,
                    w: (*w as f32 * sx) as i32,
                    y: (*y as f32 * sy) as i32,
                    h: (*h as f32 * sy) as i32,
                };
                final_bounds.send(ComputedBoundary {
                    gui_coords: bbase,
                    pixel_coords: b_comp,
                });
            }
            TGuiDraw::DrawButton {
                x,
                y,
                w,
                h,
                color,
                pressed,
                text,
            } => {
                draw_rectangle(draw_handle, *x, *y, *w, *h, *color);
                draw_string(draw_handle, text, *x + 1, *y + 1, *w - 2, *color);
                let rct = Rectangle {
                    x: *x as f32 * scale_x(draw_handle),
                    y: *y as f32 * scale_y(draw_handle),
                    width: *w as f32 * scale_x(draw_handle),
                    height: *h as f32 * scale_y(draw_handle),
                };
                let cy = draw_handle.get_mouse_position();
                let col = rct.y <= cy.y
                    && rct.x <= cy.x
                    && rct.y + rct.height > cy.y
                    && rct.x + rct.width > cy.x;
                let out =
                    col && draw_handle.is_mouse_button_released(MouseButton::MOUSE_BUTTON_LEFT);
                pressed.send(out);
            }
            TGuiDraw::Container {
                x,
                y,
                w,
                h,
                children,
                vertical: _,
                padding_x: _,
                padding_y: _,
                color,
            } => {
                draw_rectangle(draw_handle, *x, *y, *w, *h, *color);
                for i in children {
                    i.draw(draw_handle);
                }
            }
            TGuiDraw::ScrollBox {
                x,
                y,
                w,
                h,
                children,
                scroll_amount,
                current_scroll_amount,
                color,
                padding_x: _,
                padding_y: _,
                upside_down,
            } => {
                draw_rectangle(draw_handle, *x, *y, *w, *h, *color);
                let mut sy = (*h as f32 * *current_scroll_amount as f32 / 1000.0) as i32;
                if *upside_down {
                    sy = *h - sy - 1;
                }
                draw_rectangle(draw_handle, *x + *w - 1, *y + sy, 1, 1, *color);
                //:3
                let sx = scale_x(draw_handle);
                let sy = scale_y(draw_handle);
                let mut scissoring = draw_handle.begin_scissor_mode(
                    *x * sx as i32,
                    *y * sy as i32,
                    *w * sx as i32,
                    *h * sy as i32,
                );
                for i in children {
                    let b = i.get_min_boundary();
                    if b.y + b.h < *y || b.y >= *y + *h {
                        continue;
                    }
                    i.draw(&mut scissoring);
                }
                drop(scissoring);
                let rct = Rectangle {
                    x: *x as f32 * scale_x(draw_handle),
                    y: *y as f32 * scale_y(draw_handle),
                    width: *w as f32 * scale_x(draw_handle),
                    height: *h as f32 * scale_y(draw_handle),
                };
                let cy = draw_handle.get_mouse_position();
                let col = rct.y <= cy.y
                    && rct.x <= cy.x
                    && rct.y + rct.height > cy.y
                    && rct.x + rct.width > cy.x;
                let delt = draw_handle.get_mouse_delta().y;
                if col
                    && draw_handle.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT)
                    && delt != 0.0
                {
                    let mut dact =
                        (delt / (*h as f32) * 1000.0 / scale_y(draw_handle)).ceil() as i32;
                    if !*upside_down {
                        dact *= -1;
                    }
                    let mut out = *current_scroll_amount - dact;
                    out = out.clamp(0, 1000);
                    scroll_amount.send(out);
                } else {
                    scroll_amount.send(*current_scroll_amount);
                }
            }
            TGuiDraw::BoxedGuiObject { obj } => {
                obj.draw(draw_handle);
            }
            TGuiDraw::Image {
                x: _,
                y: _,
                w: _,
                h: _,
                path: _,
            } => {
                todo!()
            }
        }
    }

    pub fn shift(&mut self, amount: i32, vertical: bool) {
        match self {
            TGuiDraw::DrawString {
                string: _,
                x,
                y,
                max_width: _,
                color: _,
            } => {
                if vertical {
                    *y += amount;
                } else {
                    *x += amount;
                }
            }
            TGuiDraw::DrawBox {
                x,
                y,
                w: _,
                h: _,
                color: _,
                final_bounds: _,
            } => {
                if vertical {
                    *y += amount;
                } else {
                    *x += amount;
                }
            }
            TGuiDraw::DrawButton {
                x,
                y,
                w: _,
                h: _,
                color: _,
                pressed: _,
                text: _,
            } => {
                if vertical {
                    *y += amount;
                } else {
                    *x += amount;
                }
            }
            TGuiDraw::Container {
                x,
                y,
                w: _,
                h: _,
                children,
                vertical: _,
                padding_x: _,
                padding_y: _,
                color: _,
            } => {
                if vertical {
                    *y += amount;
                } else {
                    *x += amount;
                }
                for i in children {
                    i.shift(amount, vertical);
                }
            }
            TGuiDraw::ScrollBox {
                x,
                y,
                w: _,
                h: _,
                children,
                scroll_amount: _,
                color: _,
                padding_x: _,
                padding_y: _,
                current_scroll_amount: _,
                upside_down: _,
            } => {
                if vertical {
                    *y += amount;
                } else {
                    *x += amount;
                }
                for i in children {
                    i.shift(amount, vertical);
                }
            }
            TGuiDraw::BoxedGuiObject { obj } => {
                obj.shift(amount, vertical);
            }
            TGuiDraw::Image {
                x,
                y,
                w: _,
                h: _,
                path: _,
            } => {
                if vertical {
                    *y += amount;
                } else {
                    *x += amount;
                }
            }
        }
    }
}

pub fn draw_string(
    handle: &mut RaylibDrawHandle,
    string: &str,
    x: i32,
    y: i32,
    max_width: i32,
    color: Color,
) {
    let mut dx = x;
    let w = x + max_width;
    let mut dy = y;
    let fnt = handle.get_font_default();
    let sx = scale_x(handle);
    let sy = scale_y(handle);
    for i in string.chars() {
        if i != '\n' {
            handle.draw_text_codepoint(
                &fnt,
                i as i32,
                Vector2::new(dx as f32 * sx, dy as f32 * sy),
                sy,
                color,
            );
            dx += 1;
            if dx >= w {
                dx = x;
                dy += 1;
            }
        } else {
            dx = x;
            dy += 1;
        }
    }
}

pub fn draw_rectangle(handle: &mut RaylibDrawHandle, x: i32, y: i32, w: i32, h: i32, color: Color) {
    let sx = scale_x(handle);
    let sy = scale_y(handle);
    let p0 = Vector2::new(x as f32 * sx, y as f32 * sy);
    let p1 = Vector2::new(x as f32 * sx + w as f32 * sx, y as f32 * sy);
    let p2 = Vector2::new(x as f32 * sx, y as f32 * sy + h as f32 * sy);
    let p3 = Vector2::new(x as f32 * sx + w as f32 * sx, y as f32 * sy + h as f32 * sy);
    handle.draw_line_ex(p0, p1, 1.0, color);
    handle.draw_line_ex(p0, p2, 1.0, color);
    handle.draw_line_ex(p3, p1, 1.0, color);
    handle.draw_line_ex(p3, p2, 1.0, color);
}

pub fn set_bounds(bs: &mut [TGuiDraw], b: Boundary, vertical: bool) {
    if bs.is_empty() {
        return;
    }
    let mut used_w = 0;
    let mut used_h = 0;
    for i in bs.iter_mut() {
        let bx = i.get_min_boundary();
        if vertical {
            if bx.w > used_w {
                used_w = bx.w;
            }
            used_h += bx.h;
        } else {
            if bx.h > used_h {
                used_h = bx.h;
            }
            used_w += bx.w;
        }
    }
    let remaining_h = if b.h > used_h { b.h - used_h } else { 0 };
    let remaining_w = if b.w > used_w { b.w - used_w } else { 0 };
    let mut extra_h_per = if vertical {
        remaining_h / (bs.len() as i32 + 1)
    } else {
        remaining_h
    };
    let mut extra_w_per = if !vertical {
        remaining_w / (bs.len() as i32 + 1)
    } else {
        remaining_w
    };
    if extra_h_per > 4 {
        extra_h_per = 4;
    }
    if extra_w_per > 4 {
        extra_w_per = 4;
    }
    let mut x_coord = b.x;
    let mut y_coord = b.y;
    for i in bs.iter_mut() {
        let mut bounds = i.get_min_boundary();
        if vertical {
            y_coord += extra_h_per / 2;
            let delt = bounds.y - y_coord;
            i.shift(-delt, true);
            bounds.y = y_coord;
            bounds.h += extra_h_per / 2;
            bounds.x += extra_w_per / 2;
            i.shift(-extra_w_per / 2, false);
            bounds.w += extra_w_per / 2;
            y_coord += bounds.h;
        } else {
            x_coord += extra_w_per / 2;
            let delt = bounds.x - x_coord;
            i.shift(-delt, false);
            bounds.x = x_coord;
            bounds.w += extra_w_per / 2;
            bounds.h += extra_h_per / 2;
            i.shift(-extra_h_per / 2, true);
            bounds.y += extra_h_per / 2;
            x_coord += bounds.w;
        }
        i.update_bounds(bounds);
    }
}

#[derive(Debug)]
pub struct Div {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub draw_calls: Vec<TGuiDraw>,
    pub vertical: bool,
    pub padding_x: i32,
    pub padding_y: i32,
    pub fg_color: Color,
    pub bg_color: Color,
    pub scroll_box: Option<TGuiOutput<i32>>,
    pub scroll_amount: i32,
    pub upside_down: bool,
}

impl Div {
    pub fn bounds(&self) -> Boundary {
        let mut bs = Boundary {
            x: self.x,
            y: self.y,
            h: 0,
            w: 0,
        };
        for i in &self.draw_calls {
            let tmp = i.get_min_boundary();
            if tmp.x < bs.x {
                bs.x = tmp.x;
            }
            if tmp.y < bs.y {
                bs.y = tmp.y;
            }
            if tmp.x + tmp.w > bs.x + bs.w {
                bs.w = tmp.w + tmp.x - bs.x;
            }
            if tmp.y + tmp.h > bs.y + bs.h {
                bs.h = tmp.h + tmp.y - bs.y;
            }
        }
        bs
    }
}
pub struct TGui {
    pub draw_calls: Vec<TGuiDraw>,
    pub draw_call_stack: Vec<Div>,
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub w: i32,
    pub h: i32,
}

impl Default for TGui {
    fn default() -> Self {
        Self::new()
    }
}

impl TGui {
    pub fn new() -> Self {
        Self {
            draw_calls: Vec::new(),
            draw_call_stack: Vec::new(),
            cursor_x: 0,
            cursor_y: 0,
            w: 1000 / 16,
            h: 1000 / 20,
        }
    }

    pub fn get_padding_x(&mut self) -> i32 {
        1
    }

    pub fn get_padding_y(&mut self) -> i32 {
        1
    }

    pub fn begin_div(&mut self) {
        let fg_color = self.get_fg_color();
        let bg_color = self.get_bg_color();
        self.draw_call_stack.push(Div {
            x: self.cursor_x,
            y: self.cursor_y,
            w: 0,
            h: 0,
            draw_calls: Vec::new(),
            vertical: true,
            padding_x: 1,
            padding_y: 1,
            fg_color,
            bg_color,
            scroll_box: None,
            scroll_amount: 0,
            upside_down: false,
        });
        self.cursor_x += 1;
        self.cursor_y += 1;
    }

    pub fn begin_div_hor(&mut self) {
        let fg_color = self.get_fg_color();
        let bg_color = self.get_bg_color();
        self.draw_call_stack.push(Div {
            x: self.cursor_x,
            y: self.cursor_y,
            w: 0,
            h: 0,
            draw_calls: Vec::new(),
            vertical: false,
            padding_x: 1,
            padding_y: 1,
            fg_color,
            bg_color,
            scroll_box: None,
            scroll_amount: 0,
            upside_down: false,
        });
        //  self.cursor_x += 1;
        // self.cursor_y += 1;
    }

    pub fn begin_div_at(&mut self, x: i32, y: i32) {
        self.cursor_x = x;
        self.cursor_y = y;
        let fg_color = self.get_fg_color();
        let bg_color = self.get_bg_color();
        self.draw_call_stack.push(Div {
            x: self.cursor_x,
            y: self.cursor_y,
            w: 0,
            h: 0,
            draw_calls: Vec::new(),
            vertical: true,
            padding_x: 1,
            padding_y: 1,
            fg_color,
            bg_color,
            scroll_box: None,
            scroll_amount: 0,
            upside_down: false,
        });
        //self.cursor_x += 1;
        //self.cursor_y += 1;
    }

    pub fn begin_div_hor_at(&mut self, x: i32, y: i32) {
        self.cursor_x = x;
        self.cursor_y = y;
        let fg_color = self.get_fg_color();
        let bg_color = self.get_bg_color();
        self.draw_call_stack.push(Div {
            x: self.cursor_x,
            y: self.cursor_y,
            w: 0,
            h: 0,
            draw_calls: Vec::new(),
            vertical: false,
            padding_x: 1,
            padding_y: 1,
            fg_color,
            bg_color,
            scroll_box: None,
            scroll_amount: 0,
            upside_down: false,
        });
        self.cursor_x += 1;
        self.cursor_y += 1;
    }

    pub fn end_div(&mut self) {
        let mut x = self.draw_call_stack.pop().unwrap();
        let bounds = x.bounds();
        if let Some(sb) = x.scroll_box {
            let mut min_h = 1000000;
            let mut max_h = -1000000;
            let mut hit = false;
            for i in &x.draw_calls {
                let bounds = i.get_min_boundary();
                hit = true;
                if bounds.y + bounds.h > max_h {
                    max_h = bounds.y + bounds.h;
                }
                if bounds.y < min_h {
                    min_h = bounds.y;
                }
            }
            let dh = if hit { max_h - min_h - x.h } else { 0 };
            if let Some(mut y) = self.draw_call_stack.pop() {
                let mut shift =
                    -((dh + x.padding_x * 2) as f32 * x.scroll_amount as f32 / 1000.0) as i32;

                if x.upside_down {
                    shift += x.padding_y * 2;
                    shift *= -1;
                }

                for i in &mut x.draw_calls {
                    i.shift(shift, true);
                }
                if y.vertical {
                    self.cursor_x = bounds.x;
                    self.cursor_y = bounds.y + bounds.h;
                } else {
                    self.cursor_y = bounds.y;
                    self.cursor_x = bounds.x + bounds.w;
                }
                let call = TGuiDraw::ScrollBox {
                    x: x.x,
                    y: x.y,
                    w: x.w,
                    h: x.h,
                    children: x.draw_calls,
                    scroll_amount: sb,
                    current_scroll_amount: x.scroll_amount,
                    padding_x: x.padding_x,
                    padding_y: x.padding_y,
                    color: x.bg_color,
                    upside_down: x.upside_down,
                };
                y.draw_calls.push(call);
                self.draw_call_stack.push(y);
            } else {
                for i in x.draw_calls {
                    self.draw_calls.push(i);
                }
            }
        } else if let Some(mut y) = self.draw_call_stack.pop() {
            if y.vertical {
                self.cursor_x = bounds.x;
                self.cursor_y = bounds.y + bounds.h;
            } else {
                self.cursor_y = bounds.y;
                self.cursor_x = bounds.x + bounds.w;
            }
            let call = TGuiDraw::Container {
                x: bounds.x,
                y: bounds.y,
                w: bounds.w,
                h: bounds.h,
                children: x.draw_calls,
                vertical: x.vertical,
                padding_x: x.padding_x,
                padding_y: x.padding_y,
                color: x.bg_color,
            };
            y.draw_calls.push(call);
            self.draw_call_stack.push(y);
        } else {
            for i in x.draw_calls {
                self.draw_calls.push(i);
            }
        }
    }

    pub fn add_text(&mut self, text: impl Into<String>) {
        let txt = text.into();
        let w = if let Some(w) = self.draw_call_stack.pop() {
            if w.vertical {
                let v = w.w;
                self.draw_call_stack.push(w);
                if v > txt.len() as i32 {
                    v
                } else if txt.len() > 30 {
                    30_i32
                } else {
                    (txt.len()) as i32
                }
            } else {
                self.draw_call_stack.push(w);
                if txt.len() > 30 {
                    30_i32
                } else {
                    (txt.len()) as i32
                }
            }
        } else if txt.len() > 30 {
            30_i32
        } else {
            (txt.len()) as i32
        };
        let mut div = self.draw_call_stack.pop().unwrap();
        let bounds = get_string_bounds(&txt, self.cursor_x, self.cursor_y, w);
        div.draw_calls.push(TGuiDraw::DrawString {
            string: txt,
            x: self.cursor_x,
            y: self.cursor_y,
            max_width: w,
            color: div.fg_color,
        });
        if div.vertical {
            if div.upside_down {
                self.cursor_y -= bounds.h + div.padding_y;
            } else {
                self.cursor_y += bounds.h + div.padding_y;
            }
        } else {
            self.cursor_x += bounds.w + div.padding_x;
        }
        self.draw_call_stack.push(div);
    }

    pub fn add_box(&mut self, w: i32, h: i32) -> TGuiOutput<ComputedBoundary> {
        let out = TGuiOutput::new();
        let mut div = self.draw_call_stack.pop().unwrap();
        div.draw_calls.push(TGuiDraw::DrawBox {
            x: self.cursor_x,
            y: self.cursor_y,
            w,
            h,
            color: div.fg_color,
            final_bounds: out.clone(),
        });
        if div.vertical {
            if div.upside_down {
                self.cursor_y -= h + div.padding_y;
            } else {
                self.cursor_y += h + div.padding_y;
            }
        } else {
            self.cursor_x += w + div.padding_x;
        }
        self.draw_call_stack.push(div);
        out
    }

    pub fn add_button(&mut self, w: i32, h: i32, text: impl Into<String>) -> TGuiOutput<bool> {
        let mut div = self.draw_call_stack.pop().unwrap();
        let out = TGuiOutput::new();
        out.send(false);
        div.draw_calls.push(TGuiDraw::DrawButton {
            x: self.cursor_x,
            y: self.cursor_y,
            w,
            h,
            color: div.fg_color,
            text: text.into(),
            pressed: out.clone(),
        });
        if div.vertical {
            if div.upside_down {
                self.cursor_y -= h + div.padding_y;
            } else {
                self.cursor_y += h + div.padding_y;
            }
        } else {
            self.cursor_x += w + div.padding_x;
        }

        self.draw_call_stack.push(div);
        out
    }

    pub fn begin_scrollbox(&mut self, w: i32, h: i32, amount: i32) -> TGuiOutput<i32> {
        let out = TGuiOutput::new();
        out.send(0);
        let out_act = out.clone();
        let dv = Div {
            x: self.cursor_x,
            y: self.cursor_y,
            w,
            h,
            draw_calls: Vec::new(),
            vertical: true,
            padding_x: 1,
            padding_y: 1,
            fg_color: self.get_fg_color(),
            bg_color: self.get_bg_color(),
            scroll_box: Some(out),
            scroll_amount: amount,
            upside_down: false,
        };
        self.cursor_x += 1;
        self.cursor_y += 1;
        self.draw_call_stack.push(dv);
        out_act
    }

    pub fn get_bg_color(&mut self) -> Color {
        if let Some(x) = self.draw_call_stack.pop() {
            let out = x.bg_color;
            self.draw_call_stack.push(x);
            out
        } else {
            Color::BLACK
        }
    }
    pub fn get_fg_color(&mut self) -> Color {
        if let Some(x) = self.draw_call_stack.pop() {
            let out = x.fg_color;
            self.draw_call_stack.push(x);
            out
        } else {
            Color::GREEN
        }
    }

    pub fn begin_frame(&mut self) {
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.draw_calls.clear();
        self.draw_call_stack.clear();
        self.begin_div_hor();
        self.set_div_dims(self.w, self.h);
    }

    pub fn draw_frame(&mut self, draw_handle: &mut RaylibDrawHandle) {
        self.end_div();
        assert!(self.draw_call_stack.is_empty());
        set_bounds(
            &mut self.draw_calls,
            Boundary {
                x: 0,
                y: 0,
                h: self.h,
                w: self.w,
            },
            false,
        );
        for i in &mut self.draw_calls {
            i.draw(draw_handle);
        }
        self.draw_call_stack.clear();
        self.draw_calls.clear();
    }

    pub fn set_fg_color(&mut self, color: Color) {
        let mut x = self.draw_call_stack.pop().unwrap();
        x.fg_color = color;
        self.draw_call_stack.push(x);
    }

    pub fn set_bg_color(&mut self, color: Color) {
        let mut x = self.draw_call_stack.pop().unwrap();
        x.bg_color = color;
        self.draw_call_stack.push(x);
    }

    pub fn set_upside_down(&mut self) {
        let mut x = self.draw_call_stack.pop().unwrap();
        x.upside_down = true;
        self.cursor_y = x.y + x.h - x.padding_y;
        self.draw_call_stack.push(x);
    }

    pub fn set_rightside_up(&mut self) {
        let mut x = self.draw_call_stack.pop().unwrap();
        x.upside_down = false;
        self.draw_call_stack.push(x);
    }

    pub fn set_padding(&mut self, dx: i32, dy: i32) {
        let mut x = self.draw_call_stack.pop().unwrap();
        if x.draw_calls.is_empty() {
            self.cursor_x = x.x + dx;
            self.cursor_y = x.y + dy;
        }
        x.padding_x = dx;
        x.padding_y = dy;
        self.draw_call_stack.push(x);
    }

    pub fn set_div_dims(&mut self, w: i32, h: i32) {
        let mut x = self.draw_call_stack.pop().unwrap();
        x.w = w;
        x.h = h;
        self.draw_call_stack.push(x);
    }

    pub fn set_cursor(&mut self, x: i32, y: i32) {
        self.cursor_x = x;
        self.cursor_y = y;
    }
}
