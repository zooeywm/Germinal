use std::collections::HashMap;

use germinal_application::{
    gshell::{GShellService, GShellServiceState},
    runtime::{GerminalRuntime, RuntimeControlFlow, RuntimeEvent, RuntimeEventResult},
};
use germinal_domain::{gshell::GShellId, rendering::RenderFrame};
use germinal_infra::window::WinitWindowEventProxy;
use germinal_ports::window::{
    WindowControlFlow, WindowEvent, WindowEventHandler, WindowEventProxy, WindowEventResult,
};

use crate::{app_deps::AppDeps, effects::RuntimeEffectExecutor};

pub struct GerminalRuntimeHost {
    deps: AppDeps,
    effect_executor: RuntimeEffectExecutor,
    pending_pty_output: HashMap<GShellId, Vec<u8>>,
    pending_frame: Option<RenderFrame>,
    redraw_requested: bool,
    control_flow: WindowControlFlow,
    window_proxy: Option<WinitWindowEventProxy>,
}

impl GerminalRuntimeHost {
    pub fn new(deps: AppDeps) -> Self {
        let initial_id = <AppDeps as AsRef<GShellServiceState>>::as_ref(&deps).active();

        Self {
            deps,
            effect_executor: RuntimeEffectExecutor::new(initial_id)
                .expect("failed to create RuntimeEffectExecutor"),
            pending_pty_output: HashMap::new(),
            pending_frame: None,
            redraw_requested: false,
            control_flow: WindowControlFlow::Continue,
            window_proxy: None,
        }
    }

    fn update_window_event(&mut self, event: WindowEvent) -> RuntimeEventResult {
        let runtime = GerminalRuntime::inj_ref_mut(&mut self.deps);
        runtime.handle_window_event(event)
    }

    fn update_pty_output(&mut self, id: GShellId, bytes: Vec<u8>) -> Option<RuntimeEventResult> {
        let event = {
            let gshell_service = GShellService::inj_ref_mut(&mut self.deps);

            match gshell_service.handle_pty_output_bytes(id, bytes) {
                Ok(event) => event,
                Err(err) => {
                    eprintln!("failed to handle PTY output: {err:?}");
                    return None;
                }
            }
        };

        let runtime = GerminalRuntime::inj_ref_mut(&mut self.deps);

        match runtime.handle_event(RuntimeEvent::Pty(event)) {
            Ok(result) => Some(result),
            Err(err) => {
                eprintln!("failed to handle runtime PTY event: {err:?}");
                None
            }
        }
    }

    fn queue_pty_output(&mut self, id: GShellId, bytes: Vec<u8>) {
        if bytes.is_empty() {
            return;
        }

        self.pending_pty_output.entry(id).or_default().extend(bytes);
    }

    fn flush_pty_output(&mut self) -> WindowEventResult {
        let pending = std::mem::take(&mut self.pending_pty_output);

        for (id, bytes) in pending {
            let Some(result) = self.update_pty_output(id, bytes) else {
                continue;
            };

            let window_result = self.handle_runtime_result(result, false);

            if window_result.control_flow == WindowControlFlow::Exit {
                return window_result;
            }
        }

        WindowEventResult {
            control_flow: self.control_flow,
            frame: None,
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

        self.effect_executor.apply(&mut self.deps, effects);

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

impl WindowEventHandler for GerminalRuntimeHost {
    type Proxy = WinitWindowEventProxy;

    fn set_window_event_proxy(&mut self, proxy: Self::Proxy) {
        self.effect_executor.set_window_event_proxy(proxy.clone());
        self.window_proxy = Some(proxy);
    }

    fn handle_window_event(&mut self, event: WindowEvent) -> WindowEventResult {
        if matches!(event, WindowEvent::RedrawRequested) {
            self.redraw_requested = false;

            let result = self.flush_pty_output();
            if result.control_flow == WindowControlFlow::Exit || self.pending_frame.is_some() {
                return result;
            }
        }

        let request_redraw_on_frame = !matches!(event, WindowEvent::RedrawRequested);
        let result = self.update_window_event(event);
        self.handle_runtime_result(result, request_redraw_on_frame)
    }

    fn handle_pty_output(&mut self, id: GShellId, bytes: Vec<u8>) -> WindowEventResult {
        self.queue_pty_output(id, bytes);
        self.request_redraw();

        WindowEventResult {
            control_flow: self.control_flow,
            frame: None,
        }
    }

    fn take_pending_frame(&mut self) -> Option<RenderFrame> {
        self.pending_frame.take()
    }

    fn should_exit(&self) -> bool {
        self.control_flow == WindowControlFlow::Exit
    }
}
