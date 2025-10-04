mod app;
mod camera;
mod camera_controller;
mod particle_system;
mod renderer;
mod timer;

use {
    crate::app::App,
    winit::event_loop::{ControlFlow, EventLoop},
};

fn main() {
    let event_loop = match EventLoop::new() {
        Ok(event_loop) => event_loop,
        Err(e) => {
            eprintln!("Failed to create event loop: {}", e);
            return;
        }
    };

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    let _ = event_loop.run_app(&mut app);
}
