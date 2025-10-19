struct Particle {
    position: vec4<f32>,
    velocity: vec4<f32>,
    color: vec4<f32>,
    mass: f32,
    lifetime: f32,
}

@group(0) @binding(0) var<storage, read> particles_in: array<Particle>;
@group(0) @binding(1) var<storage, read_write> particles_out: array<Particle>;
@group(0) @binding(2) var<storage, read_write> particle_count: atomic<u32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&particles_in)) {
        return;
    }

    let particle = particles_in[index];
    
    if (particle.lifetime > 0.0) {
        let write_index = atomicAdd(&particle_count, 1u);
        
        if (write_index < arrayLength(&particles_out)) {
            particles_out[write_index] = particle;
        }
    }
}