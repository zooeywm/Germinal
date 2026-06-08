use germinal_application::{
    gshell::GShellServiceState,
    runtime::{GerminalRuntime, RuntimeControlFlow, RuntimeEventResult},
};
use germinal_domain::rendering::RenderFrame;
use germinal_infra::window::WinitWindowEventProxy;
use germinal_ports::window::{
    WindowControlFlow, WindowEvent, WindowEventHandler, WindowEventProxy, WindowEventResult,
};

use crate::{container::GerminalApp, effects::RuntimeEffectExecutor};

pub struct GerminalRuntimeHost {
    app: GerminalApp,
    effect_executor: RuntimeEffectExecutor,
    pending_frame: Option<RenderFrame>,
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
            control_flow: WindowControlFlow::Continue,
            window_proxy: None,
        }
    }

    fn update_window_event(&mut self, event: WindowEvent) -> RuntimeEventResult {
        let runtime = GerminalRuntime::inj_ref_mut(&mut self.app);
        runtime.handle_window_event(event)
    }

    fn handle_runtime_result(&mut self, result: RuntimeEventResult) -> WindowEventResult {
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

                    if let Some(proxy) = &self.window_proxy {
                        proxy.request_redraw();
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
        let result = self.update_window_event(event);
        self.handle_runtime_result(result)
    }

    fn take_pending_frame(&mut self) -> Option<RenderFrame> {
        self.pending_frame.take()
    }

    fn should_exit(&self) -> bool {
        self.control_flow == WindowControlFlow::Exit
    }
}
