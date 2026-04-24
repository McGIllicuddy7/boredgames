use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use serde::{Deserialize, Serialize};

use crate::{
    gui::{Bounds, Point},
    utils::{ObjectId, Stream},
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BoardState {
    pub objects: HashMap<ObjectId, Object>,
    pub background_image: Arc<str>,
    pub people: HashMap<UserId, Arc<str>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Object {
    pub owner: UserId,
    pub id: ObjectId,
    pub bounds: Bounds,
    pub data: ObjectData,
}

#[derive(Serialize, Deserialize, Clone, Debug, Copy, PartialEq)]
pub struct Col {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ObjectData {
    Token {
        token_image_name: Arc<str>,
    },
    DrawingRectangle {
        tint: Col,
        rotation: f32,
        width: i32,
        height: i32,
    },
    DrawingCircle {
        tint: Col,
    },
    DrawingSpline {
        tint: Col,
        points: Vec<Point>,
        rotation: f32,
    },
    Text {
        text: String,
        height: i32,
    },
}

#[repr(transparent)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserId {
    id: ObjectId,
}

impl UserId {
    pub const fn is_valid(&self) -> bool {
        self.id.is_valid()
    }

    pub const fn is_invalid(&self) -> bool {
        self.id.is_invalid()
    }

    pub const fn new_invalid() -> Self {
        Self {
            id: ObjectId::new_invalid(),
        }
    }

    pub fn new() -> Self {
        Self {
            id: ObjectId::new(),
        }
    }
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Event {
    pub source: UserId,
    pub data: EventData,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum EventData {
    Message { contents: Arc<str> },
    RequestEntireState,
    EntireState { state: BoardState },
    UserConnected { name: Arc<str> },
    UserDisconnected,
    ObjectCreated { id: ObjectId, value: Object },
    ObjectDestroyed { id: ObjectId },
    ObjectUpdated { id: ObjectId, value: Object },
    KickRequest { to_kick: UserId },
}

pub struct Interface {
    pub con: Stream<Event>,
}
