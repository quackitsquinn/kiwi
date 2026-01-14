use std::{any::Any, fmt::Debug};

use anyhow::Context;
use wgpu::TextureView;

use crate::{
    component::{ComponentHandle, ComponentStore, TypeMap},
    graphics::{
        lowlevel::WgpuRenderer,
        pipeline::{
            DeltaTime, FrameCount, RenderPipeline, UpdateRequest, downcast_pipeline_mut,
            downcast_pipeline_ref,
        },
    },
};

/// A trait representing a key for identifying render pipelines.
/// Yes this requires a lot of bounds, but keys should ideally be simple types, such as enums or newtypes around enums.
pub trait PipelineKey:
    'static + Send + Sync + std::fmt::Debug + Clone + PartialEq + Eq + std::hash::Hash + Sized
{
}

/// This struct really needs some through docs. I don't want to write them right now though.
///
/// Pretty much, you insert pipelines based off of the key type K, and then you can retrieve them later.
///
/// You HAVE to set a render order, or nothing will be rendered. (via set_render_order)
///
/// You can stash frame-specific data that can be accessed by pipelines during rendering. This data is cleared at the start of each frame before updating pipelines.
/// This data by default contains a DeltaTime (time since last frame) and FrameCount (number of frames rendered so far).
///
pub struct RenderController<K: PipelineKey> {
    pipelines: std::collections::HashMap<K, Box<dyn RenderPipeline<K> + 'static>>,
    render_list: Vec<K>,
    render_suface: Option<(K, wgpu::TextureView)>,
    frame_data: Stash,
    frame_count: u64,
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
            frame_data: Stash::new(),
            frame_count: 0,
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
    pub fn update_pipelines(&mut self, delta_time: f32) {
        let mut stash = Stash::new();
        stash.stash(DeltaTime(delta_time));
        self.frame_count += 1;
        stash.stash(FrameCount(self.frame_count));
        let keys = self.pipelines.keys().cloned().collect::<Vec<K>>();
        for pipeline_key in keys {
            let pipeline = self.get_pipeline_mut(&pipeline_key).unwrap();
            if let Some(request) = pipeline.update(&mut stash) {
                self.handle_update_request(pipeline_key, request);
            }
        }
        self.frame_data = stash;
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
                continue;
            }
            pipeline.render(self, encoder, target);
        }
        Ok(())
    }

    /// Retrieves a reference to a pipeline of the specified type.
    pub fn pipeline<P: RenderPipeline<K> + 'static>(&self, key: &K) -> anyhow::Result<&P> {
        downcast_pipeline_ref::<K, P>(self, key)?
            .with_context(|| format!("pipeline {:?} does not exist", key))
    }

    /// Retrieves a mutable reference to a pipeline of the specified type.
    pub fn pipeline_mut<P: RenderPipeline<K> + 'static>(
        &mut self,
        key: &K,
    ) -> anyhow::Result<&mut P> {
        downcast_pipeline_mut::<K, P>(self, key)?
            .with_context(|| format!("pipeline {:?} does not exist", key))
    }

    /// Stashes frame-specific data that can be accessed by pipelines during rendering.
    /// This data is cleared at the start of each frame before updating pipelines.
    pub fn stash<T: 'static + Send + Sync>(&mut self, data: T) {
        self.frame_data.stash(data);
    }

    /// Retrieves a reference to stashed frame-specific data of the specified type.
    /// Returns None if no such data exists.
    pub fn retrieve_checked<T: 'static + Send + Sync>(&self) -> Option<&T> {
        self.frame_data.retrieve_checked::<T>()
    }

    /// Retrieves a reference to stashed frame-specific data of the specified type.
    /// Panics if no such data exists. Use `retrieve_checked` if you want to handle the absence of data more gracefully.
    pub fn retrieve<T: 'static + Send + Sync>(&self) -> &T {
        self.retrieve_checked::<T>()
            .expect("Requested frame data not found")
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

/// A simple stash for storing data of various types. Used as the backing type for frame-specific data in RenderController.
#[derive(Debug, Default)]
pub struct Stash {
    inner: TypeMap,
}

impl Stash {
    /// Creates a new, empty Stash.
    pub fn new() -> Self {
        Self {
            inner: TypeMap::new(),
        }
    }

    /// Stashes data of the specified type.
    pub fn stash<T: 'static + Send + Sync>(&mut self, data: T) {
        self.inner.insert(data);
    }

    /// Retrieves a reference to stashed data of the specified type.
    pub fn retrieve_checked<T: 'static + Send + Sync>(&self) -> Option<&T> {
        self.inner.get::<T>()
    }

    /// Retrieves a reference to stashed data of the specified type.
    pub fn retrieve<T: 'static + Send + Sync>(&self) -> &T {
        self.retrieve_checked::<T>()
            .expect("Requested stashed data not found")
    }

    /// Removes all stashed data.
    pub fn clear(&mut self) {
        self.inner.clear();
    }
}
