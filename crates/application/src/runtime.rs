use crate::{
    gnative::render_gnative_request,
    gshell::{GShellPtyEvent, GShellServiceState, terminal_screen, terminal_size},
};
use germinal_ports::{
    pty::{GShellId, PtyResult, PtySize},
    renderer::{Color, RenderCommand, RenderFrame},
    terminal::{TerminalCell, TerminalColor, TerminalEnginePort, TerminalScreen},
    window::{KeyCode, KeyState, KeyboardInput, WindowEvent, WindowSize},
};

const TERMINAL_X: f32 = 12.0;
const TERMINAL_Y: f32 = 18.0;
const MIN_TERMINAL_FONT_SIZE: f32 = 10.0;
const MAX_TERMINAL_FONT_SIZE: f32 = 28.0;
const DEFAULT_TERMINAL_FONT_SIZE: f32 = 15.0;
const FONT_STEP: f32 = 1.0;
const CELL_WIDTH_SCALE: f32 = 0.62;
const LINE_HEIGHT_SCALE: f32 = 1.18;

#[kudi::target]
pub struct GerminalRuntime;

pub enum RuntimeEvent {
    Pty(GShellPtyEvent),
    Shutdown,
}

pub enum RuntimeControlFlow {
    Continue,
    Exit,
}

pub struct RuntimeEventResult {
    pub control_flow: RuntimeControlFlow,
    pub frame: Option<RenderFrame>,
    pub effects: Vec<RuntimeEffect>,
}

impl RuntimeEventResult {
    pub fn continue_without_frame() -> Self {
        Self {
            control_flow: RuntimeControlFlow::Continue,
            frame: None,
            effects: Vec::new(),
        }
    }

    pub fn continue_with_frame(frame: RenderFrame) -> Self {
        Self {
            control_flow: RuntimeControlFlow::Continue,
            frame: Some(frame),
            effects: Vec::new(),
        }
    }

    pub fn continue_with_effect(effect: RuntimeEffect) -> Self {
        Self {
            control_flow: RuntimeControlFlow::Continue,
            frame: None,
            effects: vec![effect],
        }
    }

    pub fn continue_with_frame_and_effect(frame: RenderFrame, effect: RuntimeEffect) -> Self {
        Self {
            control_flow: RuntimeControlFlow::Continue,
            frame: Some(frame),
            effects: vec![effect],
        }
    }

    pub fn continue_with_frame_and_effects(
        frame: Option<RenderFrame>,
        effects: Vec<RuntimeEffect>,
    ) -> Self {
        Self {
            control_flow: RuntimeControlFlow::Continue,
            frame,
            effects,
        }
    }

    pub fn exit() -> Self {
        Self {
            control_flow: RuntimeControlFlow::Exit,
            frame: None,
            effects: Vec::new(),
        }
    }
}

pub enum RuntimeEffect {
    WritePty { id: GShellId, bytes: Vec<u8> },
    ResizePty { id: GShellId, size: PtySize },
}

