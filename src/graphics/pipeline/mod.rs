use crate::graphics::pipeline::controller::{PipelineKey, RenderController};

pub mod controller;
pub mod pipelines;

/// A trait representing a render pipeline.
pub trait RenderPipeline<K: PipelineKey> {
    /// Returns the name of the pipeline.
    fn label(&self) -> Option<&str>;
    /// Updates the pipeline state.
    fn update(&mut self) -> Option<UpdateRequest>;
    /// Renders using the pipeline.
    fn render(
        &self,
        controller: &RenderController<K>,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
    );
}

pub enum UpdateRequest {
    /// Sets the render target that the pipeline should render to.
    /// The pipeline that provides this request will be given the swap chain's current texture as the target.
    SetRenderTarget(wgpu::TextureView),
}
