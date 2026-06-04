use crate::gshell::{GNativeSession, GShellId, PtySession};

/// Aggregate root of the GShell context.
///
/// A GShell is a runtime unit that can switch between PtyMode and GNativeMode.
/// It only keeps domain state and does not own real PTY, ConPTY, renderer, or OS resources.
pub struct GShell {
    id: GShellId,
    mode: GShellMode,
}

impl GShell {
    /// Creates a GShell that starts in PtyMode.
    ///
    /// The GShellId is allocated by the application layer so that orchestration
    /// can keep a stable reference to this shell.
    pub fn new(id: GShellId) -> Self {
        Self {
            id,
            mode: GShellMode::Pty(PtySession::new()),
        }
    }

    /// Returns the stable identity of this GShell.
    pub fn id(&self) -> GShellId {
        self.id
    }

    /// Enters GNativeMode.
    ///
    /// This only changes domain state.
    /// Starting the real GNativeApp and protocol connection is handled by application/infra.
    pub fn enter_gnative(&mut self) {
        self.mode = GShellMode::GNative(GNativeSession::new())
    }

    /// Exits GNativeMode and returns to PtyMode.
    ///
    /// This only changes domain state.
    /// Resource cleanup is handled by application/infra.
    pub fn exit_gnative(&mut self) {
        self.mode = GShellMode::Pty(PtySession::new())
    }
}

/// Current running mode of a GShell.
///
/// Pty is used for traditional shell/TUI compatibility.
/// GNative is used for structured native applications.
pub enum GShellMode {
    Pty(PtySession),
    GNative(GNativeSession),
}
