use crate::rendering::RenderCommand;

/// One frame of rendering data produced by GNativeMode.
///
/// RenderFrame is only a command list.
/// It is not a GPU texture, window surface, or swapchain frame.
pub struct RenderFrame {
    commands: Vec<RenderCommand>,
}

impl RenderFrame {
    /// Creates an empty frame.
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Appends one drawing command to this frame.
    pub fn push(&mut self, command: RenderCommand) {
        self.commands.push(command);
    }

    /// Returns drawing commands in this frame.
    ///
    /// The renderer can read commands but cannot replace the whole command list.
    pub fn commands(&self) -> &[RenderCommand] {
        &self.commands
    }
}

impl Default for RenderFrame {
    fn default() -> Self {
        Self::new()
    }
}
