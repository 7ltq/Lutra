use std::error::Error;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use browser_core::{native_tabs::{NativeTabs, MAX_TABS}, session::SessionDirectory, browser_features::{SessionLinks, wheel_lines}, BrowserState};
use egui_glow::{CallbackFn, EguiGlow};
use euclid::{Point2D, Rect, Scale, Size2D};
use servo::{
    Code, DevicePoint, EventLoopWaker, InputEvent, Key, KeyState, KeyboardEvent, Location,
    Modifiers, MouseButton, MouseButtonAction, MouseButtonEvent, MouseLeftViewportEvent,
    MouseMoveEvent, NamedKey, RenderingContext, Servo, ServoBuilder, WebView, WebViewBuilder,
    WebViewDelegate, WheelDelta, WheelEvent, WheelMode, WindowRenderingContext,
    NavigationRequest, OffscreenRenderingContext, Opts, PermissionRequest, Preferences,
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

    fn notify_page_title_changed(&self, _: WebView, _: Option<String>) {
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
    Home,
    Reopen,
    Bookmark,
    RemoveBookmark(Url),
    ClearHistory,
    Zoom(f32),
    ResetZoom,
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
    first_frame: bool,
    home_search: String,
    links: SessionLinks,
    scroll_speed: f32,
    show_history: bool,
    show_bookmarks: bool,
    show_settings: bool,
}

fn session_options(session: &SessionDirectory) -> Opts {
    Opts {
        config_dir: Some(session.path().to_path_buf()),
        temporary_storage: true,
        ignore_certificate_errors: false,
        ..Opts::default()
    }
}

fn restricted_preferences() -> Preferences {
    Preferences {
        devtools_server_enabled: false,
        dom_webrtc_enabled: false,
        dom_webrtc_transceiver_enabled: false,
        dom_geolocation_enabled: false,
        dom_bluetooth_enabled: false,
        dom_bluetooth_testing_enabled: false,
        dom_gamepad_enabled: false,
        dom_webxr_enabled: false,
        dom_credential_management_enabled: false,
        dom_async_clipboard_enabled: false,
        ..Preferences::default()
    }
}

fn window_icon() -> Option<Icon> {
    let image = image::load_from_memory(include_bytes!("../../../../assets/lutra.png")).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).ok()
}

