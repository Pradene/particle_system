#[derive(Default)]
pub struct Camera {
    position: glam::Vec3,
    orientation: glam::Quat,
    aspect: f32,
    fov_y: f32,
    up: glam::Vec3,
    znear: f32,
    zfar: f32,
}

impl Camera {
    pub fn new(
        position: glam::Vec3,
        target: glam::Vec3,
        up: glam::Vec3,
        aspect: f32,
        fov_x: f32,
        znear: f32,
        zfar: f32,
    ) -> Self {
        let fov_y = 2.0 * ((fov_x / 2.0).tan() / aspect).atan();

        let view = glam::Mat4::look_at_rh(position, target, up);
        let orientation = glam::Quat::from_mat4(&view.inverse());

        Self {
            position,
            orientation,
            aspect,
            fov_y,
            up,
            znear,
            zfar,
        }
    }

    pub fn view_proj(&self) -> glam::Mat4 {
        self.projection() * self.view()
    }

    pub fn forward(&self) -> glam::Vec3 {
        self.orientation * glam::Vec3::NEG_Z
    }

    pub fn right(&self) -> glam::Vec3 {
        self.orientation * glam::Vec3::X
    }

    pub fn up(&self) -> glam::Vec3 {
        self.up
    }

    pub fn position(&self) -> glam::Vec3 {
        self.position
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn translate(&mut self, offset: glam::Vec3) {
        self.position += offset;
    }

    pub fn rotate(&mut self, delta_yaw: f32, delta_pitch: f32) {
        let yaw_quat = glam::Quat::from_axis_angle(self.up(), -delta_yaw);
        let pitch_quat = glam::Quat::from_axis_angle(self.right(), -delta_pitch);

        self.orientation = (yaw_quat * pitch_quat * self.orientation).normalize();
    }

    pub fn projection(&self) -> glam::Mat4 {
        glam::Mat4::perspective_rh(self.fov_y, self.aspect, self.znear, self.zfar)
    }

    pub fn view(&self) -> glam::Mat4 {
        glam::Mat4::from_rotation_translation(self.orientation, self.position).inverse()
    }
}
