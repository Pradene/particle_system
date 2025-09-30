use {
    glam::{Mat4, Vec3},
    std::f32::consts::FRAC_PI_2,
    winit::{event::ElementState, keyboard::KeyCode},
};

const MAX_PITCH: f32 = FRAC_PI_2 * 0.99; // Avoids gimbal lock

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    position: [f32; 3],
    padding: f32,
}

impl CameraUniform {
    pub fn new(camera: &Camera) -> Self {
        let view = camera.look_at();
        let proj = camera.projection();

        let view_proj = proj * view;

        Self {
            view_proj: view_proj.to_cols_array_2d(),
            position: camera.position().to_array(),
            padding: 0.0,
        }
    }
}

#[derive(Default)]
pub struct Camera {
    eye: Vec3,
    up: Vec3,
    yaw: f32,   // Rotation around Y axis (left/right)
    pitch: f32, // Rotation around X axis (up/down)
    aspect: f32,
    fov_y: f32,
    near: f32,
    far: f32,
    projection: Mat4,
}

impl Camera {
    pub fn new(
        eye: Vec3,
        target: Vec3,
        up: Vec3,
        aspect: f32,
        fov_x: f32,
        near: f32,
        far: f32,
    ) -> Self {
        let fov_y = 2.0 * (fov_x / 2.0).tan().atan2(aspect);
        let projection = Mat4::perspective_rh(fov_y, aspect, near, far);

        // Calculate initial yaw and pitch from eye-to-target direction
        let direction = (target - eye).normalize();

        let yaw = direction.x.atan2(direction.z);
        let pitch = direction.y.asin().clamp(-MAX_PITCH, MAX_PITCH);

        Self {
            eye,
            up,
            aspect,
            yaw,
            pitch,
            fov_y,
            near,
            far,
            projection,
        }
    }

    pub fn look_at(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye, self.eye + self.direction(), self.up)
    }

    pub fn projection(&self) -> Mat4 {
        self.projection
    }

    pub fn view_proj(&self) -> Mat4 {
        self.projection * self.look_at()
    }

    pub fn direction(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        )
        .normalize()
    }

    pub fn position(&self) -> Vec3 {
        self.eye
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
        self.projection = Mat4::perspective_rh(self.fov_y, self.aspect, self.near, self.far);
    }
}

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

    pub fn update(&mut self, camera: &mut Camera, dt: f32) {
        let (dx, dy) = self.mouse_delta;
        camera.yaw = camera.yaw + dx * self.sensitivity;
        camera.pitch = (camera.pitch - dy * self.sensitivity).clamp(-MAX_PITCH, MAX_PITCH);

        self.mouse_delta = (0.0, 0.0);

        let forward = camera.direction();
        let right = camera.up.cross(forward).normalize();

        let mut movement = Vec3::ZERO;
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
            camera.eye += movement.normalize() * self.speed * dt;
        }
    }
}
