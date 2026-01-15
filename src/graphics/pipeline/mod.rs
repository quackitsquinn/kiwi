use anyhow::Context;

use crate::graphics::pipeline::controller::{PipelineKey, RenderController, Stash};
use std::any::Any;

pub mod controller;
pub mod pipelines;

/// A trait representing a render pipeline.
pub trait RenderPipeline<K: PipelineKey>: Send + Sync + 'static + Any {
    /// Returns the name of the pipeline.
    fn label(&self) -> Option<&str>;
    /// Updates the pipeline state.
    ///
    /// Gives the pipeline access to the frame-specific stash data.
    ///
    /// This frame-specific data can be added to, and by default includes `DeltaTime` and `FrameCount`.
    ///
    /// Returns an optional UpdateRequest to modify the rendering process.
    fn update(&mut self, stash: &mut Stash) -> Option<UpdateRequest>;

    /// Renders using the pipeline.
    ///
    /// Gives the pipeline access to the controller, command encoder, and target texture view.
    ///
    /// Pipelines can access stashed frame data via the controller's
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

/// Frame time delta in seconds. This is included in the frame data by default.
#[repr(transparent)]
pub struct DeltaTime(pub f32);

/// Frame count that counts the number of frames rendered since a undefined starting point. This is included in the frame data by default.
#[repr(transparent)]
pub struct FrameCount(pub u64);

/// Clear color for the render pass. This can be set by pipelines to specify the clear color used when beginning a render pass.
#[repr(transparent)]
pub struct ClearColor(pub wgpu::Color);

// TODO: Add more built-in frame data types as needed.
