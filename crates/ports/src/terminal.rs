use crate::pty::{GShellId, PtyResult, PtySize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub cols: u16,
    pub rows: u16,
    pub cell_width: u16,
    pub cell_height: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalCell {
    ch: char,
    foreground: Option<TerminalColor>,
    background: Option<TerminalColor>,
    continuation: bool,
}

impl TerminalCell {
    pub fn new(
        ch: char,
        foreground: Option<TerminalColor>,
        background: Option<TerminalColor>,
    ) -> Self {
        Self {
            ch,
            foreground,
            background,
            continuation: false,
        }
    }

    pub fn continuation(
        foreground: Option<TerminalColor>,
        background: Option<TerminalColor>,
    ) -> Self {
        Self {
            ch: ' ',
            foreground,
            background,
            continuation: true,
        }
    }

    pub fn ch(&self) -> char {
        self.ch
    }

    pub fn foreground(&self) -> Option<TerminalColor> {
        self.foreground
    }

    pub fn background(&self) -> Option<TerminalColor> {
        self.background
    }

    pub fn is_continuation(&self) -> bool {
        self.continuation
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalScreen {
    size: PtySize,
    cells: Vec<TerminalCell>,
}

impl TerminalScreen {
    pub fn new(size: PtySize) -> Self {
        Self {
            size,
            cells: vec![
                TerminalCell::new(' ', None, None);
                usize::from(size.cols) * usize::from(size.rows)
            ],
        }
    }

    pub fn size(&self) -> PtySize {
        self.size
    }

    pub fn cells(&self) -> &[TerminalCell] {
        &self.cells
    }

    pub fn replace(&mut self, size: PtySize, cells: Vec<TerminalCell>) {
        self.size = size;
        self.cells = cells;
    }
}

#[derive(Debug, Default)]
pub struct TerminalUpdate {
    pub responses: Vec<Vec<u8>>,
}

pub trait TerminalEnginePort {
    fn create_terminal(&mut self, id: GShellId, size: TerminalSize) -> PtyResult<()>;

    fn update_terminal_output(&mut self, id: GShellId, bytes: &[u8]) -> PtyResult<TerminalUpdate>;

    fn resize_terminal(&mut self, id: GShellId, size: TerminalSize) -> PtyResult<TerminalUpdate>;

    fn remove_terminal(&mut self, id: GShellId) -> PtyResult<()>;

    fn terminal_screen(&self, id: GShellId) -> PtyResult<&TerminalScreen>;
}