impl<Deps> GerminalRuntime<Deps>
where
    Deps: TerminalEnginePort + AsRef<GShellServiceState> + AsMut<GShellServiceState>,
{
    pub fn handle_event(&mut self, event: RuntimeEvent) -> PtyResult<RuntimeEventResult> {
        match event {
            RuntimeEvent::Pty(event) => self.handle_pty_event_result(event),
            RuntimeEvent::Shutdown => Ok(RuntimeEventResult::exit()),
        }
    }

    pub fn handle_window_event(&mut self, event: WindowEvent) -> RuntimeEventResult {
        match event {
            WindowEvent::CloseRequested => RuntimeEventResult::exit(),
            WindowEvent::Resized(size) => self.resize_terminal(size),
            WindowEvent::RedrawRequested => {
                RuntimeEventResult::continue_with_frame(self.render_active_pty_frame())
            }
            WindowEvent::KeyboardInput(input) => {
                let Some(action) = handle_keyboard_input(input) else {
                    return RuntimeEventResult::continue_without_frame();
                };

                self.handle_keyboard_action(action)
            }
        }
    }

    pub fn handle_pty_event_result(
        &mut self,
        event: GShellPtyEvent,
    ) -> PtyResult<RuntimeEventResult> {
        match event {
            GShellPtyEvent::Output { id, responses } => {
                let effects = responses
                    .into_iter()
                    .map(|bytes| RuntimeEffect::WritePty { id, bytes })
                    .collect();

                Ok(RuntimeEventResult::continue_with_frame_and_effects(
                    None, effects,
                ))
            }
            GShellPtyEvent::EnterGNative(request) => {
                if let Some(frame) = render_gnative_request(request) {
                    Ok(RuntimeEventResult::continue_with_frame(frame))
                } else {
                    Ok(RuntimeEventResult::continue_without_frame())
                }
            }
        }
    }

    fn render_active_pty_frame(&self) -> RenderFrame {
        let deps = self.prj_ref();
        let state = deps.as_ref();

        match terminal_screen(deps, state.active()) {
            Ok(screen) => render_pty_screen(screen, state.terminal_font_size()),
            Err(_) => render_current_frame(),
        }
    }

    fn handle_keyboard_action(&mut self, action: KeyboardAction) -> RuntimeEventResult {
        match action {
            KeyboardAction::Write(bytes) => {
                RuntimeEventResult::continue_with_effect(RuntimeEffect::WritePty {
                    id: self.prj_ref().as_ref().active(),
                    bytes,
                })
            }
            KeyboardAction::Exit => RuntimeEventResult::exit(),
            KeyboardAction::ZoomIn => self.zoom_terminal(FONT_STEP),
            KeyboardAction::ZoomOut => self.zoom_terminal(-FONT_STEP),
            KeyboardAction::ResetZoom => self.set_terminal_font_size(DEFAULT_TERMINAL_FONT_SIZE),
        }
    }

    fn zoom_terminal(&mut self, delta: f32) -> RuntimeEventResult {
        let font_size = self.prj_ref().as_ref().terminal_font_size();
        self.set_terminal_font_size(font_size + delta)
    }

    fn set_terminal_font_size(&mut self, font_size: f32) -> RuntimeEventResult {
        let font_size = font_size.clamp(MIN_TERMINAL_FONT_SIZE, MAX_TERMINAL_FONT_SIZE);
        let window_size = {
            let state = self.prj_ref_mut().as_mut();
            state.set_terminal_font_size(font_size);
            state.last_window_size()
        };

        if let Some(window_size) = window_size {
            return self.resize_terminal(window_size);
        }

        RuntimeEventResult::continue_with_frame(self.render_active_pty_frame())
    }

    fn resize_terminal(&mut self, window_size: WindowSize) -> RuntimeEventResult {
        let (active_id, pty_size) = {
            let state = self.prj_ref_mut().as_mut();
            state.set_last_window_size(window_size);

            let font_size = state.terminal_font_size();
            let pty_size = terminal_grid_size(window_size, font_size);

            (state.active(), pty_size)
        };

        let font_size = self.prj_ref().as_ref().terminal_font_size();
        let terminal_size = terminal_size(pty_size, font_size);

        if let Err(err) = self.prj_ref_mut().resize_terminal(active_id, terminal_size) {
            eprintln!("failed to resize terminal engine: {err:?}");
        }

        RuntimeEventResult::continue_with_frame_and_effect(
            self.render_active_pty_frame(),
            RuntimeEffect::ResizePty {
                id: active_id,
                size: pty_size,
            },
        )
    }
}

enum KeyboardAction {
    Write(Vec<u8>),
    Exit,
    ZoomIn,
    ZoomOut,
    ResetZoom,
}

fn handle_keyboard_input(input: KeyboardInput) -> Option<KeyboardAction> {
    if input.state != KeyState::Pressed {
        return None;
    }

    encode_keyboard_input(input)
}

fn encode_keyboard_input(input: KeyboardInput) -> Option<KeyboardAction> {
    if input.modifiers.ctrl
        && let KeyCode::Character(ch) = input.key
    {
        let ch = ch.to_ascii_lowercase();

        match ch {
            'd' => return Some(KeyboardAction::Exit),
            '+' | '=' => return Some(KeyboardAction::ZoomIn),
            '-' => return Some(KeyboardAction::ZoomOut),
            '0' => return Some(KeyboardAction::ResetZoom),
            _ => {}
        }

        if ch.is_ascii_lowercase() {
            return Some(KeyboardAction::Write(vec![ch as u8 - b'a' + 1]));
        }

        return match ch {
            '[' => Some(KeyboardAction::Write(vec![0x1b])),
            '\\' => Some(KeyboardAction::Write(vec![0x1c])),
            ']' => Some(KeyboardAction::Write(vec![0x1d])),
            '^' => Some(KeyboardAction::Write(vec![0x1e])),
            '_' => Some(KeyboardAction::Write(vec![0x1f])),
            '?' => Some(KeyboardAction::Write(vec![0x7f])),
            _ => None,
        };
    }

    match input.key {
        KeyCode::Enter => Some(KeyboardAction::Write(b"\r".to_vec())),
        KeyCode::Backspace => Some(KeyboardAction::Write(vec![0x7f])),
        KeyCode::Escape => Some(KeyboardAction::Write(vec![0x1b])),
        KeyCode::ArrowUp => Some(KeyboardAction::Write(b"\x1b[A".to_vec())),
        KeyCode::ArrowDown => Some(KeyboardAction::Write(b"\x1b[B".to_vec())),
        KeyCode::ArrowRight => Some(KeyboardAction::Write(b"\x1b[C".to_vec())),
        KeyCode::ArrowLeft => Some(KeyboardAction::Write(b"\x1b[D".to_vec())),
        KeyCode::Character(ch) => {
            let mut bytes = [0; 4];
            Some(KeyboardAction::Write(
                ch.encode_utf8(&mut bytes).as_bytes().to_vec(),
            ))
        }
        KeyCode::Unknown => None,
    }
}

