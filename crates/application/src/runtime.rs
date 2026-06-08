use crate::{
    gnative::render_gnative_request,
    gshell::{GShellPtyEvent, GShellService, GShellServiceState},
    rendering::render_frame,
};
use germinal_ports::{
    pty::{PtyPort, PtyResult},
    renderer::{Color, RenderCommand, RenderFrame, RendererPort},
    window::WindowEvent,
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
}

impl RuntimeEventResult {
    pub fn continue_without_frame() -> Self {
        Self {
            control_flow: RuntimeControlFlow::Continue,
            frame: None,
        }
    }

    pub fn continue_with_frame(frame: RenderFrame) -> Self {
        Self {
            control_flow: RuntimeControlFlow::Continue,
            frame: Some(frame),
        }
    }

    pub fn exit() -> Self {
        Self {
            control_flow: RuntimeControlFlow::Exit,
            frame: None,
        }
    }
}

impl<Deps> GerminalRuntime<Deps>
where
    Deps: PtyPort + RendererPort + AsRef<GShellServiceState> + AsMut<GShellServiceState>,
{
    async fn poll_event(&mut self) -> PtyResult<RuntimeEvent> {
        let gshell_service = GShellService::inj_ref_mut(self.prj_ref_mut());

        let event = gshell_service.read_active_pty_event().await?;

        Ok(RuntimeEvent::Pty(event))
    }

    pub fn handle_window_event(&mut self, event: WindowEvent) -> RuntimeEventResult {
        match event {
            WindowEvent::CloseRequested => RuntimeEventResult::exit(),
            WindowEvent::Resized(_size) => RuntimeEventResult::continue_without_frame(),
            WindowEvent::RedrawRequested => {
                let mut frame = RenderFrame::new();

                frame.push(RenderCommand::Clear(Color {
                    r: 16,
                    g: 200,
                    b: 28,
                    a: 255,
                }));

                RuntimeEventResult::continue_with_frame(frame)
            }
        }
    }

    fn handle_event(&mut self, event: RuntimeEvent) -> PtyResult<RuntimeControlFlow> {
        match event {
            RuntimeEvent::Pty(event) => self.handle_pty_event(event),
            RuntimeEvent::Shutdown => Ok(RuntimeControlFlow::Exit),
        }
    }

    fn handle_pty_event(&mut self, event: GShellPtyEvent) -> PtyResult<RuntimeControlFlow> {
        match event {
            GShellPtyEvent::Output(bytes) => {
                print!("{}", String::from_utf8_lossy(&bytes));
                Ok(RuntimeControlFlow::Continue)
            }
            GShellPtyEvent::EnterGNative(request) => {
                if let Some(frame) = render_gnative_request(request) {
                    render_frame(self.prj_ref_mut(), &frame);
                }

                Ok(RuntimeControlFlow::Continue)
            }
        }
    }
}
