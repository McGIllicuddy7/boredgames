use crate::generators::city::create_city;

pub mod engine;
pub mod libgui;
//https://lib.rs/crates/wrappe
pub mod utils;

pub mod builder;

pub mod generators;
#[tokio::main]
pub async fn main() {
    let city = create_city(500);
    city.draw();
    //  println!("{:#?}", city.buildings);
}
