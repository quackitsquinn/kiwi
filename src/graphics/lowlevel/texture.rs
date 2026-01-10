use std::fmt::Debug;

use crate::{
    component::{ComponentHandle, ComponentStoreHandle},
    graphics::lowlevel::WgpuRenderer,
};

/// A structure representing a texture, its view, and its sampler.
#[derive(Clone)]
pub struct Texture {
    /// The underlying wgpu texture.
    pub texture: wgpu::Texture,
    /// The texture bind group layout entry.
    pub texture_bind_group_entry: wgpu::BindGroupLayoutEntry,
    /// The texture sampler.
    pub sampler: wgpu::Sampler,
    /// The sampler bind group layout entry.
    pub sampler_bind_group_entry: wgpu::BindGroupLayoutEntry,
    /// The texture view.
    pub view: wgpu::TextureView,
    image_count: usize,
    handle: ComponentHandle<WgpuRenderer>,
}

impl Texture {
    /// Creates a new texture from the given texture and sampler.
    pub fn new(
        state: &ComponentStoreHandle,
        texture: wgpu::Texture,
        texture_bind_group_entry: wgpu::BindGroupLayoutEntry,
        sampler: wgpu::Sampler,
        sampler_bind_group_entry: wgpu::BindGroupLayoutEntry,
        view: wgpu::TextureView,
        image_count: usize,
    ) -> Self {
        Self {
            texture,
            texture_bind_group_entry,
            sampler,
            sampler_bind_group_entry,
            view,
            handle: state.handle_for::<WgpuRenderer>(),
            image_count,
        }
    }

    /// Creates a bind group layout for this texture.
    pub fn layout(
        &self,
        label: Option<&str>,
        sampler_index: u32,
        texture_index: u32,
    ) -> wgpu::BindGroupLayout {
        self.handle
            .get()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: sampler_index,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: texture_index,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                ],
            })
    }

    pub fn bind_group(
        &self,
        label: Option<&str>,
        layout: &wgpu::BindGroupLayout,
        sampler_binding: u32,
        texture_binding: u32,
    ) -> wgpu::BindGroup {
        self.handle
            .get()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label,
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: sampler_binding,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: texture_binding,
                        resource: wgpu::BindingResource::TextureView(&self.view),
                    },
                ],
            })
    }

    pub fn layout_and_bind_group(
        &self,
        label: Option<&str>,
        sampler_binding: u32,
        texture_binding: u32,
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let layout = self.layout(label, sampler_binding, texture_binding);
        let bind_group = self.bind_group(label, &layout, sampler_binding, texture_binding);
        (layout, bind_group)
    }

    /// Returns the number of images in this texture (for array textures).
    pub fn image_count(&self) -> usize {
        self.image_count
    }
}

impl Debug for Texture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Texture")
            .field("texture", &self.texture)
            .field("texture_bind_group_entry", &self.texture_bind_group_entry)
            .field("sampler", &self.sampler)
            .field("sampler_bind_group_entry", &self.sampler_bind_group_entry)
            .field("view", &self.view)
            .finish()
    }
}
