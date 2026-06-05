use germinal_application::gshell::GShellServiceState;
use germinal_domain::{gshell::GShellId, rendering::RenderFrame};
use germinal_infra::{pty::UnixPty, renderer::FakeRenderer};
use germinal_ports::{
    pty::{PtyPort, PtyResult, PtySize},
    renderer::RendererPort,
};

pub struct GerminalApp {
    pty_backend: UnixPty,
    renderer_backend: FakeRenderer,
    gshell_service_state: GShellServiceState,
}

impl GerminalApp {
    pub fn new() -> Self {
        Self {
            pty_backend: UnixPty::new(),
            renderer_backend: FakeRenderer,
            gshell_service_state: GShellServiceState::new(),
        }
    }
}

impl Default for GerminalApp {
    fn default() -> Self {
        Self::new()
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

impl PtyPort for GerminalApp {
    fn spawn(&mut self, id: GShellId) -> PtyResult<()> {
        self.pty_backend.spawn(id)
    }

    async fn write(&mut self, id: GShellId, bytes: &[u8]) -> PtyResult<()> {
        self.pty_backend.write(id, bytes).await
    }

    async fn read(&mut self, id: GShellId) -> PtyResult<Vec<u8>> {
        self.pty_backend.read(id).await
    }

    fn resize(&mut self, id: GShellId, size: PtySize) -> PtyResult<()> {
        self.pty_backend.resize(id, size)
    }

    fn close(&mut self, id: GShellId) -> PtyResult<()> {
        self.pty_backend.close(id)
    }
}

impl RendererPort for GerminalApp {
    fn render(&mut self, frame: &RenderFrame) {
        self.renderer_backend.render(frame);
    }
}
