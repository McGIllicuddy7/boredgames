use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct State{
    pub messages:Vec<(String, String)>,
} 
#[derive(Serialize, Deserialize)]
pub enum EventData{
    Message{from:String, contents:String,time_stamp:u128},
    Connection{username:String}, 
    Disconnection{username:String},
    Kill{password:String},
}
#[derive(Serialize, Deserialize)]
pub struct Event{
    pub source:String, 
    pub data:EventData,
}