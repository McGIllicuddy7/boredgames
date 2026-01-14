use eframe::App;

use crate::{rtils::events::{BPipe, EventSync}, state::BGEvent};

pub struct AppStruct{
    events:BPipe<BGEvent>, 
    sync:EventSync<BGEvent>,
}
impl App for AppStruct{
    fn update(&mut self, _ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        
    }
}
impl AppStruct{
    pub fn new(events:BPipe<BGEvent>,sync:EventSync<BGEvent>)->Self{
        Self {  events, sync}
    }
}