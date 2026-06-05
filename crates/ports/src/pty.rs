/// Abstract handle for a real PTY/ConPTY resource.
///
/// The concrete resource is stored by infra.
/// This handle only identifies that resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PtyHandle(u64);

impl PtyHandle {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn id(&self) -> u64 {
        self.0
    }
}

/// PTY terminal size.
///
/// cols/rows are character-grid dimensions, not pixel dimensions.
pub struct PtySize {
    pub cols: u16,
    pub rows: u16,
}

/// PTY port error.
#[derive(Debug)]
pub enum PtyError {
    SpawnFailed,
    UnknownHandle,
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
    fn spawn(&mut self) -> PtyResult<PtyHandle>;

    /// Writes input bytes to the PTY.
    fn write<'a>(
        &'a mut self,
        handle: &'a PtyHandle,
        bytes: &'a [u8],
    ) -> impl Future<Output = PtyResult<()>> + 'a;

    /// Reads output bytes from the PTY.
    fn read<'a>(
        &'a mut self,
        handle: &'a PtyHandle,
    ) -> impl Future<Output = PtyResult<Vec<u8>>> + 'a;

    /// Resizes the PTY character grid.
    fn resize(&mut self, handle: &PtyHandle, size: PtySize) -> PtyResult<()>;

    /// Closes a PTY session.
    fn close(&mut self, handle: PtyHandle) -> PtyResult<()>;
}
