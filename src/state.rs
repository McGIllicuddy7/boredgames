use crate::dos::{Color, Sprite};
pub use crate::id::GlobalId;
pub use serde::{Deserialize, Serialize};
pub use std::collections::BTreeMap;
pub use std::sync::Arc;
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Pos2 {
    pub x: i32,
    pub y: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]

pub struct State {
    pub table: BTreeMap<GlobalId, GameObject>,
}
#[derive(Serialize, Deserialize, Clone, Debug)]

pub struct UserInfo {
    pub username: String,
    pub profile_picture: String,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetState {
    pub state: State,
    pub users: BTreeMap<GlobalId, UserInfo>,
    pub sprites: BTreeMap<String, Sprite>,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameObject {
    pub center: Pos2,
    pub w: i32,
    pub h: i32,
    pub data: GameObjectData,
    pub display_name: Arc<str>,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GameObjectData {
    Token {
        image: Arc<str>,
    },
    DrawCircle {
        color: Color,
    },
    DrawRect {
        color: Color,
    },
    DrawLine {
        start: Pos2,
        end: Pos2,
        width: i32,
        color: Color,
    },
    DrawSpline {
        color: Color,
        points: Arc<[Pos2]>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    pub source: GlobalId,
    pub time: u64, //time in seconds
    pub data: MessageData,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MessageData {
    Handshake {
        id_gen_start: u64,
        new_user_id: GlobalId,
    },
    Login {
        user_name: String,
    },
    Logout,
    TextMessage {
        contents: String,
    },
    DataMessage {
        file_name: String,
        contents: Arc<[u8]>,
    },
    CreateObject {
        id: GlobalId,
        object: GameObject,
    },
    MoveObject {
        id: GlobalId,
        to: Pos2,
    },
    DestroyObject {
        id: GlobalId,
    },
    UpdateObject {
        id: GlobalId,
        object: GameObject,
    },
    SetObjectName {
        id: GlobalId,
        name: String,
    },
    SetUsername {
        id: GlobalId,
        name: String,
    },
    RequestState,
    SendState {
        state: State,
    },
    RequestNetState,
    SendNetState {
        net_state: NetState,
    },
}
