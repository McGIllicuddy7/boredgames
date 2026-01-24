use raylib::{color::Color, math::Vector2, prelude::RaylibDraw};
use transir::trans;

use crate::{
    sharedlist::SharedList,
    transgui::{ElementId, ListView, TransGui, TransIr},
};
#[allow(unused)]
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
pub mod rtils;
pub mod server;
pub mod sharedlist;
pub mod state;
pub mod tgui;
pub mod transgui;
pub fn old_main() {
    let indexes = SharedList::new();
    let indices2 = indexes.clone();
    let indices3 = indexes.clone();
    let x = trans!(
            (div ( (box name = "box" w 40 h 40)))
            (div (
                (text "hello world")
                (button "create element" ({
                    let on_click = move |x:&mut TransGui, id:ElementId|{
                        indices2.push(format!("index:{:#?}", indices2.len()));
                    //    println!("{:#?}", id);
                    } ;
                    on_click
                }))
                (button "destroy element" ({
                    let on_click = move |x:&mut TransGui, id:ElementId|{
                        indices3.pop();
                    //    println!("{:#?}", id);
                    } ;
                    on_click
                }))
                (scrollbox name = "bridget" w 10 h 15(
                ))
            )
        )


    );

    let mut gui = TransIr::to_gui(x);
    let bridget = gui.get_name_id("bridget");
    let box_id = gui.get_name_id("box");
    gui.attach_state_view(
        bridget,
        ListView::new(&indexes, |list, id, gui| {
            for i in 0..list.len() {
                let tmp = list.get(i).unwrap();
                let elem = gui.new_text(tmp);
                gui.attach_to_element(elem, id);
            }
        }),
    );
    let (mut handle, thread) = raylib::init().size(0, 0).title("testing").build();
    let w = handle.get_screen_height();
    handle.set_window_size((w * 22) / 16, w);
    handle.set_target_fps(60);
    while !handle.window_should_close() {
        let mut dr = handle.begin_drawing(&thread);
        dr.clear_background(Color::BLACK);
        gui.update(&mut dr);
        let bounds = gui.box_output(box_id).unwrap().pixel_coords;
        let center = Vector2::new(
            (bounds.x + bounds.w / 2) as f32,
            (bounds.y + bounds.h / 2) as f32,
        );
        let rad = if bounds.w < bounds.h {
            bounds.w as f32 / 2.
        } else {
            bounds.h as f32 / 2.
        };
        dr.draw_circle(center.x as i32, center.y as i32, rad, Color::BLUEVIOLET);
    }
}

pub fn main() {}
