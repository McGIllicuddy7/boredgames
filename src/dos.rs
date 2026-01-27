pub use serde::{Deserialize, Serialize};
pub use std::sync::Arc;
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Sprite {
    pub width: i32,
    pub height: i32,
    pub data: Arc<[Color]>,
}
