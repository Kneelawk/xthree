#[macro_use]
extern crate tracing;

use anyhow::Context;
use std::num::NonZeroU32;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::platform::startup_notify::{
    EventLoopExtStartupNotify, WindowAttributesExtStartupNotify,
};
use winit::window::{Window, WindowId};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    info!("UwU");

    let event_loop = EventLoop::new().context("Starting event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new();

    event_loop.run_app(&mut app).context("Running app")?;

    Ok(())
}

struct App {
    window: Option<WindowState>,
}

impl App {
    fn new() -> App {
        App { window: None }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        info!("Creating window...");

        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes()
            .with_title("foxel-rs")
            .with_inner_size(LogicalSize::new(1280, 720))
            .with_active(true)
            .with_visible(true);

        #[cfg(target_os = "linux")]
        {
            let token = event_loop.read_token_from_env();
            info!("Activation Token: {:?}", &token);
            if let Some(token) = token {
                window_attributes = window_attributes.with_activation_token(token);
            }
        }

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let context = softbuffer::Context::new(window.clone()).unwrap();
        let surface = softbuffer::Surface::new(&context, window.clone()).unwrap();

        self.window = Some(WindowState { window, surface });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let window = match self.window.as_mut() {
            None => return,
            Some(window) => window,
        };

        match event {
            WindowEvent::CloseRequested => {
                info!("Goodbye");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let (width, height) = {
                    let size = window.window.inner_size();
                    (size.width, size.height)
                };

                window
                    .surface
                    .resize(
                        NonZeroU32::new(width).unwrap(),
                        NonZeroU32::new(height).unwrap(),
                    )
                    .unwrap();

                window.surface.buffer_mut().unwrap().fill(0xff006688);

                window.window.pre_present_notify();
                window.surface.buffer_mut().unwrap().present().unwrap();

                window.window.request_redraw();
            }
            WindowEvent::CursorMoved { .. } => (),
            _ => {
                info!("{event:?}");
            }
        }
    }
}

struct WindowState {
    window: Arc<Window>,
    surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
}
