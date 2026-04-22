use std::{
    cell::{Cell, RefCell},
    fmt::format,
    rc::Rc,
    sync::{Arc, Mutex},
};

use raylib::{RaylibBuilder, RaylibHandle};

use crate::gui::GUI;

pub mod gui;
#[tokio::main]
async fn main() {
    let (mut handle, thread) = RaylibBuilder::default().title("Hello Sailor :3").build();
    let mut should_close = false;
    let mut scroll_amnt = 0.0;
    let counter = RefCell::new(0);
    while !handle.window_should_close() {
        let mut gui = GUI::new(&mut handle, &thread);
        gui.centered_horizontal(|cmds| {
            cmds.container(600, |cmds| {
                cmds.text("hello world", 16);
                cmds.text(&format!("counter:{}", counter.borrow()), 16);
                cmds.button("increment", 16, || {
                    let mut tmp = counter.borrow_mut();
                    *tmp += 1;
                });
                cmds.button("decrement", 16, || {
                    let mut tmp = counter.borrow_mut();
                    *tmp -= 1;
                });
            });
            cmds.container(600, |cmds| {
                cmds.text("list", 16);
                cmds.scroll_box(600, &mut scroll_amnt, |cmds| {
                    let count = *counter.borrow();
                    for i in 0..count {
                        cmds.text(&format!("{}", i), 16);
                    }
                });
            });
        });
        gui.render();
        drop(gui);
        if should_close {
            break;
        }
    }
}
