use {
    crate::{
        camera::Camera,
        input_handler::InputHandler,
        particle_system::{ParticleShape, ParticleSystem, ParticleSystemInfo, UpdateUniforms},
        renderer::Renderer,
        timer::Timer,
    },
    std::sync::Arc,
    winit::{
        application::ApplicationHandler,
        dpi::{PhysicalPosition, PhysicalSize},
        event::{DeviceEvent, DeviceId, ElementState, WindowEvent},
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
    timer: Timer,
    particle_system: Option<ParticleSystem>,
    input_handler: InputHandler,
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
            glam::vec3(0.0, 0.0, 20.0),
            glam::vec3(0.0, 0.0, 0.0),
            1080.0 / 720.0,
            80.0,
            0.1,
            100.0,
        );

        let surface_format = if let Some(surface_format) = renderer.surface_format() {
            surface_format
        } else {
            event_loop.exit();
            return;
        };

        self.window = Some(window);
        self.particle_system = Some(ParticleSystem::new(
            renderer.device(),
            surface_format,
            ParticleSystemInfo {
                shape: ParticleShape::Points,
                rate: 300000,
                lifetime: 10.0,
            },
        ));
        self.renderer = Some(renderer);
        self.input_handler = InputHandler::new();
        self.timer = Timer::new();
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event {
            let sensitivity = 0.002;
            let (x, y) = (dx as f32 * sensitivity, dy as f32 * sensitivity);
            self.camera.rotate(x, y);

            // Reset cursor to center
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
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                self.camera
                    .resize(physical_size.width, physical_size.height);
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(physical_size);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let key_code = match event.physical_key {
                    PhysicalKey::Code(code) => code,
                    _ => return,
                };

                // Update key state
                match event.state {
                    ElementState::Pressed => {
                        self.input_handler.set_key(key_code, true);
                    }
                    ElementState::Released => {
                        self.input_handler.set_key(key_code, false);
                    }
                }

                // Handle one-time actions on key press
                if event.state == ElementState::Pressed {
                    match key_code {
                        KeyCode::Escape => event_loop.exit(),
                        KeyCode::F11 => {
                            if let Some(monitor) = window.current_monitor() {
                                match window.fullscreen() {
                                    Some(_) => window.set_fullscreen(None),
                                    None => window.set_fullscreen(Some(Fullscreen::Borderless(
                                        Some(monitor),
                                    ))),
                                }
                            }
                        }
                        KeyCode::KeyR => {
                            if let Some(particle_system) = &mut self.particle_system {
                                particle_system.resume();
                            }
                        }
                        KeyCode::KeyP => {
                            if let Some(particle_system) = &mut self.particle_system {
                                particle_system.pause();
                            }
                        }
                        KeyCode::KeyT => {
                            if let Some(particle_system) = &mut self.particle_system {
                                if let Some(renderer) = &self.renderer {
                                    particle_system.restart(renderer.queue());
                                }
                            }
                        }
                        KeyCode::KeyQ => {
                            if let Some(particle_system) = &mut self.particle_system {
                                particle_system.toggle_shape();
                            }
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let delta_time = self.timer.tick();

                let speed = 5.0;
                let movement_amount = speed * delta_time;

                if self.input_handler.is_key_pressed(KeyCode::KeyW) {
                    self.camera
                        .translate(self.camera.forward() * movement_amount);
                }
                if self.input_handler.is_key_pressed(KeyCode::KeyS) {
                    self.camera
                        .translate(-self.camera.forward() * movement_amount);
                }
                if self.input_handler.is_key_pressed(KeyCode::KeyA) {
                    self.camera
                        .translate(-self.camera.right() * movement_amount);
                }
                if self.input_handler.is_key_pressed(KeyCode::KeyD) {
                    self.camera.translate(self.camera.right() * movement_amount);
                }

                let title = format!("Particle system ({} FPS)", (1.0 / delta_time) as u32);
                window.set_title(title.as_str());

                if let Some(renderer) = &mut self.renderer {
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
