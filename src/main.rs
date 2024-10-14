#[macro_use]
extern crate tracing;
mod backend;

use crate::backend::Backend;
use anyhow::Context;
use std::iter;
use tokio::runtime::Runtime;
use tracing::Level;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use wgpu::{
    Color, CommandEncoderDescriptor, LoadOp, Operations, RenderPassColorAttachment,
    RenderPassDescriptor, StoreOp,
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::filter::Targets::new()
                .with_default(Level::INFO)
                .with_target("wgpu_core::device::resource", Level::WARN),
        )
        .init();

    info!("x3");

    let event_loop = EventLoop::new().context("Starting event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new();

    event_loop.run_app(&mut app).context("Running app")?;

    Ok(())
}

struct App {
    runtime: Runtime,
    backend: Option<Backend>,
}

impl App {
    fn new() -> App {
        App {
            runtime: Runtime::new().unwrap(),
            backend: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        info!("Creating backend...");

        self.backend = Some(
            self.runtime
                .block_on(Backend::new(event_loop, (1280, 720), "X3"))
                .expect("Backend creation error"),
        );
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let backend = match self.backend.as_mut() {
            None => return,
            Some(backend) => backend,
        };

        match event {
            WindowEvent::CloseRequested => {
                info!("x3");
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                backend.resize(size);
            }
            WindowEvent::RedrawRequested => {
                let (texture, size) = backend.get_texture().expect("Getting texture");
                let view = texture.texture.create_view(&Default::default());

                let mut encoder =
                    backend
                        .device
                        .create_command_encoder(&CommandEncoderDescriptor {
                            label: Some("Render Encoder"),
                        });

                {
                    let _render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(Color {
                                    r: 0.0,
                                    g: 0.4,
                                    b: 0.5,
                                    a: 1.0,
                                }),
                                store: StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                }

                backend.queue.submit(iter::once(encoder.finish()));
                texture.present();

                backend.window().request_redraw();
            }
            WindowEvent::CursorMoved { .. } => (),
            _ => {
                info!("{event:?}");
            }
        }
    }
}
