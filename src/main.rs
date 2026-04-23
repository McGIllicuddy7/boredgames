use std::sync::Arc;

use raylib::RaylibBuilder;

use crate::gui::{GUI, ScrollBoxData, TextBoxData};
pub struct State {
    counter: i32,
    should_close: bool,
    rotation: f32,
    scroll_box_data: ScrollBoxData,
    text_box_data: TextBoxData,
}

pub mod engine;
pub mod gui;
pub mod utils;
#[tokio::main]
async fn main() {
    let mut state = State {
        counter: 0,
        should_close: false,
        rotation: 0.0,
        scroll_box_data: ScrollBoxData::new(),
        text_box_data: TextBoxData::new(),
    };
    let (mut handle, thread) = RaylibBuilder::default()
        .title("Hello Sailor :3")
        .vsync()
        .build();

    let nyan = Arc::new(handle.load_texture(&thread, "nyancat.png").unwrap());
    handle.set_exit_key(None);
    while !handle.window_should_close() {
        let mut gui = GUI::new(&state, &mut handle, &thread);
        gui.centered_horizontal(|cmds| {
            cmds.container(600, |cmds| {
                cmds.h1("hello sailor!");
                cmds.p1("hello world");
                cmds.p1(format!("counter:{}",state.counter));
                cmds.button("increment", 16, |state| {
                    state.counter+=1;
                });
                cmds.button("decrement", 16, |state| {
                    state.counter-=1;
                });
            });
            let nyan = nyan.clone();
            cmds.canvas(512,512,move |_bounds, state, cmds, handle,_thread|{
                cmds.draw_texture_scaled_rotated(&nyan, 128,128, 256, 256 ,state.rotation);
               state.rotation += handle.get_frame_time()*90.;
            });
            cmds.container(300, |cmds| {
                cmds.h1("list");
                cmds.scroll_box_rev(300, &state.scroll_box_data, |cmds| {
                    let count = state.counter;
                    for i in 0..count {
                        cmds.p3(format!("{}:Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.", i));
                    }
                });
                cmds.text_input(&state.text_box_data ,24, 100);
            });
        });
        gui.render_fps(&mut state);
        drop(gui);
        if state.should_close {
            break;
        }
    }
}
