use anyhow::{anyhow, Context};
use wgpu::{
    Adapter, Backends, Device, DeviceDescriptor, Features, Instance, InstanceDescriptor,
    PowerPreference, Queue, RequestAdapterOptionsBase, Surface, SurfaceConfiguration, SurfaceError,
    SurfaceTexture, TextureUsages,
};
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::platform::startup_notify::{
    EventLoopExtStartupNotify, WindowAttributesExtStartupNotify,
};
use winit::window::Window;

pub struct Backend {
    instance: Instance,
    size: PhysicalSize<u32>,
    holder: SurfaceHolder,
    adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
    config: SurfaceConfiguration,
}

#[ouroboros::self_referencing]
struct SurfaceHolder {
    window: Window,
    #[borrows(window)]
    #[covariant]
    surface: Surface<'this>,
}

impl Backend {
    pub async fn new(
        event_loop: &ActiveEventLoop,
        size: (u32, u32),
        name: impl Into<String>,
    ) -> anyhow::Result<Backend> {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes()
            .with_title(name)
            .with_inner_size(LogicalSize::new(size.0, size.1))
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

        let window = event_loop
            .create_window(window_attributes)
            .context("Creating window")?;

        let size = window.inner_size();

        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::PRIMARY,
            ..Default::default()
        });

        let holder = SurfaceHolder::try_new(window, |window| {
            instance.create_surface(window).context("Creating surface")
        })?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptionsBase {
                power_preference: PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(holder.borrow_surface()),
            })
            .await
            .context("Requesting adapter")?;

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    required_features: Features::empty(),
                    required_limits: Default::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .context("Requesting device")?;

        let surface_caps = holder.borrow_surface().get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        holder.borrow_surface().configure(&device, &config);

        Ok(Backend {
            instance,
            size,
            holder,
            adapter,
            device,
            queue,
            config,
        })
    }

    pub fn window(&self) -> &Window {
        self.holder.borrow_window()
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.holder
                .borrow_surface()
                .configure(&self.device, &self.config);
        }
    }

    pub fn get_texture(&mut self) -> anyhow::Result<(SurfaceTexture, PhysicalSize<u32>)> {
        let tex = match self.holder.borrow_surface().get_current_texture() {
            Ok(tex) => tex,
            Err(err) => return match err {
                SurfaceError::Outdated | SurfaceError::Lost => self.resize_surface(),
                _ => Err(err).context("Getting current texture"),
            },
        };

        if tex.suboptimal {
            return self.resize_surface();
        }

        Ok((tex, self.size))
    }

    fn resize_surface(&mut self) -> anyhow::Result<(SurfaceTexture, PhysicalSize<u32>)> {
        let size = self.window().inner_size();
        self.size = size;
        self.config.width = size.width;
        self.config.height = size.height;
        self.holder
            .borrow_surface()
            .configure(&self.device, &self.config);
        let texture = self
            .holder
            .borrow_surface()
            .get_current_texture()
            .context("Re-getting current texture")?;

        Ok((texture, size))
    }
}
