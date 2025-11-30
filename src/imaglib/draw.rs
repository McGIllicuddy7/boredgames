use std::{error::Error, f32::consts::PI};
use crate::imaglib::draw::colors::RED;
use serde_derive::{Deserialize, Serialize};
pub use super::letters::*;
use minifb::{Key, Window, WindowOptions};

pub use super::math::*;
#[derive(Copy, Clone)]
#[repr(C)]
#[derive(Serialize, Deserialize,Debug)]
pub struct Color {
    pub b: u8,
    pub g: u8,
    pub r: u8,
    pub a: u8,
}
pub mod colors{
    use super::Color;
    pub const BLACK:Color = Color{r:0, g:0, b:0, a:255};
    pub const WHITE:Color = Color{r:220, g:220, b:220, a:255};
    pub const BLUE:Color = Color{r:0, g:0, b:220, a:255};
    pub const TEAL:Color = Color{r:0, g:220, b:220, a:255};
    pub const RED:Color = Color{r:220, g:0, b:0, a:255};
    pub const GREEN:Color = Color{r:0, g:220, b:0, a:255};
    pub const PURPLE:Color = Color{r:220, g:0, b:220, a:255};
    pub const PINK:Color = Color{r:255, g:150, b:210, a:255};
    pub const GREY:Color = Color{r:110, g:110, b:110, a:255};
    pub const DARK_BLUE:Color = Color{r:0, g:0, b:110, a:255};
    pub const DARK_TEAL:Color = Color{r:0, g:110, b:110, a:255};
    pub const DARK_RED:Color = Color{r:110, g:0, b:0, a:255};
    pub const DARK_GREEN:Color = Color{r:0, g:110, b:0, a:255};
    pub const DARK_PURPLE:Color = Color{r:110, g:0, b:110, a:255};
    pub const DARK_PINK:Color = Color{r:125, g:75, b:105, a:255};
    pub const DARK_GREY:Color = Color{r:55, g:55, b:55, a:255};
}
#[derive(Clone)]
#[derive(Serialize, Deserialize, Debug)]
pub struct Image {
    pub height: usize,
    pub width: usize,
    pub data: Box<[Color]>,
}
impl Image {
    pub fn new(width: usize, height:usize) -> Self {
        let mut vs = Vec::new();
        vs.reserve_exact(height * width);
        for _ in 0..height * width {
            vs.push(Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            });
        }
        Image {
            height,
            width,
            data: vs.into(),
        }
    }
    pub fn draw_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x>=self.width || y>=self.height{
            return;
        }
        let c = &mut self.data[y * self.width + x];
        *c = color;
    }
    pub fn get_pixel(&self, mut x: usize, mut y: usize) -> Color {
        if x>=self.width {
            x = self.width-1;
        }
        if y>=self.height{
            y = self.height-1;
        }
        self.data[y * self.width + x]
    }
    pub fn get(&self, x:f64, y:f64)->Color{
        let mut x0 = (x*self.width as f64) as usize;
        let mut y0 = (y*self.height as f64) as usize;
        if x0>=self.width {
            x0 = self.width-1;
        }
        if y0>=self.height{
            y0 = self.height-1;
        }
        self.data[y0*self.width+x0]
    }

    pub fn clear(&mut self, color: Color) {
        self.data.fill(color);
    }
    pub fn export(&self, to: &str) {
        let mut out = String::from("P3\n");
        out += &format!("{} {}\n", self.width, self.height);
        out += "255\n";
        for i in &self.data {
            out += &format!("{} {} {}\n", i.r, i.g, i.b);
        }
        std::fs::write(to, out).unwrap();
    }
    pub fn draw(&self,window:&mut Window){
                let buffptr = self.data.as_ptr()as *mut u32;
                let buffer = unsafe{
                std::slice::from_raw_parts_mut(buffptr, self.height*self.width)
                };
                window
                .update_with_buffer(buffer, self.width, self.height)
                .unwrap();
    }
    pub fn draw_forever(&self) {
        let mut window = Window::new(
            "Test - ESC to exit",
        self.width,
            self.height,
            WindowOptions::default(),
        )
        .unwrap_or_else(|e| {
            panic!("{}", e);
        });
        window.set_target_fps(60);
        let buffptr = self.data.as_ptr()as *mut u32;
        let buffer = unsafe{
            std::slice::from_raw_parts_mut(buffptr, self.height*self.width)
        };
        while window.is_open() && !window.is_key_down(Key::Escape) {
            window.update();
            window
                .update_with_buffer(buffer, self.width, self.height)
                .unwrap();
        }
    }
    pub fn draw_triangle(&mut self, p1: Vec2, p2: Vec2, p3: Vec2, color: Color) {
        let bx = BB::from_points(&[p1, p2, p3]);
        //for y in bx.y..bx.y + bx.h + 1 {
        //   for x in bx.x..bx.x + bx.w + 1 {
        for y in bx.y..bx.y+bx.h {
            for x in bx.x..bx.x+bx.w {
                let v = Vec2 { x, y };
                if in_triangle(p1, p2, p3, v) {
                    self.draw_pixel(v.x as usize, v.y as usize, color);
                }
            }
        }
    }
    pub fn calc_uvs_lin(p1:Vec2, p2:Vec2, p3:Vec2, uv1:Vec2r, uv2:Vec2r, uv3:Vec2r, pos:Vec2)->Vec2r{ 
        let denum = triangle_area(p1, p2, p3) as f32;
        let t1 = triangle_area(pos, p2, p3) as f32/denum;
        let t2 = triangle_area(pos, p3, p1) as f32/denum;
        let t3 = triangle_area(pos, p2, p1) as f32/denum;
      // println!("{t1}, {t2},{t3},{denum}, {}", t1+t2+t3);
        uv1*t1+uv2*t2+uv3*t3
    }  
    pub fn draw_triangle_shader<Shade:Shader>(&mut self, p1: Vec2, p2: Vec2, p3: Vec2, uv1:Vec2r,uv2:Vec2r, uv3:Vec2r,shader:&Shade) {
        let bx = BB::from_points(&[p1, p2, p3]);
        for y in bx.y..bx.y+bx.h {
            for x in bx.x..bx.x+bx.w {
                let v = Vec2 { x, y };
                if in_triangle(p1, p2, p3, v) {
                    let color = shader.kernel(v, Self::calc_uvs_lin(p1, p2, p3, uv1, uv2, uv3, v));
                    self.draw_pixel(v.x as usize, v.y as usize, color);
                }
            }
        }
    }
    pub fn draw_rect(&mut self, x:i32, y:i32, w:i32, h:i32,color:Color){
        for yp in y..y+h{
            for xp in x..x+w{
                self.draw_pixel(xp as usize, yp as usize, color);
            }
        }
    }
    pub fn draw_bitmap(&mut self, x:i32, y:i32, bmp:&Bitmap, color:Color){
        let x0 = x as usize;
        let y0 = y as usize;
        for yp in 0..bmp.height{
            for xp in 0..bmp.width{
                if bmp.get(xp as i32, yp as i32){
                    self.draw_pixel(xp+x0, yp+y0, color);
                }
            }
        }
    }
    pub fn draw_bitmap_scaled(&mut self, x:i32, y:i32, w:i32, h:i32, bmp:&Bitmap, color:Color){
        let x0 = x;
        let y0 =y;
        let wf = w as f32;
        let hf = h as f32;
        if x0<0 && x0+w<0 {
            return;
        }
        if y0<0 && y0+h<0 {
            return;
        }
        if x0>self.width as i32 {
            return;
        }
        if y0>self.height as i32 {
            return;
        }
        for yp in 0..h*2{
            for xp in 0..w{
                    let xf = xp as f32/wf;
                    let yf = yp as f32/hf;
                    if bmp.getf(xf, yf){
                        self.draw_pixel((xp+x0) as usize, (yp+y0) as usize, color);
                    }
                }
            }
    }
    pub fn draw_rect_shader<Shade:Shader>(&mut self, x:i32, y:i32, w:i32, h:i32, shader:&Shade){
        for yp in y..y+h{
            for xp in x..x+w{
                let color = shader.kernel(Vec2::new(xp, yp), Vec2r::new((xp-x) as f32/(w as f32), (yp-y) as f32/(h as f32)));
                if color.a == 0{
                    continue;
                }
                self.draw_pixel(xp as usize, yp as usize, color);
            }
        }
    }
    pub fn draw_rect_rot<Shade:Shader>(&mut self, x:i32, y:i32, w:i32, h:i32, rot:f64,shader:&Shade){
        let dx = Vec2r::new(w as f32/2.0, h as f32/2.0);
        let dy = Vec2r::new(w as f32/2.0, -h  as f32/2.0);
        let rotr = Mat2::rotmat(rot);
        let xp = (rotr*dx).to_int();
        let yp = (rotr*dy).to_int();
        let c = Vec2::new(x,y);
        let p1 = c+xp;
        let p2 = c-yp;
        let p3 = c+yp;
        let p4 = c-xp;
        let u1 = Vec2r::new(0.0, 0.0);
        let u2 = Vec2r::new(1.0, 0.0);
        let u3 = Vec2r::new(0.0, 1.0);
        let u4 = Vec2r::new(1.0, 1.0);
        self.draw_triangle_shader(p1, p2, p3, u1, u2, u3, shader);
        self.draw_triangle_shader(p4, p2, p3, u4, u2, u3, shader);
    }
    pub fn load(name:&str)->Result<Self, Box<dyn Error>>{
        let img = load_image::load_path(name)?;
        let h = img.height;
        let w = img.width;
        let (ig ,_)= img.into_rgba();
        let mut out = Self::new(h,w);
        for yp in 0..h{
            for x in 0..w{
                let y = h-yp-1;
                let ct = ig[yp][x];
                let col = Color{r:ct.r, g:ct.g, b:ct.b, a:ct.a};
                out.draw_pixel(x, y, col);
            }
        }
        Ok(out)
    }
    pub fn draw_char(&mut self,x:i32, y:i32, c:char, color:Color){
        let bmp = char_to_bmp(c);
        self.draw_bitmap(x, y, bmp, color);
    }
    pub fn draw_char_scaled(&mut self,x:i32, y:i32, w:i32, h:i32,c:char, color:Color){
        let bmp = char_to_bmp(c);
        self.draw_bitmap_scaled(x, y, w,h,bmp, color);
    }
    pub fn draw_text(&mut self, x:i32, y:i32, text:&str, color:Color){
        let mut cx = x;
        let mut cy = y;
        for i in text.chars(){
            if i == '\n'{
                cy += TEXT_SPACE_V;
                cx = x;
            }else if i == '\t'{
                cx += (TEXT_SPACE)*3;
            }else{
                self.draw_char(cx, cy, i, color);
                cx += TEXT_SPACE;

            }
        }
    }
    pub fn draw_text_scaled(&mut self,x:i32, y:i32, height:i32, text:&str, color:Color){
        let sf = height as f64/TEXT_HEIGHT as f64;
        let w = (TEXT_WIDTH as f64*sf) as i32;
        let h = (TEXT_HEIGHT as f64*sf) as i32;
        let xoff = (TEXT_SPACE as f64*sf)as i32;
        let yoff = (TEXT_SPACE_V as f64 *sf ) as i32;
        let mut cx = x;
        let mut cy = y;
        for i in text.chars(){
            if i == '\n'{
                cy += yoff;
                cx = x;
            }else if i == '\t'{
                cx += xoff*3;
            }else{
                self.draw_char_scaled(cx, cy, w,h,i, color);
                cx += xoff

            }
        }
    }
    pub fn draw_text_width(&mut self,x:i32, y:i32, max_width:i32,height:i32, text:&str, color: Color){
       let sf = height as f64/TEXT_HEIGHT as f64;
        let w = (TEXT_WIDTH as f64*sf) as i32;
        let h = (TEXT_HEIGHT as f64*sf) as i32;
        let xoff = (TEXT_SPACE as f64*sf)as i32;
        let yoff = (TEXT_SPACE_V as f64 *sf ) as i32;
        let mut cx = x;
        let mut cy = y;
        for i in text.chars(){
            if i == '\n' || cx-x+xoff>=max_width{
                cy += yoff;
                cx = x;
            }else if i == '\t'{
                cx += xoff*3;
            }
            if i != '\n' && i != '\t'{
                self.draw_char_scaled(cx, cy, w,h,i, color);
                cx += xoff

            }
        }
    }
    pub fn text_bounds(&self, height:i32,w:i32 ,text:&str)->(i32, i32){
        let sf = height as f64/TEXT_HEIGHT as f64;
        let xoff = (TEXT_SPACE as f64*sf)as i32;
        let yoff = (TEXT_SPACE_V as f64 *sf ) as i32;
        let mut cx = 0;
        let mut cy = 0;
        let mut max_cx = 0;
        for i in text.chars(){
            if i == '\n' || cx+xoff>=w{
                cy += yoff;
                if cx>max_cx{
                    max_cx = cx;
                }
                cx = 0;
            }else if i == '\t'{
                cx += xoff*3;
            }else{
                cx += xoff

            }
        }
        if cx>max_cx{
            max_cx = cx;
        }
        (max_cx+xoff, cy+yoff)
    }
    pub fn draw_text_box(&mut self,x:i32, y:i32, w:i32, h:i32, text:&str, color: Color){
            let mut hp = 100;
            let ( _, mut by) = self.text_bounds(hp, w,text);
            while by+2>=h{
                hp -=1;
                (_, by) = self.text_bounds(hp, w-2,text);
                //println!("{},{}", bx,by)
            }
          //  println!("{hp}");
            self.draw_text_width(x, y, w ,hp, text, color);
    }
    pub fn draw_line(&mut self, start:Vec2, end:Vec2, w:f32, color:Color){
        let b = BB::from_points(&[start, end]);
        /*b.x =0;
        b.y =0;
        b.w = self.width as i32;
        b.h = self.height as i32;*/
        let s = start.to_real();
        let e = end.to_real();
        for y in b.y..b.y+b.h+1 as i32{
            for x in b.x..b.x+b.w+1 as i32{
                if x<0 || y<0{
                    continue;
                }
                let p = Vec2r::new(x as f32, y as f32);
                let d = dist_to_line(s, e, p);
                if d<=w{
                    self.draw_pixel(x as usize, y as usize, color);
                }
            }
        }
    }
    pub fn draw_vec2r(&mut self,location:Vec2, v:Vec2r, w:f32,s:f32,color: Color){
        let e= location.to_real()+v*s;
        let ei = e.to_int();
        println!("{:#?},{:#?}, {:#?}", location,ei,v);
        self.draw_line(location, ei, w, color);
        let delt = (v.norm()*s).rotate(-PI/4.0);
        let delt2 = (v.norm()*s).rotate(PI/4.0); 
        self.draw_line(ei, (e-delt).to_int(),w ,color);
        self.draw_line(ei, (e-delt2).to_int(),w ,color);
    }
    pub fn draw_circ(&mut self, pos:Vec2, rad:i32, color: Color){
            for y in -rad..rad+1{
                for x in -rad..rad+1{
                    let xp = pos.x+x;
                    let yp = pos.y+y;
                    if xp<0 || yp<0 || yp as usize>=self.height || xp as usize>=self.width{
                        continue;
                    }
                    let p = Vec2::new(xp, yp);
                    if p.dist(pos)<rad{
                        self.draw_pixel(xp as usize, yp as usize, color);
                    }
                }
            }
    }
    pub fn draw_rect_image(&mut self, x:i32, y:i32, w:i32, h:i32,to_draw:&Self){
        let shader = ImageShader{img:to_draw};
        self.draw_rect_shader(x, y, w, h, &shader);
    }
    pub fn draw_rect_image_rot(&mut self, x:i32, y:i32, w:i32, h:i32,rot:f64,to_draw:&Self){
        let shader = ImageShader{img:to_draw};
        self.draw_rect_rot(x, y, w, h, rot,&shader);
    }
}
pub trait Shader{
    fn kernel(&self,screen_location:Vec2, text_coord:Vec2r)->Color;
}
struct ImageShader<'a>{
    img:&'a Image,
}
impl <'a> Shader for ImageShader<'a>{
    fn kernel(&self,_screen_location:Vec2, text_coord:Vec2r)->Color {
            self.img.get(text_coord.x as f64 , text_coord.y as f64)
    }
}

