use {
    crate::{camera::Camera, renderer::RenderContext},
    std::time::Instant, wgpu::wgt::DrawIndirectArgs,
};

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Particle {
    pub position: [f32; 4],
    pub velocity: [f32; 4],
    pub mass: f32,
    pub lifetime: f32,
    pub age: f32,
    pub padding: [f32; 1],
}

#[allow(unused)]
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
    pub color_start: [f32; 4],
    pub color_end: [f32; 4],
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
    max_particles: u32,

    // Uniforms
    update_uniforms_buffer: wgpu::Buffer,
    render_uniforms_buffer: wgpu::Buffer,
    emit_uniforms_buffer: wgpu::Buffer,
    compact_uniforms_buffer: wgpu::Buffer,

    // Pipelines
    emit_pipeline: wgpu::ComputePipeline,
    emit_bind_group: wgpu::BindGroup,
    compact_pipeline: wgpu::ComputePipeline,
    compact_bind_group: wgpu::BindGroup,
    update_pipeline: wgpu::ComputePipeline,
    update_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    render_bind_group: wgpu::BindGroup,

    position: glam::Vec3,
    emission_mode: ParticleEmissionMode,
    emission_shape: ParticleEmissionShape,
    lifetime: f32,

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

        let compact_uniforms_buffer = Self::create_compact_buffer(device);
        let update_uniforms_buffer = Self::create_update_uniforms_buffer(device);
        let emit_uniforms_buffer = Self::create_emit_uniforms_buffer(device);
        let render_uniforms_buffer = Self::create_render_uniforms_buffer(device);

        let (emit_pipeline, emit_bind_group) = Self::create_emit_pipeline(
            device,
            &particles_buffers,
            &emit_uniforms_buffer,
            &compact_uniforms_buffer,
        );

        let (compact_pipeline, compact_bind_group) =
            Self::create_compact_pipeline(device, &particles_buffers, &compact_uniforms_buffer);

        let (update_pipeline, update_bind_group) =
            Self::create_update_pipeline(device, &particles_buffers, &update_uniforms_buffer);

        let (render_pipeline, render_bind_group) = Self::create_render_pipeline(
            device,
            surface_format,
            &particles_buffers,
            &render_uniforms_buffer,
        );

        Self {
            max_particles,
            compact_uniforms_buffer,
            update_uniforms_buffer,
            emit_uniforms_buffer,
            render_uniforms_buffer,
            emit_pipeline,
            emit_bind_group,
            compact_pipeline,
            compact_bind_group,
            update_pipeline,
            update_bind_group,
            render_pipeline,
            render_bind_group,
            position: info.position,
            emission_mode: info.mode,
            emission_shape: info.shape,
            lifetime: info.lifetime,
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
            size: std::mem::size_of::<DrawIndirectArgs>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::INDIRECT,
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
    ) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
        });

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

        (pipeline, bind_group)
    }

    fn create_compact_pipeline(
        device: &wgpu::Device,
        particles_buffers: &[wgpu::Buffer; 2],
        compact_uniforms_buffer: &wgpu::Buffer,
    ) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compact Bind Group"),
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
                    resource: compact_uniforms_buffer.as_entire_binding(),
                },
            ],
        });

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

        (pipeline, bind_group)
    }

    fn create_update_pipeline(
        device: &wgpu::Device,
        particles_buffers: &[wgpu::Buffer; 2],
        update_uniforms_buffer: &wgpu::Buffer,
    ) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Update Bind Group 0"),
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
        });

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

        (pipeline, bind_group)
    }

    fn create_render_pipeline(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        particles_buffers: &[wgpu::Buffer; 2],
        render_uniforms_buffer: &wgpu::Buffer,
    ) -> (wgpu::RenderPipeline, wgpu::BindGroup) {
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
        });

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

        (render_pipeline, bind_group)
    }

    fn update_particles(&mut self, context: &mut RenderContext, uniforms: UpdateUniforms) {
        context.queue().write_buffer(
            &self.update_uniforms_buffer,
            0,
            bytemuck::cast_slice(&[uniforms]),
        );

        let mut pass = context
            .encoder_mut()
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Update Pass"),
                timestamp_writes: None,
            });

        pass.set_pipeline(&self.update_pipeline);
        pass.set_bind_group(0, &self.update_bind_group, &[]);
        pass.dispatch_workgroups(self.max_particles.div_ceil(256), 1, 1);

        drop(pass);
    }

    fn compact_particles(&mut self, context: &mut RenderContext) {
        let indirect_args = DrawIndirectArgs {
            vertex_count: 1,
            instance_count: 0,
            first_vertex: 0,
            first_instance: 0,
        };
        context.queue().write_buffer(
            &self.compact_uniforms_buffer,
            0,
            bytemuck::cast_slice(&[indirect_args]),
        );

        let mut pass = context
            .encoder_mut()
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compact Pass"),
                timestamp_writes: None,
            });

        pass.set_pipeline(&self.compact_pipeline);
        pass.set_bind_group(0, &self.compact_bind_group, &[]);
        pass.dispatch_workgroups(self.max_particles.div_ceil(256), 1, 1);

        drop(pass);
    }

    fn emit_particles(&mut self, context: &mut RenderContext, actual_emit: u32) {
        let emit_uniforms = EmitUniforms {
            position: self.position.extend(1.0).to_array(),
            count: actual_emit,
            lifetime: self.lifetime,
            shape: self.emission_shape as u32,
            elapsed_time: self.elapsed_time(),
        };

        context.queue().write_buffer(
            &self.emit_uniforms_buffer,
            0,
            bytemuck::cast_slice(&[emit_uniforms]),
        );

        let mut pass = context
            .encoder_mut()
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Emit Pass"),
                timestamp_writes: None,
            });

        pass.set_pipeline(&self.emit_pipeline);
        pass.set_bind_group(0, &self.emit_bind_group, &[]);
        pass.dispatch_workgroups(actual_emit.div_ceil(256), 1, 1);

        drop(pass);
    }

    fn render_particles(&self, context: &mut RenderContext) {
        let view = context.view().clone();
        let depth_view = context.depth_view().clone();
        let mut pass =
            context
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

        pass.set_pipeline(&self.render_pipeline);
        pass.set_bind_group(0, &self.render_bind_group, &[]);
        pass.draw_indirect(&self.compact_uniforms_buffer, 0);
    }

    pub fn update(&mut self, context: &mut RenderContext, uniforms: UpdateUniforms) {
        if self.is_paused() {
            return;
        }

        self.compact_particles(context);
        self.update_particles(context, uniforms);
    }

    pub fn emit(&mut self, context: &mut RenderContext) {
        let particles_to_emit = match self.emission_mode {
            ParticleEmissionMode::Continuous(rate) => (rate as f32 * 0.016) as u32, // Fixed: 0.016 not 0.16
            ParticleEmissionMode::Burst(count) => count,
        };

        if particles_to_emit == 0 {
            return;
        }

        self.emit_particles(context, particles_to_emit);
    }

    pub fn render(&mut self, context: &mut RenderContext, camera: &Camera) {
        let uniforms = RenderUniforms {
            view_proj: camera.view_proj().to_cols_array_2d(),
            color_start: [1.0, 0.0, 0.0, 0.4],
            color_end: [0.0, 0.0, 1.0, 0.4],
        };

        context.queue().write_buffer(
            &self.render_uniforms_buffer,
            0,
            bytemuck::cast_slice(&[uniforms]),
        );

        self.render_particles(context);
    }

    pub fn pause(&mut self) {
        self.state = SimulationState::Paused;
    }

    pub fn resume(&mut self) {
        self.state = SimulationState::Playing;
    }

    pub fn restart(&mut self, queue: &wgpu::Queue) {
        self.start_time = Instant::now();
        self.state = SimulationState::Playing;

        let indirect_args = DrawIndirectArgs {
            vertex_count: 1,
            instance_count: 0,
            first_vertex: 0,
            first_instance: 0,
        };
        queue.write_buffer(
            &self.compact_uniforms_buffer,
            0,
            bytemuck::cast_slice(&[indirect_args]),
        );
    }

    pub fn elapsed_time(&self) -> f32 {
        self.start_time.elapsed().as_secs_f32()
    }

    pub fn is_paused(&self) -> bool {
        self.state == SimulationState::Paused
    }
}
