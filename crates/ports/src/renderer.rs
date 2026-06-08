pub use germinal_domain::rendering::*;

/// External rendering capability port.
///
/// The application layer submits RenderFrame to this port.
/// The concrete implementation can be a wgpu renderer or a software renderer.
pub trait RendererPort {
    /// Renders one frame of structured drawing data.
    fn render(&mut self, frame: &RenderFrame);
}
