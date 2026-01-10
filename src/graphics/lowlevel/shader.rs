//! wgpu shader abstractions

use wgpu::VertexBufferLayout;

use crate::ReadOnlyString;

#[derive(Clone, Debug)]
pub struct ShaderProgram {
    /// The shader module containing the shader code.
    pub module: wgpu::ShaderModule,
    /// The entry point for the vertex shader.
    pub vertex_entry_point: Option<ReadOnlyString>,
    /// The entry point for the fragment shader.
    pub fragment_entry_point: Option<ReadOnlyString>,
}

impl ShaderProgram {
    /// Creates a new ShaderProgram from the given parts.
    ///
    /// You probably want to use [`crate::graphics::WgpuInstance::load_shader`] to create the shader module.
    pub fn from_raw_parts<'a>(
        module: wgpu::ShaderModule,
        vertex_entry_point: Option<ReadOnlyString>,
        fragment_entry_point: Option<ReadOnlyString>,
    ) -> Self {
        Self {
            module,
            vertex_entry_point,
            fragment_entry_point,
        }
    }

    /// Returns the vertex state for this shader program.
    pub fn vertex_state<'a>(
        &'a self,
        buffers: &'a [VertexBufferLayout],
        compilation_options: Option<wgpu::PipelineCompilationOptions<'a>>,
    ) -> wgpu::VertexState<'a> {
        wgpu::VertexState {
            module: &self.module,
            entry_point: self.vertex_entry_point.as_deref(),
            buffers,
            compilation_options: compilation_options.unwrap_or_default(),
        }
    }

    /// Returns the fragment state for this shader program.
    pub fn fragment_state<'a>(
        &'a self,
        targets: &'a [Option<wgpu::ColorTargetState>],
        compilation_options: Option<wgpu::PipelineCompilationOptions<'a>>,
    ) -> Option<wgpu::FragmentState<'a>> {
        self.fragment_entry_point.as_ref()?;

        Some(wgpu::FragmentState {
            module: &self.module,
            entry_point: self.fragment_entry_point.as_deref(),
            targets,
            compilation_options: compilation_options.unwrap_or_default(),
        })
    }
}
