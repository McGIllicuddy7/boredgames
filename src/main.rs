pub mod engine;
pub mod libgui;
//https://lib.rs/crates/wrappe
pub mod utils;
#[tokio::main]
async fn main() {
    let (mut handle, thread) = raylib::RaylibBuilder::default()
        .resizable()
        .width(18 * 1920 / 20)
        .height(18 * 1080 / 20)
        .build();
    handle.set_exit_key(None);
    engine::game_loop(&mut handle, &thread).await;
}
