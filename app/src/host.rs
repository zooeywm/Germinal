use std::{
    cell::RefCell,
    collections::VecDeque,
    future::poll_fn,
    rc::Rc,
    task::{Poll, Waker},
};

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
    shared: Rc<RefCell<GerminalRuntimeHostShared>>,
    app: Option<GerminalApp>,
    effect_executor: Option<RuntimeEffectExecutor>,
}

struct GerminalRuntimeHostShared {
    events: VecDeque<WindowEvent>,
    pending_frame: Option<RenderFrame>,
    control_flow: WindowControlFlow,
    window_proxy: Option<WinitWindowEventProxy>,
    waker: Option<Waker>,
}

struct GerminalRuntimeHostRunner {
    shared: Rc<RefCell<GerminalRuntimeHostShared>>,
    app: GerminalApp,
    effect_executor: RuntimeEffectExecutor,
}

impl GerminalRuntimeHost {
    pub fn new() -> Self {
        let app = GerminalApp::new();
        let initial_id = <GerminalApp as AsRef<GShellServiceState>>::as_ref(&app).active();

        Self {
            shared: Rc::new(RefCell::new(GerminalRuntimeHostShared {
                events: VecDeque::new(),
                pending_frame: None,
                control_flow: WindowControlFlow::Continue,
                window_proxy: None,
                waker: None,
            })),
            app: Some(app),
            effect_executor: Some(
                RuntimeEffectExecutor::new(initial_id)
                    .expect("failed to create RuntimeEffectExecutor"),
            ),
        }
    }

    fn enqueue_window_event(&self, event: WindowEvent) {
        let waker = {
            let mut shared = self.shared.borrow_mut();
            shared.events.push_back(event);
            shared.waker.take()
        };

        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl GerminalRuntimeHostRunner {
    async fn run(mut self) {
        while let Some(event) = self.next_event().await {
            let result = self.handle_window_event(event);
            let should_exit = result.control_flow == WindowControlFlow::Exit;
            let (proxy, should_redraw) = self.commit_window_event_result(result);

            if let Some(proxy) = proxy {
                if should_exit {
                    proxy.request_exit();
                } else if should_redraw {
                    proxy.request_redraw();
                }
            }

            if should_exit {
                break;
            }
        }
    }

    async fn next_event(&self) -> Option<WindowEvent> {
        poll_fn(|cx| {
            let mut shared = self.shared.borrow_mut();

            if let Some(event) = shared.events.pop_front() {
                Poll::Ready(Some(event))
            } else if shared.control_flow == WindowControlFlow::Exit {
                Poll::Ready(None)
            } else {
                shared.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        })
        .await
    }

    fn handle_window_event(&mut self, event: WindowEvent) -> WindowEventResult {
        let RuntimeEventResult {
            control_flow,
            frame,
            effects,
        } = self.update_window_event(event);

        self.effect_executor.apply(&mut self.app, effects);

        Self::window_event_result(control_flow, frame)
    }

    fn update_window_event(&mut self, event: WindowEvent) -> RuntimeEventResult {
        let runtime = GerminalRuntime::inj_ref_mut(&mut self.app);
        runtime.handle_window_event(event)
    }

    fn window_event_result(
        control_flow: RuntimeControlFlow,
        frame: Option<RenderFrame>,
    ) -> WindowEventResult {
        match control_flow {
            RuntimeControlFlow::Continue => WindowEventResult {
                control_flow: WindowControlFlow::Continue,
                frame,
            },
            RuntimeControlFlow::Exit => WindowEventResult::exit(),
        }
    }

    fn commit_window_event_result(
        &self,
        result: WindowEventResult,
    ) -> (Option<WinitWindowEventProxy>, bool) {
        let mut shared = self.shared.borrow_mut();

        shared.control_flow = result.control_flow;

        let should_redraw = if let Some(frame) = result.frame {
            shared.pending_frame = Some(frame);
            true
        } else {
            false
        };

        (shared.window_proxy.clone(), should_redraw)
    }
}

impl Default for GerminalRuntimeHost {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowEventHandler for GerminalRuntimeHost {
    type Proxy = WinitWindowEventProxy;

    fn start(&mut self) {
        let Some(app) = self.app.take() else {
            return;
        };
        let effect_executor = self
            .effect_executor
            .take()
            .expect("GerminalRuntimeHost effect executor is missing");

        let runner = GerminalRuntimeHostRunner {
            shared: self.shared.clone(),
            app,
            effect_executor,
        };

        compio::runtime::spawn(runner.run()).detach();
    }

    fn set_window_event_proxy(&mut self, proxy: Self::Proxy) {
        self.shared.borrow_mut().window_proxy = Some(proxy);
    }

    fn handle_window_event(&mut self, event: WindowEvent) -> WindowEventResult {
        self.enqueue_window_event(event);

        WindowEventResult {
            control_flow: self.shared.borrow().control_flow,
            frame: None,
        }
    }

    fn take_pending_frame(&mut self) -> Option<RenderFrame> {
        self.shared.borrow_mut().pending_frame.take()
    }

    fn should_exit(&self) -> bool {
        self.shared.borrow().control_flow == WindowControlFlow::Exit
    }
}
