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
pub async fn main() {}
