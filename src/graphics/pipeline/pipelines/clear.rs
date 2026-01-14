use wgpu::{Color, LoadOp};

use crate::graphics::pipeline::{
    RenderPipeline,
    controller::{PipelineKey, Stash},
};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ClearPipeline(pub Color);

impl ClearPipeline {
    pub fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self(Color { r, g, b, a })
    }
}

impl<K: PipelineKey> RenderPipeline<K> for ClearPipeline {
    fn label(&self) -> Option<&str> {
        Some("Clear Pipeline")
    }

    fn update(&mut self, _stash: &mut Stash) -> Option<crate::graphics::pipeline::UpdateRequest> {
        None
    }

    fn render(
        &self,
        controller: &crate::graphics::pipeline::controller::RenderController<K>,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
    ) {
        let wgpu = controller.wgpu.get();
        let _render_pass_desc = wgpu.render_pass(
            Some("Clear Pipeline Render Pass"),
            encoder,
            target,
            None,
            LoadOp::Clear(self.0),
        );
    }
}
