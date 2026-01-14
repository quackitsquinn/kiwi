use wgpu::{CompareFunction, StoreOp, TextureFormat};

use crate::{
    component::{ComponentHandle, ComponentStore},
    graphics::lowlevel::WgpuRenderer,
};

/// A depth texture for use in rendering.
#[derive(Clone, Debug)]
pub struct DepthTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    wgpu_handle: ComponentHandle<WgpuRenderer>,
}

impl DepthTexture {
    /// The texture format used for the depth texture.
    pub const TEXTURE_FORMAT: TextureFormat = TextureFormat::Depth32Float;

    /// Creates a new depth texture matching the current size of the swap chain.
    pub fn new(state: &ComponentStore) -> Self {
        let wgpu = state.get::<WgpuRenderer>();
        let config = wgpu.config.read().expect("CONFIG POISONED");
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };

        let desc = wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let texture = wgpu.device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = wgpu.comparing_sampler(CompareFunction::LessEqual);

        Self {
            texture,
            view,
            sampler,
            wgpu_handle: state.handle_for(),
        }
    }

    /// Resizes the depth texture to match the current size of the swap chain.
    pub fn resize(&mut self) {
        let wgpu = self.wgpu_handle.get();
        let config = wgpu.config.read().expect("CONFIG POISONED");
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };

        let desc = wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        self.texture = wgpu.device.create_texture(&desc);
        self.view = self
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
    }

    /// Gets the depth stencil state for use in a render pipeline.
    pub fn state(&self) -> wgpu::DepthStencilState {
        wgpu::DepthStencilState {
            format: Self::TEXTURE_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }
    }

    /// Gets the depth stencil attachment for use in a render pass.
    pub fn attachment(&self) -> wgpu::RenderPassDepthStencilAttachment<'_> {
        wgpu::RenderPassDepthStencilAttachment {
            view: &self.view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: StoreOp::Store,
            }),
            stencil_ops: None,
        }
    }

    /// Creates a bind group layout entry for the depth texture.
    pub fn bind_group_layout(
        &self,
        texture_binding: u32,
        sampler_binding: u32,
        sampler_type: wgpu::SamplerBindingType,
    ) -> wgpu::BindGroupLayout {
        let wgpu = self.wgpu_handle.get();
        wgpu.bind_group_layout(
            Some("depth texture bind group layout"),
            &[
                wgpu::BindGroupLayoutEntry {
                    binding: texture_binding,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Depth,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: sampler_binding,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(sampler_type),
                    count: None,
                },
            ],
        )
    }

    /// Creates a bind group for the depth texture. Uses the given sampler.
    pub fn bind_group(
        &self,
        texture_binding: u32,
        sampler_binding: u32,
        sampler: &wgpu::Sampler,
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let wgpu = self.wgpu_handle.get();
        let layout = self.bind_group_layout(
            texture_binding,
            sampler_binding,
            wgpu::SamplerBindingType::Filtering,
        );

        (
            layout.clone(),
            wgpu.bind_group(
                Some("depth texture bind group"),
                &layout,
                &[
                    wgpu::BindGroupEntry {
                        binding: texture_binding,
                        resource: wgpu::BindingResource::TextureView(&self.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: sampler_binding,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
            ),
        )
    }
}
