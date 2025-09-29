use {
    crate::{compute_pipeline::ComputePipeline, render_pipeline::RenderPipeline},
    std::sync::Arc,
    winit::window::Window,
};

pub struct Renderer {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    compute_pipeline: Option<ComputePipeline>,
    render_pipeline: Option<RenderPipeline>,
    window: Option<Arc<Window>>,
}

impl Renderer {
    pub async fn new() -> Self {
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
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Device"),
                trace: wgpu::Trace::Off,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            })
            .await
            .unwrap();

        Self {
            instance,
            adapter,
            device,
            queue,
            surface: None,
            surface_config: None,
            compute_pipeline: None,
            render_pipeline: None,
            window: None,
        }
    }

    pub fn create_surface(&mut self, window: Arc<Window>) {
        let surface = self.instance.create_surface(window.clone()).unwrap();
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
            .unwrap_or(surface_caps.formats[0]);

        let size = window.clone().inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&self.device, &config);

        self.compute_pipeline = Some(ComputePipeline::new(&self.device, &self.queue));
        self.render_pipeline = Some(RenderPipeline::new(&self.device, surface_format));

        self.window = Some(window);
        self.surface = Some(surface);
        self.surface_config = Some(config);
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if let (Some(surface), Some(config)) = (&self.surface, &mut self.surface_config)
            && new_size.width > 0
            && new_size.height > 0
        {
            config.width = new_size.width;
            config.height = new_size.height;
            surface.configure(&self.device, config);
        }
    }

    pub fn update(&self, delta_time: f32) -> Result<(), wgpu::SurfaceError> {
        if let (Some(surface), Some(compute_pipeline), Some(render_pipeline)) =
            (&self.surface, &self.compute_pipeline, &self.render_pipeline)
        {
            compute_pipeline.update(&self.device, &self.queue, delta_time);

            let output = surface.get_current_texture()?;
            let view = output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

            render_pipeline.render(
                &mut encoder,
                &view,
                compute_pipeline.particle_buffer(),
                compute_pipeline.particles_count(),
            );

            self.queue.submit(std::iter::once(encoder.finish()));
            output.present();
        }
        Ok(())
    }
}
