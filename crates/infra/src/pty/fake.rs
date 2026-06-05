use germinal_ports::pty::{PtyHandle, PtyPort, PtySize};

/// Fake PTY used to verify PTY port wiring.
///
/// It stores written bytes in memory and returns them on read.
/// Real Unix PTY / Windows ConPTY implementations will be added later.
pub struct FakePty {
    buffer: Vec<u8>,
}

impl FakePty {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }
}

impl Default for FakePty {
    fn default() -> Self {
        Self::new()
    }
}

impl PtyPort for FakePty {
    fn spawn(&mut self) -> PtyHandle {
        PtyHandle::new(1)
    }

    fn write(&mut self, _handle: &PtyHandle, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    fn read(&mut self, _handle: &PtyHandle) -> Vec<u8> {
        std::mem::take(&mut self.buffer)
    }

    fn resize(&mut self, _handle: &PtyHandle, _size: PtySize) {}

    fn close(&mut self, _handle: PtyHandle) {}
}
