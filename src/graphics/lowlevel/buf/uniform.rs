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
    pub unsafe fn from_raw_parts(buffer: wgpu::Buffer, handle: ComponentStoreHandle) -> Self {
        assert!(
            buffer.size() as usize >= std::mem::size_of::<T>(),
            "Buffer size is smaller than type T"
        );
        Self {
            buffer,
            _marker: std::marker::PhantomData,
            handle: handle.handle_for::<WgpuRenderer>(),
        }
    }

    /// Returns the underlying wgpu::Buffer.
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Writes data to the uniform buffer.
    pub fn write(&self, data: &T) {
        self.handle
            .get()
            .queue
            .write_buffer(&self.buffer, 0, bytemuck::bytes_of(data));
    }
}
