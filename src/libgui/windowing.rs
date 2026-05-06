//more of an application framework smh
use super::Bounds;
use async_trait::async_trait;
use raylib::prelude::*;
#[async_trait]
pub trait Application {
    fn update(
        &mut self,
        current_bounds: Bounds,
        selected: bool,
        handle: &mut RaylibHandle,
        thread: &RaylibThread,
        buffer: &mut super::CommandBufferBuilder,
    );
}
