use {
    crate::{
        camera::Camera,
        camera_controller::CameraController,
        particle_system::{ParticleShape, ParticleSystem, ParticleSystemInfo, UpdateUniforms},
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
        window::{Fullscreen, Window, WindowId},
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

        let window = match event_loop.create_window(window_attributes) {
            Ok(window) => {
                window.set_cursor_visible(false);
                Arc::new(window)
            }
            Err(e) => {
                eprintln!("Failed to create window: {e:?}");
                event_loop.exit();
                return;
            }
        };

        let mut renderer = match pollster::block_on(Renderer::new()) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Failed to create renderer: {e}");
                event_loop.exit();
                return;
            }
        };

        if let Err(e) = renderer.create_surface(window.clone()) {
            eprintln!("Failed to create surface: {e}");
            event_loop.exit();
            return;
        }

        self.camera = Camera::new(
            vec3(0.0, 0.0, 20.0),
            vec3(0.0, 0.0, 0.0),
            1080.0 / 720.0,
            80.0,
            0.1,
            100.0,
        );

        if let Some(surface_format) = renderer.surface_format() {
            let particle_system = ParticleSystem::new(
                renderer.device(),
                surface_format,
                ParticleSystemInfo {
                    shape: ParticleShape::Points,
                    rate: 65536,
                    lifetime: 10.0,
                },
            );

            self.particle_system = Some(particle_system);
        }

        self.window = Some(window);
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
                if let Err(e) = window.set_cursor_position(center) {
                    eprintln!("Failed to set cursor position: {e:?}");
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let window = match self.window.as_mut() {
            Some(window) => window,
            None => return,
        };

        if window_id != window.id() {
            return;
        }

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
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::F11),
                        ..
                    },
                ..
            } => {
                if let Some(monitor) = window.current_monitor() {
                    match window.fullscreen() {
                        Some(_) => window.set_fullscreen(None),
                        None => window.set_fullscreen(Some(Fullscreen::Borderless(Some(monitor)))),
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::KeyQ),
                        ..
                    },
                ..
            } => {
                if let Some(particle_system) = &mut self.particle_system {
                    let shape = particle_system.get_shape();
                    match shape {
                        ParticleShape::Points => particle_system.set_shape(ParticleShape::Quads),
                        ParticleShape::Quads => particle_system.set_shape(ParticleShape::Points),
                    }
                }
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

                let title = format!("Particle system ({} FPS)", (1.0 / delta_time) as u32);
                window.set_title(title.as_str());

                if let Some(renderer) = &mut self.renderer {
                    self.camera_controller.update(&mut self.camera, delta_time);
                    match renderer.begin_frame() {
                        Ok(mut frame) => {
                            if let Some(particle_system) = &mut self.particle_system {
                                let uniforms = UpdateUniforms {
                                    gravity_center: [0.0, 0.0, 0.0, 1.0],
                                    gravity_strength: 10.0,
                                    delta_time,
                                    padding: [0.0; 2],
                                };

                                particle_system.update(renderer.queue(), &mut frame, uniforms);
                                particle_system.emit(renderer.queue(), &mut frame);

                                particle_system.render(renderer.queue(), &mut frame, &self.camera);
                            }

                            renderer.end_frame(frame);
                        }
                        Err(wgpu::SurfaceError::Lost) => {
                            if let Some(renderer) = &mut self.renderer
                                && let Err(e) = renderer.create_surface(window.clone())
                            {
                                eprintln!("Failed to recreate surface: {e}");
                                event_loop.exit();
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

                window.request_redraw();
            }
            _ => (),
        }
    }
}
