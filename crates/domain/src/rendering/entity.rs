use core::fmt;

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

impl fmt::Display for RenderFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "RenderFrame {{")?;

        for command in &self.commands {
            match command {
                RenderCommand::Clear(color) => {
                    writeln!(
                        f,
                        "  Clear rgba({}, {}, {}, {})",
                        color.r, color.g, color.b, color.a
                    )?;
                }
                RenderCommand::FillRect {
                    x,
                    y,
                    width,
                    height,
                    color,
                } => {
                    writeln!(
                        f,
                        "  FillRect x={} y={} width={} height={} rgba({}, {}, {}, {})",
                        x, y, width, height, color.r, color.g, color.b, color.a
                    )?;
                }
                RenderCommand::Text {
                    x,
                    y,
                    width,
                    height,
                    font_size,
                    content,
                    color,
                } => {
                    writeln!(
                        f,
                        "  Text x={} y={} width={} height={} font_size={} rgba({}, {}, {}, {}): {}",
                        x, y, width, height, font_size, color.r, color.g, color.b, color.a, content
                    )?;
                }
            }
        }

        write!(f, "}}")
    }
}
