use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;
use euclid::Scale;
use servo::{InputEvent, Modifiers, RenderingContext, Servo, ServoBuilder, WebView, WebViewBuilder, WheelDelta, WheelEvent, WheelMode, WindowRenderingContext};
use url::Url;
use webrender_api::units::DevicePoint;
use winit::application::ApplicationHandler;
use winit::event::{MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;

#[derive(Debug)]
struct Wake;

#[derive(Clone)]
struct Waker(EventLoopProxy<Wake>);

impl embedder_traits::EventLoopWaker for Waker {
    fn clone_box(&self) -> Box<dyn embedder_traits::EventLoopWaker> { Box::new(self.clone()) }
    fn wake(&self) { let _ = self.0.send_event(Wake); }
}

struct Engine {
    window: Window,
    servo: Servo,
    rendering_context: Rc<WindowRenderingContext>,
    views: RefCell<Vec<WebView>>,
}

impl servo::WebViewDelegate for Engine {
    fn notify_new_frame_ready(&self, _: WebView) { self.window.request_redraw(); }
}

enum App {
    Initial { waker: Waker, url: Url },
    Running(Rc<Engine>),
}

impl ApplicationHandler<Wake> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let Self::Initial { waker, url } = self else { return };
        let display = event_loop.display_handle().expect("Display unavailable");
        let window = event_loop.create_window(Window::default_attributes().with_title("Lutra · Servo Preview"))
            .expect("Window unavailable");
        let handle = window.window_handle().expect("Window handle unavailable");
        let context = Rc::new(WindowRenderingContext::new(display, handle, window.inner_size())
            .expect("Rendering context unavailable"));
        let _ = context.make_current();
        let servo = ServoBuilder::default().event_loop_waker(Box::new(waker.clone())).build();
        servo.setup_logging();
        let engine = Rc::new(Engine { window, servo, rendering_context: context, views: RefCell::new(Vec::new()) });
        let view = WebViewBuilder::new(&engine.servo, engine.rendering_context.clone())
            .url(url.clone())
            .hidpi_scale_factor(Scale::new(engine.window.scale_factor() as f32))
            .delegate(engine.clone())
            .build();
        engine.views.borrow_mut().push(view);
        *self = Self::Running(engine);
    }

    fn user_event(&mut self, _: &ActiveEventLoop, _: Wake) {
        if let Self::Running(engine) = self { engine.servo.spin_event_loop(); }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: winit::window::WindowId, event: WindowEvent) {
        if let Self::Running(engine) = self { engine.servo.spin_event_loop(); }
        match event {
            WindowEvent::CloseRequested => {
                if let Self::Running(engine) = self { engine.views.borrow_mut().clear(); }
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let Self::Running(engine) = self {
                    if let Some(view) = engine.views.borrow().last() { view.paint(); }
                    engine.rendering_context.present();
                }
            }
            WindowEvent::Resized(size) => {
                if let Self::Running(engine) = self {
                    if let Some(view) = engine.views.borrow().last() { view.resize(size); }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let Self::Running(engine) = self {
                    if let Some(view) = engine.views.borrow().last() {
                        let (x, y, mode) = match delta {
                            MouseScrollDelta::LineDelta(x, y) => ((x * 76.0) as f64, (y * 76.0) as f64, WheelMode::DeltaLine),
                            MouseScrollDelta::PixelDelta(p) => (p.x, p.y, WheelMode::DeltaPixel),
                        };
                        view.notify_input_event(InputEvent::Wheel(WheelEvent::new(
                            WheelDelta { x, y, z: 0.0, mode },
                            DevicePoint::default().into(),
                            Modifiers::default(),
                        )));
                    }
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    rustls::crypto::aws_lc_rs::default_provider().install_default()
        .map_err(|_| "Failed to install TLS provider")?;
    let input = std::env::args().nth(1).unwrap_or_else(|| "https://servo.org".into());
    let url = Url::parse(&input)?;
    if !servo_integration::ServoAdapter::accepts(&url) { return Err("Only web URLs without credentials are supported".into()); }
    let event_loop = EventLoop::<Wake>::with_user_event().build()?;
    let waker = Waker(event_loop.create_proxy());
    let mut app = App::Initial { waker, url };
    event_loop.run_app(&mut app)?;
    Ok(())
}
