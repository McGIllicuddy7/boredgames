use crate::dos::{SysHandle, setup};

pub mod client;
pub mod dos;
pub mod id;
pub mod rtils;
pub mod state;
pub mod voip;
pub fn testing123() -> i32 {
    24
}
global!(x:i32 = testing123());
pub fn main() {
    if std::env::args().len() > 1 {
        state::run_text_mode();
    }
    setup(main_func);
}
pub fn main_func(mut handle: SysHandle) {
    client::run_client(&mut handle);
}
