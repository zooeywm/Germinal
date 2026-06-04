/// RGBA color.
///
/// The channels use u8 so renderer implementations can convert them to common
/// pixel formats directly.
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Drawing command in a frame.
///
/// This defines the minimal rendering semantics that GNativeMode can produce.
/// Real GPU/software drawing is handled by renderer infra.
pub enum RenderCommand {
    /// Clears the target surface.
    Clear(Color),
    /// Fills a rectangle area.
    FillRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    },
    /// Draws a text run.
    ///
    /// Font family, font size, and weight are currently renderer-defined defaults.
    Text {
        x: f32,
        y: f32,
        content: String,
        color: Color,
    },
}
