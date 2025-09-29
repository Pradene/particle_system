struct Particle {
    position: vec3<f32>,
    velocity: vec3<f32>,
}

struct ComputeUniforms {
    delta_time: f32,
}

@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@group(0) @binding(1)
var<uniform> uniforms: ComputeUniforms;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&particles)) {
        return;
    }

    particles[index].position += particles[index].velocity * uniforms.delta_time;
}
