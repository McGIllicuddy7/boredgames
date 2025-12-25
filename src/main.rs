use std::{error::Error, process::exit};

use eframe::{App, AppCreator, CreationContext, NativeOptions, egui::{self, Style, Ui}};

use crate::client::Client;
pub mod utils;
pub mod server;
pub mod client;
pub mod communication;
pub struct GuiState{
    pub client:Client
}
fn main() ->Result<(), Box< dyn Error>>{
    gui_run()?;
    Ok(())
}

pub fn gui_run()->Result<(),impl Error>{
    eframe::run_native("bored games", NativeOptions::default(), Box::new(app_create))
}
pub fn app_create<'b>(c:&CreationContext<'b>)->Result<Box<dyn App>,Box<dyn Error + Send + Sync + 'static>> {
    let out = Box::new(GuiState{client:Client::new()});
    let theme = if let Some(theme) = c.egui_ctx.system_theme(){
        theme
    } else{
        c.egui_ctx.theme()
    };
    c.egui_ctx.set_theme(theme);
    Ok(out)
}

impl eframe::App for GuiState{
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
       egui::CentralPanel::default().show(ctx, |ui| {
            self.render(ui);
       });
       
    }
}
impl GuiState{
    pub fn render(&mut self, ui:&mut Ui){
        self.client.update(ui);

    }
}