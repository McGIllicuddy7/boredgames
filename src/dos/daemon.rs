use crate::dos::dos::DosRt;
use raylib::texture::RenderTexture2D;
use std::collections::HashMap;
pub struct Window {
    pub pos_x: i32,
    pub pos_y: i32,
    pub w: i32,
    pub h: i32,
    pub texture: RenderTexture2D,
    pub rt: DosRt,
    pub depth: i32,
}

#[allow(unused)]
pub struct Daemon {
    pub windows: HashMap<u64, Window>,
}

impl Daemon {
    pub fn run(&mut self) {
        let (mut handle, thread) = raylib::init().fullscreen().undecorated().build();
        while !handle.window_should_close() {
            for i in &mut self.windows {
                i.1.rt.external_update(&mut handle, &thread);
            }
            let mut draw = handle.begin_drawing(&thread);
            let mut v = Vec::new();
            for i in &mut self.windows {
                v.push(i.1);
            }
            v.sort_by(|x, y| x.depth.cmp(&y.depth));
            for i in v {
                i.rt.external_draw_to(&mut draw, &thread, i.pos_x, i.pos_y, i.w, i.h);
            }
        }
    }
}