fn render_current_frame() -> RenderFrame {
    let mut frame = RenderFrame::new();

    frame.push(RenderCommand::Clear(Color {
        r: 16,
        g: 20,
        b: 28,
        a: 255,
    }));

    frame
}

fn render_pty_screen(screen: &TerminalScreen, font_size: f32) -> RenderFrame {
    let mut frame = render_current_frame();
    let size = screen.size();
    let cols = usize::from(size.cols);
    let cell_width = terminal_cell_width(font_size);
    let line_height = terminal_line_height(font_size);

    if cols == 0 {
        return frame;
    }

    for (row, cells) in screen.cells().chunks(cols).enumerate() {
        render_background_runs(&mut frame, row, cells, cell_width, line_height);
        render_text_runs(&mut frame, row, cells, font_size, cell_width, line_height);
    }

    frame
}

fn render_background_runs(
    frame: &mut RenderFrame,
    row: usize,
    cells: &[TerminalCell],
    cell_width: f32,
    line_height: f32,
) {
    let mut col = 0;

    while col < cells.len() {
        let Some(background) = cells[col].background() else {
            col += 1;
            continue;
        };

        let start = col;

        while col < cells.len() && cells[col].background() == Some(background) {
            col += 1;
        }

        frame.push(RenderCommand::FillRect {
            x: TERMINAL_X + start as f32 * cell_width,
            y: TERMINAL_Y + row as f32 * line_height,
            width: (col - start) as f32 * cell_width,
            height: line_height,
            color: render_color(background),
        });
    }
}

fn render_text_runs(
    frame: &mut RenderFrame,
    row: usize,
    cells: &[TerminalCell],
    font_size: f32,
    cell_width: f32,
    line_height: f32,
) {
    let mut col = 0;

    while col < cells.len() {
        let foreground = cells[col].foreground();
        let start = col;

        while col < cells.len() && cells[col].foreground() == foreground {
            col += 1;
        }

        let content = cells[start..col]
            .iter()
            .map(|cell| cell.ch())
            .collect::<String>();
        let content = content.trim_end();

        if content.is_empty() {
            continue;
        }

        frame.push(RenderCommand::Text {
            x: TERMINAL_X + start as f32 * cell_width,
            y: TERMINAL_Y + row as f32 * line_height,
            width: (col - start) as f32 * cell_width,
            height: line_height,
            font_size,
            content: content.to_string(),
            color: foreground.map(render_color).unwrap_or(Color {
                r: 220,
                g: 226,
                b: 235,
                a: 255,
            }),
        });
    }
}

fn render_color(color: TerminalColor) -> Color {
    Color {
        r: color.r,
        g: color.g,
        b: color.b,
        a: 255,
    }
}

fn terminal_grid_size(window_size: WindowSize, font_size: f32) -> PtySize {
    let cols = ((window_size.width as f32 - TERMINAL_X * 2.0) / terminal_cell_width(font_size))
        .floor()
        .max(1.0) as u16;
    let rows = ((window_size.height as f32 - TERMINAL_Y * 2.0) / terminal_line_height(font_size))
        .floor()
        .max(1.0) as u16;

    PtySize { cols, rows }
}

fn terminal_cell_width(font_size: f32) -> f32 {
    (font_size * CELL_WIDTH_SCALE).max(1.0)
}

fn terminal_line_height(font_size: f32) -> f32 {
    (font_size * LINE_HEIGHT_SCALE).max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_space_input() {
        let input = KeyboardInput {
            state: KeyState::Pressed,
            key: KeyCode::Character(' '),
            modifiers: Default::default(),
        };

        assert!(matches!(
            handle_keyboard_input(input),
            Some(KeyboardAction::Write(bytes)) if bytes == b" "
        ));
    }

    #[test]
    fn maps_ctrl_d_to_exit() {
        let input = KeyboardInput {
            state: KeyState::Pressed,
            key: KeyCode::Character('d'),
            modifiers: germinal_ports::window::KeyModifiers {
                ctrl: true,
                ..Default::default()
            },
        };

        assert!(matches!(
            handle_keyboard_input(input),
            Some(KeyboardAction::Exit)
        ));
    }

    #[test]
    fn maps_ctrl_equals_to_zoom_in() {
        let input = KeyboardInput {
            state: KeyState::Pressed,
            key: KeyCode::Character('='),
            modifiers: germinal_ports::window::KeyModifiers {
                ctrl: true,
                ..Default::default()
            },
        };

        assert!(matches!(
            handle_keyboard_input(input),
            Some(KeyboardAction::ZoomIn)
        ));
    }
}
