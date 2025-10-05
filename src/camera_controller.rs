use {
    crate::camera::Camera,
    winit::{event::ElementState, keyboard::KeyCode},
};

#[derive(Default)]
pub struct CameraController {
    speed: f32,
    sensitivity: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    mouse_delta: (f32, f32),
}

impl CameraController {
    pub fn new() -> Self {
        Self {
            speed: 5.0,
            sensitivity: 0.002,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            mouse_delta: (0.0, 0.0),
        }
    }

    pub fn process_mouse(&mut self, delta_x: f32, delta_y: f32) {
        self.mouse_delta.0 = delta_x;
        self.mouse_delta.1 = delta_y;
    }

    pub fn process_keyboard(&mut self, state: ElementState, keycode: KeyCode) {
        let is_pressed = state == ElementState::Pressed;
        match keycode {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.is_forward_pressed = is_pressed;
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.is_left_pressed = is_pressed;
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.is_backward_pressed = is_pressed;
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.is_right_pressed = is_pressed;
            }
            _ => {}
        }
    }

    pub fn update(&mut self, camera: &mut Camera, delta_time: f32) {
        let (dx, dy) = self.mouse_delta;
        camera.rotate(dx * self.sensitivity, dy * self.sensitivity);

        let forward = camera.forward();
        let right = camera.right();

        let mut movement = glam::Vec3::ZERO;
        if self.is_forward_pressed {
            movement += forward;
        }
        if self.is_backward_pressed {
            movement -= forward;
        }
        if self.is_right_pressed {
            movement += right;
        }
        if self.is_left_pressed {
            movement -= right;
        }

        if movement.length_squared() > 0.0 {
            camera.translate(movement.normalize() * self.speed * delta_time);
        }

        self.mouse_delta = (0.0, 0.0);
    }
}
