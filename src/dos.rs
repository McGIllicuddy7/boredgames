pub use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}
