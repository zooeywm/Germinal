use std::collections::HashMap;

use germinal_domain::gshell::{GShell, GShellId};
use germinal_ports::pty::{PtyError, PtyPort, PtyResult, PtySize};

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
        }
    }

    fn allocate_id(&mut self) -> GShellId {
        let id = GShellId::new(self.next_id);
        self.next_id += 1;
        id
    }

    pub fn active(&self) -> GShellId {
        self.active
    }

    pub fn contains(&self, id: GShellId) -> bool {
        self.shells.contains_key(&id)
    }

    fn insert(&mut self, shell: GShell) -> PtyResult<()> {
        let id = shell.id();

        if self.shells.contains_key(&id) {
            return Err(PtyError::AlreadyExists);
        }

        self.shells.insert(id, shell);

        self.active = id;

        Ok(())
    }

    fn activate(&mut self, id: GShellId) -> PtyResult<()> {
        if !self.shells.contains_key(&id) {
            return Err(PtyError::UnknownHandle);
        }

        self.active = id;
        Ok(())
    }

    fn remove(&mut self, id: GShellId) -> PtyResult<CloseShellResult> {
        if !self.shells.contains_key(&id) {
            return Err(PtyError::UnknownHandle);
        }

        if self.shells.len() == 1 {
            self.shells.remove(&id);
            return Ok(CloseShellResult::LastShellClosed);
        }

        self.shells.remove(&id);

        if self.active == id {
            self.active = *self
                .shells
                .keys()
                .next()
                .expect("shells is non-empty after removing non-last shell");
        }

        Ok(CloseShellResult::Closed {
            next_active: self.active,
        })
    }

    fn apply_pty_output_bytes(&mut self, id: GShellId, bytes: &[u8]) -> PtyResult<()> {
        let shell = self.shells.get_mut(&id).ok_or(PtyError::UnknownHandle)?;
        shell.apply_pty_output_bytes(bytes);
        Ok(())
    }
}

impl Default for GShellServiceState {
    fn default() -> Self {
        Self::new()
    }
}

impl<Deps> GShellService<Deps>
where
    Deps: PtyPort + AsRef<GShellServiceState> + AsMut<GShellServiceState>,
{
    /// Starts a GShell in PtyMode through a PTY port.
    pub fn spawn(&mut self) -> PtyResult<GShellId> {
        let id = {
            let state: &mut GShellServiceState = self.prj_ref_mut().as_mut();
            state.allocate_id()
        };

        self.prj_ref_mut().spawn(id)?;

        let state: &mut GShellServiceState = self.prj_ref_mut().as_mut();
        state.insert(GShell::new(id))?;

        Ok(id)
    }

    pub fn activate(&mut self, id: GShellId) -> PtyResult<()> {
        self.prj_ref_mut().as_mut().activate(id)
    }

    /// Writes input bytes to the PTY bound to this running GShell.
    pub async fn write_pty(&mut self, id: GShellId, bytes: &[u8]) -> PtyResult<()> {
        if !self.prj_ref().as_ref().contains(id) {
            return Err(PtyError::UnknownHandle);
        }

        self.prj_ref_mut().write(id, bytes).await
    }

    /// Reads output bytes from the PTY bound to this running GShell.
    pub async fn read_pty(&mut self, id: GShellId) -> PtyResult<Vec<u8>> {
        if !self.prj_ref().as_ref().contains(id) {
            return Err(PtyError::UnknownHandle);
        }

        let bytes = self.prj_ref_mut().read(id).await?;

        self.prj_ref_mut()
            .as_mut()
            .apply_pty_output_bytes(id, &bytes)?;

        Ok(bytes)
    }

    /// Resizes the PTY bound to this running GShell.
    pub fn resize(&mut self, id: GShellId, size: PtySize) -> PtyResult<()> {
        if !self.prj_ref().as_ref().contains(id) {
            return Err(PtyError::UnknownHandle);
        }

        self.prj_ref_mut().resize(id, size)
    }

    /// Closes the PTY bound to this running GShell.
    pub fn close(&mut self, id: GShellId) -> PtyResult<CloseShellResult> {
        if !self.prj_ref().as_ref().contains(id) {
            return Err(PtyError::UnknownHandle);
        }

        self.prj_ref_mut().close(id)?;
        self.prj_ref_mut().as_mut().remove(id)
    }

    pub async fn write_active_pty(&mut self, bytes: &[u8]) -> PtyResult<()> {
        let id = self.active_id();
        self.write_pty(id, bytes).await
    }

    pub async fn read_active_pty(&mut self) -> PtyResult<Vec<u8>> {
        let id = self.active_id();
        self.read_pty(id).await
    }

    pub fn close_active(&mut self) -> PtyResult<CloseShellResult> {
        let id = self.active_id();
        self.close(id)
    }

    fn active_id(&self) -> GShellId {
        self.prj_ref().as_ref().active()
    }
}

pub enum CloseShellResult {
    Closed { next_active: GShellId },
    LastShellClosed,
}
