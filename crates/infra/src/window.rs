use std::sync::Arc;

use germinal_ports::{
    renderer::RendererPort,
    window::{
        KeyCode, KeyModifiers, KeyState, KeyboardInput as GerminalKeyboardInput, WindowControlFlow,
        WindowEvent as GerminalWindowEvent, WindowEventHandler, WindowEventProxy,
        WindowEventResult, WindowSize,
    },
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{Key, NamedKey},
    window::{Window, WindowAttributes, WindowId},
};

use crate::renderer::WgpuRendererBackend;

pub struct GerminalWindowApp<Handler> {
    handler: Handler,
    window: Option<Arc<Window>>,
    gpu: Option<WgpuRendererBackend>,
    modifiers: KeyModifiers,
}

impl<Handler> GerminalWindowApp<Handler>
where
    Handler: WindowEventHandler<Proxy = WinitWindowEventProxy>,
{
    pub fn new(handler: Handler) -> Self {
        Self {
            handler,
            window: None,
            gpu: None,
            modifiers: KeyModifiers::default(),
        }
    }

    pub fn run(mut self) {
        let event_loop = EventLoop::<GerminalWindowUserEvent>::with_user_event()
            .build()
            .expect("failed to create winit event loop");

        let proxy = event_loop.create_proxy();

        self.handler
            .set_window_event_proxy(WinitWindowEventProxy::new(proxy));

        event_loop.set_control_flow(ControlFlow::Wait);

        event_loop
            .run_app(&mut self)
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

impl<Handler> ApplicationHandler<GerminalWindowUserEvent> for GerminalWindowApp<Handler>
where
    Handler: WindowEventHandler<Proxy = WinitWindowEventProxy>,
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
            WindowEvent::KeyboardInput { event, .. } => {
                let input = map_keyboard_input(&event, self.modifiers);

                let result = self.handle_germinal_window_event(
                    event_loop,
                    GerminalWindowEvent::KeyboardInput(input),
                );

                if let (Some(gpu), Some(frame)) = (&mut self.gpu, result.frame) {
                    gpu.render(&frame);
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let state = modifiers.state();

                self.modifiers = KeyModifiers {
                    ctrl: state.control_key(),
                    alt: state.alt_key(),
                    shift: state.shift_key(),
                    logo: state.super_key(),
                };
            }
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: GerminalWindowUserEvent) {
        match event {
            GerminalWindowUserEvent::RequestRedraw => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            GerminalWindowUserEvent::PtyOutput => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
        }
    }
}

fn map_keyboard_input(event: &KeyEvent, modifiers: KeyModifiers) -> GerminalKeyboardInput {
    let state = match event.state {
        ElementState::Pressed => KeyState::Pressed,
        ElementState::Released => KeyState::Released,
    };

    let key = match &event.logical_key {
        Key::Named(NamedKey::Enter) => KeyCode::Enter,
        Key::Named(NamedKey::Backspace) => KeyCode::Backspace,
        Key::Named(NamedKey::Escape) => KeyCode::Escape,
        Key::Named(NamedKey::ArrowUp) => KeyCode::ArrowUp,
        Key::Named(NamedKey::ArrowDown) => KeyCode::ArrowDown,
        Key::Named(NamedKey::ArrowLeft) => KeyCode::ArrowLeft,
        Key::Named(NamedKey::ArrowRight) => KeyCode::ArrowRight,
        Key::Character(text) => text
            .chars()
            .next()
            .map(KeyCode::Character)
            .unwrap_or(KeyCode::Unknown),
        _ => KeyCode::Unknown,
    };

    GerminalKeyboardInput {
        state,
        key,
        modifiers,
    }
}

#[derive(Debug)]
pub enum GerminalWindowUserEvent {
    RequestRedraw,
    PtyOutput,
}

pub struct WinitWindowEventProxy {
    proxy: EventLoopProxy<GerminalWindowUserEvent>,
}

impl WinitWindowEventProxy {
    fn new(proxy: EventLoopProxy<GerminalWindowUserEvent>) -> Self {
        Self { proxy }
    }
}

impl WindowEventProxy for WinitWindowEventProxy {
    fn request_redraw(&self) {
        let _ = self
            .proxy
            .send_event(GerminalWindowUserEvent::RequestRedraw);
    }

    fn notify_pty_output(&self) {
        let _ = self.proxy.send_event(GerminalWindowUserEvent::PtyOutput);
    }
}
