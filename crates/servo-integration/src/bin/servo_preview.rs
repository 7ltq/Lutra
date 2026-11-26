use std::error::Error;
use std::rc::Rc;

use euclid::Scale;
use servo::{
    Code, DevicePoint, EventLoopWaker, InputEvent, Key, KeyState, KeyboardEvent, Location,
    Modifiers, MouseButton, MouseButtonAction, MouseButtonEvent, MouseLeftViewportEvent,
    MouseMoveEvent, NamedKey, RenderingContext, Servo, ServoBuilder, WebView, WebViewBuilder,
    WebViewDelegate, WheelDelta, WheelEvent, WheelMode, WindowRenderingContext,
};
use url::Url;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton as WinitButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key as WinitKey, NamedKey as WinitNamedKey};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::{Window, WindowId};

#[derive(Debug)]
struct Wake;

#[derive(Clone)]
struct Waker(EventLoopProxy<Wake>);

impl EventLoopWaker for Waker {
    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(self.clone())
    }

    fn wake(&self) {
        let _ = self.0.send_event(Wake);
    }
}

struct Delegate(Rc<Window>);

impl WebViewDelegate for Delegate {
    fn notify_new_frame_ready(&self, _: WebView) {
        self.0.request_redraw();
    }
}

struct Engine {
    view: WebView,
    servo: Servo,
    context: Rc<WindowRenderingContext>,
    window: Rc<Window>,
    cursor: Option<DevicePoint>,
    modifiers: Modifiers,
}

impl Engine {
    fn start(event_loop: &ActiveEventLoop, waker: Waker, url: Url) -> Result<Self, Box<dyn Error>> {
        let display = event_loop.display_handle()?;
        let window = Rc::new(event_loop.create_window(
            Window::default_attributes().with_title("Lutra — experimental renderer"),
        )?);
        let handle = window.window_handle()?;
        let context = Rc::new(
            WindowRenderingContext::new(display, handle, window.inner_size())
                .map_err(|error| format!("Rendering context: {error:?}"))?,
        );
        context.make_current().map_err(|error| format!("Graphics context: {error:?}"))?;
        let servo = ServoBuilder::default().event_loop_waker(Box::new(waker)).build();
        servo.setup_logging();
        let view = WebViewBuilder::new(&servo, context.clone())
            .url(url)
            .hidpi_scale_factor(Scale::new(window.scale_factor() as f32))
            .delegate(Rc::new(Delegate(window.clone())))
            .build();
        window.request_redraw();
        Ok(Self {
            view,
            servo,
            context,
            window,
            cursor: None,
            modifiers: Modifiers::empty(),
        })
    }

    fn input(&self, input: InputEvent) {
        self.view.notify_input_event(input);
    }

    fn handle(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::RedrawRequested => {
                let size = self.window.inner_size();
                if size.width != 0 && size.height != 0 && self.context.make_current().is_ok() {
                    self.view.paint();
                    self.context.present();
                }
            }
            WindowEvent::Resized(size) if size.width != 0 && size.height != 0 => {
                self.view.resize(size);
                self.window.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let point = DevicePoint::new(position.x as f32, position.y as f32);
                self.cursor = Some(point);
                self.input(InputEvent::MouseMove(MouseMoveEvent::new(point.into())));
            }
            WindowEvent::CursorLeft { .. } => {
                self.cursor = None;
                self.input(InputEvent::MouseLeftViewport(MouseLeftViewportEvent::default()));
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(point) = self.cursor {
                    let button = match button {
                        WinitButton::Left => MouseButton::Primary,
                        WinitButton::Middle => MouseButton::Auxiliary,
                        WinitButton::Right => MouseButton::Secondary,
                        WinitButton::Back => MouseButton::Back,
                        WinitButton::Forward => MouseButton::Forward,
                        WinitButton::Other(value) => MouseButton::Other(value),
                    };
                    let action = match state {
                        ElementState::Pressed => MouseButtonAction::Down,
                        ElementState::Released => MouseButtonAction::Up,
                    };
                    self.input(InputEvent::MouseButton(MouseButtonEvent::new(action, button, point.into())));
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(point) = self.cursor {
                    let (x, y, mode) = match delta {
                        MouseScrollDelta::LineDelta(x, y) => (f64::from(x), f64::from(y), WheelMode::DeltaLine),
                        MouseScrollDelta::PixelDelta(p) => (p.x, p.y, WheelMode::DeltaPixel),
                    };
                    self.input(InputEvent::Wheel(WheelEvent::new(
                        WheelDelta { x, y, z: 0.0, mode },
                        point.into(),
                    )));
                }
            }
            WindowEvent::ModifiersChanged(value) => {
                let state = value.state();
                self.modifiers = Modifiers::empty();
                self.modifiers.set(Modifiers::SHIFT, state.shift_key());
                self.modifiers.set(Modifiers::CONTROL, state.control_key());
                self.modifiers.set(Modifiers::ALT, state.alt_key());
                self.modifiers.set(Modifiers::META, state.super_key());
            }
            WindowEvent::Focused(false) => {
                self.modifiers = Modifiers::empty();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(key) = translate_key(&event.logical_key) {
                    let state = match event.state {
                        ElementState::Pressed => KeyState::Down,
                        ElementState::Released => KeyState::Up,
                    };
                    self.input(InputEvent::Keyboard(KeyboardEvent::new_without_event(
                        state, key, Code::Unidentified, Location::Standard, self.modifiers,
                        event.repeat, false,
                    )));
                }
            }
            _ => {}
        }
    }
}

