use germinal::container::GerminalApp;
use germinal_application::runtime::{GerminalRuntime, RuntimeControlFlow};
use germinal_infra::window::GerminalWindowApp;
use germinal_ports::window::{
    WindowControlFlow, WindowEvent, WindowEventHandler, WindowEventResult,
};

struct GerminalWindowHandler {
    app: GerminalApp,
}

impl GerminalWindowHandler {
    fn new() -> Self {
        Self {
            app: GerminalApp::new().expect("failed to create GerminalApp"),
        }
    }
}

impl WindowEventHandler for GerminalWindowHandler {
    fn handle_window_event(&mut self, event: WindowEvent) -> WindowEventResult {
        let runtime = GerminalRuntime::inj_ref_mut(&mut self.app);
        let result = runtime.handle_window_event(event);

        match result.control_flow {
            RuntimeControlFlow::Continue => WindowEventResult {
                control_flow: WindowControlFlow::Continue,
                frame: result.frame,
            },
            RuntimeControlFlow::Exit => WindowEventResult::exit(),
        }
    }
}

#[compio::main]
async fn main() {
    GerminalWindowApp::new(GerminalWindowHandler::new()).run();
}
