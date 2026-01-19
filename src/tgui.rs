use std::{any::Any, cell::Cell, collections::{BTreeMap, HashMap, HashSet}, fmt::Debug, rc::Rc, sync::{Arc, Mutex}};

use raylib::{
    color::Color,
    ffi::MouseButton,
    math::{Rectangle, Vector2},
    prelude::{RaylibDraw, RaylibDrawHandle, RaylibScissorModeExt},
};

use crate::{state::{Exception, Immutable, Throws}, throw};
pub const SCALE_X: f32 = 16.;
pub const SCALE_Y: f32 = 20.;
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


pub trait GuiObject:Debug{
    fn as_clone(&self)->Box<dyn GuiObject>;
    fn shift(&mut self, amount:i32, vertical: bool);
    fn update_bounds(&mut self, bounds:Boundary);
    fn draw(&mut self, handle:&mut RaylibDrawHandle);
}
impl Clone for Box<dyn GuiObject>{
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
    BoxedGuiObject{
        obj:Box<dyn GuiObject>,
    }
}

#[derive(Clone, Debug)]
pub struct Boundary {
    pub x: i32,
    pub y: i32,
    pub h: i32,
    pub w: i32,
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
        w: max_x - x,
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
                dw += padding_x * 2;
                dh += padding_y * 2;
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
            TGuiDraw::BoxedGuiObject {obj:_  }=>{
                todo!()
            }
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
                for i in children{
                    i.shift(dx, false);
                }
                *x = b.x;
                *y = b.y;
                *w = b.w;
            }
            TGuiDraw::BoxedGuiObject { obj }=>{
                obj.update_bounds(b);
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
            TGuiDraw::DrawBox { x, y, w, h, color } => {
                draw_rectangle(draw_handle, *x, *y, *w, *h, *color);
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
                    x: *x as f32 * SCALE_X,
                    y: *y as f32 * SCALE_Y,
                    width: *w as f32 * SCALE_X,
                    height: *h as f32 * SCALE_Y,
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
                let mut scissoring = draw_handle.begin_scissor_mode(*x*SCALE_X as i32, *y*SCALE_Y as i32, *w *SCALE_X as i32, *h*SCALE_Y as i32);
                for i in children {
                    let b = i.get_min_boundary();
                    if b.y+b.h < *y || b.y >= *y + *h {
                        continue;
                    }
                    i.draw(&mut scissoring);
                }
                drop(scissoring);
                let rct = Rectangle {
                    x: *x as f32 * SCALE_X,
                    y: *y as f32 * SCALE_Y,
                    width: *w as f32 * SCALE_X,
                    height: *h as f32 * SCALE_Y,
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
                    let mut dact = (delt / (*h as f32) * 1000.0 / SCALE_Y).ceil() as i32;
                    if  !*upside_down{
                        dact*= -1;
                    }
                    let mut out = *current_scroll_amount - dact;
                    if out > 999 {
                        out = 999;
                    }
                    if out < 0 {
                        out = 0;
                    }
                    scroll_amount.send(out);
                } else {
                    scroll_amount.send(*current_scroll_amount);
                }
            }
            TGuiDraw::BoxedGuiObject { obj
             }=>{
                obj.draw(draw_handle);
            }
        }
    }
    pub fn shift(&mut self, amount: i32, vertical: bool) {
        match self {
            TGuiDraw::DrawString {
                string: _,
                x ,
                y,
                max_width: _,
                color: _,
            } => {
                if vertical{
                    *y += amount;
                }else{
                    *x += amount;
                }
            }
            TGuiDraw::DrawBox {
                x,
                y,
                w: _,
                h: _,
                color: _,
            } => {
                if vertical{
                    *y += amount;
                }else{
                    *x += amount;
                }
            }
            TGuiDraw::DrawButton {
                x ,
                y,
                w: _,
                h: _,
                color: _,
                pressed: _,
                text: _,
            } => {
                if vertical{
                    *y += amount;
                }else{
                    *x += amount;
                }
            }
            TGuiDraw::Container {
                x ,
                y,
                w: _,
                h: _,
                children,
                vertical: _,
                padding_x: _,
                padding_y: _,
                color: _,
            } => {
                if vertical{
                    *y += amount;
                }else{
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
              if vertical{
                    *y += amount;
                }else{
                    *x += amount;
                }
                for i in children {
                    i.shift(amount, vertical);
                }
            }
            TGuiDraw::BoxedGuiObject {  obj}=>{
                obj.shift(amount, vertical);
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
    // println!("{}:{}, {}", string, x, y);
    let mut dx = x;
    let w = x + max_width;
    let mut dy = y;
    let fnt = handle.get_font_default();
    for i in string.chars() {
        if i != '\n' {
            handle.draw_text_codepoint(
                &fnt,
                i as i32,
                Vector2::new(dx as f32 * SCALE_X, dy as f32 * SCALE_Y),
                SCALE_Y,
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
    let p0 = Vector2::new(x as f32 * SCALE_X, y as f32 * SCALE_Y);
    let p1 = Vector2::new(x as f32 * SCALE_X + w as f32 * SCALE_X, y as f32 * SCALE_Y);
    let p2 = Vector2::new(x as f32 * SCALE_X, y as f32 * SCALE_Y + h as f32 * SCALE_Y);
    let p3 = Vector2::new(
        x as f32 * SCALE_X + w as f32 * SCALE_X,
        y as f32 * SCALE_Y + h as f32 * SCALE_Y,
    );
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
        remaining_h / (bs.len() as i32+1)
    } else {
        remaining_h
    };
    let mut extra_w_per = if !vertical {
        remaining_w / (bs.len() as i32+1)
    } else {
        remaining_w
    };
    if extra_h_per>4{
        extra_h_per = 4;
    }
    if extra_w_per>4{
        extra_w_per = 4;
    }
    let mut x_coord = b.x;
    let mut y_coord = b.y;
    for i in bs.iter_mut() {
        let mut bounds = i.get_min_boundary();
        if vertical {
            y_coord += extra_h_per/2;
            let delt = bounds.y - y_coord;
            i.shift(-delt, true);
            bounds.y = y_coord; 
            bounds.h += extra_h_per/2;
            bounds.x += extra_w_per/2;
            i.shift(-extra_w_per/2, false);
            bounds.w += extra_w_per/2;
            y_coord += bounds.h;
        } else {
            x_coord += extra_w_per/2;
            let delt = bounds.x - x_coord;
            i.shift(-delt, false);
            bounds.x = x_coord;
            bounds.w += extra_w_per/2;
            bounds.h += extra_h_per/2;
            i.shift(-extra_h_per/2, true);
            bounds.y += extra_h_per/2;
            x_coord += bounds.w;
        } 
        i.update_bounds(bounds);
      //  println!("bounds:{:#?}", i);
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

impl Div{
    pub fn bounds(&self)->Boundary{
        let mut bs =Boundary{x:self.x, y:self.y, h:0, w:0};
        for i in &self.draw_calls{
            let tmp = i.get_min_boundary();
            if tmp.x< bs.x{
                bs.x = tmp.x;
            }
            if tmp.y< bs.y{
                bs.y = tmp.y;
            }
            if tmp.x+tmp.w>bs.x+bs.w{
                bs.w = tmp.w+tmp.x-bs.x;
            }
            if tmp.y+tmp.h>bs.y+bs.h{
                bs.h = tmp.h +tmp.y-bs.y;
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
        if let Some(sb) =x.scroll_box{       
            let mut min_h = 1000000;
            let mut max_h = -1000000;
            let mut hit = false;
            for i in &x.draw_calls{
                let bounds = i.get_min_boundary();
                hit = true;
                if bounds.y+bounds.h >max_h{
                    max_h = bounds.y+bounds.h;
                }
                if bounds.y<min_h{
                    min_h = bounds.y;
                }
            }
            let dh = if hit{max_h-min_h-x.h} else{0};
            if let Some(mut y) = self.draw_call_stack.pop(){
                let mut shift =-((dh +x.padding_x*2)as f32 * x.scroll_amount as f32/1000.0 ) as i32;
         
                if x.upside_down{
                    shift += x.padding_y*2;
                    shift *= -1;
                }
      
                for i in &mut x.draw_calls{
                    i.shift(shift, true);
                }
                if y.vertical{
                    self.cursor_x = bounds.x;
                    self.cursor_y = bounds.y + bounds.h;
                }else{
                    self.cursor_y = bounds.y;
                    self.cursor_x = bounds.x + bounds.w;
                }
                let call = TGuiDraw::ScrollBox { x: x.x, y: x.y, w: x.w, h: x.h, children: x.draw_calls, scroll_amount: sb, current_scroll_amount: x.scroll_amount, padding_x: x.padding_x, padding_y: x.padding_y, color: x.bg_color, upside_down: x.upside_down } ;
                y.draw_calls.push(call);
                self.draw_call_stack.push(y);
            }else{
                for i in x.draw_calls{
                    self.draw_calls.push(i);
                }
            }
        }else if let Some(mut y) = self.draw_call_stack.pop(){
            if y.vertical{
                self.cursor_x = bounds.x;
                self.cursor_y = bounds.y + bounds.h;
            }else{
                self.cursor_y = bounds.y;
                self.cursor_x = bounds.x + bounds.w;
            }
            let call = TGuiDraw::Container { x: bounds.x, y: bounds.y, w: bounds.w, h: bounds.h, children: x.draw_calls, vertical: x.vertical, padding_x: x.padding_x, padding_y: x.padding_y, color:x.bg_color };
            y.draw_calls.push(call);
            self.draw_call_stack.push(y);
        }else{
            for i in x.draw_calls{
                self.draw_calls.push(i);
            }
        }

  
    }

    pub fn add_text(&mut self, text: impl Into<String>) {
        let txt = text.into();
        let w = if let Some(w) = self.draw_call_stack.pop(){ 
            if w.vertical{

                let v = w.w;
                self.draw_call_stack.push(w);
                if v> txt.len() as i32{
                    v
                }else if txt.len() > 30 { 30_i32} else { (txt.len())  as i32}
            }else{            
                self.draw_call_stack.push(w);
               if txt.len() > 30 { 30_i32} else { (txt.len())  as i32} 
            }

        }else if txt.len() > 30 { 30_i32} else { (txt.len()) as i32};
        let mut div = self.draw_call_stack.pop().unwrap();
        let bounds = get_string_bounds(
            &txt,
            self.cursor_x ,
            self.cursor_y ,
            w,
        );
        div.draw_calls.push(TGuiDraw::DrawString {
            string: txt,
            x: self.cursor_x ,
            y: self.cursor_y ,
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

    pub fn add_box(&mut self, w: i32, h: i32) {
        let mut div = self.draw_call_stack.pop().unwrap();
        div.draw_calls.push(TGuiDraw::DrawBox {
            x: self.cursor_x,
            y: self.cursor_y,
            w,
            h,
            color: div.fg_color,
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
                self.cursor_y -= h + div.padding_y ;
            } else {
                self.cursor_y += h + div.padding_y ;
            }
        } else {
            self.cursor_x += w + div.padding_x ;
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
        println!("setting bounds");
        println!("before draw calls:{:#?}", self.draw_calls);
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
        println!("after draw calls:{:#?}", self.draw_calls);
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

#[repr(transparent)]
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Copy, Hash)]
pub struct ElementId{
    v:u32
}
impl Default for ElementId {
    fn default() -> Self {
        Self::new()
    }
}

impl ElementId{
    pub const fn new()->Self{
        Self { v: 0 }
    }
    pub const fn is_valid(&self)->bool{
        self.v != 0
    }
    pub const fn inner(&self)->u32{
        self.v 
    }
}

pub struct TransGui{
    elements:BTreeMap<ElementId,TransGuiElement>, 
    roots:Vec<ElementId>,
    fg_color:Color, 
    bg_color:Color,
    name_table:HashMap<String, ElementId>,
    gui:TGui,
    scrollbar_outputs:HashMap<ElementId, TGuiOutput<i32>>, 
    button_outputs:HashMap<ElementId, TGuiOutput<bool>>,
    mutated:bool,
    modifications:usize,
    hidden:HashSet<ElementId>,
    list_cache:HashMap<ElementId, Box<dyn Any>>
}



#[derive(Clone)]
pub enum TransGuiElement{
    String{s:String, color:Color, parent:Immutable<ElementId>},
    Box{h:i32, w:i32, color:Color,parent:Immutable<ElementId>},
    Button{color:Color, on_pressed:Arc<Mutex<dyn FnMut(&mut TransGui, ElementId)>> ,parent:Immutable<ElementId>, text:String},
    Container{children:Immutable<Vec<ElementId>>, horizontal:bool,parent:Immutable<ElementId>, color:Color, upside_down:bool},
    ScrollBox{scroll_amount:i32, w:i32, h:i32, children:Immutable<Vec<ElementId>>, parent:Immutable<ElementId>, color:Color, upside_down:bool},
    BoxedGuiObject{obj:Box<dyn GuiObject>, parent:Immutable<ElementId>}
}

impl TransGuiElement{
    pub fn get_parent(&self)->ElementId{
        match self{
            TransGuiElement::String { s:_, color:_, parent } => {
                *parent.get()
            }
            TransGuiElement::Box { h:_, w:_, color:_, parent } =>{
                *parent.get()
            }
            TransGuiElement::Button { color:_, on_pressed:_, parent, text:_ } =>{
                *parent.get()
            }
            TransGuiElement::Container { children:_, horizontal:_, parent, color:_ , upside_down:_} => {
                *parent.get()
            }
            TransGuiElement::ScrollBox { scroll_amount:_, w:_, h:_, children:_, parent, color:_ , upside_down:_} => {
                *parent.get()
            }
            TransGuiElement::BoxedGuiObject { obj:_, parent } => {
                *parent.get()
            }
        }
    }
    fn set_parent(&mut self, new_parent:ElementId){
        match self{
            TransGuiElement::String { s:_, color:_, parent } => {
                *parent.unsafe_get_mut_please_dont_use() = new_parent;
            }
            TransGuiElement::Box { h:_, w:_, color:_, parent } =>{
                *parent.unsafe_get_mut_please_dont_use() = new_parent;
            }
            TransGuiElement::Button { color:_, on_pressed:_, parent, text:_ } =>{
                *parent.unsafe_get_mut_please_dont_use() = new_parent;
            }
            TransGuiElement::Container { children:_, horizontal:_, parent, color:_, upside_down:_ } => {
                *parent.unsafe_get_mut_please_dont_use() = new_parent;
            }
            TransGuiElement::ScrollBox { scroll_amount:_, w:_, h:_, children:_, parent , color:_, upside_down:_} => {
                *parent.unsafe_get_mut_please_dont_use() = new_parent;
            }
            TransGuiElement::BoxedGuiObject { obj:_, parent } => {
                *parent.unsafe_get_mut_please_dont_use() = new_parent;
            }
        }
    }
}

impl Default for TransGui {
    fn default() -> Self {
        Self::new()
    }
}

impl TransGui{
    pub fn new()->Self{
        Self { elements: BTreeMap::new(), roots:Vec::new(), fg_color:Color::GREEN, bg_color:Color::GREEN, name_table:HashMap::new() , gui:TGui::new(), mutated:false, scrollbar_outputs:HashMap::new(), button_outputs:HashMap::new(), 
        modifications:0, hidden:HashSet::new(), list_cache:HashMap::new()}
    }

    pub fn new_element(&mut self,e:TransGuiElement)->ElementId{
        self.mutated = true;
        self.modifications+=1;
        let mut idx = 0;
        for i in 1..u32::MAX{
            let id = ElementId{v:i};
            if  !self.elements.contains_key(&id){
                idx  = i;
                break;
            }
        }
        assert!(idx != 0);
        let out =  ElementId { v: idx };
        self.elements.insert(out,e);
        out
    }

    pub fn new_text(&mut self, text:impl Into<String>)->ElementId{
        self.mutated = true;
        self.modifications+=1;
        let e = TransGuiElement::String { s: text.into(), color: self.fg_color, parent: Immutable::new(ElementId::new()) };
        self.new_element(e)
    }

    pub fn new_box(&mut self, h:i32, w:i32)->ElementId{
        self.mutated = true;
        self.modifications+=1;
        let e = TransGuiElement::Box { h, w, color: self.fg_color, parent: Immutable::new(ElementId::new())} ;
        self.new_element(e)
    }

    pub fn new_button(&mut self, on_click:impl FnMut(&mut TransGui,ElementId)+'static, text:impl  Into<String>)->ElementId{
        self.mutated = true;
        self.modifications+=1;
        let e = TransGuiElement::Button { color: self.fg_color, on_pressed: Arc::new(Mutex::new(on_click)), parent:Immutable::new(ElementId::new()) , text:text.into()};
        self.new_element(e)
    }

    pub fn new_section(&mut self)->ElementId{
        self.mutated = true;
        self.modifications+=1;
        let e = TransGuiElement::Container { children:Immutable::new(Vec::new()), horizontal: false, parent: Immutable::new(ElementId::new()), color:self.bg_color, upside_down:false};
        self.new_element(e)
    }


    pub fn new_section_upside_down(&mut self)->ElementId{
        self.mutated = true;
        self.modifications+=1;
        let e = TransGuiElement::Container { children:Immutable::new(Vec::new()), horizontal: false, parent: Immutable::new(ElementId::new()), color:self.bg_color, upside_down:true};
        self.new_element(e)
    }
    pub fn new_horizontal_section(&mut self)->ElementId{
        self.mutated = true;
        self.modifications+=1;
        let e = TransGuiElement::Container { children:Immutable::new(Vec::new()), horizontal: true, parent: Immutable::new(ElementId::new()), color:self.bg_color, upside_down:false};
        self.new_element(e)
    }

    pub fn new_scroll_box(&mut self, w:i32, h:i32)->ElementId{
        self.mutated = true;
        self.modifications+=1;
        let e = TransGuiElement::ScrollBox {scroll_amount:0, w, h ,children:Immutable::new(Vec::new()), parent: Immutable::new(ElementId::new()), color:self.bg_color, upside_down:false};
        self.new_element(e)
    }

    pub fn new_scroll_box_upside_down(&mut self, w:i32, h:i32)->ElementId{
        self.mutated = true;
        self.modifications+=1;
        let e = TransGuiElement::ScrollBox {scroll_amount:0, w, h ,children:Immutable::new(Vec::new()), parent: Immutable::new(ElementId::new()), color:self.bg_color, upside_down:true};
        self.new_element(e)
    }

    pub fn new_gui_object(&mut self,obj:Box<dyn GuiObject>)->ElementId{
        self.mutated = true;
        self.modifications+=1;
        let e = TransGuiElement::BoxedGuiObject { obj, parent:Immutable::new(ElementId::new())};
        self.new_element(e)
    }

    pub fn attach_to_doc(&mut self, id:ElementId){
        self.modifications+=1;
        self.mutated = true;
        self.detach_element(id);
        self.roots.push(id);
    }

    pub fn attach_to_element(&mut self,id:ElementId, parent_to:ElementId){
        self.mutated =true;
        self.modifications+=1;
        self.detach_element(id);
        let prs = self.get_element(parent_to).unwrap();
        match prs{
            TransGuiElement::Container { children, horizontal:_, parent:_ , color:_, upside_down:_}=>{
                children.unsafe_get_mut_please_dont_use().push(id);
            }
            TransGuiElement::ScrollBox { scroll_amount:_, w:_, h:_, children, parent:_, color:_ ,upside_down:_}=>{
                children.unsafe_get_mut_please_dont_use().push(id);
            }
            _=>{
                todo!()
            }
        }
    }

    pub fn detach_element(&mut self, id:ElementId){
        if !id.is_valid(){
            return;
        }
        self.mutated = true;
        self.modifications+=1;
        let element = self.get_element(id).unwrap();
        let parent = element.get_parent();
        let is_valid = parent.is_valid();
        element.set_parent(ElementId::new());
        if is_valid{
            let pr = self.get_element(parent).unwrap();
            match pr{
                TransGuiElement::Container { children, horizontal:_, parent:_ , color:_, upside_down:_} => {
                    let mut idx = -1;
                    for i in 0..children.get().len(){
                        if children.unsafe_get_mut_please_dont_use()[i] == id{
                            idx = i as i32;
                            break;
                        }
                    }
                    if idx == -1{
                        todo!()
                    }else{
                        children.unsafe_get_mut_please_dont_use().remove(idx as usize);
                    }
                }
                TransGuiElement::ScrollBox { scroll_amount:_, w:_, h:_, children, parent :_, color:_, upside_down:_} => {
                    let mut idx = -1;
                    for i in 0..children.get().len(){
                        if children.unsafe_get_mut_please_dont_use()[i] == id{
                            idx = i as i32;
                            break;
                        }
                    }
                    if idx == -1{
                        todo!()
                    }else{
                        children.unsafe_get_mut_please_dont_use().remove(idx as usize);
                    }
                }
                _=>{
                    todo!()
                }
            }
        }
        let mut idx = -1;
        for i in 0..self.roots.len(){
            if self.roots[i] == id{
                idx = i as i32;
                break;
            }
        }
        if idx != -1{
            self.roots.remove(idx as usize); 
        }

    }

    pub fn get_element(&mut self, id:ElementId)->Throws<&mut TransGuiElement>{
        self.mutated =true;
        self.modifications+=1;
        if let Some(x) = self.elements.get_mut(&id){
            Ok(x)
        }else{
            throw!(format!("element not found:{:#?}", id));
        }
    }
    
    pub fn get_element_const(&self, id:ElementId)->Throws<& TransGuiElement>{
        if let Some(x) = self.elements.get(&id){
            Ok(x)
        }else{
            throw!(format!("element not found:{:#?}", id));
        }
    }

    pub fn get_name_id(&self, s:&str)->ElementId{
        if let Some(id)  = self.name_table.get(s){
            *id
        }else{
            ElementId::new()
        }
    }

    pub fn remove_name(&mut self, s:&str){
        self.name_table.remove(s);
    }

    pub fn hide_element(&mut self, id:ElementId){
        self.modifications+=1;
        self.mutated = true;
        self.hidden.insert(id);
    }

    pub fn reveal_element(&mut self, id:ElementId){
        self.modifications+=1;
        self.mutated = true;
        self.hidden.remove(&id);
    }

    pub fn name_element(&mut self, id:ElementId, name:impl Into<String>){ 
        self.name_table.insert(name.into(), id);
    }

    fn render(&mut self, handle: &mut RaylibDrawHandle){
        self.gui.draw_frame(handle);
    }

    pub fn recompute_list<T:PartialEq+Clone+'static>(&mut self,element:ElementId, list:&[T],create_element:impl FnMut(&mut TransGui,&T)->ElementId){
        let mut ce = create_element;
        if let Some(cache) = self.list_cache.remove(&element){
            let dc:Box<Vec<T>> = cache.downcast().unwrap();
            if *dc == list{
                return;
            }
        }
        self.remove_children(element);
        for i in list{
            let e = ce(self, i);
            self.attach_to_element(e, element);
        }
    
    }
    pub fn remove_children(&mut self, elem:ElementId){
        let e = self.get_element(elem).unwrap();
        match e.clone(){

            TransGuiElement::Container { children, horizontal:_, parent:_, color:_, upside_down:_ } => {
                for i in children.get(){
                    self.detach_element(*i);
                }
            },
            TransGuiElement::ScrollBox { scroll_amount:_, w:_, h:_, children, parent:_, color:_, upside_down:_ } => {
                for i in children.get(){
                    self.detach_element(*i);
                }
            }
            _=>{

            }
        }

    }

    fn recompute_element(&mut self, id:ElementId){
        if self.hidden.contains(&id){
            return;
        }
        let g = self.get_element_const(id).unwrap().clone();
        match g{
            TransGuiElement::String { s, color, parent:_ } => {
                self.gui.set_fg_color(color);
                self.gui.add_text(&*s);
            }
            TransGuiElement::Box { h, w, color, parent:_ } => {
                self.gui.set_fg_color(color);
                self.gui.add_box(w, h);
            }
            TransGuiElement::Button { color, on_pressed:_, parent :_, text
            } => {
                self.gui.set_fg_color(color);
                let l = text.len();
                let w = if l>15{
                    17
                }else{
                    l as i32+2
                };
                let h = get_string_bounds(&text, 0,0, w ).h+3;
                let pressed = self.gui.add_button(w, h, text);
                self.button_outputs.insert(id, pressed);
            } 
            TransGuiElement::Container { children, horizontal, parent:_ , color, upside_down} =>{
                self.gui.set_bg_color(color);
                if horizontal{
                    self.gui.begin_div_hor();
                }else{
                    self.gui.begin_div();
                }
                    if upside_down{
                    self.gui.set_upside_down();
                }else{
                    self.gui.set_rightside_up();
                }
                for i in children.get(){
                    self.recompute_element(*i);
                }
                self.gui.end_div();
            }
            TransGuiElement::ScrollBox { scroll_amount, w, h, children, parent:_, color, upside_down } =>{
                self.gui.set_bg_color(color);
                let x = self.gui.begin_scrollbox(w, h, scroll_amount);
                self.scrollbar_outputs.insert(id, x);
                if upside_down{
                    self.gui.set_upside_down();
                }else{
                    self.gui.set_rightside_up();
                }
      
                for i in children.get(){
                    self.recompute_element(*i);
                }
                self.gui.end_div();
            }
            TransGuiElement::BoxedGuiObject { obj:_, parent :_} =>{
                todo!()
            }
        }
    }

    fn recompute(&mut self){
        self.button_outputs.clear();
        self.scrollbar_outputs.clear();
        self.gui.begin_frame();
        let ids = self.roots.clone();
        for i in ids{
            self.recompute_element(i);
        }
    }

    pub fn update(&mut self, handle: &mut RaylibDrawHandle){
        self.recompute();
        self.render(handle);
        self.handle_updates();
        if self.should_collect(){
            self.collect();
        }
    }

    fn handle_updates(&mut self){
        self.mutated = false;
        let mut to_run = Vec::new();
        for (id, button) in &self.button_outputs{
            if button.take().unwrap(){
                let e = self.get_element_const(*id).unwrap();
                match e{
                    TransGuiElement::Button { color:_, on_pressed, parent:_, text:_ }=>{
                        to_run.push((*id, on_pressed.clone()));
                    }
                    _=>{
                        todo!()
                    }
                }
            }
        }
        let mut scroll_updates = Vec::new();
        for (id, scroll) in &self.scrollbar_outputs{
            let x = self.get_element_const(*id).unwrap();
            let Some(s )= scroll.take()else{
                continue;
            };
            match x{
                TransGuiElement::ScrollBox { scroll_amount, w:_, h:_, children:_, parent:_, color:_ ,upside_down :_} => {
                    if s != *scroll_amount{
                        scroll_updates.push((*id, s));
                    }
                }
                _=>{
                    todo!()
                }
            }
        }
        for (id, amount) in scroll_updates{
            let Ok(x) = self.get_element(id) else{
                continue;
            };
            match x{
                    TransGuiElement::ScrollBox { scroll_amount, w:_, h:_, children:_, parent:_, color:_ , upside_down:_}=>{
                        *scroll_amount = amount;
                    }
                    _=>{
                        todo!()
                    }
                }
        }
        for (id, tor) in to_run{
            let mut func = tor.lock().unwrap();
            (func)(self,id);
        }
    }

    fn collect_element(&self, element:ElementId, reachable_set:&mut HashSet<ElementId>){
        if reachable_set.contains(&element){
            return;
        }
        if self.get_element_const(element).is_err(){
            return;
        }
        reachable_set.insert(element);
        match self.get_element_const(element).unwrap(){
            TransGuiElement::Container { children, horizontal:_, parent:_, color:_,upside_down:_ } => {
                for i in children.get(){
                    self.collect_element(*i, reachable_set);
                }
            }
            TransGuiElement::ScrollBox { scroll_amount:_, w:_, h:_, children, parent:_, color:_ ,upside_down:_} => {
                for i in children.get(){
                    self.collect_element(*i, reachable_set);
                }
                
            }
            _=>{
            }
        }
    }

    fn collect(&mut self){
        let mut reachable_set = HashSet::new();
        for i in& self.roots{
            self.collect_element(*i, &mut reachable_set);
        }
        for id in self.name_table.values(){
            reachable_set.insert(*id);
        }
        let mut purge_list = Vec::new();
        for id in self.elements.keys(){
            if !reachable_set.contains(id){
                purge_list.push(*id);
            }
        }
        for i in purge_list{
            self.elements.remove(&i);
            self.hidden.remove(&i);
        }
        self.modifications = 0;
    }

    pub fn should_collect(&self)->bool{
        self.modifications>20
    }
}

extern crate transir;
#[derive(Clone)]
pub enum TransIr{
    String{s:String, color:Option<Color>, name:Option<String>}, 
    Box{h:i32, w:i32, color:Option<Color>, name:Option<String>}, 
    Button{color:Option<Color>, on_pressed:Arc<Mutex<dyn FnMut(&mut TransGui, ElementId)>>, text:String, name:Option<String>}, 
    Container{
        children:Vec<TransIr>, horizontal:bool, color:Option<Color>, upside_down:bool, name:Option<String>,
    },
    ScrollBox{
        w:i32, h:i32, children:Vec<ElementId>, color:Option<Color>, upside_down:bool, name:Option<String>
    }
}
impl TransIr{
    pub fn add_to_gui(self, _gui:&mut TransGui){
        match self{
            TransIr::String { s: _, color: _,name: _ } => {
                
            }
            TransIr::Box { h: _, w: _, color: _,name: _ } => {

            }
            TransIr::Button { color: _, on_pressed: _, text: _, name: _ } =>{

            }
            TransIr::Container { children: _, horizontal: _, color: _, upside_down: _ , name: _} => {

            }
            TransIr::ScrollBox { w: _, h: _, children: _, color: _, upside_down: _ ,name: _} => {

            }
        }
    }
    pub fn to_gui(list:Vec<Self>)->TransGui{
        let mut out = TransGui::new();
        for i in list{
            i.add_to_gui(&mut out);
        }
        out
    }
}