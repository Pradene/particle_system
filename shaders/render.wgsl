struct RenderUniforms {
    view_proj: mat4x4<f32>,
    color: vec4<f32>,
}

struct Particle {
    position: vec4<f32>,
    velocity: vec4<f32>,
    mass: f32,
    lifetime: f32,
}

@group(0) @binding(0) var<uniform> uniforms: RenderUniforms;
@group(0) @binding(1) var<storage, read> particles: array<Particle>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(@builtin(instance_index) instance_index: u32) -> VertexOutput {
    let particle = particles[instance_index];

    var out: VertexOutput;
    
    out.clip_position = uniforms.view_proj * particle.position;
    out.color = uniforms.color;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}