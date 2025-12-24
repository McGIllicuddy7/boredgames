use std::net::TcpStream;

use eframe::egui::Ui;

use crate::communication::*;
pub struct Client{
    pub state:State,
    pub typed_message:String,
    pub ip_address:String,
    pub connection:Option<TcpStream>,
}
impl Client{
    pub fn new()->Self{
        Self { state: State::new(), typed_message: String::new(), ip_address:String::new(), connection: None}
    }
    pub fn update(&mut self, ui:&mut Ui){
        let mut should_send = false;
        let mut should_connect = false;
        let mut should_host = false;
        ui.vertical_centered(|ui|{
            for i in &self.state.messages{
                ui.label(format!("{:#?}:{:#?}", i.0, i.1));
            }
            ui.horizontal(|ui|{
                if ui.text_edit_singleline(&mut self.typed_message).clicked(){
                    should_send = true;
                }
            })
        });
        ui.horizontal(|ui| {
            ui.label("enter ip address");
            if ui.text_edit_singleline(&mut self.ip_address ).clicked(){
                should_connect = true;
            }
            if ui.button("host own server").clicked(){
                should_host = true;
            }
        });

        if should_connect{
            println!("should connect to:{:#?}", self.ip_address);
            self.ip_address.clear();
        }
        if should_send{
            println!("should send:{:#}", self.typed_message);
            self.typed_message.clear();
        }
        if should_host{
            println!("should host");
        }

    }
}