use std::f32::consts;

use glam::{Mat4, Vec2, Vec3, Vec4, vec2};

#[derive(Clone, Debug)]
pub struct Camera {
    projection: Mat4,
    view: Mat4,
    pub rot: Vec2,
    pub position: Vec3,
    direction_vector: Vec3,
}

const FOV_Y_RADS: f32 = consts::FRAC_PI_2;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Mat4= Mat4::from_cols(
    Vec4::new(1.0, 0.0, 0.0, 0.0),
    Vec4::new(0.0, 1.0, 0.0, 0.0),
    Vec4::new(0.0, 0.0, 0.5, 0.0),
    Vec4::new(0.0, 0.0, 0.5, 1.0),
);

impl Camera {
    /// Creates a new Camera with the given projection and view matrices.
    pub fn new(aspect_ratio: f32, z_near: f32, z_far: f32) -> Self {
        let projection = Mat4::perspective_rh(FOV_Y_RADS, aspect_ratio, z_near, z_far);
        let view = Mat4::look_at_rh(Vec3::ZERO, Vec3::ZERO, Vec3::Y);

        Self {
            projection,
            view,
            rot: Vec2::ZERO,
            position: Vec3::ZERO,
            direction_vector: Self::calculate_direction(0.0, 0.0),
        }
    }

    fn calculate_direction(yaw: f32, pitch: f32) -> Vec3 {
        Vec3::new(
            yaw.cos() * pitch.cos(),
            pitch.sin(),
            yaw.sin() * pitch.cos(),
        )
        .normalize()
    }

    /// Resizes the camera's projection matrix.
    pub fn resize(&mut self, aspect_ratio: f32, z_near: f32, z_far: f32) {
        self.projection = Mat4::perspective_rh(FOV_Y_RADS, aspect_ratio, z_near, z_far);
    }

    /// Points the camera in the given yaw and pitch (in radians).
    ///
    /// Yaw's origin is facing down the positive Z axis, increasing clockwise.
    ///
    /// Pitch's origin is facing down the negative Y axis, increasing upwards.
    pub fn set_orientation(&mut self, yaw: f32, pitch: f32) {
        self.rot = Vec2::new(yaw, pitch);

        let direction = Vec3::new(
            yaw.cos() * pitch.cos(),
            pitch.sin(),
            yaw.sin() * pitch.cos(),
        )
        .normalize();
        self.direction_vector = direction;

        let target = Vec3::ZERO;
        let position = target - direction * 2.0;

        self.position = position;
        self.view = Mat4::look_at_rh(position, target, Vec3::Y);
    }

    /// Points the camera to look at the given target position.
    pub fn look_at(&mut self, target: Vec3) {
        let direction = (target - self.position).normalize();
        self.rot = vec2(direction.z.atan2(direction.x), direction.y.asin());

        self.direction_vector = direction;
        self.view = Mat4::look_at_rh(self.position, target, Vec3::Y);
    }

    /// Sets the position of the camera.
    pub fn pos(&mut self, position: Vec3) {
        self.position = position;
        let target = position + self.direction_vector;
        self.view = Mat4::look_at_rh(position, target, Vec3::Y);
    }

    /// Returns the projection matrix of the camera.
    pub fn projection(&self) -> Mat4 {
        self.projection
    }

    /// Returns the view matrix of the camera.
    pub fn view(&self) -> Mat4 {
        self.view
    }

    /// Returns the front direction vector of the camera.
    pub fn front(&self) -> Vec3 {
        self.direction_vector
    }

    /// Returns the combined projection and view matrix of the camera.
    pub fn projection_view_matrix(&self) -> Mat4 {
        OPENGL_TO_WGPU_MATRIX * self.projection * self.view
    }

    /// Flushes the camera's view matrix based on its current position and direction.
    pub fn flush(&mut self) {
        self.direction_vector = Self::calculate_direction(self.rot.x, self.rot.y);
        let target = self.position + self.direction_vector;
        self.view = Mat4::look_at_rh(self.position, target, Vec3::Y);
    }
}
