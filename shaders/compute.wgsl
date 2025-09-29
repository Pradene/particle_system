struct Particle {
    position: vec2<f32>,
    velocity: vec2<f32>,
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

    var bounds: vec2<f32>;

    bounds = vec2<f32>(1.0, 1.0);

    if (particles[index].position.x > bounds.x || particles[index].position.x < -bounds.x) {
        particles[index].velocity.x *= -1.0;
    }
    if (particles[index].position.y > bounds.y || particles[index].position.y < -bounds.y) {
        particles[index].velocity.y *= -1.0;
    }

    particles[index].position.x = clamp(particles[index].position.x, -bounds.x, bounds.x);
    particles[index].position.y = clamp(particles[index].position.y, -bounds.y, bounds.y);
}
