use {
    crate::{camera::Camera, renderer::RenderContext},
    std::time::Instant,
};

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Particle {
    pub position: [f32; 4],
    pub velocity: [f32; 4],
    pub color: [f32; 4],
    pub mass: f32,
    pub lifetime: f32,
    pub padding: [f32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParticleEmissionShape {
    Point,
    Sphere,
    Cube,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EmitUniforms {
    pub position: [f32; 4],
    pub count: u32,
    pub shape: u32,
    pub lifetime: f32,
    pub elapsed_time: f32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UpdateUniforms {
    pub gravity_center: [f32; 4],
    pub elapsed_time: f32,
    pub delta_time: f32,
    pub padding: [f32; 2],
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RenderUniforms {
    pub view_proj: [[f32; 4]; 4],
}

#[allow(unused)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParticleEmissionMode {
    Burst(u32),
    Continuous(u32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SimulationState {
    Playing,
    Paused,
}

pub struct ParticleSystemInfo {
    pub position: glam::Vec3,
    pub mode: ParticleEmissionMode,
    pub shape: ParticleEmissionShape,
    pub lifetime: f32,
}

pub struct ParticleSystem {
    particles_count: u32,
    max_particles: u32,
    current_buffer: usize,

    // Uniforms
    update_uniforms_buffer: wgpu::Buffer,
    render_uniforms_buffer: wgpu::Buffer,
    emit_uniforms_buffer: wgpu::Buffer,
    compact_buffer: wgpu::Buffer,

    // Pipelines
    emit_pipeline: wgpu::ComputePipeline,
    emit_bind_groups: [wgpu::BindGroup; 2],
    compact_pipeline: wgpu::ComputePipeline,
    compact_bind_groups: [wgpu::BindGroup; 2],
    update_pipeline: wgpu::ComputePipeline,
    update_bind_groups: [wgpu::BindGroup; 2],
    render_pipeline: wgpu::RenderPipeline,
    render_bind_groups: [wgpu::BindGroup; 2],

    position: glam::Vec3,
    emission_mode: ParticleEmissionMode,
    emission_shape: ParticleEmissionShape,
    lifetime: f32,
    accumulated_emit: u32,

    state: SimulationState,
    start_time: Instant,
}

impl ParticleSystem {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        info: ParticleSystemInfo,
    ) -> Self {
        let max_particles = match info.mode {
            ParticleEmissionMode::Burst(count) => count,
            ParticleEmissionMode::Continuous(rate) => rate * info.lifetime.ceil() as u32,
        };

        let particles_buffers = Self::create_particle_buffers(device, max_particles);
        let compact_buffer = Self::create_compact_buffer(device);
        let update_uniforms_buffer = Self::create_update_uniforms_buffer(device);
        let emit_uniforms_buffer = Self::create_emit_uniforms_buffer(device);
        let render_uniforms_buffer = Self::create_render_uniforms_buffer(device);

        let (emit_pipeline, emit_bind_groups) = Self::create_emit_pipeline(
            device,
            &particles_buffers,
            &emit_uniforms_buffer,
            &compact_buffer,
        );

        let (compact_pipeline, compact_bind_groups) =
            Self::create_compact_pipeline(device, &particles_buffers, &compact_buffer);

        let (update_pipeline, update_bind_groups) =
            Self::create_update_pipeline(device, &particles_buffers, &update_uniforms_buffer);

        let (render_pipeline, render_bind_groups) = Self::create_render_pipeline(
            device,
            surface_format,
            &particles_buffers,
            &render_uniforms_buffer,
        );

        Self {
            particles_count: 0,
            max_particles,
            current_buffer: 0,
            compact_buffer,
            update_uniforms_buffer,
            emit_uniforms_buffer,
            render_uniforms_buffer,
            emit_pipeline,
            emit_bind_groups,
            compact_pipeline,
            compact_bind_groups,
            update_pipeline,
            update_bind_groups,
            render_pipeline,
            render_bind_groups,
            position: info.position,
            emission_mode: info.mode,
            emission_shape: info.shape,
            lifetime: info.lifetime,
            accumulated_emit: 0,
            state: SimulationState::Playing,
            start_time: Instant::now(),
        }
    }

    fn create_particle_buffers(device: &wgpu::Device, max_particles: u32) -> [wgpu::Buffer; 2] {
        let buffer_size = (max_particles as usize * std::mem::size_of::<Particle>()) as u64;

        [
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Particle Buffer 0"),
                size: buffer_size,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Particle Buffer 1"),
                size: buffer_size,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        ]
    }

    fn create_compact_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Counter Buffer"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        })
    }

    fn create_update_uniforms_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Compute Uniform Buffer"),
            size: std::mem::size_of::<UpdateUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn create_emit_uniforms_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Emit Uniform Buffer"),
            size: std::mem::size_of::<EmitUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn create_render_uniforms_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Render Uniform Buffer"),
            size: std::mem::size_of::<RenderUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn create_emit_pipeline(
        device: &wgpu::Device,
        particles_buffers: &[wgpu::Buffer; 2],
        emit_uniforms_buffer: &wgpu::Buffer,
        compact_buffer: &wgpu::Buffer,
    ) -> (wgpu::ComputePipeline, [wgpu::BindGroup; 2]) {
        let emit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Emit Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/emit.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Emit Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_groups = [
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Emit Bind Group 0"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: emit_uniforms_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: particles_buffers[0].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: compact_buffer.as_entire_binding(),
                    },
                ],
            }),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Emit Bind Group 1"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: emit_uniforms_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: particles_buffers[1].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: compact_buffer.as_entire_binding(),
                    },
                ],
            }),
        ];

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Emit Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Emit Pipeline"),
            layout: Some(&pipeline_layout),
            module: &emit_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        (pipeline, bind_groups)
    }

    fn create_compact_pipeline(
        device: &wgpu::Device,
        particles_buffers: &[wgpu::Buffer; 2],
        compact_buffer: &wgpu::Buffer,
    ) -> (wgpu::ComputePipeline, [wgpu::BindGroup; 2]) {
        let compact_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compact Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/compact.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Compact Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_groups = [
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Compact Bind Group 0->1"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: particles_buffers[0].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: particles_buffers[1].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: compact_buffer.as_entire_binding(),
                    },
                ],
            }),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Compact Bind Group 1->0"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: particles_buffers[1].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: particles_buffers[0].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: compact_buffer.as_entire_binding(),
                    },
                ],
            }),
        ];

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compact Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compact Pipeline"),
            layout: Some(&pipeline_layout),
            module: &compact_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        (pipeline, bind_groups)
    }

    fn create_update_pipeline(
        device: &wgpu::Device,
        particles_buffers: &[wgpu::Buffer; 2],
        update_uniforms_buffer: &wgpu::Buffer,
    ) -> (wgpu::ComputePipeline, [wgpu::BindGroup; 2]) {
        let update_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/update.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Compute Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_groups = [
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Update Bind Group 0"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: update_uniforms_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: particles_buffers[0].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: particles_buffers[1].as_entire_binding(),
                    },
                ],
            }),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Update Bind Group 1"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: update_uniforms_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: particles_buffers[1].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: particles_buffers[0].as_entire_binding(),
                    },
                ],
            }),
        ];

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compute Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &update_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        (pipeline, bind_groups)
    }

    fn create_render_pipeline(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        particles_buffers: &[wgpu::Buffer; 2],
        render_uniforms_buffer: &wgpu::Buffer,
    ) -> (wgpu::RenderPipeline, [wgpu::BindGroup; 2]) {
        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Render Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/render.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Render Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_groups = [
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Render Bind Group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: render_uniforms_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: particles_buffers[0].as_entire_binding(),
                    },
                ],
            }),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Render Bind Group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: render_uniforms_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: particles_buffers[1].as_entire_binding(),
                    },
                ],
            }),
        ];

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &render_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::OVER,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::PointList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        (render_pipeline, bind_groups)
    }

    fn update_particles(&mut self, frame: &mut RenderContext, uniforms: UpdateUniforms) {
        frame.queue().write_buffer(
            &self.update_uniforms_buffer,
            0,
            bytemuck::cast_slice(&[uniforms]),
        );

        let mut pass = frame
            .encoder_mut()
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Update Pass"),
                timestamp_writes: None,
            });

        pass.set_pipeline(&self.update_pipeline);
        pass.set_bind_group(0, &self.update_bind_groups[self.current_buffer], &[]);
        pass.dispatch_workgroups(self.max_particles.div_ceil(256), 1, 1);

        drop(pass);
    }

    fn compact_particles(&mut self, frame: &mut RenderContext) {
        frame
            .queue()
            .write_buffer(&self.compact_buffer, 0, bytemuck::cast_slice(&[0u32]));

        let mut pass = frame
            .encoder_mut()
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compact Pass"),
                timestamp_writes: None,
            });

        pass.set_pipeline(&self.compact_pipeline);
        pass.set_bind_group(0, &self.compact_bind_groups[self.current_buffer], &[]);
        pass.dispatch_workgroups(self.max_particles.div_ceil(256), 1, 1);

        drop(pass);
    }

    fn emit_particles(&mut self, frame: &mut RenderContext, actual_emit: u32) {
        let emit_uniforms = EmitUniforms {
            position: self.position.extend(1.0).to_array(),
            count: actual_emit,
            lifetime: self.lifetime,
            shape: self.emission_shape as u32,
            elapsed_time: self.elapsed_time(),
        };

        frame.queue().write_buffer(
            &self.emit_uniforms_buffer,
            0,
            bytemuck::cast_slice(&[emit_uniforms]),
        );

        let mut pass = frame
            .encoder_mut()
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Emit Pass"),
                timestamp_writes: None,
            });

        pass.set_pipeline(&self.emit_pipeline);
        pass.set_bind_group(0, &self.emit_bind_groups[self.current_buffer], &[]);
        pass.dispatch_workgroups(actual_emit.div_ceil(256), 1, 1);

        drop(pass);
    }

    fn render_particles(&self, frame: &mut RenderContext) {
        let view = frame.view().clone();
        let depth_view = frame.depth_view().clone();
        let mut render_pass = frame
            .encoder_mut()
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.render_bind_groups[self.current_buffer], &[]);
        render_pass.draw(0..1, 0..self.particles_count);
    }

    pub fn update(&mut self, frame: &mut RenderContext, uniforms: UpdateUniforms) {
        if self.is_paused() {
            return;
        }

        let delta_time = uniforms.delta_time;

        self.accumulated_emit += match self.emission_mode {
            ParticleEmissionMode::Continuous(rate) => (rate as f32 * delta_time) as u32,
            ParticleEmissionMode::Burst(count) => count,
        };

        self.compact_particles(frame);

        self.update_particles(frame, uniforms);
        self.swap_buffer();
    }

    pub fn emit(&mut self, frame: &mut RenderContext) {
        let particles_to_emit = self.accumulated_emit;
        if particles_to_emit == 0 {
            return;
        }

        // After compaction, particles_count is accurate
        let space_available = self.max_particles.saturating_sub(self.particles_count);
        let actual_emit = particles_to_emit.min(space_available);

        if actual_emit > 0 {
            self.emit_particles(frame, actual_emit);
            self.accumulated_emit -= actual_emit;
            self.particles_count += actual_emit;
        }
    }

    pub fn render(&self, frame: &mut RenderContext, camera: &Camera) {
        let uniforms = RenderUniforms {
            view_proj: camera.view_proj().to_cols_array_2d(),
        };

        frame.queue().write_buffer(
            &self.render_uniforms_buffer,
            0,
            bytemuck::cast_slice(&[uniforms]),
        );

        self.render_particles(frame);
    }

    pub fn swap_buffer(&mut self) {
        self.current_buffer = 1 - self.current_buffer;
    }

    pub fn pause(&mut self) {
        self.state = SimulationState::Paused;
    }

    pub fn resume(&mut self) {
        self.state = SimulationState::Playing;
    }

    pub fn restart(&mut self, queue: &wgpu::Queue) {
        self.particles_count = 0;
        self.accumulated_emit = 0;
        self.start_time = Instant::now();
        self.state = SimulationState::Playing;

        queue.write_buffer(&self.compact_buffer, 0, bytemuck::cast_slice(&[0u32]));
    }

    pub fn elapsed_time(&self) -> f32 {
        self.start_time.elapsed().as_secs_f32()
    }

    pub fn is_paused(&self) -> bool {
        self.state == SimulationState::Paused
    }
}
