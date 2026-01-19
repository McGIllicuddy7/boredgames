use std::any::type_name_of_val;
use raylib::{color::Color, prelude::RaylibDraw};
use transir::trans;

use crate::tgui::TransIr;

pub mod rtils;
pub mod server;
pub mod state;
pub mod tgui;
fn main() {
    let x = trans!((div ((text "hello world"))));
    println!("{:#?}", type_name_of_val(&x));
    let (mut handle, thread) = raylib::init().size(1000, 1000).title("testing").build();
    handle.set_target_fps(60);
  /*   let mut gui = TransGui::new();
    let first = gui.new_section();
    println!("{:#?}", first);
    gui.attach_to_doc(first);
    let t1 = gui.new_text("hello world!");
    gui.attach_to_element(t1, first);
    let t2 = gui.new_text("testing 1 2 3");
    gui.attach_to_element(t2, first);
    let second = gui.new_section();
    gui.attach_to_doc(second);
    let sb = gui.new_scroll_box(20, 40);
    gui.attach_to_element(sb, second);
    for i in 0..100{
        let bx = gui.new_button(|_,_|{
            exit(0);
        },format!("testing {}",i));
        gui.attach_to_element(bx, sb);
    }*/

    while !handle.window_should_close() {
        let mut dr = handle.begin_drawing(&thread);
        dr.clear_background(Color::BLACK);
    }
}
