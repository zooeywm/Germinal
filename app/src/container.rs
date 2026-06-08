use germinal_application::gshell::GShellServiceState;
use germinal_domain::gshell::GShellId;
use germinal_domain::rendering::RenderFrame;
use germinal_infra::{renderer::FakeRenderer, terminal::AlacrittyTerminalEngine};
use germinal_ports::{
    pty::PtyResult,
    renderer::RendererPort,
    terminal::{TerminalEnginePort, TerminalScreen, TerminalSize, TerminalUpdate},
};

const DEFAULT_TERMINAL_SIZE: TerminalSize = TerminalSize {
    cols: 80,
    rows: 24,
    cell_width: 9,
    cell_height: 18,
};

pub struct GerminalApp {
    renderer_backend: FakeRenderer,
    terminal_engine: AlacrittyTerminalEngine,
    gshell_service_state: GShellServiceState,
}

impl GerminalApp {
    pub fn new() -> Self {
        let gshell_service_state = GShellServiceState::new();
        let terminal_engine =
            AlacrittyTerminalEngine::new(gshell_service_state.active(), DEFAULT_TERMINAL_SIZE)
                .expect("failed to create terminal engine");

        Self {
            renderer_backend: FakeRenderer,
            terminal_engine,
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

impl TerminalEnginePort for GerminalApp {
    fn create_terminal(&mut self, id: GShellId, size: TerminalSize) -> PtyResult<()> {
        self.terminal_engine.create_terminal(id, size)
    }

    fn update_terminal_output(&mut self, id: GShellId, bytes: &[u8]) -> PtyResult<TerminalUpdate> {
        self.terminal_engine.update_terminal_output(id, bytes)
    }

    fn resize_terminal(&mut self, id: GShellId, size: TerminalSize) -> PtyResult<TerminalUpdate> {
        self.terminal_engine.resize_terminal(id, size)
    }

    fn remove_terminal(&mut self, id: GShellId) -> PtyResult<()> {
        self.terminal_engine.remove_terminal(id)
    }

    fn terminal_screen(&self, id: GShellId) -> PtyResult<&TerminalScreen> {
        self.terminal_engine.terminal_screen(id)
    }
}
