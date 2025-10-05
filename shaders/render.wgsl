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
    // UVs only used by quads
    @location(1) uv: vec2<f32>,
}

// Shared constants
var<private> VERTICES: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 1.0, -1.0),
    vec2<f32>( 1.0,  1.0),
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 1.0,  1.0),
    vec2<f32>(-1.0,  1.0),
);

// === POINT RENDERING ===
@vertex
fn vs_point(particle: Particle) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.view_proj * particle.position;
    out.color = vec4<f32>(0.2, 0.6, 1.0, 1.0);
    out.uv = vec2<f32>(0.0, 0.0);
    return out;
}

// === QUAD RENDERING ===
@vertex
fn vs_quad(
    @builtin(vertex_index) vertex_index: u32,
    particle: Particle,
) -> VertexOutput {
    var out: VertexOutput;
    
    let quad_pos = VERTICES[vertex_index];
    let size = 0.05;
    
    let view_right = vec4<f32>(uniforms.view_proj[0][0], uniforms.view_proj[1][0], uniforms.view_proj[2][0], 0.0);
    let view_up = vec4<f32>(uniforms.view_proj[0][1], uniforms.view_proj[1][1], uniforms.view_proj[2][1], 0.0);
    
    let world_pos = particle.position + view_right * quad_pos.x * size + view_up * quad_pos.y * size;
    
    out.clip_position = uniforms.view_proj * world_pos;
    out.color = vec4<f32>(0.2, 0.6, 1.0, 1.0);
    out.uv = quad_pos;
    
    return out;
}

// === FRAGMENT SHADERS ===
@fragment
fn fs_point(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}

@fragment
fn fs_quad(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = length(in.uv);
    let alpha = smoothstep(1.0, 0.0, dist);
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}