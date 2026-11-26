use std::error::Error;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use browser_core::BrowserState;
use egui_glow::{CallbackFn, EguiGlow};

use euclid::{Point2D, Rect, Scale, Size2D};
use servo::{
    Code, DevicePoint, EventLoopWaker, InputEvent, Key, KeyState, KeyboardEvent, Location,
    Modifiers, MouseButton, MouseButtonAction, MouseButtonEvent, MouseLeftViewportEvent,
    MouseMoveEvent, NamedKey, RenderingContext, Servo, ServoBuilder, WebView, WebViewBuilder,
    WebViewDelegate, WheelDelta, WheelEvent, WheelMode, WindowRenderingContext,
    NavigationRequest, OffscreenRenderingContext, PermissionRequest,
};
use url::Url;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
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

    fn notify_url_changed(&self, _: WebView, _: Url) {
        self.0.request_redraw();
    }

    fn notify_load_status_changed(&self, _: WebView, _: servo::LoadStatus) {
        self.0.request_redraw();
    }

    fn notify_animating_changed(&self, _: WebView, _: bool) {
        self.0.request_redraw();
    }

    fn request_permission(&self, _: WebView, request: PermissionRequest) {
        request.deny();
    }

    fn request_navigation(&self, _: WebView, request: NavigationRequest) {
        if request.url.as_str() == "about:blank" || servo_integration::ServoAdapter::accepts(&request.url) {
            request.allow();
        } else {
            request.deny();
        }
    }
}

struct Engine {
    view: WebView,
    servo: Servo,
    gui: EguiGlow,
    context: Rc<OffscreenRenderingContext>,
    parent: Rc<WindowRenderingContext>,
    window: Rc<Window>,
    cursor: Option<DevicePoint>,
    modifiers: Modifiers,
    address: String,
    address_dirty: bool,
    status: String,
    viewport: egui::Rect,
    viewport_scale: f32,
    next_animation: Instant,
}

