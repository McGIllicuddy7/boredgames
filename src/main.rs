use std::error::Error;

use eframe::{
    App, CreationContext, NativeOptions,
    egui::{self, Ui, Vec2},
};

use crate::client::Client;
pub mod client;
pub mod communication;
pub mod server;
pub mod database;
pub mod utils;
pub struct GuiState {
    pub client: Client,
}
fn main() -> Result<(), Box<dyn Error>> {
    gui_run()?;
    Ok(())
}

pub fn gui_run() -> Result<(), impl Error> {
    eframe::run_native(
        "bored games",
        NativeOptions::default(),
        Box::new(app_create),
    )
}
pub fn app_create<'b>(
    c: &CreationContext<'b>,
) -> Result<Box<dyn App>, Box<dyn Error + Send + Sync + 'static>> {
    egui_extras::install_image_loaders(&c.egui_ctx);
   // c.egui_ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
    c.egui_ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(Vec2::new(1280., 960.)));
    let out = Box::new(GuiState {
        client: Client::new(),
    });
    let theme = if let Some(theme) = c.egui_ctx.system_theme() {
        theme
    } else {
        c.egui_ctx.theme()
    };

    c.egui_ctx.set_theme(theme);
    Ok(out)
}

impl eframe::App for GuiState {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render(ui);
        });
    }
}
impl GuiState {
    pub fn render(&mut self, ui: &mut Ui) {
        self.client.update(ui);
    }
}
