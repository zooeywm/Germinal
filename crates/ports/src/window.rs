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
    fn handle_window_event(&mut self, event: WindowEvent) -> WindowEventResult;
}
