use std::{collections::HashMap, sync::LazyLock};

use eframe::egui::Pos2;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Token {
    pub location: Pos2,
    pub scale: i32,
    pub image: String,
    pub display_name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Layer {
    Base,
    Map,
    Gm,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub messages: Vec<(String, String)>,
    pub tokens: HashMap<String, Token>,
    pub map: HashMap<String, Token>,
    pub gm: HashMap<String, Token>,
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
        layer: Layer,
    },
    TokenCreated {
        name: String,
        token: Token,
        layer: Layer,
    },
    TokenDestroyed {
        name: String,
        layer: Layer,
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
    let ip = if let Ok(t) = local_ip_address::local_ip() {
        t.to_string()
    } else {
        "127.0.0.1".to_string()
    };
    ip
}
