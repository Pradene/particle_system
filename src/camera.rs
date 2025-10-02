use {
    glam::{Mat4, Vec3},
    std::f32::consts::FRAC_PI_2,
};

const MAX_PITCH: f32 = FRAC_PI_2 * 0.99; // Avoids gimbal lock

#[derive(Default)]
pub struct Camera {
    eye: Vec3,
    up: Vec3,
    pitch: f32, // Rotation around X axis (up/down)
    yaw: f32,   // Rotation around Y axis (left/right)
    aspect: f32,
    fov_y: f32,
    near: f32,
    far: f32,
    view: Mat4,
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

        let view = Mat4::look_at_rh(eye, target, up);

        Self {
            eye,
            up,
            aspect,
            yaw,
            pitch,
            fov_y,
            near,
            far,
            view,
            projection,
        }
    }

    pub fn view(&self) -> Mat4 {
        self.view
    }

    pub fn projection(&self) -> Mat4 {
        self.projection
    }

    pub fn view_proj(&self) -> Mat4 {
        self.projection() * self.view()
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

    pub fn look_at(&mut self, target: Vec3) {
        self.view = Mat4::look_at_rh(self.eye, target, self.up);
    }

    pub fn translate(&mut self, offset: Vec3) {
        self.eye += offset;
    }

    pub fn rotate(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        self.pitch = (self.pitch + delta_pitch).clamp(-MAX_PITCH, MAX_PITCH);
    }
}