impl Engine {
    fn start(event_loop: &ActiveEventLoop, waker: Waker, url: Url) -> Result<Self, Box<dyn Error>> {
        let display = event_loop.display_handle()?;
        let window = Rc::new(event_loop.create_window(
            Window::default_attributes()
                .with_title("Lutra — experimental browser")
                .with_inner_size(winit::dpi::LogicalSize::new(1180.0, 760.0))
                .with_min_inner_size(winit::dpi::LogicalSize::new(640.0, 420.0)),
        )?);
        let handle = window.window_handle()?;
        let parent = Rc::new(
            WindowRenderingContext::new(display, handle, window.inner_size())
                .map_err(|error| format!("Rendering context: {error:?}"))?,
        );
        parent.make_current().map_err(|error| format!("Graphics context: {error:?}"))?;
        let context = Rc::new(parent.offscreen_context(window.inner_size()));
        context.make_current().map_err(|error| format!("Offscreen context: {error:?}"))?;
        let gui = EguiGlow::new(event_loop, context.glow_gl_api(), None, None, false);
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(16, 28, 54);
        visuals.selection.bg_fill = egui::Color32::from_rgb(54, 79, 120);
        gui.egui_ctx.set_visuals(visuals);
        let address = url.to_string();
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
            gui,
            context,
            parent,
            window,
            cursor: None,
            modifiers: Modifiers::empty(),
            address,
            address_dirty: false,
            status: String::new(),
            viewport: egui::Rect::NOTHING,
            viewport_scale: 1.0,
            next_animation: Instant::now(),
        })
    }

    fn input(&self, input: InputEvent) {
        self.view.notify_input_event(input);
    }

    fn draw(&mut self) {
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 || self.context.make_current().is_err() {
            return;
        }

        let Self { gui, context, parent, window, view, address, address_dirty,
            status, viewport, viewport_scale, .. } = self;

        if !*address_dirty {
            if let Some(url) = view.url() {
                if *address != url.as_str() {
                    *address = url.to_string();
                }
            }
        }

        gui.run(window, |ui| {
            let panel = egui::Frame::new()
                .fill(egui::Color32::from_rgb(16, 28, 54))
                .inner_margin(egui::Margin::symmetric(10, 8));

            egui::Panel::top("browser_controls").frame(panel).show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Lutra").strong().color(egui::Color32::from_rgb(255, 197, 64)));
                    if ui.add_enabled(view.can_go_back(), egui::Button::new("‹")).clicked() {
                        *address_dirty = false;
                        view.go_back(1);
                    }
                    if ui.add_enabled(view.can_go_forward(), egui::Button::new("›")).clicked() {
                        *address_dirty = false;
                        view.go_forward(1);
                    }
                    if ui.button("Reload").clicked() {
                        view.reload();
                    }
                    let width = (ui.available_width() - 54.0).max(80.0);
                    let field = ui.add_sized([width, 29.0], egui::TextEdit::singleline(address));
                    if field.changed() {
                        *address_dirty = true;
                    }
                    if ui.input(|input| input.modifiers.command && input.key_pressed(egui::Key::L)) {
                        field.request_focus();
                    }
                    if (field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter))) || ui.button("Go").clicked() {
                        match BrowserState::parse_address(address) {
                            Ok(url) => {
                                *address = url.to_string();
                                *address_dirty = false;
                                status.clear();
                                view.load(url);
                            }
                            Err(error) => *status = error.to_owned(),
                        }
                    }
                    if !field.has_focus() && !field.changed() && !ui.memory(|m| m.has_focus(field.id)) {
                        if let Some(url) = view.url() {
                            if *address == url.as_str() {
                                *address_dirty = false;
                            }
                        }
                    }
                });
                let origin = view.url().and_then(|url| {
                    if servo_integration::ServoAdapter::accepts(&url) {
                        Some(url.origin().ascii_serialization())
                    } else {
                        None
                    }
                });
                ui.label(egui::RichText::new(origin.as_deref().unwrap_or("No page loaded")).small());
                if !status.is_empty() {
                    ui.label(status.as_str());
                }
            });

            let rect = ui.available_rect_before_wrap();
            let scale = ui.ctx().pixels_per_point();
            *viewport = rect;
            *viewport_scale = scale;
            let width = (rect.width() * scale).round().max(1.0) as u32;
            let height = (rect.height() * scale).round().max(1.0) as u32;
            let current = view.size();
            if current.width != width as f32 || current.height != height as f32 {
                view.resize(PhysicalSize::new(width, height));
            }
            view.paint();

            if let Some(composite) = context.render_to_parent_callback() {
                ui.layer_painter(egui::LayerId::background()).add(egui::PaintCallback {
                    rect,
                    callback: Arc::new(CallbackFn::new(move |info, painter| {
                        let pixels = info.viewport_in_pixels();
                        let destination = Rect::new(
                            Point2D::new(pixels.left_px, pixels.from_bottom_px),
                            Size2D::new(pixels.width_px, pixels.height_px),
                        );
                        composite(painter.gl(), destination);
                    })),
                });
            }
        });

        if context.make_current().is_ok() {
            parent.prepare_for_rendering();
            gui.paint(window);
            parent.present();
        }
        if gui.egui_ctx.has_requested_repaint() {
            window.request_redraw();
        }
    }

    fn handle(&mut self, event: WindowEvent) {
        let response = self.gui.on_window_event(&self.window, &event);
        if response.repaint && !matches!(&event, WindowEvent::RedrawRequested) {
            self.window.request_redraw();
        }
        match event {
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::Resized(size) if size.width != 0 && size.height != 0 => {
                self.parent.resize(size);
                self.window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.view.set_hidpi_scale_factor(Scale::new(scale_factor as f32));
                self.window.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let logical = egui::pos2(
                    position.x as f32 / self.viewport_scale,
                    position.y as f32 / self.viewport_scale,
                );
                if self.viewport.contains(logical) {
                    let relative = logical - self.viewport.min;
                    let point = DevicePoint::new(relative.x * self.viewport_scale, relative.y * self.viewport_scale);
                    self.cursor = Some(point);
                    self.input(InputEvent::MouseMove(MouseMoveEvent::new(point.into())));
                } else if self.cursor.take().is_some() {
                    self.input(InputEvent::MouseLeftViewport(MouseLeftViewportEvent::default()));
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.cursor = None;
                self.input(InputEvent::MouseLeftViewport(MouseLeftViewportEvent::default()));
            }
            WindowEvent::MouseInput { state, button, .. } if !response.consumed => {
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
            WindowEvent::MouseWheel { delta, .. } if !response.consumed => {
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
            WindowEvent::Focused(focused) => {
                self.view.set_focused(focused);
                if !focused { self.modifiers = Modifiers::empty(); }
            }
            WindowEvent::KeyboardInput { event, .. } if !response.consumed => {
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

impl Drop for Engine {
    fn drop(&mut self) {
        if self.context.make_current().is_ok() {
            self.gui.destroy();
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

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(engine) = &mut self.engine else { return };
        if !engine.view.animating() || engine.window.is_minimized() == Some(true) {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }
        let now = Instant::now();
        if now >= engine.next_animation {
            engine.servo.spin_event_loop();
            engine.next_animation = now + Duration::from_millis(16);
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(engine.next_animation));
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
    let input = std::env::args().nth(1).unwrap_or_else(|| "about:blank".to_owned());
    let url = Url::parse(&input)?;
    if url.as_str() != "about:blank" && !servo_integration::ServoAdapter::accepts(&url) {
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
