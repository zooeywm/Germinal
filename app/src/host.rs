use germinal_application::{
    gshell::{GShellService, GShellServiceState},
    runtime::{GerminalRuntime, RuntimeControlFlow, RuntimeEvent, RuntimeEventResult},
};
use germinal_domain::{gshell::GShellId, rendering::RenderFrame};
use germinal_infra::window::WinitWindowEventProxy;
use germinal_ports::window::{
    WindowControlFlow, WindowEvent, WindowEventHandler, WindowEventProxy, WindowEventResult,
};

use crate::{container::GerminalApp, effects::RuntimeEffectExecutor};

pub struct GerminalRuntimeHost {
    app: GerminalApp,
    effect_executor: RuntimeEffectExecutor,
    pending_frame: Option<RenderFrame>,
    redraw_requested: bool,
    control_flow: WindowControlFlow,
    window_proxy: Option<WinitWindowEventProxy>,
}

impl GerminalRuntimeHost {
    pub fn new() -> Self {
        let app = GerminalApp::new();
        let initial_id = <GerminalApp as AsRef<GShellServiceState>>::as_ref(&app).active();

        Self {
            app,
            effect_executor: RuntimeEffectExecutor::new(initial_id)
                .expect("failed to create RuntimeEffectExecutor"),
            pending_frame: None,
            redraw_requested: false,
            control_flow: WindowControlFlow::Continue,
            window_proxy: None,
        }
    }

    fn update_window_event(&mut self, event: WindowEvent) -> RuntimeEventResult {
        let runtime = GerminalRuntime::inj_ref_mut(&mut self.app);
        runtime.handle_window_event(event)
    }

    fn update_pty_output(&mut self, id: GShellId, bytes: Vec<u8>) -> Option<RuntimeEventResult> {
        let event = {
            let gshell_service = GShellService::inj_ref_mut(&mut self.app);

            match gshell_service.handle_pty_output_bytes(id, bytes) {
                Ok(event) => event,
                Err(err) => {
                    eprintln!("failed to handle PTY output: {err:?}");
                    return None;
                }
            }
        };

        let runtime = GerminalRuntime::inj_ref_mut(&mut self.app);

        match runtime.handle_event(RuntimeEvent::Pty(event)) {
            Ok(result) => Some(result),
            Err(err) => {
                eprintln!("failed to handle runtime PTY event: {err:?}");
                None
            }
        }
    }

    fn request_redraw(&mut self) {
        if self.redraw_requested {
            return;
        }

        if let Some(proxy) = &self.window_proxy {
            proxy.request_redraw();
            self.redraw_requested = true;
        }
    }

    fn handle_runtime_result(
        &mut self,
        result: RuntimeEventResult,
        request_redraw_on_frame: bool,
    ) -> WindowEventResult {
        let RuntimeEventResult {
            control_flow,
            frame,
            effects,
        } = result;

        self.effect_executor.apply(&mut self.app, effects);

        match control_flow {
            RuntimeControlFlow::Continue => {
                self.control_flow = WindowControlFlow::Continue;

                if let Some(frame) = frame {
                    self.pending_frame = Some(frame);

                    if request_redraw_on_frame {
                        self.request_redraw();
                    }
                }

                WindowEventResult {
                    control_flow: WindowControlFlow::Continue,
                    frame: None,
                }
            }
            RuntimeControlFlow::Exit => {
                self.control_flow = WindowControlFlow::Exit;

                if let Some(proxy) = &self.window_proxy {
                    proxy.request_exit();
                }

                WindowEventResult::exit()
            }
        }
    }
}

impl Default for GerminalRuntimeHost {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowEventHandler for GerminalRuntimeHost {
    type Proxy = WinitWindowEventProxy;

    fn set_window_event_proxy(&mut self, proxy: Self::Proxy) {
        self.effect_executor.set_window_event_proxy(proxy.clone());
        self.window_proxy = Some(proxy);
    }

    fn handle_window_event(&mut self, event: WindowEvent) -> WindowEventResult {
        if matches!(event, WindowEvent::RedrawRequested) {
            self.redraw_requested = false;
        }

        let request_redraw_on_frame = !matches!(event, WindowEvent::RedrawRequested);
        let result = self.update_window_event(event);
        self.handle_runtime_result(result, request_redraw_on_frame)
    }

    fn handle_pty_output(&mut self, id: GShellId, bytes: Vec<u8>) -> WindowEventResult {
        let Some(result) = self.update_pty_output(id, bytes) else {
            return WindowEventResult {
                control_flow: self.control_flow,
                frame: None,
            };
        };

        if result.frame.is_none() && matches!(result.control_flow, RuntimeControlFlow::Continue) {
            self.request_redraw();
        }

        self.handle_runtime_result(result, true)
    }

    fn take_pending_frame(&mut self) -> Option<RenderFrame> {
        self.pending_frame.take()
    }

    fn should_exit(&self) -> bool {
        self.control_flow == WindowControlFlow::Exit
    }
}
