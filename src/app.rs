use {
    crate::{
        camera::{Camera, CameraController},
        particle_system::ParticleSystem,
        renderer::Renderer,
        timer::Timer,
    },
    glam::vec3,
    std::sync::Arc,
    winit::{
        application::ApplicationHandler,
        dpi::{PhysicalPosition, PhysicalSize},
        event::{DeviceEvent, DeviceId, ElementState, KeyEvent, WindowEvent},
        event_loop::ActiveEventLoop,
        keyboard::{KeyCode, PhysicalKey},
        window::{Window, WindowId},
    },
};

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    camera: Camera,
    camera_controller: CameraController,
    timer: Timer,
    particle_system: Option<ParticleSystem>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("Particle System")
            .with_inner_size(PhysicalSize::new(1080, 720))
            .with_resizable(true);

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        window.set_cursor_visible(false);
        self.window = Some(window.clone());

        let mut renderer = pollster::block_on(Renderer::new());
        renderer.create_surface(window);

        self.camera = Camera::new(
            vec3(0.0, 0.0, 10.0),
            vec3(0.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
            1080.0 / 720.0,
            80.0,
            0.1,
            100.0,
        );

        if let Some(surface_format) = renderer.surface_format() {
            let mut particle_system = ParticleSystem::new(renderer.device(), surface_format, 65536);

            particle_system.emit(renderer.device(), renderer.queue());

            self.particle_system = Some(particle_system);
        }

        self.renderer = Some(renderer);
        self.timer = Timer::new();
        self.camera_controller = CameraController::new();
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event {
            self.camera_controller.process_mouse(dx as f32, dy as f32);

            // reset cursor to center
            if let Some(window) = &self.window {
                let size = window.inner_size();
                let center = PhysicalPosition::new(size.width / 2, size.height / 2);
                window.set_cursor_position(center).unwrap();
            }
        }
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
                self.camera
                    .resize(physical_size.width, physical_size.height);
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(physical_size);
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key: PhysicalKey::Code(keycode),
                        ..
                    },
                ..
            } => {
                self.camera_controller.process_keyboard(state, keycode);
            }
            WindowEvent::RedrawRequested => {
                let delta_time = self.timer.tick();

                if let Some(window) = &self.window {
                    let title = format!("Particle system ({} FPS)", (1.0 / delta_time) as u32);
                    window.set_title(title.as_str());
                }

                if let Some(renderer) = &mut self.renderer {
                    self.camera_controller.update(&mut self.camera, delta_time);
                    match renderer.begin_frame() {
                        Ok(mut frame) => {
                            if let Some(particle_system) = &mut self.particle_system {
                                particle_system.update(renderer.queue(), &mut frame, delta_time);
                                particle_system.render(renderer.queue(), &mut frame, &self.camera);
                            }

                            renderer.end_frame(frame);
                        }
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
