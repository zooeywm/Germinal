use std::{cell::RefCell, rc::Rc, sync::Arc, time::Duration};

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
    platform::pump_events::{EventLoopExtPumpEvents, PumpStatus},
    window::{Window, WindowAttributes, WindowId},
};

use crate::renderer::WgpuRendererBackend;

pub struct GerminalWindowApp<Handler> {
    handler: Handler,
    window: Option<Arc<Window>>,
    gpu: Rc<RefCell<Option<WgpuRendererBackend>>>,
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
            gpu: Rc::new(RefCell::new(None)),
            modifiers: KeyModifiers::default(),
        }
    }

    pub async fn run(mut self) {
        let mut event_loop = EventLoop::<GerminalWindowUserEvent>::with_user_event()
            .build()
            .expect("failed to create winit event loop");

        let proxy = event_loop.create_proxy();

        self.handler
            .set_window_event_proxy(WinitWindowEventProxy::new(proxy));
        self.handler.start();

        event_loop.set_control_flow(ControlFlow::Poll);

        loop {
            let status = event_loop.pump_app_events(Some(Duration::ZERO), &mut self);

            if matches!(status, PumpStatus::Exit(_)) || self.handler.should_exit() {
                break;
            }

            compio::runtime::time::sleep(Duration::from_millis(1)).await;
        }
    }

    fn handle_germinal_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: GerminalWindowEvent,
    ) -> WindowEventResult {
        let result = self.handler.handle_window_event(event);

        if result.control_flow == WindowControlFlow::Exit || self.handler.should_exit() {
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

        let gpu = self.gpu.clone();
        compio::runtime::spawn({
            let window = window.clone();
            async move {
                let renderer = WgpuRendererBackend::new(window.clone()).await;
                *gpu.borrow_mut() = Some(renderer);
                window.request_redraw();
            }
        })
        .detach();

        window.request_redraw();

        self.window = Some(window);
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

                if let Some(gpu) = self.gpu.borrow_mut().as_mut() {
                    gpu.resize(size);
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(frame) = self.handler.take_pending_frame() {
                    if let Some(gpu) = self.gpu.borrow_mut().as_mut() {
                        gpu.render(&frame);
                    }

                    return;
                }

                let _ = self
                    .handle_germinal_window_event(event_loop, GerminalWindowEvent::RedrawRequested);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let input = map_keyboard_input(&event, self.modifiers);

                let _ = self.handle_germinal_window_event(
                    event_loop,
                    GerminalWindowEvent::KeyboardInput(input),
                );
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
            GerminalWindowUserEvent::Exit => {
                _event_loop.exit();
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
    Exit,
}

#[derive(Clone)]
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

    fn request_exit(&self) {
        let _ = self.proxy.send_event(GerminalWindowUserEvent::Exit);
    }
}