#[derive(Clone)]
pub struct Bitmap{
    pub width:usize,
    pub height:usize,
    pub data:&'static [u8],
}
impl Bitmap{
    pub const fn from_data<const W:usize, const H:usize>(inp:&'static [[u8;W];H])->Self{
        let data = unsafe{std::slice::from_raw_parts(inp.as_ptr() as * const u8, W*H)};
        Self { width: W, height: H, data}
    }
    pub const fn get(&self,x:i32, y:i32)->bool{
        if x as usize>=self.width || y as usize>= self.height  || y<0 || x<0{
            return false;
        }
        self.data[y as usize*self.width+x as usize] != 0
    }
    pub const fn getf(&self, x:f32, y:f32)->bool{
        const OF:f32 = 0.30;
        self.get((x*self.width as f32) as i32, (y*self.width as f32) as i32) ||       
        self.get((x*self.width as f32+OF) as i32, (y*self.width as f32) as i32)||
        self.get((x*self.width as f32-OF) as i32, (y*self.width as f32) as i32)||
        self.get((x*self.width as f32) as i32, (y*self.width as f32+OF) as i32)||
        self.get((x*self.width as f32) as i32, (y*self.width as f32-OF) as i32)
    }
}
pub fn begin_rendering(width:usize, height:usize)->(Image, Window){
        let img = Image::new(width, height);
        let mut window = Window::new(
            "Test - ESC to exit",
            img.width,
            img.height,
            WindowOptions::default(),
        )
        .unwrap_or_else(|e| {
            panic!("{}", e);
        });
        window.set_target_fps(60);
        (img, window)
}
pub fn window_should_continue(window:&Window, use_esc:bool)->bool{
    window.is_open() && (!window.is_key_down(Key::Escape) ||!use_esc)
}