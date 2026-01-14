use bytemuck::Pod;

use crate::{
    component::{ComponentHandle, ComponentStoreHandle},
    graphics::lowlevel::WgpuRenderer,
};

/// A buffer for uniform data.
#[derive(Clone, Debug)]
pub struct UniformBuffer<T>
where
    T: Pod,
{
    label: Option<String>,
    buffer: wgpu::Buffer,
    handle: ComponentHandle<WgpuRenderer>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Pod> UniformBuffer<T> {
    /// Creates a new UniformBuffer from a wgpu::Buffer.
    ///
    /// This function will panic if the buffer size is smaller than the size of type T.
    ///
    /// see also: [`crate::graphics::WgpuInstance::create_buffer`]
    /// # Safety
    /// The caller must ensure that the provided buffer is valid for the type T.
    pub unsafe fn from_raw_parts(
        label: Option<&str>,
        buffer: wgpu::Buffer,
        handle: ComponentStoreHandle,
    ) -> Self {
        assert!(
            buffer.size() as usize >= std::mem::size_of::<T>(),
            "Buffer size is smaller than type T"
        );
        Self {
            buffer,
            _marker: std::marker::PhantomData,
            handle: handle.handle_for::<WgpuRenderer>(),
            label: label.map(|s| s.to_string()),
        }
    }

    /// Returns the underlying wgpu::Buffer.
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Creates a bind group layout for the uniform buffer.
    pub fn bind_group_layout(&self, binding: u32) -> wgpu::BindGroupLayout {
        let wgpu = self.handle.get();
        wgpu.bind_group_layout(
            self.label.as_deref(),
            &[wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        )
    }

    /// Creates a bind group for the uniform buffer.
    pub fn bind_group(&self, binding: u32) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let wgpu = self.handle.get();
        let layout = self.bind_group_layout(binding);
        (
            layout.clone(),
            wgpu.bind_group(
                self.label.as_deref(),
                &layout,
                &[wgpu::BindGroupEntry {
                    binding,
                    resource: self.buffer.as_entire_binding(),
                }],
            ),
        )
    }

    /// Writes data to the uniform buffer.
    pub fn write(&self, data: &T) {
        self.handle
            .get()
            .queue
            .write_buffer(&self.buffer, 0, bytemuck::bytes_of(data));
    }
}
