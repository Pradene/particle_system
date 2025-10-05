struct Particle {
    @location(0) position: vec4<f32>,
    @location(1) velocity: vec4<f32>,
    @location(2) mass: f32,
    @location(3) lifetime: f32,
}

struct Uniforms {
    view_proj: mat4x4<f32>,
    camera_position: vec4<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(particle: Particle) -> VertexOutput {
    var out: VertexOutput;
    
    out.clip_position = uniforms.view_proj * particle.position;
    out.color = vec4<f32>(0.2, 0.6, 1.0, 1.0);
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}