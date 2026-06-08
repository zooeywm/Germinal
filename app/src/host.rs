use germinal_application::runtime::{GerminalRuntime, RuntimeControlFlow};
use germinal_infra::window::WinitWindowEventProxy;
use germinal_ports::window::{
    WindowControlFlow, WindowEvent, WindowEventHandler, WindowEventResult,
};

use crate::{container::GerminalApp, effects::RuntimeEffectExecutor};

pub struct GerminalRuntimeHost {
    app: GerminalApp,
    effect_executor: RuntimeEffectExecutor,
    window_proxy: Option<WinitWindowEventProxy>,
}

impl GerminalRuntimeHost {
    pub fn new() -> Self {
        Self {
            app: GerminalApp::new().expect("failed to create GerminalApp"),
            effect_executor: RuntimeEffectExecutor::new(),
            window_proxy: None,
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
        self.window_proxy = Some(proxy);
    }

    fn handle_window_event(&mut self, event: WindowEvent) -> WindowEventResult {
        let result = {
            let runtime = GerminalRuntime::inj_ref_mut(&mut self.app);
            runtime.handle_window_event(event)
        };

        self.effect_executor.apply(&mut self.app, result.effects);

        match result.control_flow {
            RuntimeControlFlow::Continue => WindowEventResult {
                control_flow: WindowControlFlow::Continue,
                frame: result.frame,
            },
            RuntimeControlFlow::Exit => WindowEventResult::exit(),
        }
    }
}
