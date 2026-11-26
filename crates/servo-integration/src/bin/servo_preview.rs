use std::error::Error;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use browser_core::{native_tabs::{NativeTabs, MAX_TABS}, BrowserState};
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
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{ElementState, MouseButton as WinitButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key as WinitKey, NamedKey as WinitNamedKey};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::{Icon, Window, WindowId};

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

    fn notify_history_changed(&self, _: WebView, _: Vec<Url>, _: usize) {
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
        if servo_integration::ServoAdapter::accepts(&request.url) {
            request.allow();
        } else {
            request.deny();
        }
    }
}

enum Action {
    NewTab,
    Select(usize),
    Close(usize),
    Navigate(Url),
    Back,
    Forward,
    Reload,
}

struct Engine {
    tabs: NativeTabs<WebView>,
    servo: Servo,
    gui: EguiGlow,
    context: Rc<OffscreenRenderingContext>,
    parent: Rc<WindowRenderingContext>,
    window: Rc<Window>,
    delegate: Rc<Delegate>,
    cursor: Option<DevicePoint>,
    modifiers: Modifiers,
    address: String,
    address_dirty: bool,
    focus_address: bool,
    page_focus: bool,
    window_focus: bool,
    status: String,
    viewport: egui::Rect,
    viewport_scale: f32,
    next_animation: Instant,
}

fn window_icon() -> Option<Icon> {
    let image = image::load_from_memory(include_bytes!("../../../../assets/lutra.png")).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).ok()
}

