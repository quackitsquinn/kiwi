use std::fmt::Debug;

use glam::{Mat4, Vec2, Vec3, vec2};
use winit::keyboard::{Key, KeyCode};

use crate::{
    component::{ComponentHandle, ComponentStore},
    graphics::{
        callback::TargetHandle,
        camera::Camera,
        lowlevel::{WgpuRenderer, buf::UniformBuffer},
    },
};

#[derive(Clone)]
pub struct CameraController {
    /// Mouse sensitivity.
    pub sensitivity: f32,
    camera: Camera,
    uniform: UniformBuffer<Mat4>,
    wgpu_handle: ComponentHandle<WgpuRenderer>,
}

impl Debug for CameraController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CameraController")
            .field("pos", &self.camera.position)
            .field("rot", &self.camera.rot)
            .field("inner_camera", &self.camera)
            .finish()
    }
}

impl CameraController {
    /// Creates a new CameraController with the given parameters.
    pub fn new(
        state: &ComponentStore,
        dimensions: (u32, u32),
        z_near: f32,
        z_far: f32,
    ) -> CameraController {
        let wgpu = state.get::<WgpuRenderer>();
        let (width, height) = dimensions;
        let camera = Camera::new(width as f32 / height as f32, z_near, z_far);

        let uniform = wgpu.uniform_buffer(&camera.projection_view_matrix(), Some("Camera Uniform"));
        CameraController {
            wgpu_handle: state.handle_for::<WgpuRenderer>(),
            camera,
            uniform,
            sensitivity: 0.1,
        }
    }

    /// Returns a clone of the camera's uniform buffer.
    pub fn uniform(&self) -> UniformBuffer<Mat4> {
        self.uniform.clone()
    }

    /// Creates a bind group layout for the camera uniform buffer.
    pub fn bind_group_layout(&self, binding: u32) -> wgpu::BindGroupLayout {
        self.wgpu_handle.get().bind_group_layout(
            Some("camera bind group layout"),
            &[wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        )
    }

    /// Writes the current camera matrix to the uniform buffer.
    pub fn flush(&mut self) {
        let matrix = self.camera.projection_view_matrix();
        self.uniform.write(&matrix);
    }

    /// Sets the camera to look at a specific target point.
    pub fn look_at(&mut self, target: Vec3) {
        self.camera.look_at(target);
        self.flush();
    }

    /// Creates a bind group for the camera uniform buffer.
    pub fn bind_group(&self, layout: &wgpu::BindGroupLayout, binding: u32) -> wgpu::BindGroup {
        self.wgpu_handle.get().bind_group(
            Some("camera bind group"),
            layout,
            &[wgpu::BindGroupEntry {
                binding,
                resource: wgpu::BindingResource::Buffer(
                    self.uniform.buffer().as_entire_buffer_binding(),
                ),
            }],
        )
    }

    /// Creates a bind group for the camera uniform buffer.
    pub fn bind_group_and_layout(&self, binding: u32) -> (wgpu::BindGroup, wgpu::BindGroupLayout) {
        let layout = self.bind_group_layout(binding);
        (self.bind_group(&layout.clone(), binding), layout)
    }

    /// Updates the camera rotation based on mouse movement.
    pub fn update_with_mouse_coords(&mut self, mouse_delta: Vec2, delta_time: f64) {
        let delta = mouse_delta * self.sensitivity * delta_time as f32;

        self.camera.rot += delta;

        // Clamp the pitch to avoid flipping. rot is in radians.
        self.camera.rot.y = self
            .camera
            .rot
            .y
            .clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());

        self.camera.flush();
    }

    /// Updates the camera position based on keyboard input.
    pub fn update_camera(&mut self, keyboard: &crate::input::keyboard::Keyboard, delta_time: f64) {
        let speed = 10.0 * delta_time as f32;
        let front = self.camera.front();
        if keyboard.is_key_held(KeyCode::KeyW) {
            self.update_position(|c| c + front * speed);
        }
        if keyboard.is_key_held(KeyCode::KeyS) {
            self.update_position(|c| c - front * speed);
        }
        if keyboard.is_key_held(KeyCode::KeyA) {
            let right = front.cross(Vec3::Y).normalize();
            self.update_position(|c| c - right * speed);
        }
        if keyboard.is_key_held(KeyCode::KeyD) {
            let right = front.cross(Vec3::Y).normalize();
            self.update_position(|c| c + right * speed);
        }

        self.flush();
    }

    /// Sets the position of the camera.
    pub fn update_position(&mut self, f: impl FnOnce(Vec3) -> Vec3) {
        let new = f(self.camera.position);
        self.camera.pos(new);
    }

    /// Returns a reference to the inner camera.
    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    /// Returns a mutable reference to the inner camera.
    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    /// Returns the position of the camera.
    pub fn position(&self) -> Vec3 {
        self.camera.position
    }
}
