use std::rc::Rc;

use wgpu::{BindGroupLayout, VertexBufferLayout};

use crate::graphics::lowlevel::{WgpuRenderer, buf::VertexLayout, shader::ShaderProgram};

/// A builder for creating render pipelines.
#[derive(Debug)]
pub struct PipelineBuilder<'a> {
    wgpu: &'a WgpuRenderer,
    label: &'a str,
    bind_group_layouts: Vec<wgpu::BindGroupLayout>,
    shader_module: Option<ShaderProgram>,
    layouts: Vec<VertexBufferLayout<'static>>,
    primitive_state: wgpu::PrimitiveState,
    color_targets: Vec<Option<wgpu::ColorTargetState>>,
    depth_stencil: Option<wgpu::DepthStencilState>,
}

impl<'a> PipelineBuilder<'a> {
    /// Creates a new PipelineBuilder.
    pub fn new(wgpu: &'a WgpuRenderer, label: &'a str) -> PipelineBuilder<'a> {
        PipelineBuilder {
            wgpu,
            label,
            bind_group_layouts: Vec::new(),
            layouts: Vec::new(),
            shader_module: None,
            primitive_state: wgpu::PrimitiveState::default(),
            color_targets: Vec::new(),
            depth_stencil: None,
        }
    }

    /// Sets the shader module for the pipeline.
    pub fn shader(
        mut self,
        label: &str,
        source: &str,
        vs_entry: Option<&str>,
        fs_entry: Option<&str>,
    ) -> Self {
        let shader_module = self
            .wgpu
            .load_shader(source, Some(label), vs_entry, fs_entry);
        self.shader_module = Some(shader_module);
        self
    }

    /// Adds a bind group layout to the pipeline.
    pub fn push_bind_group(mut self, layout: BindGroupLayout) -> Self {
        self.bind_group_layouts.push(layout);
        self
    }

    /// Adds a vertex layout to the pipeline.
    pub fn add_vertex_layout<T: VertexLayout>(mut self) -> Self {
        self.layouts.push(T::LAYOUT);
        self
    }

    pub fn add_color_target(mut self, format: wgpu::TextureFormat) -> Self {
        self.color_targets.push(Some(wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        }));
        self
    }

    /// Sets the primitive state for the pipeline.
    pub fn primitive_state(mut self, state: wgpu::PrimitiveState) -> Self {
        self.primitive_state = state;
        self
    }

    /// Sets the depth stencil state for the pipeline.
    pub fn depth(mut self, state: wgpu::DepthStencilState) -> Self {
        self.depth_stencil = Some(state);
        self
    }

    pub fn build(
        self,
        compilation_options: Option<wgpu::PipelineCompilationOptions<'_>>,
    ) -> WgpuPipeline {
        let shader = self.shader_module.expect("Shader module must be set");

        let pipeline_layout =
            self.wgpu
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some(self.label),
                    bind_group_layouts: &self.bind_group_layouts.iter().collect::<Vec<_>>(),
                    immediate_size: 0,
                });

        let pipeline = self
            .wgpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(self.label),
                layout: Some(&pipeline_layout),
                vertex: shader.vertex_state(&self.layouts, compilation_options.clone()),
                fragment: shader.fragment_state(&self.color_targets, compilation_options),
                primitive: self.primitive_state,
                depth_stencil: self.depth_stencil,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

        WgpuPipeline {
            pipeline,
            shader,
            layout: pipeline_layout,
        }
    }
}

pub struct WgpuPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub shader: ShaderProgram,
    pub layout: wgpu::PipelineLayout,
}
