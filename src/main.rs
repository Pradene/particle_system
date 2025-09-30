mod app;
mod camera;
mod compute_pipeline;
mod render_pipeline;
mod renderer;
mod timer;

use {
    crate::app::App,
    winit::event_loop::{ControlFlow, EventLoop},
};

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    let _ = event_loop.run_app(&mut app);
}