fn translate_key(key: &WinitKey) -> Option<Key> {
    let named = match key {
        WinitKey::Character(text) => return Some(Key::Character(text.to_string())),
        WinitKey::Named(key) => match key {
            WinitNamedKey::Enter => NamedKey::Enter,
            WinitNamedKey::Tab => NamedKey::Tab,
            WinitNamedKey::Backspace => NamedKey::Backspace,
            WinitNamedKey::Delete => NamedKey::Delete,
            WinitNamedKey::Escape => NamedKey::Escape,
            WinitNamedKey::ArrowUp => NamedKey::ArrowUp,
            WinitNamedKey::ArrowDown => NamedKey::ArrowDown,
            WinitNamedKey::ArrowLeft => NamedKey::ArrowLeft,
            WinitNamedKey::ArrowRight => NamedKey::ArrowRight,
            WinitNamedKey::Home => NamedKey::Home,
            WinitNamedKey::End => NamedKey::End,
            WinitNamedKey::PageUp => NamedKey::PageUp,
            WinitNamedKey::PageDown => NamedKey::PageDown,
            WinitNamedKey::Space => return Some(Key::Character(" ".into())),
            _ => return None,
        },
        _ => return None,
    };
    Some(Key::Named(named))
}

struct App {
    initial: Option<(Waker, Url)>,
    engine: Option<Engine>,
    error: Option<String>,
}

impl ApplicationHandler<Wake> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some((waker, url)) = self.initial.take() {
            match Engine::start(event_loop, waker, url) {
                Ok(engine) => self.engine = Some(engine),
                Err(error) => {
                    self.error = Some(error.to_string());
                    event_loop.exit();
                }
            }
        }
    }

    fn user_event(&mut self, _: &ActiveEventLoop, _: Wake) {
        if let Some(engine) = &self.engine {
            engine.servo.spin_event_loop();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(engine) = &mut self.engine else { return };
        if engine.window.id() != id {
            return;
        }
        if matches!(event, WindowEvent::CloseRequested) {
            event_loop.exit();
            return;
        }
        engine.handle(event);
        engine.servo.spin_event_loop();
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .map_err(|_| "Could not initialize TLS")?;
    let input = std::env::args().nth(1).ok_or("A URL is required")?;
    let url = Url::parse(&input)?;
    if !servo_integration::ServoAdapter::accepts(&url) {
        return Err("Only HTTP and HTTPS URLs without credentials are supported".into());
    }
    let event_loop = EventLoop::<Wake>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App {
        initial: Some((Waker(event_loop.create_proxy()), url)),
        engine: None,
        error: None,
    };
    event_loop.run_app(&mut app)?;
    drop(app.engine.take());
    if let Some(error) = app.error {
        return Err(error.into());
    }
    Ok(())
}
