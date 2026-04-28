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
        .width(9 * 1920 / 10)
        .height(9 * 1080 / 10)
        .build();
    engine::ClientState::create_and_run(None, &mut handle, &thread).await;
}
