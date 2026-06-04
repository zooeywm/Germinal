/// Abstract handle for a real PTY/ConPTY resource.
///
/// The concrete content is defined by infra.
/// The application layer only uses this handle to refer to the external resource.
pub struct PtyHandle;

/// PTY terminal size.
///
/// cols/rows are character-grid dimensions, not pixel dimensions.
pub struct PtySize {
    pub cols: u16,
    pub rows: u16,
}

/// External capability port for PTY/ConPTY.
///
/// Unix PTY and Windows ConPTY implementations should both implement this trait.
/// The application layer uses this port to spawn a shell, write input, read output,
/// and resize the terminal.
pub trait PtyPort {
    /// Spawns a PTY session.
    fn spawn(&mut self) -> PtyHandle;

    /// Writes input bytes to the PTY.
    fn write(&mut self, handle: &PtyHandle, bytes: &[u8]);

    /// Writes input bytes to the PTY.
    fn read(&mut self, handle: &PtyHandle) -> Vec<u8>;

    /// Resizes the PTY character grid.
    fn resize(&mut self, handle: &PtyHandle, size: PtySize);

    /// Closes a PTY session.
    ///
    /// The handle is consumed, so callers should not use it after closing.
    fn close(&mut self, handle: PtyHandle);
}
