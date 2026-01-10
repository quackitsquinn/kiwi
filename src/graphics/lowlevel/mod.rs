//! Low-level graphics abstractions using WGPU.
use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
};

use anyhow::Context;
use bytemuck::Pod;
use log::debug;
use wgpu::{
    self as w, Color, CommandBuffer, CommandEncoder, CompareFunction, Device, DeviceDescriptor,
    Instance, InstanceDescriptor, Origin3d, PowerPreference, PresentMode, Queue, RenderPass,
    RequestAdapterOptions, StoreOp, Surface, SurfaceConfiguration, SurfaceTexture, TextureAspect,
    TextureView, util::DeviceExt,
};

use crate::{
    ReadOnly,
    component::{ComponentStore, ComponentStoreHandle},
    graphics::lowlevel::{
        buf::{IndexBuffer, IndexLayout, UniformBuffer, VertexBuffer, VertexLayout},
        shader::ShaderProgram,
        texture::Texture,
    },
    window::GlfwWindow,
};

pub mod buf;
pub mod depth;
pub mod pipeline;
pub mod shader;
pub mod texture;

/// A WGPU-based renderer.
pub struct WgpuRenderer {
    /// The WGPU instance.
    pub instance: Instance,
    /// The WGPU surface.
    pub surface: Surface<'static>,
    /// The WGPU device.
    pub device: Device,
    /// The WGPU queue.
    pub queue: Queue,
    /// The surface configuration.
    pub config: RwLock<SurfaceConfiguration>,
    state: ComponentStoreHandle,
}

impl WgpuRenderer {
    /// Attaches a WGPU renderer to the given state and window.
    pub async fn attach_to(state: &mut ComponentStore, window: &GlfwWindow) -> anyhow::Result<()> {
        let size = window.size();

        let instance = Instance::new(&InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = unsafe { window.create_surface(&instance) };

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .with_context(|| "Failed to find an appropriate adapter")?;

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: Some("root device"),
                required_features: wgpu::Features::POLYGON_MODE_LINE,
                ..Default::default()
            })
            .await
            .with_context(|| "Failed to create device")?;

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.0 as u32,
            height: size.1 as u32,
            present_mode: PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let this = WgpuRenderer {
            instance,
            surface,
            device,
            queue,
            config: RwLock::new(config),
            state: state.handle(),
        };

