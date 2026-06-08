use crate::gshell::{GNativeSession, GShellId};

/// Aggregate root of the GShell context.
///
/// A GShell is a runtime unit that can switch between PtyMode and GNativeMode.
/// It only keeps domain state and does not own real PTY, ConPTY, renderer, or OS resources.
pub struct GShell {
    id: GShellId,
    gnative_session: Option<GNativeSession>,
    active_mode: GShellMode,
}

impl GShell {
    /// Creates a GShell that starts in PtyMode.
    ///
    /// The GShellId is allocated by the application layer so that orchestration
    /// can keep a stable reference to this shell.
    pub fn new(id: GShellId) -> Self {
        Self {
            id,
            gnative_session: None,
            active_mode: GShellMode::Pty,
        }
    }

    /// Returns the stable identity of this GShell.
    pub fn id(&self) -> GShellId {
        self.id
    }

    pub fn active_mode(&self) -> GShellMode {
        self.active_mode
    }

    pub fn initialize_gnative(&mut self) {
        if self.gnative_session.is_none() {
            self.gnative_session = Some(GNativeSession::new());
        }
    }

    /// Enters GNativeMode.
    ///
    /// This only changes domain state.
    /// Starting the real GNativeApp and protocol connection is handled by application/infra.
    pub fn enter_gnative(&mut self) {
        if self.gnative_session.is_some() {
            self.active_mode = GShellMode::GNative;
        }
    }

    /// Exits GNativeMode and returns to PtyMode.
    ///
    /// This only changes domain state.
    /// Resource cleanup is handled by application/infra.
    pub fn exit_gnative(&mut self) {
        self.gnative_session = None;
        self.active_mode = GShellMode::Pty;
    }

    pub fn switch_mode(&mut self, mode: GShellMode) {
        match mode {
            GShellMode::Pty => {
                self.active_mode = GShellMode::Pty;
            }
            GShellMode::GNative => {
                if self.gnative_session.is_some() {
                    self.active_mode = GShellMode::GNative;
                }
            }
        }
    }
}

/// Current running mode of a GShell.
///
/// Pty is used for traditional shell/TUI compatibility.
/// GNative is used for structured native applications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GShellMode {
    Pty,
    GNative,
}
