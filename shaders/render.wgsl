struct Particle {
    @location(0) position: vec2<f32>,
    @location(1) velocity: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec2<f32>,
    @location(1) particle_center: vec2<f32>,
    @location(2) color: vec3<f32>,
}

// Quad vertices
var<private> VERTICES: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 1.0, -1.0),
    vec2<f32>( 1.0,  1.0),
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 1.0,  1.0),
    vec2<f32>(-1.0,  1.0),
);

var<private> PARTICLE_SIZE: f32 = 0.01;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    particle: Particle,
) -> VertexOutput {
    var out: VertexOutput;

    let vertex_pos = VERTICES[vertex_index];
    let world_pos = particle.position + vertex_pos * PARTICLE_SIZE;

    out.clip_position = vec4<f32>(world_pos, 0.0, 1.0);
    out.world_position = world_pos;
    out.particle_center = particle.position;
    out.color = vec3<f32>(0.2, 0.6, 1.0);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
