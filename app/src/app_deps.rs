use germinal_application::gshell::GShellServiceState;
use germinal_domain::gshell::GShellId;
use germinal_infra::terminal::AlacrittyTerminalEngine;
use germinal_ports::{
    pty::PtyResult,
    terminal::{TerminalEnginePort, TerminalScreen, TerminalSize, TerminalUpdate},
};

const DEFAULT_TERMINAL_SIZE: TerminalSize = TerminalSize {
    cols: 80,
    rows: 24,
    cell_width: 9,
    cell_height: 18,
};

pub struct AppDeps {
    terminal_engine: AlacrittyTerminalEngine,
    gshell_service_state: GShellServiceState,
}

impl AppDeps {
    pub fn new() -> Self {
        let gshell_service_state = GShellServiceState::new();
        let terminal_engine =
            AlacrittyTerminalEngine::new(gshell_service_state.active(), DEFAULT_TERMINAL_SIZE)
                .expect("failed to create terminal engine");

        Self {
            terminal_engine,
            gshell_service_state,
        }
    }
}

impl Default for AppDeps {
    fn default() -> Self {
        Self::new()
    }
}

impl AsRef<GShellServiceState> for AppDeps {
    fn as_ref(&self) -> &GShellServiceState {
        &self.gshell_service_state
    }
}

impl AsMut<GShellServiceState> for AppDeps {
    fn as_mut(&mut self) -> &mut GShellServiceState {
        &mut self.gshell_service_state
    }
}

impl TerminalEnginePort for AppDeps {
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
