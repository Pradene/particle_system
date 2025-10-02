struct CameraUniform {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct Particle {
    @location(0) position: vec3<f32>,
    @location(1) velocity: vec3<f32>,
    @location(2) mass: f32,
    @location(3) lifetime: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) particle_center: vec3<f32>,
    @location(2) color: vec3<f32>,
}

// Quad vertices in local space
var<private> VERTICES: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 1.0, -1.0),
    vec2<f32>( 1.0,  1.0),
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 1.0,  1.0),
    vec2<f32>(-1.0,  1.0),
);

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    particle: Particle,
) -> VertexOutput {
    var out: VertexOutput;

    let particle_size = 0.01;
    let vertex_offset = VERTICES[vertex_index] * particle_size;

    let to_camera = normalize(camera.position - particle.position);
    let right = cross(vec3<f32>(0.0, 1.0, 0.0), to_camera);
    let up = cross(to_camera, right);

    let world_posititon = particle.position + right * vertex_offset.x + up * vertex_offset.y;

    out.clip_position = camera.view_proj * vec4<f32>(world_posititon, 1.0);
    out.world_position = world_posititon;
    out.particle_center = particle.position;
    out.color = vec3<f32>(0.2, 0.6, 1.0);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
