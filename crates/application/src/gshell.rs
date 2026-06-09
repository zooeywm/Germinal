use std::collections::HashMap;

use germinal_domain::gshell::{GRequest, GShell, GShellId, detect_gnative_request};
use germinal_ports::window::WindowSize;
use germinal_ports::{
    pty::{PtyError, PtyPort, PtyResult, PtySize},
    terminal::{TerminalEnginePort, TerminalScreen, TerminalSize},
};

const DEFAULT_TERMINAL_FONT_SIZE: f32 = 15.0;
const TERMINAL_CELL_WIDTH_SCALE: f32 = 0.62;
const TERMINAL_LINE_HEIGHT_SCALE: f32 = 1.18;
const DEFAULT_TERMINAL_COLS: u16 = 80;
const DEFAULT_TERMINAL_ROWS: u16 = 24;

/// Runtime binding maintained by the application layer.
///
/// GShell is domain state.
/// The real PTY resource is associated by GShellId in the PTY port.
#[derive(kudi::DepInj)]
#[target(GShellService)]
pub struct GShellServiceState {
    next_id: u64,
    active: GShellId,
    shells: HashMap<GShellId, GShell>,
    terminal_font_size: f32,
    last_window_size: Option<WindowSize>,
}

#[derive(Debug)]
pub enum GShellPtyEvent {
    Output {
        id: GShellId,
        responses: Vec<Vec<u8>>,
    },
    EnterGNative(GRequest),
}

impl GShellServiceState {
    pub fn new() -> Self {
        let mut shells = HashMap::new();
        let initial_id = GShellId::new(1);
        shells.insert(initial_id, GShell::new(initial_id));

        Self {
            next_id: initial_id.value() + 1,
            active: initial_id,
            shells,
            terminal_font_size: DEFAULT_TERMINAL_FONT_SIZE,
            last_window_size: None,
        }
    }

    pub fn active(&self) -> GShellId {
        self.active
    }

    pub fn contains(&self, id: GShellId) -> bool {
        self.shells.contains_key(&id)
    }

    fn allocate_id(&mut self) -> GShellId {
        let id = GShellId::new(self.next_id);
        self.next_id += 1;
        id
    }

    fn insert(&mut self, shell: GShell) -> PtyResult<()> {
        let id = shell.id();

        if self.shells.contains_key(&id) {
            return Err(PtyError::SessionAlreadyExists);
        }

        self.shells.insert(id, shell);

        self.active = id;

        Ok(())
    }

    fn activate(&mut self, id: GShellId) -> PtyResult<()> {
        if !self.shells.contains_key(&id) {
            return Err(PtyError::SessionNotFound);
        }

        self.active = id;
        Ok(())
    }

    fn remove(&mut self, id: GShellId) -> PtyResult<CloseShellResult> {
        if !self.shells.contains_key(&id) {
            return Err(PtyError::SessionNotFound);
        }

        if self.shells.len() == 1 {
            self.shells.remove(&id);
            return Ok(CloseShellResult::LastShellClosed);
        }

        let next_active = if self.active == id {
            self.shells
                .keys()
                .copied()
                .find(|candidate| *candidate != id)
                .ok_or(PtyError::SessionNotFound)?
        } else {
            self.active
        };

        self.shells.remove(&id);
        self.active = next_active;

        Ok(CloseShellResult::Closed { next_active })
    }

    fn rollback_insert(&mut self, id: GShellId, previous_active: GShellId) {
        self.shells.remove(&id);
        self.active = previous_active;
    }

    fn shell_mut(&mut self, id: GShellId) -> PtyResult<&mut GShell> {
        self.shells.get_mut(&id).ok_or(PtyError::SessionNotFound)
    }

    pub fn terminal_font_size(&self) -> f32 {
        self.terminal_font_size
    }

    pub fn set_terminal_font_size(&mut self, font_size: f32) {
        self.terminal_font_size = font_size;
    }

    pub fn last_window_size(&self) -> Option<WindowSize> {
        self.last_window_size
    }

    pub fn set_last_window_size(&mut self, size: WindowSize) {
        self.last_window_size = Some(size);
    }
}

impl Default for GShellServiceState {
    fn default() -> Self {
        Self::new()
    }
}

impl<Deps> GShellService<Deps>
where
    Deps: TerminalEnginePort + AsRef<GShellServiceState> + AsMut<GShellServiceState>,
{
    fn active_id(&self) -> GShellId {
        self.prj_ref().as_ref().active()
    }

    pub fn handle_pty_output_bytes(
        &mut self,
        id: GShellId,
        bytes: Vec<u8>,
    ) -> PtyResult<GShellPtyEvent> {
        if !self.prj_ref().as_ref().contains(id) {
            return Err(PtyError::SessionNotFound);
        }

        if let Some(request) = detect_gnative_request(&bytes) {
            self.enter_gnative(id, request.clone())?;
            return Ok(GShellPtyEvent::EnterGNative(request));
        }

        let responses = self
            .prj_ref_mut()
            .update_terminal_output(id, &bytes)?
            .responses;

        Ok(GShellPtyEvent::Output { id, responses })
    }

    pub fn enter_gnative(&mut self, id: GShellId, request: GRequest) -> PtyResult<()> {
        if !self.prj_ref().as_ref().contains(id) {
            return Err(PtyError::SessionNotFound);
        }

        match request {
            GRequest::EnterGNative { .. } => {
                let state: &mut GShellServiceState = self.prj_ref_mut().as_mut();
                let shell = state.shell_mut(id)?;

                shell.initialize_gnative();
                shell.enter_gnative();

                Ok(())
            }
        }
    }

    pub fn enter_active_gnative(&mut self, request: GRequest) -> PtyResult<()> {
        let id = self.active_id();
        self.enter_gnative(id, request)
    }

    pub fn exit_gnative(&mut self, id: GShellId) -> PtyResult<()> {
        if !self.prj_ref().as_ref().contains(id) {
            return Err(PtyError::SessionNotFound);
        }

        let state: &mut GShellServiceState = self.prj_ref_mut().as_mut();
        let shell = state.shell_mut(id)?;

        shell.exit_gnative();

        Ok(())
    }

    pub fn exit_active_gnative(&mut self) -> PtyResult<()> {
        let id = self.active_id();
        self.exit_gnative(id)
    }
}

