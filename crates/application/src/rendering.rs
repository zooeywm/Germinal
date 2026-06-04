use germinal_domain::rendering::RenderFrame;
use germinal_ports::renderer::RendererPort;

/// Submits one RenderFrame to the renderer port.
///
/// The application layer does not know whether the renderer is wgpu or software.
/// It only passes structured rendering data to the external rendering capability.
pub fn render_frame(renderer: &mut impl RendererPort, frame: &RenderFrame) {
    renderer.render(frame);
}
