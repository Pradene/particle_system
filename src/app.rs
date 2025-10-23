use {
    crate::{
        camera::Camera,
        input_handler::InputHandler,
        particle_system::{
            ParticleEmissionMode, ParticleEmissionShape, ParticleSystem, ParticleSystemInfo,
            UpdateUniforms,
        },
        renderer::Renderer,
        timer::Timer,
    },
    core::f32,
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
struct Parameters {
    sensitivity: f32,
    move_speed: f32,
}

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    camera: Camera,
    timer: Timer,
    particle_system: Option<ParticleSystem>,
    input_handler: InputHandler,
    parameters: Parameters,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let width = 1080;
        let height = 720;

        let window_attributes = Window::default_attributes()
            .with_title("Particle System")
            .with_inner_size(PhysicalSize::new(width, height))
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

        let renderer = match pollster::block_on(Renderer::new(window.clone())) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Failed to create renderer: {e}");
                event_loop.exit();
                return;
            }
        };

        self.camera = Camera::new(
            glam::vec3(0.0, 0.0, 20.0),
            glam::vec3(0.0, 0.0, 0.0),
            glam::vec3(0.0, 1.0, 0.0),
            width as f32 / height as f32,
            (120.0f32).to_radians(),
            0.1,
            1000.0,
        );

        let surface_format = renderer.surface_format();

        let particle_system = ParticleSystem::new(
            renderer.device(),
            surface_format,
            ParticleSystemInfo {
                position: glam::Vec3::ZERO,
                shape: ParticleEmissionShape::Sphere,
                mode: ParticleEmissionMode::Burst(1000000),
                lifetime: f32::INFINITY,
            },
        );

        let parameters = Parameters {
            sensitivity: 1.0,
            move_speed: 10.0,
        };

        self.particle_system = Some(particle_system);
        self.window = Some(window);
        self.renderer = Some(renderer);

        self.parameters = parameters;
        self.input_handler = InputHandler::new();
        self.timer = Timer::new();
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event {
            if let Some(window) = &self.window {
                let size = window.inner_size();

                let x = (dx as f32 / size.width as f32) * self.parameters.sensitivity;
                let y = (dy as f32 / size.height as f32) * self.parameters.sensitivity;

                self.camera.rotate(x, y);

                // Reset cursor to center
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
                let width = physical_size.width;
                let height = physical_size.height;

                self.camera.resize(width, height);
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(width, height);
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
                            let monitor = window
                                .current_monitor()
                                .or_else(|| window.available_monitors().next());

                            if let Some(monitor) = monitor {
                                match window.fullscreen() {
                                    Some(_) => window.set_fullscreen(None),
                                    None => window.set_fullscreen(Some(Fullscreen::Borderless(
                                        Some(monitor),
                                    ))),
                                }

                                if let Some(renderer) = &mut self.renderer {
                                    let size = window.inner_size();
                                    renderer.resize(size.width, size.height);
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
                            if let Some(particle_system) = &mut self.particle_system
                                && let Some(renderer) = &self.renderer
                            {
                                particle_system.restart(renderer.queue());
                            }
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let delta_time = self.timer.tick();

                let speed = self.parameters.move_speed;
                let scale = speed * delta_time;

                if self.input_handler.is_key_pressed(KeyCode::KeyW) {
                    self.camera.translate(self.camera.forward() * scale);
                }
                if self.input_handler.is_key_pressed(KeyCode::KeyA) {
                    self.camera.translate(-self.camera.right() * scale);
                }
                if self.input_handler.is_key_pressed(KeyCode::KeyS) {
                    self.camera.translate(-self.camera.forward() * scale);
                }
                if self.input_handler.is_key_pressed(KeyCode::KeyD) {
                    self.camera.translate(self.camera.right() * scale);
                }

                let title = format!("Particle system ({} FPS)", (1.0 / delta_time) as u32);
                window.set_title(title.as_str());

                if let Some(renderer) = &mut self.renderer {
                    match renderer.begin_frame() {
                        Ok(mut frame) => {
                            if let Some(particle_system) = &mut self.particle_system {
                                let gravity_center = (self.camera.position()
                                    + self.camera.forward() * 20.0)
                                    .extend(1.0)
                                    .to_array();
                                let uniforms = UpdateUniforms {
                                    gravity_center,
                                    elapsed_time: particle_system.elapsed_time(),
                                    delta_time,
                                    padding: [0.0; 2],
                                };

                                particle_system.update(&mut frame, uniforms);
                                particle_system.emit(&mut frame);
                                particle_system.render(&mut frame, &self.camera);
                            }

                            renderer.end_frame(frame);
                        }
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            let size = window.inner_size();
                            renderer.resize(size.width, size.height);
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
