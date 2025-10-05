use glam::{Mat4, Quat, Vec3};

#[derive(Default)]
pub struct Camera {
    position: Vec3,
    orientation: Quat,
    aspect: f32,
    fov_y: f32,
    near: f32,
    far: f32,
    projection: Mat4,
}

impl Camera {
    pub fn new(position: Vec3, target: Vec3, aspect: f32, fov_x: f32, near: f32, far: f32) -> Self {
        let fov_y = 2.0 * (fov_x / 2.0).tan().atan2(aspect);

        let forward = (target - position).normalize();
        let orientation = Quat::from_rotation_arc(Vec3::NEG_Z, forward);

        let projection = Self::perspective(fov_y, aspect, near, far);

        Self {
            position,
            orientation,
            aspect,
            fov_y,
            near,
            far,
            projection,
        }
    }

    pub fn projection(&self) -> Mat4 {
        self.projection
    }

    pub fn view_proj(&self) -> Mat4 {
        self.projection() * self.view()
    }

    pub fn forward(&self) -> Vec3 {
        self.orientation * Vec3::NEG_Z
    }

    pub fn right(&self) -> Vec3 {
        self.orientation * Vec3::NEG_X
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
        self.projection = Self::perspective(self.fov_y, self.aspect, self.near, self.far);
    }

    pub fn translate(&mut self, offset: Vec3) {
        self.position += offset;
    }

    pub fn rotate(&mut self, delta_yaw: f32, delta_pitch: f32) {
        let yaw_quat = Quat::from_rotation_y(delta_yaw);
        let pitch_quat = Quat::from_axis_angle(Vec3::X, delta_pitch);

        let orientation = yaw_quat * self.orientation * pitch_quat;
        self.orientation = orientation.normalize();
    }

    pub fn view(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.orientation, self.position).inverse()
    }

    fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
        Mat4::perspective_rh(fov_y, aspect, near, far)
    }
}
