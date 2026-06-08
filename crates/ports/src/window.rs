use germinal_domain::rendering::RenderFrame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowEvent {
    CloseRequested,
    Resized(WindowSize),
    RedrawRequested,
    KeyboardInput(KeyboardInput),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowControlFlow {
    Continue,
    Exit,
}

pub struct WindowEventResult {
    pub control_flow: WindowControlFlow,
    pub frame: Option<RenderFrame>,
}

impl WindowEventResult {
    pub fn continue_without_frame() -> Self {
        Self {
            control_flow: WindowControlFlow::Continue,
            frame: None,
        }
    }

    pub fn continue_with_frame(frame: RenderFrame) -> Self {
        Self {
            control_flow: WindowControlFlow::Continue,
            frame: Some(frame),
        }
    }

    pub fn exit() -> Self {
        Self {
            control_flow: WindowControlFlow::Exit,
            frame: None,
        }
    }
}

pub trait WindowEventHandler {
    type Proxy: WindowEventProxy;

    fn set_window_event_proxy(&mut self, _proxy: Self::Proxy) {}

    fn handle_window_event(&mut self, event: WindowEvent) -> WindowEventResult;
}

pub trait WindowEventProxy {
    fn request_redraw(&self);
    fn notify_pty_output(&self);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    Enter,
    Backspace,
    Escape,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Character(char),
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub logo: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardInput {
    pub state: KeyState,
    pub key: KeyCode,
    pub modifiers: KeyModifiers,
}
