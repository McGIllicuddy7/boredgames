use std::{error::Error, process::exit};

use eframe::{App, AppCreator, CreationContext, NativeOptions, egui::{self, Ui}};
pub mod utils;
pub struct GuiState{
    pub username:String,
}
fn main() ->Result<(), Box< dyn Error>>{
    gui_run()?;
    Ok(())
}

pub fn gui_run()->Result<(),impl Error>{
    eframe::run_native("bored games", NativeOptions::default(), Box::new(app_create))
}
pub fn app_create<'b>(c:&CreationContext<'b>)->Result<Box<dyn App>,Box<dyn Error + Send + Sync + 'static>> {
    let out = Box::new(GuiState{username:"Bridget".into()});
    let theme = if let Some(theme) = c.egui_ctx.system_theme(){
        theme
    } else{
        c.egui_ctx.theme()
    };
    c.egui_ctx.set_theme(theme);
    Ok(out)
}

impl eframe::App for GuiState{
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
       egui::CentralPanel::default().show(ctx, |ui| {
            self.render(ui);
       });
       
    }
}
impl GuiState{
    pub fn render(&mut self, ui:&mut Ui){
        ui.vertical_centered(|ui|{
            if ui.button("testing 1 2 3").clicked(){
                exit(0);
            }
            ui.label("hi there");
        });

    }
}