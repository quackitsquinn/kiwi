use anyhow::Context;

use crate::graphics::pipeline::controller::{PipelineKey, RenderController};
use std::any::Any;

pub mod controller;
pub mod pipelines;

/// A trait representing a render pipeline.
pub trait RenderPipeline<K: PipelineKey>: Send + Sync + 'static + Any {
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

/// Attempts to downcast a pipeline to a specific type.
pub fn downcast_pipeline_ref<'a, K: PipelineKey, P: RenderPipeline<K> + Sized + 'static>(
    controller: &'a RenderController<K>,
    key: &K,
) -> Result<Option<&'a P>, IncorrectPipelineType> {
    let pipeline = match controller.get_pipeline(key) {
        Some(p) => p,
        None => return Ok(None),
    };

    let any = pipeline as &dyn Any;

    any.downcast_ref::<P>()
        .map(|p| Some(p))
        .ok_or(IncorrectPipelineType)
}

/// Attempts to downcast a pipeline to a specific type.
pub fn downcast_pipeline_mut<'a, K: PipelineKey, P: RenderPipeline<K> + Sized + 'static>(
    controller: &'a mut RenderController<K>,
    key: &K,
) -> Result<Option<&'a mut P>, IncorrectPipelineType> {
    let pipeline = controller
        .get_pipeline_mut(key)
        .ok_or(IncorrectPipelineType)?;

    let any = pipeline as &mut dyn Any;

    any.downcast_mut::<P>()
        .map(|p| Some(p))
        .ok_or(IncorrectPipelineType)
}

#[derive(thiserror::Error, Debug)]
#[error("Pipeline is not of the expected type")]
pub struct IncorrectPipelineType;
