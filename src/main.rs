use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use crate::utils::{Timer, generate_id};

pub mod engine;
pub mod gui;
pub mod utils;
#[tokio::main]
async fn main() {
    let (mut handle, thread) = raylib::RaylibBuilder::default()
        .resizable()
        .width(8 * 1920 / 10)
        .height(8 * 1080 / 10)
        .build();
    handle.set_exit_key(None);
    engine::game_loop(&mut handle, &thread).await;
}
