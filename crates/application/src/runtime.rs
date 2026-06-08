use crate::{
    gnative::render_gnative_request,
    gshell::{GShellPtyEvent, GShellService, GShellServiceState},
};
use germinal_ports::{
    pty::{GShellId, PtyPort, PtyResult},
    renderer::{Color, RenderCommand, RenderFrame},
    window::{KeyCode, KeyState, KeyboardInput, WindowEvent},
};

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
}

impl<Deps> GerminalRuntime<Deps>
where
    Deps: AsRef<GShellServiceState> + AsMut<GShellServiceState>,
{
    pub fn handle_window_event(&mut self, event: WindowEvent) -> RuntimeEventResult {
        match event {
            WindowEvent::CloseRequested => RuntimeEventResult::exit(),
            WindowEvent::Resized(_size) => RuntimeEventResult::continue_without_frame(),
            WindowEvent::RedrawRequested => {
                RuntimeEventResult::continue_with_frame(render_current_frame())
            }
            WindowEvent::KeyboardInput(input) => {
                let Some(bytes) = handle_keyboard_input(input) else {
                    return RuntimeEventResult::continue_without_frame();
                };

                RuntimeEventResult::continue_with_frame_and_effect(
                    render_current_frame(),
                    RuntimeEffect::WritePty {
                        id: self.prj_ref().as_ref().active(),
                        bytes,
                    },
                )
            }
        }
    }

    pub fn handle_pty_event_result(
        &mut self,
        event: GShellPtyEvent,
    ) -> PtyResult<RuntimeEventResult> {
        match event {
            GShellPtyEvent::Output(_bytes) => Ok(RuntimeEventResult::continue_with_frame(
                render_current_frame(),
            )),
            GShellPtyEvent::EnterGNative(request) => {
                if let Some(frame) = render_gnative_request(request) {
                    Ok(RuntimeEventResult::continue_with_frame(frame))
                } else {
                    Ok(RuntimeEventResult::continue_without_frame())
                }
            }
        }
    }
}

impl<Deps> GerminalRuntime<Deps>
where
    Deps: PtyPort + AsRef<GShellServiceState> + AsMut<GShellServiceState>,
{
    async fn poll_event(&mut self) -> PtyResult<RuntimeEvent> {
        let gshell_service = GShellService::inj_ref_mut(self.prj_ref_mut());

        let event = gshell_service.read_active_pty_event().await?;

        Ok(RuntimeEvent::Pty(event))
    }

    fn handle_event(&mut self, event: RuntimeEvent) -> PtyResult<RuntimeControlFlow> {
        match event {
            RuntimeEvent::Pty(event) => {
                let result = self.handle_pty_event_result(event)?;
                Ok(result.control_flow)
            }
            RuntimeEvent::Shutdown => Ok(RuntimeControlFlow::Exit),
        }
    }
}

fn handle_keyboard_input(input: KeyboardInput) -> Option<Vec<u8>> {
    if input.state != KeyState::Pressed {
        return None;
    }

    encode_keyboard_input(input)
}

fn encode_keyboard_input(input: KeyboardInput) -> Option<Vec<u8>> {
    if input.modifiers.ctrl
        && let KeyCode::Character(ch) = input.key
    {
        let ch = ch.to_ascii_lowercase();

        if ch.is_ascii_lowercase() {
            return Some(vec![ch as u8 - b'a' + 1]);
        }

        return match ch {
            '[' => Some(vec![0x1b]),
            '\\' => Some(vec![0x1c]),
            ']' => Some(vec![0x1d]),
            '^' => Some(vec![0x1e]),
            '_' => Some(vec![0x1f]),
            '?' => Some(vec![0x7f]),
            _ => None,
        };
    }

    match input.key {
        KeyCode::Enter => Some(b"\r".to_vec()),
        KeyCode::Backspace => Some(vec![0x7f]),
        KeyCode::Escape => Some(vec![0x1b]),
        KeyCode::ArrowUp => Some(b"\x1b[A".to_vec()),
        KeyCode::ArrowDown => Some(b"\x1b[B".to_vec()),
        KeyCode::ArrowRight => Some(b"\x1b[C".to_vec()),
        KeyCode::ArrowLeft => Some(b"\x1b[D".to_vec()),
        KeyCode::Character(ch) => {
            let mut bytes = [0; 4];
            Some(ch.encode_utf8(&mut bytes).as_bytes().to_vec())
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
