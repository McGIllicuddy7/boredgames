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
        .width(3 * 1920 / 4)
        .height(3 * 1080 / 4)
        .build();
    engine::ClientState::create_and_run(None, &mut handle, &thread).await;
}
