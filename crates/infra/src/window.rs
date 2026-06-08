use std::sync::Arc;

use germinal_ports::{
    renderer::RendererPort,
    window::{
        WindowControlFlow, WindowEvent as GerminalWindowEvent, WindowEventHandler,
        WindowEventResult, WindowSize,
    },
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

use crate::renderer::WgpuRendererBackend;

pub struct GerminalWindowApp<Handler> {
    handler: Handler,
    window: Option<Arc<Window>>,
    gpu: Option<WgpuRendererBackend>,
}

impl<Handler> GerminalWindowApp<Handler>
where
    Handler: WindowEventHandler,
{
    pub fn new(handler: Handler) -> Self {
        Self {
            handler,
            window: None,
            gpu: None,
        }
    }

    pub fn run(self) {
        let event_loop = EventLoop::new().expect("failed to create winit event loop");
        event_loop.set_control_flow(ControlFlow::Wait);

        let mut app = self;

        event_loop
            .run_app(&mut app)
            .expect("failed to run winit event loop");
    }

    fn handle_germinal_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: GerminalWindowEvent,
    ) -> WindowEventResult {
        let result = self.handler.handle_window_event(event);

        if result.control_flow == WindowControlFlow::Exit {
            event_loop.exit();
        }

        result
    }
}

impl<Handler> ApplicationHandler for GerminalWindowApp<Handler>
where
    Handler: WindowEventHandler,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_title("Germinal")
                        .with_visible(true)
                        .with_inner_size(LogicalSize::new(960.0, 540.0)),
                )
                .expect("failed to create Germinal window"),
        );

        let runtime = compio::runtime::Runtime::new().expect("failed to create compio runtime");

        let gpu = runtime.block_on(WgpuRendererBackend::new(window.clone()));

        window.request_redraw();

        self.window = Some(window);
        self.gpu = Some(gpu);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                let _ = self
                    .handle_germinal_window_event(event_loop, GerminalWindowEvent::CloseRequested);
            }
            WindowEvent::Resized(size) => {
                let _ = self.handle_germinal_window_event(
                    event_loop,
                    GerminalWindowEvent::Resized(WindowSize {
                        width: size.width,
                        height: size.height,
                    }),
                );

                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size);
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let result = self
                    .handle_germinal_window_event(event_loop, GerminalWindowEvent::RedrawRequested);

                if let (Some(gpu), Some(frame)) = (&mut self.gpu, result.frame) {
                    gpu.render(&frame);
                }
            }
            _ => {}
        }
    }
}
