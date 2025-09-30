struct Particle {
    position: vec3<f32>,
    velocity: vec3<f32>,
}

struct ComputeUniforms {
    delta_time: f32,
}

@group(0) @binding(0) var<storage, read> particles_in: array<Particle>;
@group(0) @binding(1) var<storage, read_write> particles_out: array<Particle>;
@group(0) @binding(2) var<uniform> uniforms: ComputeUniforms;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&particles_in)) {
        return;
    }

    let dt = uniforms.delta_time;

    // Read from input buffer
    let particle = particles_in[index];

    // Update particle
    var new_velocity = particle.velocity;
    var new_position = particle.position + particle.velocity * dt;

    // Write to output buffer
    particles_out[index].position = new_position;
    particles_out[index].velocity = new_velocity;
}
