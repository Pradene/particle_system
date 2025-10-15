use crate::{camera::Camera, renderer::RenderFrame};

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

impl Particle {
    const ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        0 => Float32x4,
        1 => Float32x4,
        2 => Float32x4,
        3 => Float32,
        4 => Float32,
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Particle>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EmitUniforms {
    pub frame: u32,
    pub count: u32,
    pub lifetime: f32,
    pub padding: [f32; 1],
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UpdateUniforms {
    pub gravity_center: [f32; 4],
    pub gravity_strength: f32,
    pub delta_time: f32,
    pub padding: [f32; 2],
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RenderUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub position: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParticleShape {
    Points,
    Quads,
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
    pub shape: ParticleShape,
    pub emission_mode: ParticleEmissionMode,
    pub lifetime: f32,
}

pub struct ParticleSystem {
    particles_buffers: [wgpu::Buffer; 2],
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
    render_point_pipeline: wgpu::RenderPipeline,
    render_quad_pipeline: wgpu::RenderPipeline,
    render_bind_group: wgpu::BindGroup,

    particles_shape: ParticleShape,
    emission_mode: ParticleEmissionMode,
    lifetime: f32,
    frame: u32,
    accumulated_emit: u32,

    state: SimulationState,
}

impl ParticleSystem {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        info: ParticleSystemInfo,
    ) -> Self {
        let max_particles = match info.emission_mode {
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

        let (render_point_pipeline, render_quad_pipeline, render_bind_group) =
            Self::create_render_pipelines(device, surface_format, &render_uniforms_buffer);

        Self {
            particles_buffers,
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
            particles_shape: info.shape,
            render_point_pipeline,
            render_quad_pipeline,
            render_bind_group,
            emission_mode: info.emission_mode,
            frame: 0,
            lifetime: info.lifetime,
            accumulated_emit: 0,
            state: SimulationState::Playing,
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
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
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
                        resource: particles_buffers[0].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: emit_uniforms_buffer.as_entire_binding(),
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
                        resource: particles_buffers[1].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: emit_uniforms_buffer.as_entire_binding(),
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
                        ty: wgpu::BufferBindingType::Uniform,
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
                        resource: particles_buffers[0].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: particles_buffers[1].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: update_uniforms_buffer.as_entire_binding(),
                    },
                ],
            }),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Update Bind Group 1"),
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
                        resource: update_uniforms_buffer.as_entire_binding(),
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

    fn create_render_pipelines(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        render_uniforms_buffer: &wgpu::Buffer,
    ) -> (wgpu::RenderPipeline, wgpu::RenderPipeline, wgpu::BindGroup) {
        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Render Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/render.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Render Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: render_uniforms_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_point_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Point Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &render_shader,
                    entry_point: Some("vs_point"),
                    buffers: &[Particle::desc()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &render_shader,
                    entry_point: Some("fs_point"),
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

        let render_quad_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Quad Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &render_shader,
                entry_point: Some("vs_quad"),
                buffers: &[Particle::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader,
                entry_point: Some("fs_quad"),
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
                topology: wgpu::PrimitiveTopology::TriangleList,
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

        (render_point_pipeline, render_quad_pipeline, bind_group)
    }

    fn update_particles(
        &mut self,
        queue: &wgpu::Queue,
        frame: &mut RenderFrame,
        uniforms: UpdateUniforms,
    ) {
        queue.write_buffer(
            &self.update_uniforms_buffer,
            0,
            bytemuck::cast_slice(&[uniforms]),
        );

        let mut compute_pass =
            frame
                .encoder_mut()
                .begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Update Pass"),
                    timestamp_writes: None,
                });

        compute_pass.set_pipeline(&self.update_pipeline);
        compute_pass.set_bind_group(0, &self.update_bind_groups[self.current_buffer], &[]);
        compute_pass.dispatch_workgroups(self.max_particles.div_ceil(256), 1, 1);
    }

    fn compact_particles(&mut self, queue: &wgpu::Queue, frame: &mut RenderFrame) {
        queue.write_buffer(&self.compact_buffer, 0, bytemuck::cast_slice(&[0u32]));

        let mut compute_pass =
            frame
                .encoder_mut()
                .begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Compact Pass"),
                    timestamp_writes: None,
                });

