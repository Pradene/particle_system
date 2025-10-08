use crate::{camera::Camera, renderer::RenderFrame};

#[repr(C)]
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

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ComputeUniforms {
    pub gravity_center: [f32; 4],
    pub gravity_strength: f32,
    pub delta_time: f32,
    pub padding: [f32; 2],
}

#[repr(C)]
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

pub struct ParticleSystemInfo {
    pub shape: ParticleShape,
    pub particles_count: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EmitUniforms {
    pub frame: u32,
    pub count: u32,
    pub padding: [f32; 2],
}

pub struct ParticleSystem {
    particles_buffers: [wgpu::Buffer; 2],
    particles_count: u32,
    max_particles: u32,
    current_buffer: usize,

    // Uniforms
    compute_uniforms_buffer: wgpu::Buffer,
    render_uniforms_buffer: wgpu::Buffer,
    emit_uniforms_buffer: wgpu::Buffer,
    compact_buffer: wgpu::Buffer,

    // Pipelines
    emit_pipeline: wgpu::ComputePipeline,
    emit_bind_groups: [wgpu::BindGroup; 2], // Can write to either buffer
    compact_pipeline: wgpu::ComputePipeline,
    compact_bind_groups: [wgpu::BindGroup; 2],
    update_pipeline: wgpu::ComputePipeline,
    update_bind_groups: [wgpu::BindGroup; 2],
    render_point_pipeline: wgpu::RenderPipeline,
    render_quad_pipeline: wgpu::RenderPipeline,
    render_bind_group: wgpu::BindGroup,

    particles_shape: ParticleShape,
    emit_rate: f32,
    frame: u32,
    accumulated_emit: f32,
}

impl ParticleSystem {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        info: ParticleSystemInfo,
    ) -> Self {
        let max_particles = info.particles_count;
        let buffer_size = (max_particles as usize * std::mem::size_of::<Particle>()) as u64;

        let particles_buffers = [
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
        ];

        let compact_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Counter Buffer"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let compute_uniforms_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Compute Uniform Buffer"),
            size: std::mem::size_of::<ComputeUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let emit_uniforms_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Emit Uniform Buffer"),
            size: std::mem::size_of::<EmitUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let render_uniforms_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Render Uniform Buffer"),
            size: std::mem::size_of::<RenderUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // === EMIT PIPELINE ===
        let emit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Emit Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/emit.wgsl").into()),
        });

        let emit_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        // Emit bind groups for both buffers
        let emit_bind_groups = [
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Emit Bind Group 0"),
                layout: &emit_bind_group_layout,
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
                layout: &emit_bind_group_layout,
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

        let emit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Emit Pipeline Layout"),
            bind_group_layouts: &[&emit_bind_group_layout],
            push_constant_ranges: &[],
        });

        let emit_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Emit Pipeline"),
            layout: Some(&emit_pipeline_layout),
            module: &emit_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // === COMPACT PIPELINE ===
        let compact_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compact Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/compact.wgsl").into()),
        });

        let compact_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let compact_bind_groups = [
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Compact Bind Group 0->1"),
                layout: &compact_bind_group_layout,
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
                layout: &compact_bind_group_layout,
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

        let compact_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compact Pipeline Layout"),
                bind_group_layouts: &[&compact_bind_group_layout],
                push_constant_ranges: &[],
            });

        let compact_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compact Pipeline"),
            layout: Some(&compact_pipeline_layout),
            module: &compact_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // === UPDATE PIPELINE ===
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/update.wgsl").into()),
        });

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let update_bind_groups = [
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Update Bind Group 0"),
                layout: &compute_bind_group_layout,
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
                        resource: compute_uniforms_buffer.as_entire_binding(),
                    },
                ],
            }),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Update Bind Group 1"),
                layout: &compute_bind_group_layout,
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
                        resource: compute_uniforms_buffer.as_entire_binding(),
                    },
                ],
            }),
        ];

        let update_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });

        let update_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&update_pipeline_layout),
            module: &compute_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // === RENDER PIPELINES ===
        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Render Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/render.wgsl").into()),
        });

        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group"),
            layout: &render_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: render_uniforms_buffer.as_entire_binding(),
            }],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&render_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_point_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Point Render Pipeline"),
                layout: Some(&render_pipeline_layout),
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
            layout: Some(&render_pipeline_layout),
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

        Self {
            particles_buffers,
            particles_count: 0,
            max_particles,
            current_buffer: 0,
            compact_buffer,
            compute_uniforms_buffer,
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
            emit_rate: 65536.0,
            frame: 0,
            accumulated_emit: 0.0,
        }
    }

    pub fn update(
        &mut self,
        queue: &wgpu::Queue,
        frame: &mut RenderFrame,
        uniforms: ComputeUniforms,
    ) {
        let dt = uniforms.delta_time;

        self.frame += 1;
        self.accumulated_emit += self.emit_rate * dt;

        // === STEP 1: Update existing particles ===
        queue.write_buffer(
            &self.compute_uniforms_buffer,
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
        compute_pass.dispatch_workgroups(self.max_particles.div_ceil(64), 1, 1);
        drop(compute_pass);

        self.current_buffer = 1 - self.current_buffer;

        // === STEP 2: Compact (remove dead particles) ===
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
        compute_pass.dispatch_workgroups(self.max_particles.div_ceil(64), 1, 1);
        drop(compute_pass);

        self.current_buffer = 1 - self.current_buffer;

        // === STEP 3: Emit new particles ===
        let particles_to_emit = self.accumulated_emit.floor() as u32;
        if particles_to_emit > 0 {
            let estimated_alive = self.particles_count.saturating_sub(
                (particles_to_emit as f32 * 0.1) as u32,
            );

            let space_available = self.max_particles.saturating_sub(estimated_alive);
            let actual_emit = particles_to_emit.min(space_available);

            if actual_emit > 0 {
                let emit_uniforms = EmitUniforms {
                    frame: self.frame,
                    count: actual_emit,
                    padding: [0.0; 2],
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
                compute_pass.dispatch_workgroups(actual_emit.div_ceil(64), 1, 1);
                drop(compute_pass);

                self.accumulated_emit -= actual_emit as f32;
                self.particles_count = (estimated_alive + actual_emit).min(self.max_particles);
            }
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
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
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
        drop(render_pass);
    }

    pub fn get_shape(&self) -> ParticleShape {
        self.particles_shape
    }

    pub fn set_shape(&mut self, shape: ParticleShape) {
        self.particles_shape = shape;
    }
}
