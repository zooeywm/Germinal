#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PaneId(u64);

impl PaneId {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn num(&self) -> u64 {
        self.0
    }
}