        state.insert(this);
        Ok(())
    }

    /// Resize the surface to the new size.
    ///
    /// # Panics
    /// Panics if the new size has a width or height less than or equal to zero.
    pub fn resize(&self, new_size: (i32, i32)) {
        debug_assert!(new_size.0 > 0 && new_size.1 > 0, "Window size <= 0");
        let mut cfg = self.config.write().expect("CONFIG POISONED");
        cfg.width = new_size.0 as u32;
        cfg.height = new_size.1 as u32;
        self.surface.configure(&self.device, &cfg);
    }

    /// Creates a command encoder.
    pub fn create_encoder(&self, label: Option<&str>) -> CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label })
    }

    /// Creates a buffer with the given usage and data.
    pub fn vertex_buffer<T>(&self, data: &[T], label: Option<&str>) -> VertexBuffer<T>
    where
        T: VertexLayout,
    {
        let buffer = self
            .device
            .create_buffer_init(&w::util::BufferInitDescriptor {
                label,
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

        // Safety: The buffer is valid for type T as it was created from a slice of T.
        unsafe { VertexBuffer::from_raw_parts(buffer) }
    }

    /// Creates an index buffer with the given usage and data.
    pub fn index_buffer<T>(&self, data: &[T], label: Option<&str>) -> IndexBuffer<T>
    where
        T: IndexLayout,
    {
        let buffer = self
            .device
            .create_buffer_init(&w::util::BufferInitDescriptor {
                label,
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });

        // Safety: The buffer is valid for type T as it was created from a slice of T.
        unsafe { IndexBuffer::from_raw_parts(buffer, data.len()) }
    }

    /// Creates a uniform buffer with the given data.
    pub fn uniform_buffer<T>(&self, data: &T, label: Option<&str>) -> UniformBuffer<T>
    where
        T: Pod,
    {
        let buffer = self
            .device
            .create_buffer_init(&w::util::BufferInitDescriptor {
                label,
                contents: bytemuck::bytes_of(data),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Safety: The buffer is valid for type T as it was created from a slice of T.
        unsafe { UniformBuffer::from_raw_parts(buffer, self.state.clone()) }
    }

    /// Loads a shader module from WGSL source code.
    pub fn load_shader(
        &self,
        shader_source: &str,
        label: Option<&str>,
        vs_entry: Option<&str>,
        fs_entry: Option<&str>,
    ) -> ShaderProgram {
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label,
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        ShaderProgram::from_raw_parts(module, vs_entry.map(Arc::from), fs_entry.map(Arc::from))
    }

    /// Creates a texture with the given descriptor.
    pub fn create_texture(&self, desc: &wgpu::TextureDescriptor) -> wgpu::Texture {
        self.device.create_texture(desc)
    }

    /// Creates a texture from the given parameters, sized to the current surface configuration.
    ///
    /// This will upload the image pixel data to the texture.
    pub fn texture(
        &self,
        label: Option<&str>,
        sampler: &wgpu::Sampler,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        dims: (u32, u32),
        image: &[ReadOnly<u8>],
    ) -> Texture {
        assert!(!image.is_empty(), "Image slice must not be empty");
        let (width, height) = dims;

        // TODO: So we have a dilemma here.. we currently treat all textures as texture arrays.
        // This is nice because a. concrete type and b. is simple to implement. However, this means
        // that single-layer textures are still treated as arrays.
        // This is not a *huge* issue, but it can be confusing for users expecting a 2D texture.
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: image.len() as u32,
        };

        let text = self.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });

        let text_layout = wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                view_dimension: wgpu::TextureViewDimension::D2Array,
                // TODO: Allow this to be configurable based on texture format.
                // Minecraft clone probably means that using a integer format is easier.
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
            },
            count: None,
        };

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        for (i, image) in image.iter().enumerate() {
            debug!("Uploading texture layer {} ({} bytes)", i, image.len());
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfoBase {
                    texture: &text,
                    mip_level: 0,
                    origin: Origin3d {
                        x: 0,
                        y: 0,
                        z: i as u32,
                    },
                    aspect: TextureAspect::All,
                },
                image.as_ref(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * width),
                    rows_per_image: Some(height),
                },
                size,
            );
        }

        let texture_view = text.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });

        let sampler_layout = wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };

        Texture::new(
            &self.state,
            text,
            text_layout,
            sampler.clone(),
            sampler_layout,
            texture_view,
            image.len(),
        )
    }

    /// Creates a texture from the given parameters, sized to the current surface configuration. The given image data is uninitialized.
    pub fn texture_uninit(
        &self,
        label: Option<&str>,
        sampler: &wgpu::Sampler,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        dims: (u32, u32),
        layers: u32,
    ) -> Texture {
        let (width, height) = dims;
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: layers,
        };

        let text = self.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });

        let text_layout = wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                view_dimension: wgpu::TextureViewDimension::D2Array,
                // TODO: Allow this to be configurable based on texture format.
                // Minecraft clone probably means that using a integer format is easier.
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
            },
            count: None,
        };

        let texture_view = text.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });

        let sampler_layout = wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };

        Texture::new(
            &self.state,
            text,
            text_layout,
            sampler.clone(),
            sampler_layout,
            texture_view,
            layers as usize,
        )
    }

    /// Creates a bind group layout from the given descriptor.
    pub fn create_bind_group_layout(
        &self,
        desc: &wgpu::BindGroupLayoutDescriptor,
    ) -> wgpu::BindGroupLayout {
        self.device.create_bind_group_layout(desc)
    }

    /// Creates a bind group layout from the given entries.
    pub fn bind_group_layout(
        &self,
        label: Option<&str>,
        entries: &[wgpu::BindGroupLayoutEntry],
    ) -> wgpu::BindGroupLayout {
        self.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label, entries })
    }

    /// Creates a bind group from the given descriptor.
    pub fn create_bind_group(&self, desc: &wgpu::BindGroupDescriptor) -> wgpu::BindGroup {
        self.device.create_bind_group(desc)
    }

    /// Creates a bind group from the given parts.
    pub fn bind_group(
        &self,
        label: Option<&str>,
        layout: &wgpu::BindGroupLayout,
        entries: &[wgpu::BindGroupEntry],
    ) -> wgpu::BindGroup {
        self.create_bind_group(&wgpu::BindGroupDescriptor {
            label,
            layout,
            entries,
        })
    }

    /// Creates a sampler with the given descriptor.
    pub fn create_sampler(&self, desc: &wgpu::SamplerDescriptor) -> wgpu::Sampler {
        self.device.create_sampler(desc)
    }

    /// Creates a sampler with linear filtering and the specified address mode.
    pub fn sampler(&self, label: Option<&str>, address_mode: wgpu::AddressMode) -> wgpu::Sampler {
        self.create_sampler(&wgpu::SamplerDescriptor {
            label,
            address_mode_u: address_mode,
            address_mode_v: address_mode,
            address_mode_w: address_mode,
            mag_filter: wgpu::FilterMode::Nearest, // gotta love the n64
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        })
    }

    /// Creates a sampler with comparison functionality.
    pub fn comparing_sampler(&self, func: CompareFunction) -> wgpu::Sampler {
        self.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("comparing sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            compare: Some(func),
            lod_max_clamp: 100.0,
            lod_min_clamp: 0.0,
            ..Default::default()
        })
    }

    /// Creates a pipeline layout from the given descriptor.
    pub fn create_pipeline_layout(
        &self,
        desc: &wgpu::PipelineLayoutDescriptor,
    ) -> wgpu::PipelineLayout {
        self.device.create_pipeline_layout(desc)
    }

    /// Creates a pipeline layout from the given bind group layouts.
    pub fn pipeline_layout(
        &self,
        label: Option<&str>,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
    ) -> wgpu::PipelineLayout {
        self.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label,
            bind_group_layouts,
            immediate_size: 0,
        })
    }

    /// Creates a render pipeline from the given descriptor.
    pub fn create_pipeline(&self, desc: &wgpu::RenderPipelineDescriptor) -> wgpu::RenderPipeline {
        self.device.create_render_pipeline(desc)
    }

    pub fn pipeline_builder<'a>(&'a self, label: &'a str) -> pipeline::PipelineBuilder<'a> {
        pipeline::PipelineBuilder::new(self, label)
    }

    /// Acquires the current texture view from the surface.
    pub fn current_view(&self) -> anyhow::Result<(SurfaceTexture, TextureView)> {
        let frame = self
            .surface
            .get_current_texture()
            .with_context(|| "Failed to acquire next swap chain texture")?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        Ok((frame, view))
    }

    /// Clears the given texture view with the specified color using the provided command encoder.
    pub fn render_pass<'b>(
        &self,
        label: Option<&str>,
        encoder: &'b mut CommandEncoder,
        view: &TextureView,
        depth_stencil_attachment: Option<wgpu::RenderPassDepthStencilAttachment>,
        color_load_op: wgpu::LoadOp<Color>,
    ) -> RenderPass<'b> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: color_load_op,
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment,
            ..Default::default()
        })
    }

    /// Submits a single command encoder to the queue. This is a direct wrapper around `Queue::submit`.
    pub fn submit_single(&self, encoder: CommandBuffer) {
        self.queue.submit(std::iter::once(encoder));
    }

    /// Submits multiple command buffers to the queue.
    pub fn submit<I: IntoIterator<Item = CommandBuffer>>(&self, bufs: I) {
        self.queue.submit(bufs);
    }

    /// Returns the current dimensions of the surface.
    pub fn dimensions(&self) -> (u32, u32) {
        let cfg = self.config.read().expect("CONFIG POISONED");
        (cfg.width, cfg.height)
    }
}

impl Debug for WgpuRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WgpuRenderer").finish_non_exhaustive()
    }
}
