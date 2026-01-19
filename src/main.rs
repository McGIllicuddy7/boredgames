use raylib::{color::Color, prelude::RaylibDraw};
use transir::trans;

use crate::tgui::{ElementId, TransIr};
use std::sync::{Arc, Mutex};
pub mod rtils;
pub mod server;
pub mod state;
pub mod tgui;

fn main() {
    let indexs = Arc::new(Mutex::new(Vec::new()));
    let indexs2 = indexs.clone();
    let x = trans!(
            (div (
                (text "hello world")
                (button ":3" ({
                    let on_click = move |x:&mut tgui::TransGui, id:ElementId|{
                        let mut lck = indexs2.lock().unwrap();
                        let len = lck.len();
                        lck.push(format!("index:{:#?}", len));
                    //    println!("{:#?}", id);
                    } ;
                    on_click
                }))
                (scrollbox name = "bridget" (
                ))
            )
        )

    );
    let mut gui = TransIr::to_gui(x);
    let (mut handle, thread) = raylib::init().size(2000, 2000).title("testing").build();
    let w = handle.get_screen_height();
    handle.set_window_size((w*20)/16, w);
    handle.set_target_fps(60);
    while !handle.window_should_close() {
        let mut dr = handle.begin_drawing(&thread);
        dr.clear_background(Color::BLACK);
        let id = gui.get_name_id("bridget");
        let index_lock = indexs.lock().unwrap();
        gui.recompute_list(id, &index_lock, |gui, x| gui.new_text(x));
        drop(index_lock);
        gui.update(&mut dr);
    }
}
