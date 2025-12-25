use std::collections::HashMap;

use eframe::egui::{Image, ImageData, Pos2};
use image::{ImageBuffer, RgbImage, RgbaImage};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Token {
    pub location: Pos2,
    pub image: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub messages: Vec<(String, String)>,
    pub tokens: HashMap<String, Token>,
}
#[derive(Serialize, Deserialize, Clone)]
pub enum EventData {
    Message {
        from: String,
        contents: String,
        time_stamp: u128,
    },
    Connection {
        username: String,
    },
    Disconnection {
        username: String,
    },
    Kill {
        password: String,
    },
    TokenMoved {
        name: String,
        to: Pos2,
        time_stamp: i32,
    },
    TokenCreated {
        name: String,
        token: Token,
    },
    TokenDestroyed {
        name: String,
    },
    ImageUpload {
        name: String,
        image: ImageData,
    },
    SendState {
        state: State,
    },
    HeartBeat,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct Event {
    pub source: String,
    pub data: EventData,
}
