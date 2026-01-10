use std::{marker::PhantomData, ops::RangeBounds};

use bytemuck::{Pod, Zeroable};
use wgpu::VertexBufferLayout;
#[derive(Debug, Clone)]
pub struct VertexBuffer<T>
where
    T: VertexLayout,
{
    buffer: wgpu::Buffer,
    _marker: PhantomData<T>,
}

impl<T> VertexBuffer<T>
where
    T: VertexLayout,
{
    /// The layout of the vertex buffer.
    pub const LAYOUT: VertexBufferLayout<'static> = T::LAYOUT;

    /// Creates a new WgpuBuffer from a wgpu::Buffer.
    ///
    /// see also: [`crate::graphics::WgpuInstance::create_buffer`]
    /// # Safety
    /// The caller must ensure that the provided buffer is valid for the type T.
    pub unsafe fn from_raw_parts(buffer: wgpu::Buffer) -> Self {
        Self {
            buffer,
            _marker: PhantomData,
        }
    }

    /// Returns the layout of the vertex buffer.
    pub fn layout(&self) -> VertexBufferLayout<'static> {
        T::LAYOUT
    }

    /// Returns the underlying wgpu::Buffer.
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Returns the number of vertices in the buffer.
    pub fn count(&self) -> usize {
        (self.buffer.size() as usize) / std::mem::size_of::<T>()
    }

    /// Sets the vertex buffer on the given render pass at the specified slot and range.
    pub fn set_on(&self, pass: &mut wgpu::RenderPass<'_>, slot: u32, range: impl RangeBounds<u64>) {
        pass.set_vertex_buffer(slot, self.buffer.slice(range));
    }
}

/// A trait for types that can be used as buffer layouts.
///
/// # Safety
/// The implementor must ensure that the given `BufferLayout` correctly describes the memory layout of the type.
pub unsafe trait VertexLayout: Pod + Zeroable {
    const LAYOUT: VertexBufferLayout<'static>;

    const _ASSERT: () = {
        assert!(
            Self::LAYOUT.array_stride as usize == std::mem::size_of::<Self>(),
            "VertexLayout array_stride does not match size of type"
        );
    };
}