        compute_pass.set_pipeline(&self.compact_pipeline);
        compute_pass.set_bind_group(0, &self.compact_bind_groups[self.current_buffer], &[]);
        compute_pass.dispatch_workgroups(self.max_particles.div_ceil(256), 1, 1);
    }

    fn emit_particles(&mut self, queue: &wgpu::Queue, frame: &mut RenderFrame, actual_emit: u32) {
        let emit_uniforms = EmitUniforms {
            frame: self.frame,
            count: actual_emit,
            lifetime: self.lifetime,
            padding: [0.0; 1],
        };

        queue.write_buffer(
            &self.emit_uniforms_buffer,
            0,
            bytemuck::cast_slice(&[emit_uniforms]),
        );

        let mut compute_pass =
            frame
                .encoder_mut()
                .begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Emit Pass"),
                    timestamp_writes: None,
                });

        compute_pass.set_pipeline(&self.emit_pipeline);
        compute_pass.set_bind_group(0, &self.emit_bind_groups[self.current_buffer], &[]);
        compute_pass.dispatch_workgroups(actual_emit.div_ceil(256), 1, 1);
    }

    fn render_particles(&self, frame: &mut RenderFrame) {
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

        match self.particles_shape {
            ParticleShape::Points => {
                render_pass.set_pipeline(&self.render_point_pipeline);
                render_pass.set_bind_group(0, &self.render_bind_group, &[]);
                render_pass
                    .set_vertex_buffer(0, self.particles_buffers[self.current_buffer].slice(..));
                render_pass.draw(0..1, 0..self.particles_count);
            }
            ParticleShape::Quads => {
                render_pass.set_pipeline(&self.render_quad_pipeline);
                render_pass.set_bind_group(0, &self.render_bind_group, &[]);
                render_pass
                    .set_vertex_buffer(0, self.particles_buffers[self.current_buffer].slice(..));
                render_pass.draw(0..6, 0..self.particles_count);
            }
        }
    }

    pub fn update(
        &mut self,
        queue: &wgpu::Queue,
        frame: &mut RenderFrame,
        uniforms: UpdateUniforms,
    ) {
        if self.is_paused() {
            return;
        }

        let delta_time = uniforms.delta_time;

        self.accumulated_emit += match self.emission_mode {
            ParticleEmissionMode::Continuous(rate) => (rate as f32 * delta_time) as u32,
            ParticleEmissionMode::Burst(count) => {
                if self.frame == 0 {
                    count
                } else {
                    0
                }
            }
        };

        // Update particles
        self.update_particles(queue, frame, uniforms);
        self.swap_buffer();

        // Remove dead particles
        self.compact_particles(queue, frame);
        self.swap_buffer();

        self.frame += 1;
    }

    pub fn emit(&mut self, queue: &wgpu::Queue, frame: &mut RenderFrame) {
        let particles_to_emit = self.accumulated_emit;
        if particles_to_emit == 0 {
            return;
        }

        // After compaction, particles_count is accurate
        let space_available = self.max_particles.saturating_sub(self.particles_count);
        let actual_emit = particles_to_emit.min(space_available);

        if actual_emit > 0 {
            self.emit_particles(queue, frame, actual_emit);
            self.accumulated_emit -= actual_emit;
            self.particles_count += actual_emit;
        }
    }

    pub fn render(&self, queue: &wgpu::Queue, frame: &mut RenderFrame, camera: &Camera) {
        let camera_position = camera.position();
        let position =
            glam::Vec4::new(camera_position.x, camera_position.y, camera_position.z, 1.0);

        let uniforms = RenderUniforms {
            view_proj: camera.view_proj().to_cols_array_2d(),
            position: position.to_array(),
        };

        queue.write_buffer(
            &self.render_uniforms_buffer,
            0,
            bytemuck::cast_slice(&[uniforms]),
        );

        self.render_particles(frame);
    }

    pub fn swap_buffer(&mut self) {
        self.current_buffer = 1 - self.current_buffer;
    }

    pub fn toggle_shape(&mut self) {
        self.particles_shape = match self.particles_shape {
            ParticleShape::Points => ParticleShape::Quads,
            ParticleShape::Quads => ParticleShape::Points,
        };
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
        self.frame = 0;
        queue.write_buffer(&self.compact_buffer, 0, bytemuck::cast_slice(&[0u32]));
        self.state = SimulationState::Playing;
    }

    pub fn is_paused(&self) -> bool {
        self.state == SimulationState::Paused
    }
}
