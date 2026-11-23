//! Darklight application shell: window, event loop and GPU surface.
//!
//! The same code runs on Linux (`darklight` binary) and Android (`android_main`,
//! packaged as an APK by cargo-apk).

mod gfx;
mod page;

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crate::gfx::Gfx;
use crate::page::Page;

struct App {
    page: Page,
    window: Option<Arc<Window>>,
    gfx: Option<Gfx>,
}

impl App {
    fn new() -> Self {
        let page = Page::welcome();
        log::info!(
            "welcome page: {} DOM nodes, {} CSS rules",
            page.document.len(),
            page.stylesheet.rules.len()
        );
        Self {
            page,
            window: None,
            gfx: None,
        }
    }
}

impl ApplicationHandler for App {
    // On Android the surface is destroyed on `suspended` and recreated on
    // `resumed`, so GPU state is rebuilt every time we are resumed.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = match &self.window {
            Some(w) => w.clone(),
            None => {
                let attrs = Window::default_attributes().with_title(self.page.title.clone());
                match event_loop.create_window(attrs) {
                    Ok(w) => Arc::new(w),
                    Err(e) => {
                        log::error!("could not create window: {e}");
                        event_loop.exit();
                        return;
                    }
                }
            }
        };
        match pollster::block_on(Gfx::new(window.clone(), event_loop.owned_display_handle())) {
            Ok(gfx) => self.gfx = Some(gfx),
            Err(e) => {
                log::error!("graphics init failed: {e}");
                event_loop.exit();
                return;
            }
        }
        window.request_redraw();
        self.window = Some(window);
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.gfx = None;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gfx) = &mut self.gfx {
                    gfx.resize(size.width, size.height);
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(gfx) = &mut self.gfx {
                    gfx.render();
                }
            }
            _ => {}
        }
    }
}

fn run(event_loop: EventLoop<()>) {
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new();
    if let Err(e) = event_loop.run_app(&mut app) {
        log::error!("event loop error: {e}");
    }
}

#[cfg(not(target_os = "android"))]
pub fn run_desktop() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    match EventLoop::new() {
        Ok(event_loop) => run(event_loop),
        Err(e) => eprintln!("could not start the event loop: {e}"),
    }
}

// SAFETY: `android_main` is the symbol the Android glue (`android-activity`) looks up
// by name when the NativeActivity starts; it must not be mangled, and it is defined once.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("darklight"),
    );
    match EventLoop::builder().with_android_app(app).build() {
        Ok(event_loop) => run(event_loop),
        Err(e) => log::error!("could not start the event loop: {e}"),
    }
}
