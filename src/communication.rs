use std::{collections::HashMap, sync::LazyLock};

use eframe::egui::{Pos2, Vec2};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Token {
    pub location: Pos2,
    pub scale: Vec2,
    pub image: String,
    pub display_name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum LayerType {
    Base,
    Map,
    Gm,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Layer{
   pub tokens:HashMap<String, Token>, 

}




#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub messages: Vec<(String, String)>,
    pub tokens: Layer,
    pub map: Layer,
    pub gm: Layer,
    pub name: String,
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
        to: Token,
        time_stamp: i32,
        layer: LayerType,
    },
    TokenCreated {
        name: String,
        token: Token,
        layer: LayerType,
    },
    TokenDestroyed {
        name: String,
        layer: LayerType,
    },
    ImageUpload {
        name: String,
        image: Vec<u8>,
    },
    SendState {
        state: State,
    },
    PersonalUpdate {
        people: Vec<String>,
    },
    HeartBeat,
}
impl Default for Layer {
    fn default() -> Self {
        Self::new()
    }
}

impl Layer{
    pub fn new()->Self{
        Self { tokens: HashMap::new() }
    }
}
#[derive(Serialize, Deserialize, Clone)]
pub struct Event {
    pub source: String,
    pub data: EventData,
}
pub fn path() -> &'static str {
    static S: LazyLock<&'static str> = std::sync::LazyLock::new(|| {
        let dir = std::env::home_dir().unwrap().to_string_lossy().to_string();
        let d = (dir.clone() + "/boredgames/assets/").leak() as &str;
        let d0 = dir.clone() + "/boardgames";
        if !std::fs::exists(&d0).unwrap() {
            std::fs::create_dir(&d0).unwrap();
        }
        if !std::fs::exists(d).unwrap() {
            std::fs::create_dir(d).unwrap();
        }
        d
    });
    &S
}
pub fn get_ip() -> String {
    
    if let Ok(t) = local_ip_address::local_ip() {
        t.to_string()
    } else {
        "127.0.0.1".to_string()
    }
}
