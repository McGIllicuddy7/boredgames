use eframe::NativeOptions;
use tokio::spawn;

use crate::{app::AppStruct, rtils::events::{EventForwarder, EventHandler, EventSync}, state::{BGEvent, setup}};
pub mod state;
pub mod app;
pub mod rtils;
#[tokio::main]
pub async fn main(){
    let (sender, mut handler) = EventHandler::<BGEvent>::new();
    let fwd = EventForwarder::new(|_|{
        true
    }, EventSync::new(sender.clone()));
    spawn(async move{
        handler.run(setup).await
    });
    eframe::run_native("bored_games", NativeOptions::default(), Box::new(|_context|{
        Ok(Box::new(AppStruct::new(fwd, EventSync::new(sender))))
    })).unwrap();
}