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

/// Character-grid size of a PtyMode screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtyScreenSize {
    pub cols: u16,
    pub rows: u16,
}

impl PtyScreenSize {
    /// Creates a PtyMode screen size.
    pub fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows }
    }

    /// Returns total cell count for this screen size.
    pub fn cell_count(self) -> usize {
        self.cols as usize * self.rows as usize
    }
}

/// One character cell in a PtyMode screen.
///
/// This first version only stores one character.
/// Cell style, wide characters, and grapheme clusters will be added later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyCell {
    ch: char,
}

impl PtyCell {
    /// Creates a PtyMode screen cell.
    pub fn new(ch: char) -> Self {
        Self { ch }
    }

    /// Returns the visible character in this cell.
    pub fn ch(&self) -> char {
        self.ch
    }

    /// Creates an empty cell.
    pub fn empty() -> Self {
        Self { ch: ' ' }
    }
}
