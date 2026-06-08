/// Structured native application session.
///
/// This means that the GShell has initialized a GNative session.
/// The active mode is tracked separately by GShellMode.
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
