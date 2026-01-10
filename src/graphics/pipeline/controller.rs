use std::fmt::Debug;

use anyhow::Context;
use wgpu::TextureView;

use crate::{
    component::{ComponentHandle, ComponentStore},
    graphics::{
        lowlevel::WgpuRenderer,
        pipeline::{RenderPipeline, UpdateRequest},
    },
};

/// A trait representing a key for identifying render pipelines.
/// Yes this requires a lot of bounds, but keys should ideally be simple types, such as enums or newtypes around enums.
pub trait PipelineKey:
    'static + Send + Sync + std::fmt::Debug + Clone + PartialEq + Eq + std::hash::Hash + Sized
{
}

pub struct RenderController<K: PipelineKey> {
    pipelines: std::collections::HashMap<K, Box<dyn RenderPipeline<K>>>,
    render_list: Vec<K>,
    render_suface: Option<(K, wgpu::TextureView)>,
    /// The WGPU renderer. Convenience access for pipelines.
    pub wgpu: ComponentHandle<WgpuRenderer>,
}

impl<K: PipelineKey> RenderController<K> {
    /// Creates a new RenderController.
    pub fn new(state: &ComponentStore) -> Self {
        Self {
            pipelines: std::collections::HashMap::new(),
            render_list: Vec::new(),
            render_suface: None,
            wgpu: state.handle_for::<WgpuRenderer>(),
        }
    }

    /// Adds a render pipeline to the controller.
    pub fn add_pipeline<P: RenderPipeline<K> + 'static>(&mut self, key: K, pipeline: P) {
        self.pipelines.insert(key, Box::new(pipeline));
    }

    /// Retrieves a mutable reference to a render pipeline by its key.
    /// Returns None if the pipeline does not exist.
    pub fn get_pipeline_mut(&mut self, key: &K) -> Option<&mut dyn RenderPipeline<K>> {
        match self.pipelines.get_mut(key) {
            Some(pipeline) => Some(pipeline.as_mut()),
            None => None,
        }
    }

    /// Retrieves an immutable reference to a render pipeline by its key.
    /// Returns None if the pipeline does not exist.
    pub fn get_pipeline(&self, key: &K) -> Option<&dyn RenderPipeline<K>> {
        self.pipelines.get(key).map(|p| p.as_ref())
    }

    /// Sets the render order of the pipelines. This must be set, or no pipelines will be rendered.
    pub fn set_render_order(&mut self, order: Vec<K>) {
        self.render_list = order;
    }

    fn handle_update_request(&mut self, source: K, request: UpdateRequest) {
        match request {
            UpdateRequest::SetRenderTarget(view) => {
                self.render_suface = Some((source, view));
            }
        }
    }

    /// Updates all pipelines managed by the controller.
    pub fn update_pipelines(&mut self) {
        let keys = self.pipelines.keys().cloned().collect::<Vec<K>>();
        for pipeline_key in keys {
            let pipeline = self.get_pipeline_mut(&pipeline_key).unwrap();
            if let Some(request) = pipeline.update() {
                self.handle_update_request(pipeline_key, request);
            }
        }
    }

    /// Renders all pipelines in the order specified by `set_render_order`.
    pub fn render_pipelines(
        &self,
        encoder: &mut wgpu::CommandEncoder,
    ) -> anyhow::Result<(wgpu::SurfaceTexture, TextureView)> {
        let wgpu = self.wgpu.get();
        let (surf, swapchain_texture) = wgpu
            .current_view()
            .with_context(|| "Failed to get swapchain texture")?;

        if let Some((ref key, ref target)) = self.render_suface {
            self.render_with_target(encoder, &swapchain_texture, key, target)?;
            return Ok((surf, swapchain_texture));
        }

        for pipeline_key in &self.render_list {
            let pipeline = self
                .get_pipeline(pipeline_key)
                .with_context(|| format!("Pipeline {:?} not found in controller", pipeline_key))?;
            pipeline.render(self, encoder, &swapchain_texture);
        }

        Ok((surf, swapchain_texture))
    }

    fn render_with_target(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        key: &K,
        target: &wgpu::TextureView,
    ) -> anyhow::Result<()> {
        for pipeline_key in &self.render_list {
            let pipeline = self
                .get_pipeline(pipeline_key)
                .with_context(|| format!("Pipeline {:?} not found in controller", pipeline_key))?;
            if pipeline_key == key {
                pipeline.render(self, encoder, output);
            }
            pipeline.render(self, encoder, target);
        }
        Ok(())
    }
}

impl<K: PipelineKey> Debug for RenderController<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderController")
            .field(
                "pipelines",
                &self
                    .pipelines
                    .iter()
                    .map(|(k, p)| (k, p.label().unwrap_or("?")))
                    .collect::<Vec<(&K, &str)>>(),
            )
            .finish()
    }
}
