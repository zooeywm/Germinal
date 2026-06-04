/// Traditional PTY session.
///
/// This only means that the GShell is currently carrying a PTY session.
/// Real PTY/ConPTY handles, processes, and I/O belong to ports/infra.
pub struct PtySession;

impl PtySession {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PtySession {
    fn default() -> Self {
        Self::new()
    }
}

/// Structured native application session.
///
/// This only means that the GShell has entered GNative mode.
/// The real app process, protocol connection, and render output are not stored here.
pub struct GNativeSession;

impl GNativeSession {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GNativeSession {
    fn default() -> Self {
        Self::new()
    }
}
