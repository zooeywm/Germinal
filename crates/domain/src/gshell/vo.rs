/// Stable identity of a GShell.
///
/// The application layer allocates the identity.
/// The domain layer only stores and compares it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GShellId(u64);

impl GShellId {
    pub fn new(value: u64) -> Self {
        Self(value)
    }
}
