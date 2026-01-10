use std::fmt::Debug;

use glam::{Mat4, Vec2, Vec3, vec2};
use glfw::Key;

use crate::{
    component::{ComponentHandle, ComponentStore},
    graphics::{
        callback::TargetHandle,
        camera::Camera,
        lowlevel::{WgpuRenderer, buf::UniformBuffer},
    },
    window::GlfwWindow,
};

#[derive(Clone)]
pub struct CameraController {
    pub pos: Vec3,
    /// Pitch and yaw rotation.
    pub rot: Vec2,
    camera: Camera,
    uniform: UniformBuffer<Mat4>,
    callback_handle: Option<TargetHandle<(f64, f64)>>,
    wgpu_handle: ComponentHandle<WgpuRenderer>,
}

impl Debug for CameraController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CameraController")
            .field("pos", &self.pos)
            .field("rot", &self.rot)
            .field("inner_camera", &self.camera)
            .finish()
    }
}

impl CameraController {
    pub fn new(state: &ComponentStore) -> CameraController {
        let wgpu = state.get::<WgpuRenderer>();
        let (width, height) = wgpu.dimensions();
        let camera = Camera::new(
            width as f32 / height as f32,
            0.1,
            16.0 * 32.0, // TODO: render distance setting? i think this is in world units
        );

        let uniform = wgpu.uniform_buffer(&camera.projection_view_matrix(), Some("Camera Uniform"));
        CameraController {
            wgpu_handle: state.handle_for::<WgpuRenderer>(),
            camera,
            uniform,
            pos: Vec3::ZERO,
            callback_handle: None,
            rot: Vec2::ZERO,
        }
    }

    pub fn process_rot(&mut self, direction: Vec2) {
        let sensitivity = 0.1;
        self.rot.x += direction.x * sensitivity;
        self.rot.y += direction.y * sensitivity;

        // Clamp the pitch to avoid flipping
        self.rot.y = self.rot.y.clamp(-89.0, 89.0);

        let yaw_radians = self.rot.x.to_radians();
        let pitch_radians = self.rot.y.to_radians();

        self.camera.set_orientation(yaw_radians, pitch_radians);
        self.camera.pos(self.pos);
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
        self.camera.pos(self.pos);
        self.camera.look_at(target);
        self.flush();
    }

    /// Creates a bind group for the camera uniform buffer.
    pub fn bind_group_with_layout(
        &self,
        layout: &wgpu::BindGroupLayout,
        binding: u32,
    ) -> wgpu::BindGroup {
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
    pub fn bind_group(&self, binding: u32) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let layout = self.bind_group_layout(binding);
        (
            layout.clone(),
            self.bind_group_with_layout(&layout, binding),
        )
    }

    /// Registers mouse movement callbacks to control the camera rotation.
    pub fn register_callback(this: ComponentHandle<CameraController>, window: &GlfwWindow) {
        let closure_camera = this.clone();
        let mut last = Vec2::ZERO;
        let mut first_mouse = true;
        let handle = window.register_mouse_pos_callback(Some("camera"), move |(x, y)| {
            let container = closure_camera.clone();
            let mut camera = container.get_mut();
            let pos = vec2(x as f32, y as f32);
            if first_mouse {
                last = pos;
                first_mouse = false;
                return;
            }

            let mut offset = pos - last;
            last = pos;

            // Invert y-axis for typical FPS camera control
            offset *= Vec2::NEG_Y + Vec2::X;

            camera.process_rot(offset);
        });

        this.get_mut().callback_handle = Some(handle);
    }

    pub fn update_camera(&mut self, keyboard: &crate::input::keyboard::Keyboard, delta_time: f64) {
        let speed = 10.0 * delta_time as f32;
        let front = self.front();
        if keyboard.is_key_held(Key::W) {
            let front = self.front();
            self.update_position(|c| c + front * speed);
        }
        if keyboard.is_key_held(Key::S) {
            let front = self.front();
            self.update_position(|c| c - front * speed);
        }
        if keyboard.is_key_held(Key::A) {
            let right = front.cross(Vec3::Y).normalize();
            self.update_position(|c| c - right * speed);
        }
        if keyboard.is_key_held(Key::D) {
            let right = front.cross(Vec3::Y).normalize();
            self.update_position(|c| c + right * speed);
        }

        self.flush();
    }

    pub fn front(&self) -> Vec3 {
        self.camera.front()
    }

    /// Sets the position of the camera.
    pub fn update_position(&mut self, f: impl FnOnce(Vec3) -> Vec3) {
        let new = f(self.pos);
        self.pos = new;
        self.camera.pos(new);
    }

    /// Returns the position of the camera.
    pub fn position(&self) -> Vec3 {
        self.pos
    }
}
