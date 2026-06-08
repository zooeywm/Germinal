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

    pub fn value(self) -> u64 {
        self.0
    }
}

pub struct GNativeSurface {
    commands: Vec<SurfaceCommand>,
}

impl GNativeSurface {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn push(&mut self, command: SurfaceCommand) {
        self.commands.push(command);
    }

    pub fn commands(&self) -> &[SurfaceCommand] {
        &self.commands
    }
}

impl Default for GNativeSurface {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub enum SurfaceCommand {
    Clear(Color),
    FillRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    },
    Text {
        x: f32,
        y: f32,
        content: String,
        color: Color,
    },
}
