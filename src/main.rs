use raylib::{color::Color, prelude::RaylibDraw};

pub mod rtils;
pub mod server;
pub mod state;
pub mod tgui;
fn main() {
    let (mut handle, thread) = raylib::init().size(1000, 1000).title("testing").build();
    handle.set_target_fps(60);
    let mut cl = tgui::TGui::new();
    let mut idx = 0;
    while !handle.window_should_close() {
        cl.begin_frame();
        cl.set_bg_color(Color::GREEN);
        cl.begin_div();

        cl.add_text("hello window".to_string());
        let idx2 = cl.begin_scrollbox(10, 10, idx);
        cl.set_upside_down();
        for i in 0..5 {
            cl.add_text(format!("testing:{i}"));
        }
        cl.end_div();
        cl.end_div();
        cl.begin_div();
        let mut bvec = Vec::new();
        for i in 0..10 {
            bvec.push(cl.add_button(5, 3, format!("click:{}", i)));
        }
        cl.end_div();
        let mut dh = handle.begin_drawing(&thread);
        dh.clear_background(Color::BLACK);
        cl.draw_frame(&mut dh);
        idx = idx2.take().unwrap();
    }
}
