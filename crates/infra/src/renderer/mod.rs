pub mod wgpu;

pub use wgpu::WgpuRendererBackend;

use germinal_ports::renderer::{RenderFrame, RendererPort};

/// Fake renderer used to verify the rendering port wiring.
///
/// It accepts RenderFrame but does not draw anything.
/// Real wgpu/software rendering will be added later.
pub struct FakeRenderer;

impl RendererPort for FakeRenderer {
    fn render(&mut self, frame: &RenderFrame) {
        println!("{frame}");
    }
}
