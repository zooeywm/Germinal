use crate::gshell::{PtyCell, vo::PtyScreenSize};

const DEFAULT_PTY_COLS: u16 = 80;
const DEFAULT_PTY_ROWS: u16 = 24;

/// Domain state for PtyMode.
///
/// PtySession owns the screen state produced from PTY output.
pub struct PtySession {
    screen: PtyScreen,
}

impl PtySession {
    /// Creates an empty PtyMode session.
    pub fn new() -> Self {
        Self {
            screen: PtyScreen::new(PtyScreenSize::new(DEFAULT_PTY_COLS, DEFAULT_PTY_ROWS)),
        }
    }

    /// Returns the PtyMode screen state.
    pub fn screen(&self) -> &PtyScreen {
        &self.screen
    }

    /// Resizes the PtyMode screen.
    pub fn resize_screen(&mut self, size: PtyScreenSize) {
        self.screen.resize(size);
    }

    /// Writes one visible character into the PtyMode screen.
    pub fn put_char(&mut self, ch: char) {
        self.screen.put_char(ch);
    }

    /// Applies raw PTY output bytes to the PtyMode screen.
    ///
    /// This first version only handles visible ASCII bytes.
    /// Escape sequences, UTF-8, newline, carriage return, and scrolling are not implemented yet.
    pub fn apply_output_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            if byte.is_ascii_graphic() || *byte == b' ' {
                self.put_char(char::from(*byte));
            }
        }
    }
}

impl Default for PtySession {
    fn default() -> Self {
        Self::new()
    }
}

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

/// Screen state for PtyMode.
///
/// This is the domain-side terminal screen model.
/// This first version is built directly from visible PTY output bytes.
pub struct PtyScreen {
    size: PtyScreenSize,
    cells: Vec<PtyCell>,
    cursor: usize,
}

impl PtyScreen {
    /// Creates an empty PtyMode screen.
    pub fn new(size: PtyScreenSize) -> Self {
        Self {
            size,
            cells: vec![PtyCell::empty(); size.cell_count()],
            cursor: 0,
        }
    }

    /// Returns the character-grid size of this screen.
    pub fn size(&self) -> PtyScreenSize {
        self.size
    }

    pub fn cells(&self) -> &[PtyCell] {
        &self.cells
    }

    /// Resizes the PtyMode screen.
    ///
    /// Cell preservation is not implemented yet.
    /// This currently rebuilds an empty screen for the new size.
    fn resize(&mut self, size: PtyScreenSize) {
        self.size = size;
        self.cells = vec![PtyCell::empty(); size.cell_count()];
        self.cursor = 0;
    }

    fn put_char(&mut self, ch: char) {
        if self.cursor >= self.cells.len() {
            return;
        }

        self.cells[self.cursor] = PtyCell::new(ch);
        self.cursor += 1;
    }
}
