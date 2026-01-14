use raylib::{color::Color, prelude::RaylibDraw};

fn main() {
    let (mut handle, thread) = raylib::init().size(1000, 1000).title("boredgames").build();
    while !handle.window_should_close() {
        let mut dr = handle.begin_drawing(&thread);
        dr.clear_background(Color::WHITE);
        dr.draw_text("testing 1 2 3", 100, 100, 12, Color::BLUEVIOLET);
    }
}
