use crate::state::run_text_mode;

pub mod dos;
pub mod id;
pub mod rtils;
pub mod state;
pub mod voip;
pub fn main() {
    run_text_mode();
}
