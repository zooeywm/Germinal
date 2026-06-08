pub use germinal_domain::gshell::GShellId;

/// PTY terminal size.
///
/// cols/rows are character-grid dimensions, not pixel dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtySize {
    pub cols: u16,
    pub rows: u16,
}

/// PTY port error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyError {
    /// Failed to spawn a new PTY session or its child shell process.
    SpawnFailed,

    /// A PTY session already exists for the given GShellId.
    SessionAlreadyExists,

    /// No PTY session exists for the given GShellId.
    SessionNotFound,

    /// Failed to read from, write to, resize, or otherwise operate on the PTY.
    IoFailed,
}

/// PTY port result.
pub type PtyResult<T> = Result<T, PtyError>;

/// External capability port for PTY/ConPTY.
///
/// This is the preferred port for runtime code.
/// It does not require Send because the runtime may be single-threaded.
pub trait PtyPort {
    /// Spawns a PTY session.
    fn spawn(&mut self, id: GShellId) -> PtyResult<()>;

    /// Writes input bytes to the PTY.
    fn write<'a>(
        &'a mut self,
        id: GShellId,
        bytes: &'a [u8],
    ) -> impl Future<Output = PtyResult<()>> + 'a;

    /// Reads output bytes from the PTY.
    fn read(&mut self, id: GShellId) -> impl Future<Output = PtyResult<Vec<u8>>> + '_;

    /// Resizes the PTY character grid.
    fn resize(&mut self, id: GShellId, size: PtySize) -> PtyResult<()>;

    /// Closes a PTY session.
    fn close(&mut self, id: GShellId) -> PtyResult<()>;
}