fn terminal_cell_width(font_size: f32) -> f32 {
    (font_size * TERMINAL_CELL_WIDTH_SCALE).round().max(1.0)
}

fn terminal_line_height(font_size: f32) -> f32 {
    (font_size * TERMINAL_LINE_HEIGHT_SCALE).round().max(1.0)
}

impl<Deps> GShellService<Deps>
where
    Deps: PtyPort + TerminalEnginePort + AsRef<GShellServiceState> + AsMut<GShellServiceState>,
{
    /// Starts a GShell in PtyMode through a PTY port.
    pub fn spawn(&mut self) -> PtyResult<GShellId> {
        let (id, previous_active) = {
            let state: &mut GShellServiceState = self.prj_ref_mut().as_mut();
            let previous_active = state.active();
            let id = state.allocate_id();

            state.insert(GShell::new(id))?;

            (id, previous_active)
        };

        if let Err(err) = self.prj_ref_mut().spawn(id) {
            self.prj_ref_mut()
                .as_mut()
                .rollback_insert(id, previous_active);

            return Err(err);
        }

        let size = default_terminal_size(self.prj_ref().as_ref().terminal_font_size());

        if let Err(err) = self.prj_ref_mut().create_terminal(id, size) {
            self.prj_ref_mut().close(id)?;
            self.prj_ref_mut()
                .as_mut()
                .rollback_insert(id, previous_active);

            return Err(err);
        }

        Ok(id)
    }

    pub fn activate(&mut self, id: GShellId) -> PtyResult<()> {
        self.prj_ref_mut().as_mut().activate(id)
    }

    pub async fn read_pty_event(&mut self, id: GShellId) -> PtyResult<GShellPtyEvent> {
        if !self.prj_ref().as_ref().contains(id) {
            return Err(PtyError::SessionNotFound);
        }

        let bytes = self.prj_ref_mut().read(id).await?;

        self.handle_pty_output_bytes(id, bytes)
    }

    pub async fn read_active_pty_event(&mut self) -> PtyResult<GShellPtyEvent> {
        let id = self.active_id();
        self.read_pty_event(id).await
    }

    /// Writes input bytes to the PTY bound to this running GShell.
    pub async fn write_pty(&mut self, id: GShellId, bytes: &[u8]) -> PtyResult<()> {
        if !self.prj_ref().as_ref().contains(id) {
            return Err(PtyError::SessionNotFound);
        }

        self.prj_ref_mut().write(id, bytes).await
    }

    pub async fn write_active_pty(&mut self, bytes: &[u8]) -> PtyResult<()> {
        let id = self.active_id();
        self.write_pty(id, bytes).await
    }

    /// Resizes the PTY bound to this running GShell.
    pub fn resize(&mut self, id: GShellId, size: PtySize) -> PtyResult<()> {
        if !self.prj_ref().as_ref().contains(id) {
            return Err(PtyError::SessionNotFound);
        }

        let terminal_size = terminal_size(size, self.prj_ref().as_ref().terminal_font_size());
        let _ = self.prj_ref_mut().resize_terminal(id, terminal_size)?;

        self.prj_ref_mut().resize(id, size)
    }

    pub fn resize_active(&mut self, size: PtySize) -> PtyResult<()> {
        let id = self.active_id();
        self.resize(id, size)
    }

    /// Closes the PTY bound to this running GShell.
    pub fn close(&mut self, id: GShellId) -> PtyResult<CloseShellResult> {
        if !self.prj_ref().as_ref().contains(id) {
            return Err(PtyError::SessionNotFound);
        }

        self.prj_ref_mut().close(id)?;
        self.prj_ref_mut().remove_terminal(id)?;
        self.prj_ref_mut().as_mut().remove(id)
    }

    pub fn close_active(&mut self) -> PtyResult<CloseShellResult> {
        let id = self.active_id();
        self.close(id)
    }
}

pub fn terminal_screen<Deps>(deps: &Deps, id: GShellId) -> PtyResult<&TerminalScreen>
where
    Deps: TerminalEnginePort,
{
    deps.terminal_screen(id)
}

pub fn resize_terminal_engine<Deps>(
    deps: &mut Deps,
    id: GShellId,
    size: PtySize,
    font_size: f32,
) -> PtyResult<()>
where
    Deps: TerminalEnginePort,
{
    let _ = deps.resize_terminal(id, terminal_size(size, font_size))?;
    Ok(())
}

fn default_terminal_size(font_size: f32) -> TerminalSize {
    terminal_size(
        PtySize {
            cols: DEFAULT_TERMINAL_COLS,
            rows: DEFAULT_TERMINAL_ROWS,
        },
        font_size,
    )
}

pub fn terminal_size(size: PtySize, font_size: f32) -> TerminalSize {
    TerminalSize {
        cols: size.cols,
        rows: size.rows,
        cell_width: terminal_cell_width(font_size) as u16,
        cell_height: terminal_line_height(font_size) as u16,
    }
}

pub enum CloseShellResult {
    Closed { next_active: GShellId },
    LastShellClosed,
}
