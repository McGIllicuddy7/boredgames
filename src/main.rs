use std::fmt::Debug;

use crate::{
    generators::buildings::{generate_ground_floor, post_process_floor},
    utils::{Heap, HeapRef, WeakHeap},
};

pub mod engine;
pub mod libgui;
//https://lib.rs/crates/wrappe
pub mod utils;

pub mod builder;

pub mod generators;
#[tokio::main]
pub async fn main() {
    /*let (mut handle, thread) = raylib::RaylibBuilder::default()
        .resizable()
        .width(18 * 1920 / 20)
        .height(18 * 1080 / 20)
        .build();
    handle.set_exit_key(None);
    builder::game_loop(&mut handle, &thread, None).await;*/
    let mut g = generate_ground_floor(128, 128);
    post_process_floor(&mut g, 64, 64, true);
    println!("{}", g.rooms.len());
    g.render("test.png");
}
