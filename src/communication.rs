use std::collections::HashMap;


use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Loc{
    pub x:i32, pub y:i32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Token{
    pub location:Loc,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State{
    pub messages:Vec<(String, String)>,
    pub tokens:HashMap<String, Token>,
} 
#[derive(Serialize, Deserialize, Debug,Clone)]
pub enum EventData{
    Message{from:String, contents:String,time_stamp:u128},
    Connection{username:String}, 
    Disconnection{username:String},
    Kill{password:String},
    HeartBeat,
}
#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct Event{
    pub source:String, 
    pub data:EventData,
}