use {std::sync::Arc, winit::window::Window};

pub struct Renderer {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    depth_texture: Option<wgpu::TextureView>,
    window: Option<Arc<Window>>,
}

#[derive(Debug)]
pub enum RendererError {
    AdapterNotFound,
    DeviceRequestFailed,
    SurfaceCreationFailed,
}

impl std::fmt::Display for RendererError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RendererError::AdapterNotFound => write!(f, "Failed to find a suitable GPU adapter"),
            RendererError::DeviceRequestFailed => write!(f, "Failed to request device"),
            RendererError::SurfaceCreationFailed => write!(f, "Failed to create surface"),
        }
    }
}

impl std::error::Error for RendererError {}

impl Renderer {
    pub async fn new() -> Result<Self, RendererError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .map_err(|_| RendererError::AdapterNotFound)?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Device"),
                trace: wgpu::Trace::Off,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            })
            .await
            .map_err(|_| RendererError::DeviceRequestFailed)?;

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            surface: None,
            surface_config: None,
            depth_texture: None,
            window: None,
        })
    }

    pub fn create_surface(&mut self, window: Arc<Window>) -> Result<(), RendererError> {
        let surface = self
            .instance
            .create_surface(window.clone())
            .map_err(|_| RendererError::SurfaceCreationFailed)?;

        let surface_caps = surface.get_capabilities(&self.adapter);

        let present_mode = if surface_caps
            .present_modes
            .contains(&wgpu::PresentMode::Mailbox)
        {
            wgpu::PresentMode::Mailbox
        } else {
            wgpu::PresentMode::Fifo
        };

        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or_else(|| {
                surface_caps
                    .formats
                    .first()
                    .copied()
                    .unwrap_or(wgpu::TextureFormat::Bgra8UnormSrgb)
            });

        let size = window.clone().inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode,
            alpha_mode: surface_caps
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Auto),
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&self.device, &config);

        self.depth_texture = Some(self.create_depth_texture(size.width, size.height));
        self.window = Some(window);
        self.surface = Some(surface);
        self.surface_config = Some(config);

        Ok(())
    }

    fn create_depth_texture(&self, width: u32, height: u32) -> wgpu::TextureView {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if let (Some(surface), Some(config)) = (&self.surface, &mut self.surface_config)
            && new_size.width > 0
            && new_size.height > 0
        {
            config.width = new_size.width;
            config.height = new_size.height;
            surface.configure(&self.device, config);

            self.depth_texture = Some(self.create_depth_texture(new_size.width, new_size.height));
        }
    }

    pub fn begin_frame(&self) -> Result<RenderFrame, wgpu::SurfaceError> {
        if let (Some(surface), Some(depth_view)) = (&self.surface, &self.depth_texture) {
            let output = surface.get_current_texture()?;
            let view = output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            let encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Main Encoder"),
                });

            Ok(RenderFrame {
                output,
                view,
                depth_view: depth_view.clone(),
                encoder,
            })
        } else {
            Err(wgpu::SurfaceError::Lost)
        }
    }

    pub fn end_frame(&self, frame: RenderFrame) {
        self.queue.submit(std::iter::once(frame.encoder.finish()));
        frame.output.present();
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn surface_format(&self) -> Option<wgpu::TextureFormat> {
        self.surface_config.as_ref().map(|c| c.format)
    }
}

pub struct RenderFrame {
    output: wgpu::SurfaceTexture,
    view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    encoder: wgpu::CommandEncoder,
}

impl RenderFrame {
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn depth_view(&self) -> &wgpu::TextureView {
        &self.depth_view
    }

    pub fn encoder_mut(&mut self) -> &mut wgpu::CommandEncoder {
        &mut self.encoder
    }
}