impl Engine {
    fn start(event_loop: &ActiveEventLoop, waker: Waker, initial: Option<Url>, session: &SessionDirectory) -> Result<Self, Box<dyn Error>> {
        eprintln!("[Lutra] Initializing Servo");
        let servo = ServoBuilder::default()
            .opts(session_options(session))
            .preferences(restricted_preferences())
            .event_loop_waker(Box::new(waker))
            .build();
        servo.setup_logging();
        eprintln!("[Lutra] Servo initialized; creating window");
        let display = event_loop.display_handle()?;
        let window = Rc::new(event_loop.create_window(
            Window::default_attributes()
                .with_title("Lutra")
                .with_visible(false)
                .with_window_icon(window_icon())
                .with_inner_size(LogicalSize::new(1180.0, 760.0))
                .with_min_inner_size(LogicalSize::new(640.0, 420.0)),
        )?);
        let handle = window.window_handle()?;
        eprintln!("[Lutra] Creating graphics context");
        let parent = Rc::new(
            WindowRenderingContext::new(display, handle, window.inner_size())
                .map_err(|error| format!("Rendering context: {error:?}"))?,
        );
        parent.make_current().map_err(|error| format!("Graphics context: {error:?}"))?;
        eprintln!("[Lutra] Creating offscreen framebuffer");
        let context = Rc::new(parent.offscreen_context(window.inner_size()));
        context.make_current().map_err(|error| format!("Offscreen context: {error:?}"))?;
        eprintln!("[Lutra] Initializing interface painter");
        let gui = EguiGlow::new(event_loop, context.glow_gl_api(), None, None, false);
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(24, 25, 29);
        visuals.selection.bg_fill = egui::Color32::from_rgb(56, 87, 127);
        gui.egui_ctx.set_visuals(visuals);

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
            first_frame: false,
            home_search: String::new(),
            links: SessionLinks::default(),
            scroll_speed: 1.0,
            show_history: false,
            show_bookmarks: false,
            show_settings: false,
        };
        if let Some(url) = initial {
            engine.apply(Action::Navigate(url));
        }
        engine.window.set_visible(true);
        eprintln!("[Lutra] Window initialized");
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
        self.home_search.clear();
        if changed {
            self.status.clear();
        }
    }

    fn apply(&mut self, action: Action) {
        match action {
            Action::Home => {
                if self.tabs.active().is_some() {
                    self.apply(Action::NewTab);
                } else {
                    self.focus_address = true;
                }
            }
            Action::Reopen => {
                if self.tabs.slots().len() < MAX_TABS {
                    if let Some(url) = self.links.reopen() {
                        self.apply(Action::NewTab);
                        self.apply(Action::Navigate(url));
                    }
                }
            }
            Action::Bookmark => {
                if let Some(url) = self.tabs.active().and_then(WebView::url) {
                    self.links.toggle_bookmark(url);
                }
            }
            Action::RemoveBookmark(url) => { self.links.remove_bookmark(&url); }
            Action::ClearHistory => { self.links.clear_history(); }
            Action::Zoom(factor) => {
                if let Some(view) = self.tabs.active() {
                    view.set_page_zoom((view.page_zoom() * factor).clamp(0.5, 3.0));
                }
            }
            Action::ResetZoom => {
                if let Some(view) = self.tabs.active() { view.set_page_zoom(1.0); }
            }
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
                if let Some(url) = self.tabs.slots().get(index).and_then(Option::as_ref).and_then(WebView::url) {
                    self.links.record_closed(url);
                }
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
                self.gui.egui_ctx.memory_mut(|memory| {
                    if let Some(id) = memory.focused() { memory.surrender_focus(id); }
                });
                self.focus_page(true);
                if let Some(view) = self.tabs.active() {
                    view.load(url);
                } else {
                    let view = WebViewBuilder::new(&self.servo, self.context.clone())
                        .url(url)
                        .hidpi_scale_factor(Scale::new(self.window.scale_factor() as f32))
                        .delegate(self.delegate.clone())
                        .build();
                    view.show();
                    view.set_focused(self.window_focus);
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

        if let Some(url) = self.tabs.active().and_then(WebView::url) {
            self.links.record_visit(url);
        }
        let mut action = None;
        let Self { tabs, gui, context, parent, window, address, address_dirty, focus_address,
            status, viewport, viewport_scale, links, home_search, scroll_speed, show_history, show_bookmarks, show_settings, .. } = self;
        let view = tabs.active();

        if !*address_dirty {
            if let Some(url) = view.and_then(WebView::url) {
                if address.as_str() != url.as_str() {
                    *address = url.to_string();
                }
            }
        }

        let caption = view.and_then(WebView::page_title).filter(|title| !title.is_empty()).unwrap_or_else(|| "New tab".into());
        window.set_title(&format!("{} — Lutra", caption));
        gui.run(window, |ui| {
            egui::Panel::top("tabs")
                .frame(egui::Frame::new().fill(egui::Color32::from_rgb(18, 19, 22)).inner_margin(6))
                .show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.add_enabled(tabs.slots().len() < MAX_TABS, egui::Button::new("+")).clicked() {
                            action = Some(Action::NewTab);
                        }
                        egui::ScrollArea::horizontal().id_salt("tab_strip").show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for (index, slot) in tabs.slots().iter().enumerate() {
                                    let url = slot.as_ref().and_then(WebView::url);
                                    let title = slot.as_ref().and_then(WebView::page_title).filter(|title| !title.is_empty()).unwrap_or_else(|| url.as_ref().and_then(Url::host_str).unwrap_or("New tab").to_owned());
                                    let text: String = title.chars().take(28).collect();
                                    if ui.selectable_label(index == tabs.active_index(), text).on_hover_text(title).clicked() {
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
                .frame(egui::Frame::new().fill(egui::Color32::from_rgb(29, 30, 35)).inner_margin(8))
                .show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("⌂").on_hover_text("Home").clicked() { action = Some(Action::Home); }
                        if ui.add_enabled(view.is_some_and(WebView::can_go_back), egui::Button::new("‹")).clicked() {
                            action = Some(Action::Back);
                        }
                        if ui.add_enabled(view.is_some_and(WebView::can_go_forward), egui::Button::new("›")).clicked() {
                            action = Some(Action::Forward);
                        }
                        if ui.add_enabled(view.is_some(), egui::Button::new("Reload")).clicked() {
                            action = Some(Action::Reload);
                        }
                        let width = (ui.available_width() - 114.0).max(80.0);
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
                        let bookmarked = view.and_then(WebView::url).is_some_and(|url| links.bookmarks().contains(&url));
                        if ui.add_enabled(view.is_some(), egui::Button::new(if bookmarked { "★" } else { "☆" })).on_hover_text("Bookmark (session only)").clicked() { action = Some(Action::Bookmark); }
                        if ui.button("⋯").on_hover_text("Browser settings").clicked() { *show_settings = !*show_settings; }
                        if enter || go {
                            match BrowserState::parse_address(address) {
                                Ok(url) => action = Some(Action::Navigate(url)),
                                Err(error) => *status = error.to_owned(),
                            }
                        }
                    });
                    let origin = view.and_then(WebView::url).filter(servo_integration::ServoAdapter::accepts)
                        .map(|url| url.origin().ascii_serialization());
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(origin.as_deref().unwrap_or("Private new tab")).small());
                        if let Some(view) = view {
                            if ui.small_button("−").clicked() { action = Some(Action::Zoom(1.0 / 1.1)); }
                            if ui.small_button(format!("{:.0}%", view.page_zoom() * 100.0)).on_hover_text("Reset zoom").clicked() { action = Some(Action::ResetZoom); }
                            if ui.small_button("+").clicked() { action = Some(Action::Zoom(1.1)); }
                        }
                        if ui.small_button("Bookmarks").clicked() { *show_bookmarks = !*show_bookmarks; }
                        if ui.small_button("History").clicked() { *show_history = !*show_history; }
                    });
                    if !status.is_empty() {
                        ui.label(status.as_str());
                    }
                });

            if *show_settings || *show_history || *show_bookmarks {
                egui::Panel::right("browser_sidebar").show_inside(ui, |ui| {
                    if *show_settings {
                        ui.heading("Settings");
                        ui.add(egui::Slider::new(scroll_speed, 0.5..=3.0).text("Scroll speed"));
                        if ui.button("Reset scroll speed").clicked() { *scroll_speed = 1.0; }
                        ui.label("Settings and browsing data are session-only.");
                        if ui.button("Close settings").clicked() { *show_settings = false; }
                    }
                    if *show_bookmarks {
                        ui.heading("Bookmarks");
                        egui::ScrollArea::vertical().id_salt("bookmarks").max_height(220.0).show(ui, |ui| {
                            for url in links.bookmarks().iter().rev() {
                                ui.horizontal(|ui| {
                                    if ui.button(url.host_str().unwrap_or("Website")).on_hover_text(url.as_str()).clicked() { action = Some(Action::Navigate(url.clone())); }
                                    if ui.small_button("×").clicked() { action = Some(Action::RemoveBookmark(url.clone())); }
                                });
                            }
                        });
                        if links.bookmarks().is_empty() { ui.label("No bookmarks in this session."); }
                        if ui.button("Close bookmarks").clicked() { *show_bookmarks = false; }
                    }
                    if *show_history {
                        ui.heading("Session history");
                        if ui.button("Clear list").clicked() { action = Some(Action::ClearHistory); }
                        if ui.button("Reopen closed tab").clicked() { action = Some(Action::Reopen); }
                        egui::ScrollArea::vertical().id_salt("history").max_height(240.0).show(ui, |ui| {
                            for url in links.history().iter().rev() {
                                if ui.button(url.host_str().unwrap_or("Website")).on_hover_text(url.as_str()).clicked() { action = Some(Action::Navigate(url.clone())); }
                            }
                        });
                        if ui.button("Close history").clicked() { *show_history = false; }
                    }
                });
            }
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
                    ui.label(egui::RichText::new("Lutra").size(54.0).strong());
                    ui.label(egui::RichText::new("A quieter way to browse").size(16.0).color(egui::Color32::GRAY));
                    ui.add_space(22.0);
                    let search = ui.add_sized([(rect.width() * 0.65).clamp(160.0, 560.0), 40.0], egui::TextEdit::singleline(home_search).hint_text("Search the web or enter a URL"));
                    if (search.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter))) || ui.button("Search").clicked() {
                        match BrowserState::parse_address(home_search) {
                            Ok(url) => { action = Some(Action::Navigate(url)); home_search.clear(); }
                            Err(error) => *status = error.to_owned(),
                        }
                    }
                    ui.add_space(20.0);
                    ui.horizontal(|ui| {
                        for (name, url) in [("DuckDuckGo", "https://duckduckgo.com/"), ("Wikipedia", "https://www.wikipedia.org/"), ("GitHub", "https://github.com/")] {
                            if ui.button(name).clicked() {
                                if let Ok(url) = Url::parse(url) { action = Some(Action::Navigate(url)); }
                            }
                        }
                    });
                    ui.add_space(28.0);
                    ui.label("Private session · no persistent browsing history");
                    ui.label(egui::RichText::new("Experimental engine — not for sensitive browsing").small().color(egui::Color32::GRAY));
                });
            }
        });

        if context.make_current().is_ok() {
            parent.prepare_for_rendering();
            gui.paint(window);
            parent.present();
            if !self.first_frame {
                self.first_frame = true;
                eprintln!("[Lutra] First interface frame submitted");
            }
        }
        if gui.egui_ctx.has_requested_repaint() {
            window.request_redraw();
        }
        if let Some(action) = action {
            self.apply(action);
        }
    }

    fn handle(&mut self, event: WindowEvent) {
        let page_pointer = self.tabs.active().is_some() && self.cursor.is_some();
        let response = self.gui.on_window_event(&self.window, &event);
        if response.repaint && !matches!(&event, WindowEvent::RedrawRequested) {
            self.window.request_redraw();
        }
        match event {
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::Resized(size) if size.width != 0 && size.height != 0 => {
                if self.parent.size() != size {
                    self.parent.resize(size);
                }
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
                    self.focus_page(page_pointer);
                }
                if page_pointer {
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
            WindowEvent::MouseWheel { delta, .. } if page_pointer => {
                if let Some(point) = self.cursor {
                    let (x, y, mode) = match delta {
                        MouseScrollDelta::LineDelta(x, y) => {
                            let (x, y) = wheel_lines(x, y, self.scroll_speed);
                            (x, y, WheelMode::DeltaLine)
                        }
                        MouseScrollDelta::PixelDelta(p) => (p.x, p.y, WheelMode::DeltaPixel),
                    };
                    if self.modifiers.contains(Modifiers::CONTROL) && y != 0.0 {
                        self.apply(Action::Zoom(if y > 0.0 { 1.1 } else { 1.0 / 1.1 }));
                    } else {
                        self.input(InputEvent::Wheel(WheelEvent::new(WheelDelta { x, y, z: 0.0, mode }, point.into())));
                        self.window.request_redraw();
                    }
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
                if event.state == ElementState::Pressed && !event.repeat {
                    let shortcut = match &event.logical_key {
                        WinitKey::Named(WinitNamedKey::F5) => Some(Action::Reload),
                        WinitKey::Named(WinitNamedKey::ArrowLeft) if self.modifiers.contains(Modifiers::ALT) => Some(Action::Back),
                        WinitKey::Named(WinitNamedKey::ArrowRight) if self.modifiers.contains(Modifiers::ALT) => Some(Action::Forward),
                        WinitKey::Named(WinitNamedKey::Home) if self.modifiers.contains(Modifiers::ALT) => Some(Action::Home),
                        WinitKey::Named(WinitNamedKey::Tab) if self.modifiers.contains(Modifiers::CONTROL) => {
                            let len = self.tabs.slots().len();
                            let offset = if self.modifiers.contains(Modifiers::SHIFT) { len - 1 } else { 1 };
                            Some(Action::Select((self.tabs.active_index() + offset) % len))
                        }
                        _ => None,
                    };
                    if let Some(action) = shortcut { self.apply(action); return; }
                }
                if self.modifiers.contains(Modifiers::CONTROL) {
                    if let WinitKey::Character(key) = &event.logical_key {
                        let shortcut = match key.to_ascii_lowercase().as_str() {
                            "t" if self.modifiers.contains(Modifiers::SHIFT) => Some(Action::Reopen),
                            "t" => Some(Action::NewTab),
                            "r" => Some(Action::Reload),
                            "d" => Some(Action::Bookmark),
                            "+" | "=" => Some(Action::Zoom(1.1)),
                            "-" => Some(Action::Zoom(1.0 / 1.1)),
                            "0" => Some(Action::ResetZoom),
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
                if self.page_focus && self.window_focus {
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
            self.tabs = NativeTabs::default();
            self.servo.spin_event_loop();
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
    session: SessionDirectory,
    initial: Option<(Waker, Option<Url>)>,
    engine: Option<Engine>,
    error: Option<String>,
    smoke: bool,
    smoke_exit_at: Option<Instant>,
}

impl ApplicationHandler<Wake> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some((waker, url)) = self.initial.take() {
            match Engine::start(event_loop, waker, url, &self.session) {
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
        if self.smoke {
            if engine.first_frame {
                let deadline = *self.smoke_exit_at.get_or_insert_with(|| Instant::now() + Duration::from_secs(2));
                if Instant::now() >= deadline {
                    eprintln!("[Lutra] Startup smoke check passed");
                    event_loop.exit();
                    return;
                }
                event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
            } else {
                event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(100)));
                engine.window.request_redraw();
            }
            return;
        }
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

fn check_config() -> Result<(), Box<dyn Error>> {
    let session = SessionDirectory::create()?;
    let path = session.path().to_path_buf();
    let opts = session_options(&session);
    let prefs = restricted_preferences();
    if !opts.temporary_storage || opts.ignore_certificate_errors
        || opts.config_dir.as_deref() != Some(path.as_path())
        || prefs.devtools_server_enabled || prefs.dom_webrtc_enabled
        || prefs.dom_webrtc_transceiver_enabled || prefs.dom_geolocation_enabled
        || prefs.dom_bluetooth_enabled || prefs.dom_bluetooth_testing_enabled
        || prefs.dom_gamepad_enabled || prefs.dom_webxr_enabled
        || prefs.dom_credential_management_enabled || prefs.dom_async_clipboard_enabled
    {
        return Err("Unexpected engine configuration".into());
    }
    session.close()?;
    if path.exists() {
        return Err("Session directory was not removed".into());
    }
    println!("Configuration and session cleanup checks passed.");
    Ok(())
}

#[cfg(target_os = "windows")]
fn check_graphics_files() -> Result<(), Box<dyn Error>> {
    let executable = std::env::current_exe()?;
    let directory = executable.parent().ok_or("Executable directory unavailable")?;
    for name in ["libEGL.dll", "libGLESv2.dll"] {
        if !directory.join(name).is_file() {
            return Err(format!("Required graphics library is missing: {name}. Extract the complete Lutra package.").into());
        }
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn check_graphics_files() -> Result<(), Box<dyn Error>> {
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .map_err(|_| "Could not initialize TLS")?;
    let argument = std::env::args().nth(1);
    if argument.as_deref() == Some("--check-config") {
        return check_config();
    }
    check_graphics_files()?;
    let smoke = argument.as_deref() == Some("--smoke-startup");
    let initial = if smoke { None } else { argument.map(|input| Url::parse(&input)).transpose()? };
    if initial.as_ref().is_some_and(|url| !servo_integration::ServoAdapter::accepts(url)) {
        return Err("Only HTTPS websites without credentials are supported".into());
    }
    let event_loop = EventLoop::<Wake>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App {
        session: SessionDirectory::create()?,
        initial: Some((Waker(event_loop.create_proxy()), initial)),
        engine: None,
        error: None,
        smoke,
        smoke_exit_at: None,
    };
    let run_result = event_loop.run_app(&mut app);
    let smoke_ok = !smoke || app.smoke_exit_at.is_some_and(|deadline| Instant::now() >= deadline);
    drop(app.engine.take());
    let cleanup_result = app.session.close();
    run_result?;
    if let Some(error) = app.error {
        return Err(error.into());
    }
    cleanup_result?;
    if !smoke_ok { return Err("Startup smoke check ended before first frame".into()); }
    Ok(())
}
