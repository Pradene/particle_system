use {
    crate::{renderer::Renderer, timer::Timer},
    std::sync::Arc,
    winit::{
        application::ApplicationHandler,
        dpi::PhysicalSize,
        event::{ElementState, KeyEvent, WindowEvent},
        event_loop::ActiveEventLoop,
        keyboard::{KeyCode, PhysicalKey},
        window::{Window, WindowId},
    },
};

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    timer: Timer,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("Particle System")
            .with_inner_size(PhysicalSize::new(1080, 720))
            .with_resizable(true);

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window = Some(window.clone());

        let mut renderer = pollster::block_on(Renderer::new());
        renderer.create_surface(window);

        self.renderer = Some(renderer);
        self.timer = Timer::new();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(physical_size);
                }
            }
            WindowEvent::RedrawRequested => {
                self.timer.update();

                if let Some(renderer) = &mut self.renderer {
                    let delta_time = self.timer.delta_time();

                    match renderer.update(delta_time) {
                        Ok(()) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            if let (Some(window), Some(renderer)) =
                                (&self.window, &mut self.renderer)
                            {
                                renderer.create_surface(window.clone());
                            }
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            event_loop.exit();
                        }
                        Err(e) => {
                            eprintln!("Render error: {e:?}");
                        }
                    }
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => (),
        }
    }
}