impl Engine {
    fn start(event_loop: &ActiveEventLoop, waker: Waker, initial: Option<Url>) -> Result<Self, Box<dyn Error>> {
        let display = event_loop.display_handle()?;
        let window = Rc::new(event_loop.create_window(
            Window::default_attributes()
                .with_title("Lutra — experimental browser")
                .with_window_icon(window_icon())
                .with_inner_size(LogicalSize::new(1180.0, 760.0))
                .with_min_inner_size(LogicalSize::new(640.0, 420.0)),
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
        let servo = ServoBuilder::default().event_loop_waker(Box::new(waker)).build();
        servo.setup_logging();
        let delegate = Rc::new(Delegate(window.clone()));
        let mut engine = Self {
            tabs: NativeTabs::default(),
            servo,
            gui,
            context,
            parent,
            window,
            delegate,
            cursor: None,
            modifiers: Modifiers::empty(),
            address: String::new(),
            address_dirty: false,
            focus_address: true,
            page_focus: false,
            window_focus: true,
            status: String::new(),
            viewport: egui::Rect::NOTHING,
            viewport_scale: 1.0,
            next_animation: Instant::now(),
        };
        if let Some(url) = initial {
            engine.apply(Action::Navigate(url));
        }
        engine.window.request_redraw();
        Ok(engine)
    }

    fn input(&self, event: InputEvent) {
        if let Some(view) = self.tabs.active() {
            view.notify_input_event(event);
        }
    }

    fn focus_page(&mut self, focused: bool) {
        self.page_focus = focused;
        if let Some(view) = self.tabs.active() {
            view.set_focused(focused && self.window_focus);
        }
    }

    fn switch<F: FnOnce(&mut NativeTabs<WebView>) -> bool>(&mut self, change: F) {
        if let Some(view) = self.tabs.active() {
            view.set_focused(false);
            view.hide();
        }
        let changed = change(&mut self.tabs);
        if let Some(view) = self.tabs.active() {
            view.show();
            view.set_focused(false);
        }
        self.page_focus = false;
        self.cursor = None;
        self.address = self.tabs.active().and_then(WebView::url).map(|url| url.to_string()).unwrap_or_default();
        self.address_dirty = false;
        self.focus_address = self.tabs.active().is_none();
        if changed {
            self.status.clear();
        }
    }

    fn apply(&mut self, action: Action) {
        match action {
            Action::NewTab => {
                if self.tabs.slots().len() >= MAX_TABS {
                    self.status = "Tab limit reached.".into();
                } else {
                    self.switch(|tabs| tabs.open());
                }
            }
            Action::Select(index) => {
                if index != self.tabs.active_index() {
                    self.switch(|tabs| tabs.select(index));
                }
            }
            Action::Close(index) => {
                if index == self.tabs.active_index() {
                    self.switch(|tabs| tabs.close(index));
                } else {
                    self.tabs.close(index);
                }
            }
            Action::Navigate(url) => {
                if !servo_integration::ServoAdapter::accepts(&url) {
                    self.status = "Only HTTPS websites are supported.".into();
                    return;
                }
                self.address = url.to_string();
                self.address_dirty = false;
                self.status.clear();
                self.focus_page(false);
                if let Some(view) = self.tabs.active() {
                    view.load(url);
                } else {
                    let view = WebViewBuilder::new(&self.servo, self.context.clone())
                        .url(url)
                        .hidpi_scale_factor(Scale::new(self.window.scale_factor() as f32))
                        .delegate(self.delegate.clone())
                        .build();
                    view.show();
                    view.set_focused(false);
                    let _ = self.tabs.insert_active(view);
                }
            }
            Action::Back => {
                self.address_dirty = false;
                if let Some(view) = self.tabs.active() {
                    if view.can_go_back() {
                        view.go_back(1);
                    }
                }
            }
            Action::Forward => {
                self.address_dirty = false;
                if let Some(view) = self.tabs.active() {
                    if view.can_go_forward() {
                        view.go_forward(1);
                    }
                }
            }
            Action::Reload => {
                if let Some(view) = self.tabs.active() {
                    view.reload();
                }
            }
        }
        self.window.request_redraw();
    }

    fn draw(&mut self) {
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 || self.context.make_current().is_err() {
            return;
        }

        let mut action = None;
        let Self { tabs, gui, context, parent, window, address, address_dirty, focus_address,
            status, viewport, viewport_scale, .. } = self;
        let view = tabs.active();

        if !*address_dirty {
            if let Some(url) = view.and_then(WebView::url) {
                if address.as_str() != url.as_str() {
                    *address = url.to_string();
                }
            }
        }

        gui.run(window, |ui| {
            egui::Panel::top("tabs")
                .frame(egui::Frame::new().fill(egui::Color32::from_rgb(12, 23, 46)).inner_margin(6))
                .show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.add_enabled(tabs.slots().len() < MAX_TABS, egui::Button::new("+")).clicked() {
                            action = Some(Action::NewTab);
                        }
                        egui::ScrollArea::horizontal().id_salt("tab_strip").show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for (index, slot) in tabs.slots().iter().enumerate() {
                                    let url = slot.as_ref().and_then(WebView::url);
                                    let title = url.as_ref().and_then(Url::host_str).unwrap_or("New tab");
                                    let text: String = title.chars().take(28).collect();
                                    if ui.selectable_label(index == tabs.active_index(), text).clicked() {
                                        action = Some(Action::Select(index));
                                    }
                                    if ui.small_button("×").clicked() {
                                        action = Some(Action::Close(index));
                                    }
                                }
                            });
                        });
                    });
                });

            egui::Panel::top("browser_controls")
                .frame(egui::Frame::new().fill(egui::Color32::from_rgb(16, 28, 54)).inner_margin(8))
                .show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Lutra").strong().color(egui::Color32::from_rgb(255, 197, 64)));
                        if ui.add_enabled(view.is_some_and(WebView::can_go_back), egui::Button::new("‹")).clicked() {
                            action = Some(Action::Back);
                        }
                        if ui.add_enabled(view.is_some_and(WebView::can_go_forward), egui::Button::new("›")).clicked() {
                            action = Some(Action::Forward);
                        }
                        if ui.add_enabled(view.is_some(), egui::Button::new("Reload")).clicked() {
                            action = Some(Action::Reload);
                        }
                        let width = (ui.available_width() - 56.0).max(80.0);
                        let field = ui.add_sized([width, 29.0], egui::TextEdit::singleline(&mut *address).hint_text("Search or enter an address"));
                        if field.changed() {
                            *address_dirty = true;
                        }
                        if *focus_address {
                            field.request_focus();
                            *focus_address = false;
                        }
                        let enter = field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                        let go = ui.button("Go").clicked();
                        if enter || go {
                            match BrowserState::parse_address(address) {
                                Ok(url) => action = Some(Action::Navigate(url)),
                                Err(error) => *status = error.to_owned(),
                            }
                        }
                    });
                    let origin = view.and_then(WebView::url).filter(servo_integration::ServoAdapter::accepts)
                        .map(|url| url.origin().ascii_serialization());
                    ui.label(egui::RichText::new(origin.as_deref().unwrap_or("No page loaded")).small());
                    if !status.is_empty() {
                        ui.label(status.as_str());
                    }
                });

            let rect = ui.available_rect_before_wrap();
            let scale = ui.ctx().pixels_per_point();
            *viewport = rect;
            *viewport_scale = scale;

            if let Some(view) = view {
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
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space((rect.height() * 0.2).max(0.0));
                    ui.label(egui::RichText::new("Lutra").size(42.0).color(egui::Color32::from_rgb(255, 197, 64)));
                    ui.label("A browser in development.");
                    ui.label("No page is opened until you enter an address.");
                    ui.add_space(8.0);
                    ui.label("Do not rely on this build for sensitive browsing.");
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
        if let Some(action) = action {
            self.apply(action);
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
                self.cursor = None;
                self.window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                for view in self.tabs.slots().iter().flatten() {
                    view.set_hidpi_scale_factor(Scale::new(scale_factor as f32));
                }
                self.cursor = None;
                self.window.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let logical = egui::pos2(position.x as f32 / self.viewport_scale, position.y as f32 / self.viewport_scale);
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
            WindowEvent::MouseInput { state, button, .. } => {
                if state == ElementState::Pressed {
                    self.focus_page(self.cursor.is_some() && self.tabs.active().is_some() && !response.consumed);
                }
                if !response.consumed {
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
            }
            WindowEvent::MouseWheel { delta, .. } if !response.consumed => {
                if let Some(point) = self.cursor {
                    let (x, y, mode) = match delta {
                        MouseScrollDelta::LineDelta(x, y) => (f64::from(x), f64::from(y), WheelMode::DeltaLine),
                        MouseScrollDelta::PixelDelta(p) => (p.x, p.y, WheelMode::DeltaPixel),
                    };
                    self.input(InputEvent::Wheel(WheelEvent::new(WheelDelta { x, y, z: 0.0, mode }, point.into())));
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
                self.window_focus = focused;
                if !focused {
                    self.modifiers = Modifiers::empty();
                    self.focus_page(false);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if self.modifiers.contains(Modifiers::CONTROL) {
                    if let WinitKey::Character(key) = &event.logical_key {
                        let shortcut = match key.to_ascii_lowercase().as_str() {
                            "t" => Some(Action::NewTab),
                            "w" => Some(Action::Close(self.tabs.active_index())),
                            "l" => {
                                if event.state == ElementState::Pressed {
                                    self.focus_page(false);
                                    self.focus_address = true;
                                    self.window.request_redraw();
                                }
                                return;
                            }
                            _ => None,
                        };
                        if let Some(action) = shortcut {
                            if event.state == ElementState::Pressed && !event.repeat {
                                self.apply(action);
                            }
                            return;
                        }
                    }
                }
                if self.page_focus && self.window_focus && !response.consumed {
                    if let Some(key) = translate_key(&event.logical_key) {
                        let state = match event.state {
                            ElementState::Pressed => KeyState::Down,
                            ElementState::Released => KeyState::Up,
                        };
                        self.input(InputEvent::Keyboard(KeyboardEvent::new_without_event(
                            state, key, Code::Unidentified, Location::Standard, self.modifiers, event.repeat, false,
                        )));
                    }
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
    initial: Option<(Waker, Option<Url>)>,
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
        if !engine.tabs.active().is_some_and(WebView::animating) || engine.window.is_minimized() == Some(true) {
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
        if matches!(&event, WindowEvent::CloseRequested) {
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
    let initial = std::env::args().nth(1).map(|input| Url::parse(&input)).transpose()?;
    if initial.as_ref().is_some_and(|url| !servo_integration::ServoAdapter::accepts(url)) {
        return Err("Only HTTPS websites without credentials are supported".into());
    }
    let event_loop = EventLoop::<Wake>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App {
        initial: Some((Waker(event_loop.create_proxy()), initial)),
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
