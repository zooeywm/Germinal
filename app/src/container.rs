use germinal_application::gshell::GShellServiceState;
use germinal_domain::rendering::RenderFrame;
use germinal_infra::renderer::FakeRenderer;
use germinal_ports::renderer::RendererPort;

pub struct GerminalApp {
    renderer_backend: FakeRenderer,
    gshell_service_state: GShellServiceState,
}

impl GerminalApp {
    pub fn new() -> Self {
        let gshell_service_state = GShellServiceState::new();

        Self {
            renderer_backend: FakeRenderer,
            gshell_service_state,
        }
    }
}

impl RendererPort for GerminalApp {
    fn render(&mut self, frame: &RenderFrame) {
        self.renderer_backend.render(frame);
    }
}

impl AsRef<GShellServiceState> for GerminalApp {
    fn as_ref(&self) -> &GShellServiceState {
        &self.gshell_service_state
    }
}

impl AsMut<GShellServiceState> for GerminalApp {
    fn as_mut(&mut self) -> &mut GShellServiceState {
        &mut self.gshell_service_state
    }
}
