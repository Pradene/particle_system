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

        let direction = (target - eye).normalize();
        let yaw = direction.x.atan2(direction.z);
        let pitch = direction.y.asin().clamp(-MAX_PITCH, MAX_PITCH);

        let projection = Self::perspective(fov_y, aspect, near, far);
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
        self.projection = Self::perspective(self.fov_y, self.aspect, self.near, self.far);
    }

    pub fn look_at(&mut self, target: Vec3) {
        self.view = Mat4::look_at_rh(self.eye, target, self.up);
    }

    pub fn translate(&mut self, offset: Vec3) {
        self.eye += offset;
    }

    pub fn rotate(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.pitch = (self.pitch + delta_pitch).clamp(-MAX_PITCH, MAX_PITCH);
        self.yaw = self.yaw + delta_yaw;
    }

    pub fn perspective(fov_y_radians: f32, aspect_ratio: f32, z_near: f32, z_far: f32) -> Mat4 {
        let (sin_fov, cos_fov) = f32::sin_cos(0.5 * fov_y_radians);
        let h = cos_fov / sin_fov;
        let w = h / aspect_ratio;
        let r = z_far / (z_near - z_far);

        glam::Mat4::from_cols(
            glam::Vec4::new(-w, 0.0, 0.0, 0.0),
            glam::Vec4::new(0.0, -h, 0.0, 0.0),
            glam::Vec4::new(0.0, 0.0, r, -1.0),
            glam::Vec4::new(0.0, 0.0, r * z_near, 0.0),
        )
    }
}
