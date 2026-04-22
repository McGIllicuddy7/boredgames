use std::cell::RefCell;

use raylib::RaylibBuilder;

use crate::gui::GUI;

pub mod gui;
#[tokio::main]
async fn main() {
    let (mut handle, thread) = RaylibBuilder::default()
        .title("Hello Sailor :3")
        .vsync()
        .build();
    handle.set_exit_key(None);
    let should_close = false;
    let mut scroll_amnt = 0.0;
    let mut text = String::new();
    let mut selected = false;
    let mut text_output = None;
    let counter = RefCell::new(0);
    while !handle.window_should_close() {
        let mut gui = GUI::new(&mut handle, &thread);
        gui.centered_horizontal(|cmds| {
            cmds.container(600, |cmds| {
                cmds.h1("hello sailor!");
                cmds.p1("hello world");
                cmds.p1(format!("counter:{}", counter.borrow()));
                cmds.button("increment", 16, || {
                    let mut tmp = counter.borrow_mut();
                    *tmp += 1;
                });
                cmds.button("decrement", 16, || {
                    let mut tmp = counter.borrow_mut();
                    *tmp -= 1;
                });
            });
            cmds.container(300, |cmds| {
                cmds.h1("list");
                cmds.scroll_box_rev(300, &mut scroll_amnt, |cmds| {
                    let count = *counter.borrow();
                    for i in 0..count {
                        cmds.p3(format!("{}:Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.", i));
                    }
                });
                cmds.text_input(&mut text, 24, 100, &mut selected, &mut text_output);
            });
        });
        gui.render_fps();
        drop(gui);
        if should_close {
            break;
        }
    }
}
